use gcfeeder_core::adapter::{Adapter, Error, Port};
use gcinput::{Input, Rumble};
use tokio::sync::{mpsc, oneshot, watch};
use tokio_stream::StreamExt;
use tokio_util::task::TaskTracker;
use tracing::{debug, error, info, warn};

pub type Rumbles = [Rumble; Port::COUNT];

pub struct Service {
    tx_shutdown: mpsc::UnboundedSender<oneshot::Sender<()>>,
    rx_inputs: Vec<watch::Receiver<Option<Input>>>,
    tx_rumbles: watch::Sender<Rumbles>,
}

impl Service {
    pub async fn stop(self) {
        let (tx, rx) = oneshot::channel();
        self.tx_shutdown.send(tx).expect("sending shutdown signal");
        rx.await.expect("waiting for shutdown");
    }

    pub async fn recv_input(&mut self, port: Port) -> Option<Input> {
        let rx = &mut self.rx_inputs[port.index()];
        rx.changed().await.expect("waiting for input");
        *rx.borrow()
    }

    pub fn set_rumble(&self, port: Port, rumble: Rumble) {
        _ = self.tx_rumbles.send_if_modified(|rumbles| {
            if rumbles[port.index()] != rumble {
                rumbles[port.index()] = rumble;
                true
            } else {
                false
            }
        });
    }
}

pub fn start(task_tracker: &TaskTracker) -> Service {
    let (tx_shutdown, rx_shutdown) = mpsc::unbounded_channel();

    let (tx_inputs, rx_inputs) = {
        let mut txs = Vec::with_capacity(Port::COUNT);
        let mut rxs = Vec::with_capacity(Port::COUNT);

        for _ in 0..Port::COUNT {
            let (tx, rx) = watch::channel(None);
            txs.push(tx);
            rxs.push(rx);
        }

        (txs, rxs)
    };

    let (tx_rumbles, rx_rumbles) = watch::channel(Rumbles::default());

    task_tracker.spawn(run(rx_shutdown, tx_inputs, tx_rumbles.clone(), rx_rumbles));

    Service {
        tx_shutdown,
        rx_inputs,
        tx_rumbles,
    }
}

async fn run(
    mut rx_shutdown: mpsc::UnboundedReceiver<oneshot::Sender<()>>,
    tx_inputs: Vec<watch::Sender<Option<Input>>>,
    tx_rumbles: watch::Sender<Rumbles>,
    mut rx_rumbles: watch::Receiver<Rumbles>,
) {
    let mut adapter: Option<(nusb::DeviceId, Adapter)> = match nusb::list_devices() {
        Ok(mut devices) => loop {
            if let Some(device_info) = devices.next() {
                if let Ok(adapter) = Adapter::try_open(&device_info).await {
                    break Some((device_info.id(), adapter));
                }
            } else {
                break None;
            }
        },
        Err(err) => {
            warn!("Failed to enumerate USB devices: {err}");
            None
        }
    };

    if adapter.is_some() {
        info!("Connected to adapter!");
    } else {
        debug!("Adapter is not connected");
    }

    let Ok(mut usb_watch) = nusb::watch_devices() else {
        error!("Failed to watch USB hotplug events");
        return;
    };

    loop {
        let input_task = async {
            if let Some((_, adapter)) = &adapter {
                adapter.read_inputs().await
            } else {
                std::future::pending().await
            }
        };

        tokio::select! {
            tx = rx_shutdown.recv() => {
                if let Some((_, adapter)) = adapter.take() {
                    let _ = adapter.reset_rumble().await;
                }

                if let Some(tx) = tx {
                    tx.send(()).expect("sending shutdown signal");
                }
                debug!("Adapter task finished");
                break;
            }
            inputs = input_task => {
                match inputs {
                    Ok(inputs) => {
                        for port in Port::all() {
                            _ = tx_inputs[port.index()].send_if_modified(|input| {
                                if *input != inputs[port.index()] {
                                    *input = inputs[port.index()];
                                    true
                                } else {
                                    false
                                }
                            });
                        }
                    }
                    Err(Error::Disconnected) => {
                        adapter = None;

                        for tx in &tx_inputs {
                            tx.send(None).expect("sending input");
                        }

                        info!("Adapter disconnected");
                    }
                    Err(err) => {
                        warn!("Failed to read inputs: {err}");
                    }
                }
            }
            Ok(()) = rx_rumbles.changed() => {
                let rumbles = *rx_rumbles.borrow();
                if let Some((_, a)) = &adapter {
                    match a.write_rumble(rumbles).await {
                        Ok(()) => {}
                        Err(Error::Disconnected) => {
                            adapter = None;

                            for tx in &tx_inputs {
                                tx.send(None).expect("sending input");
                            }

                            info!("Adapter disconnected");
                        }
                        Err(err) => {
                            warn!("Failed to write rumble states: {err}");
                        }
                    }
                }
            }
            Some(event) = usb_watch.next() => {
                match event {
                    nusb::hotplug::HotplugEvent::Connected(device_info) => {
                        if adapter.is_none() {
                            if let Ok(a) = Adapter::try_open(&device_info).await {
                                adapter = Some((device_info.id(), a));

                                // Resume previous rumble state when reconnecting.
                                let _ = tx_rumbles.send_if_modified(|rumbles| {
                                    !rumbles.iter().all(|r| matches!(r, Rumble::Off))
                                });
                            }
                        }
                    }
                    nusb::hotplug::HotplugEvent::Disconnected(device_id) => {
                        if adapter
                            .as_ref()
                            .map(|(id, _)| *id == device_id)
                            .unwrap_or(false)
                        {
                            adapter = None;

                            for tx in &tx_inputs {
                                tx.send(None).expect("sending input");
                            }

                            info!("Adapter disconnected");
                        }
                    }
                }
            }
        }
    }
}

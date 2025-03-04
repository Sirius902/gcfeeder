use gcfeeder_core::adapter::{Adapter, Error, Port};
use gcinput::{Input, Rumble};
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio_stream::StreamExt;
use tokio_util::task::TaskTracker;
use tracing::{error, info, warn};

pub type Rumbles = [Rumble; Port::COUNT];

pub struct Service {
    tx_shutdown: mpsc::UnboundedSender<oneshot::Sender<()>>,
    rx_inputs: Vec<broadcast::Receiver<Option<Input>>>,
    tx_rumbles: mpsc::UnboundedSender<(Rumbles, oneshot::Sender<()>)>,
}

impl Service {
    pub async fn stop(&self) {
        let (tx, rx) = oneshot::channel();
        self.tx_shutdown.send(tx).expect("sending shutdown signal");
        rx.await.expect("waiting for shutdown");
    }

    pub fn subscribe_input(&self, port: Port) -> broadcast::Receiver<Option<Input>> {
        self.rx_inputs[port.index()].resubscribe()
    }

    pub async fn set_rumble(&self, rumbles: Rumbles) {
        let (tx, rx) = oneshot::channel();
        self.tx_rumbles
            .send((rumbles, tx))
            .expect("sending rumble sender");
        rx.await.expect("waiting for rumble")
    }
}

pub fn start(task_tracker: &TaskTracker) -> Service {
    let (tx_shutdown, rx_shutdown) = mpsc::unbounded_channel();

    let (tx_inputs, rx_inputs) = {
        let mut txs = Vec::with_capacity(Port::COUNT);
        let mut rxs = Vec::with_capacity(Port::COUNT);

        for _ in 0..Port::COUNT {
            let (tx, rx) = broadcast::channel(1);
            txs.push(tx);
            rxs.push(rx);
        }

        (txs, rxs)
    };

    let (tx_rumbles, rx_rumbles) = mpsc::unbounded_channel();

    task_tracker.spawn(run(rx_shutdown, tx_inputs, rx_rumbles));

    Service {
        tx_shutdown,
        rx_inputs,
        tx_rumbles,
    }
}

async fn run(
    mut rx_shutdown: mpsc::UnboundedReceiver<oneshot::Sender<()>>,
    tx_inputs: Vec<broadcast::Sender<Option<Input>>>,
    mut rx_rumbles: mpsc::UnboundedReceiver<(Rumbles, oneshot::Sender<()>)>,
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
        info!("Adapter connected");
    } else {
        info!("Adapter is not connected");
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
                info!("Adapter service finished");
                break;
            }
            inputs = input_task => {
                match inputs {
                    Ok(inputs) => {
                        for (i, tx) in tx_inputs.iter().enumerate() {
                            tx.send(inputs[i]).expect("input channels are not closed");
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
            Some((rumbles, tx)) = rx_rumbles.recv() => {
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

                tx.send(()).expect("sending rumble complete signal");
            }
            // FUTURE(Sirius902) Don't just watch for hotplug, also check every second or so to see
            // if a busy adapter became available.
            Some(event) = usb_watch.next() => {
                match event {
                    nusb::hotplug::HotplugEvent::Connected(device_info) => {
                        if adapter.is_none() {
                            if let Ok(a) = Adapter::try_open(&device_info).await {
                                adapter = Some((device_info.id(), a));

                                info!("Adapter connected");
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

use std::sync::Arc;
use std::time::Duration;

use gcfeeder_core::adapter::{Adapter, Error, Port};
use gcinput::{Input, Rumble};
use tokio::sync::{mpsc, oneshot, watch};
use tokio_stream::StreamExt;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use tracing::{debug, error, info, warn};

pub type Rumbles = [Rumble; Port::COUNT];

pub struct Service {
    tx_shutdown: mpsc::UnboundedSender<oneshot::Sender<()>>,
    rx_inputs: Vec<watch::Receiver<Option<Input>>>,
    tx_rumbles: mpsc::Sender<Rumbles>,
}

impl Service {
    pub async fn stop(&self) {
        let (tx, rx) = oneshot::channel();
        self.tx_shutdown.send(tx).expect("sending shutdown signal");
        rx.await.expect("waiting for shutdown");
    }

    pub fn watch_input(&self, port: Port) -> watch::Receiver<Option<Input>> {
        self.rx_inputs[port.index()].clone()
    }

    pub async fn set_rumble(&self, rumbles: Rumbles) {
        self.tx_rumbles.send(rumbles).await.expect("send rumbles");
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

    let (tx_rumbles, rx_rumbles) = mpsc::channel(1);

    task_tracker.spawn(run(rx_shutdown, tx_inputs, rx_rumbles));

    Service {
        tx_shutdown,
        rx_inputs,
        tx_rumbles,
    }
}

async fn run(
    mut rx_shutdown: mpsc::UnboundedReceiver<oneshot::Sender<()>>,
    tx_inputs: Vec<watch::Sender<Option<Input>>>,
    rx_rumbles: mpsc::Receiver<Rumbles>,
) {
    let (tx_adapter, rx_adapter) = watch::channel(None);
    let mut adapter_id: Option<nusb::DeviceId> = None;

    let mut try_connect_interval = tokio::time::interval(Duration::from_secs(1));

    let Ok(mut usb_watch) = nusb::watch_devices() else {
        error!("Failed to watch USB hotplug events");
        return;
    };

    let task_token = CancellationToken::new();

    let tasks = TaskTracker::new();
    tasks.spawn(input_task(
        task_token.clone(),
        rx_adapter.clone(),
        tx_inputs,
    ));
    tasks.spawn(rumble_task(task_token.clone(), rx_adapter, rx_rumbles));
    tasks.close();

    loop {
        tokio::select! {
            tx = rx_shutdown.recv() => {
                debug!("Shutting down adapter tasks...");
                task_token.cancel();
                tasks.wait().await;
                debug!("Adapter tasks finished!");

                if let Some(tx) = tx {
                    tx.send(()).expect("sending shutdown signal");
                }
                info!("Adapter service finished");
                break;
            }
            _ = try_connect_interval.tick() => {
                let adapter_is_none = { tx_adapter.borrow().is_none() };
                if adapter_is_none {
                    if let Some((adapter, device_id)) = try_connect_adapter().await {
                        adapter_id = Some(device_id);
                        tx_adapter.send(Some(Arc::new(adapter))).expect("adapter send");

                        info!("Adapter connected");
                    } else {
                        debug!("Adapter is still disconnected, trying again in {}s", try_connect_interval.period().as_secs());
                    }
                }
            }
            event = usb_watch.next() => {
                let Some(event) = event else { continue; };

                match event {
                    nusb::hotplug::HotplugEvent::Connected(device_info) => {
                        let adapter_is_none = { tx_adapter.borrow().is_none() };
                        if adapter_is_none {
                            if let Ok(adapter) = Adapter::try_open(&device_info).await {
                                adapter_id = Some(device_info.id());
                                tx_adapter.send(Some(Arc::new(adapter))).expect("adapter send");

                                info!("Adapter connected");
                            }
                        }
                    }
                    nusb::hotplug::HotplugEvent::Disconnected(device_id) => {
                        if adapter_id == Some(device_id) {
                            adapter_id = None;
                            tx_adapter.send(None).expect("adapter send");

                            info!("Adapter disconnected");
                        }
                    }
                }
            }
        }
    }
}

async fn try_connect_adapter() -> Option<(Adapter, nusb::DeviceId)> {
    match nusb::list_devices() {
        Ok(mut devices) => loop {
            if let Some(device_info) = devices.next() {
                if let Ok(adapter) = Adapter::try_open(&device_info).await {
                    break Some((adapter, device_info.id()));
                }
            } else {
                break None;
            }
        },
        Err(err) => {
            warn!("Failed to enumerate USB devices: {err}");
            None
        }
    }
}

async fn input_task(
    token: CancellationToken,
    mut rx_adapter: watch::Receiver<Option<Arc<Adapter>>>,
    tx_inputs: Vec<watch::Sender<Option<Input>>>,
) {
    let mut adapter_ref: Option<Arc<Adapter>> = None;

    loop {
        let input_fut = async {
            if let Some(adapter) = &adapter_ref {
                adapter.read_inputs().await
            } else {
                std::future::pending().await
            }
        };

        tokio::select! {
            _ = token.cancelled() => break,
            _ = rx_adapter.changed() => {
                adapter_ref = rx_adapter.borrow_and_update().clone();
            }
            inputs = input_fut => {
                match inputs {
                    Ok(inputs) => {
                        for (i, tx) in tx_inputs.iter().enumerate() {
                            tx.send_if_modified(|input| {
                                if inputs[i] != *input {
                                    *input = inputs[i];
                                    true
                                } else {
                                    false
                                }
                            });
                        }
                    }
                    Err(Error::Disconnected) => {
                        adapter_ref = None;
                    }
                    Err(err) => {
                        warn!("Failed to read inputs: {err}");
                    }
                }
            }
        }
    }
}

async fn rumble_task(
    token: CancellationToken,
    mut rx_adapter: watch::Receiver<Option<Arc<Adapter>>>,
    mut rx_rumbles: mpsc::Receiver<Rumbles>,
) {
    let mut adapter_ref: Option<Arc<Adapter>> = None;

    loop {
        tokio::select! {
            _ = token.cancelled() => {
                if let Some(adapter) = &adapter_ref {
                    let _ = adapter.reset_rumble().await;
                }
                break;
            }
            _ = rx_adapter.changed() => {
                adapter_ref = rx_adapter.borrow_and_update().clone();
            }
            rumbles = rx_rumbles.recv() => {
                let Some(rumbles) = rumbles else { continue; };

                if let Some(adapter) = &adapter_ref {
                    match adapter.write_rumble(rumbles).await {
                        Ok(()) => {}
                        Err(Error::Disconnected) => {
                            adapter_ref = None;
                        }
                        Err(err) => {
                            warn!("Failed to write rumble states: {err}");
                        }
                    }
                }
            }
        }
    }
}

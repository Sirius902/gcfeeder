use std::sync::Arc;
use std::time::Duration;

use gcfeeder_core::adapter::Port;
use gcfeeder_core::driver::{evdev, Driver};
use gcfeeder_core::mapping::{layers, Layer};
use tokio::sync::{mpsc, oneshot};
use tokio_util::task::TaskTracker;
use tracing::{debug, warn};

use super::adapter;

pub struct Service {
    tx_shutdown: mpsc::UnboundedSender<oneshot::Sender<()>>,
}

impl Service {
    pub async fn stop(&self) {
        let (tx, rx) = oneshot::channel();
        self.tx_shutdown.send(tx).expect("sending shutdown signal");
        rx.await.expect("waiting for shutdown");
    }
}

pub fn start(task_tracker: &TaskTracker, adapter_service: Arc<adapter::Service>) -> Service {
    let (tx_shutdown, rx_shutdown) = mpsc::unbounded_channel();

    task_tracker.spawn(run(rx_shutdown, adapter_service));

    Service { tx_shutdown }
}

async fn run(
    mut rx_shutdown: mpsc::UnboundedReceiver<oneshot::Sender<()>>,
    adapter_service: Arc<adapter::Service>,
) {
    // NOTE(Sirius902) This expects `Port::all` to be sorted by `Port::index`.
    let mut rx_inputs = Port::all()
        .iter()
        .map(|port| adapter_service.watch_input(*port))
        .collect::<Vec<_>>();

    // TODO(Sirius902) Read inputs and pass to driver, forward rumble to adapter, handle config
    // updates. Don't hardcode testing stuff.
    let driver: Box<dyn Driver> = Box::new(evdev::Driver::default());
    let mut layers: Vec<Box<dyn Layer>> = vec![Box::new(layers::CenterCalibration::default())];

    // FUTURE(Sirius902) Somehow get this from the adapter's polling rate?
    let mut rumble_interval = tokio::time::interval(Duration::from_millis(8));

    loop {
        tokio::select! {
            tx = rx_shutdown.recv() => {
                if let Some(tx) = tx {
                    tx.send(()).expect("sending shutdown signal");
                }
                debug!("Driver service finished");
                break;
            }
            // TODO(Sirius902) Do more than `Port::One`.
            Ok(()) = rx_inputs[Port::One.index()].changed() => {
                let raw_input = *rx_inputs[Port::One.index()].borrow_and_update();

                let input = layers
                    .iter_mut()
                    .fold(raw_input, |input, layer| layer.apply(input));

                if let Err(err) = driver.feed(&input).await {
                    warn!("Error feeding with {} driver: {err}", driver.name());
                }
            }
            _ = rumble_interval.tick() => {
                adapter_service.set_rumble(Port::One, driver.consume_rumble_state().await);
            }
        }
    }
}

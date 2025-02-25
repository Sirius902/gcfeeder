use std::sync::Arc;

use gcfeeder_core::adapter::Port;
use tokio::sync::{mpsc, oneshot};
use tokio_util::task::TaskTracker;
use tracing::debug;

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

    // TODO(Sirius902) Read inputs and pass to bridge, forward rumble to adapter, handle config
    // updates.

    loop {
        tokio::select! {
            tx = rx_shutdown.recv() => {
                if let Some(tx) = tx {
                    tx.send(()).expect("sending shutdown signal");
                }
                debug!("Feeder service finished");
                break;
            }
            // TODO(Sirius902) Remove.
            Ok(()) = rx_inputs[Port::One.index()].changed() => {
                let input = *rx_inputs[Port::One.index()].borrow_and_update();
                tracing::debug!("Port one input: {input:#?}");
            }
        }
    }
}

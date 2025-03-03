use std::sync::Arc;

use gcfeeder_core::adapter::Port;
use gcfeeder_core::driver::{Driver, DriverType};
use gcfeeder_core::feeder;
use gcfeeder_core::layers::{self, Layer};
use gcinput::Rumble;
use tokio::sync::{mpsc, oneshot};
use tokio_util::task::TaskTracker;
use tracing::{error, info, warn};

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
        .map(|port| adapter_service.subscribe_input(*port))
        .collect::<Vec<_>>();

    // TODO(Sirius902) Read inputs and pass to driver, forward rumble to adapter, handle config
    // updates. Don't hardcode testing stuff.
    let driver: Option<Box<dyn Driver>> = match DriverType::default().create(&feeder::Config {
        #[cfg(target_os = "windows")]
        vigem_config: gcfeeder_core::driver::vigem::Config {
            trigger_mode: gcfeeder_core::driver::vigem::TriggerMode::Digital,
            ..Default::default()
        },
        ..Default::default()
    }) {
        Ok(driver) => Some(driver),
        Err(err) => {
            error!("Error creating driver: {err}");
            None
        }
    };

    if let Some(driver) = &driver {
        info!("{} driver created", driver.name());
    } else {
        warn!("No driver");
    }

    let mut layers: Vec<Box<dyn Layer>> = vec![Box::new(layers::CenterCalibration::default())];

    loop {
        let recv_rumble_strength = async {
            if let Some(driver) = &driver {
                driver.recv_rumble_strength().await
            } else {
                std::future::pending().await
            }
        };

        tokio::select! {
            tx = rx_shutdown.recv() => {
                if let Some(tx) = tx {
                    tx.send(()).expect("sending shutdown signal");
                }
                info!("Driver service finished");
                break;
            }
            // TODO(Sirius902) Do more than `Port::One`.
            Ok(raw_input) = rx_inputs[Port::One.index()].recv() => {
                let Some(driver) = &driver else { continue; };

                let input = layers
                    .iter_mut()
                    .fold(raw_input, |input, layer| layer.apply(input));

                let driver_name = driver.name();
                if let Err(err) = driver.feed(&input).await {
                    warn!("Error feeding with {} driver: {err}", driver_name);
                }
            }
            Ok(strength) = recv_rumble_strength => {
                // TODO(Sirius902) Use `PatternRumbler`.
                adapter_service.set_rumble([(strength != 0).into(), Rumble::Off, Rumble::Off, Rumble::Off]).await;
            }
        }
    }
}

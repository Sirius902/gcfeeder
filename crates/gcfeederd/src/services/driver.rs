use std::sync::Arc;

use gcfeeder_core::adapter::Port;
use gcfeeder_core::driver::{Driver, DriverType};
use gcfeeder_core::layers::{self, Layer};
use gcinput::Rumble;
use tokio::sync::{mpsc, oneshot};
use tokio_util::task::TaskTracker;
use tracing::{debug, error, info, warn};

use super::adapter;
use crate::config::Config;

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
    let config_file_path = directories::BaseDirs::new().map(|dirs| {
        dirs.config_local_dir()
            .join("gcfeeder")
            .join("gcfeeder.toml")
    });

    let config = if let Some(config_file_path) = config_file_path {
        match tokio::fs::read_to_string(config_file_path).await {
            Ok(config_file) => toml::from_str::<Config>(&config_file).unwrap_or_else(|err| {
                warn!("Failed to parse config file, using default config: {err}");
                Default::default()
            }),
            Err(err) => {
                warn!("Failed to read config file, using default config: {err}");
                Default::default()
            }
        }
    } else {
        debug!("No config file, using default config");
        Default::default()
    };

    let mut rx_inputs = Port::all()
        .iter()
        .map(|port| adapter_service.subscribe_input(*port))
        .collect::<Vec<_>>();

    let profile = config
        .profile
        .selected(Port::One)
        .cloned()
        .unwrap_or_default();

    info!(
        "Using profile \"{}\"",
        config.profile.selected[Port::One.index()]
    );

    // TODO(Sirius902) Read inputs and pass to driver, forward rumble to adapter, handle config
    // updates. Don't hardcode testing stuff.
    let driver: Option<Box<dyn Driver>> = match DriverType::default().create(&profile) {
        Ok(driver) => driver,
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

    if profile.calibration.enabled {
        layers.push(Box::new(layers::Calibration::new(
            profile.calibration.stick_data,
            profile.calibration.trigger_data,
        )));
    }

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
                adapter_service.set_rumble([(strength != 0).into(), Rumble::Off, Rumble::Off, Rumble::Off]);
            }
        }
    }
}

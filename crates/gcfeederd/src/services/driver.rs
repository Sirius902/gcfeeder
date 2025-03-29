use std::sync::Arc;
use std::time::Duration;

use gcfeeder_core::adapter::Port;
use gcfeeder_core::driver::rumble::PatternRumbler;
use gcfeeder_core::driver::{Driver, DriverType};
use gcfeeder_core::feeder::{self, RumbleSetting};
use gcfeeder_core::layers::{self, Layer};
use gcinput::Input;
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;
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

    let task_token = CancellationToken::new();
    let tasks = TaskTracker::new();

    let (tx_rumble, rx_rumble) = tokio::sync::mpsc::unbounded_channel();

    tasks.spawn(rumble_task(
        task_token.clone(),
        rx_rumble,
        adapter_service.clone(),
    ));

    for port in Port::all() {
        // Give the virtual controllers time to be created otherwise the order isn't consistent.
        if port != Port::all().first().expect("there is at least one port") {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        let rx_inputs = adapter_service.subscribe_input(*port);

        let profile = config.profile.selected(*port).cloned().unwrap_or_default();

        info!(
            "Using profile \"{}\" for port {:?}",
            config.profile.selected[port.index()],
            port,
        );

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

        tasks.spawn(driver_task(
            task_token.clone(),
            rx_inputs,
            tx_rumble.clone(),
            driver,
            *port,
            profile,
        ));
    }

    drop(tx_rumble);

    tasks.close();

    let tx = rx_shutdown.recv().await;

    debug!("Shutting down driver tasks...");
    task_token.cancel();
    tasks.wait().await;
    debug!("Driver tasks finished!");

    if let Some(tx) = tx {
        tx.send(()).expect("sending shutdown signal");
    }

    info!("Driver service finished");
}

async fn driver_task(
    token: CancellationToken,
    mut rx_inputs: tokio::sync::broadcast::Receiver<Option<Input>>,
    tx_rumble: tokio::sync::mpsc::UnboundedSender<(Port, u8)>,
    driver: Option<Box<dyn Driver>>,
    port: Port,
    profile: feeder::Config,
) {
    let mut layers: Vec<Box<dyn Layer>> = vec![Box::new(layers::CenterCalibration::default())];

    if profile.calibration.enabled {
        layers.push(Box::new(layers::Calibration::new(
            profile.calibration.stick_data,
            profile.calibration.trigger_data,
        )));
    }

    if (1.0 - profile.analog_scale).abs() > 1e-6 {
        layers.push(Box::new(layers::AnalogScaling::new(profile.analog_scale)));
    }

    match profile.ess.inversion_mapping {
        Some(layers::EssInversion::OotVc) => {
            layers.push(Box::new(layers::oot_vc::InverseVc));
            layers.push(Box::new(layers::oot_vc::InverseClamp));
        }
        Some(layers::EssInversion::MmVc) => {
            layers.push(Box::new(layers::mm_vc::InverseVc));
            layers.push(Box::new(layers::mm_vc::InverseClamp));
        }
        Some(layers::EssInversion::Z64Gc) => {
            layers.push(Box::new(layers::z64_gc::InverseGc));
            layers.push(Box::new(layers::z64_gc::InverseClamp));
        }
        None => {}
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
            _ = token.cancelled() => {
                break;
            }
            Ok(raw_input) = rx_inputs.recv() => {
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
                if profile.rumble == RumbleSetting::On {
                    tx_rumble.send((port, strength)).expect("failed to send rumble");
                }
            }
        }
    }
}

async fn rumble_task(
    token: CancellationToken,
    mut rx_rumble: tokio::sync::mpsc::UnboundedReceiver<(Port, u8)>,
    adapter_service: Arc<adapter::Service>,
) {
    let mut rumblers: [PatternRumbler; Port::COUNT] =
        std::array::from_fn(|_| PatternRumbler::new());

    loop {
        tokio::select! {
            _ = token.cancelled() => {
                break;
            }
            _ = tokio::time::timeout(Duration::from_millis(8), adapter_service.rumble_written()) => {
                adapter_service.set_rumble([
                    rumblers[Port::One.index()].consume_rumble().into(),
                    rumblers[Port::Two.index()].consume_rumble().into(),
                    rumblers[Port::Three.index()].consume_rumble().into(),
                    rumblers[Port::Four.index()].consume_rumble().into(),
                ]);
            }
            Some((port, strength)) = rx_rumble.recv() => {
                rumblers[port.index()].update_strength(strength);
            }
        }
    }
}

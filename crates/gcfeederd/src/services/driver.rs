use std::sync::Arc;
use std::time::Duration;

use gcfeeder_core::adapter::Port;
use gcfeeder_core::driver::rumble::PatternRumbler;
use gcfeeder_core::driver::Driver;
use gcfeeder_core::feeder::RumbleSetting;
use gcfeeder_core::layers::{self, Layer};
use gcinput::Input;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use tracing::{debug, error, info, warn};

use super::{adapter, config};
use crate::config::{Config, Profile};

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

pub fn start(
    task_tracker: &TaskTracker,
    adapter_service: Arc<adapter::Service>,
    config_service: Arc<config::Service>,
) -> Service {
    let (tx_shutdown, rx_shutdown) = mpsc::unbounded_channel();

    task_tracker.spawn(run(
        rx_shutdown,
        adapter_service,
        config_service.subscribe_config(),
    ));

    Service { tx_shutdown }
}

async fn run(
    mut rx_shutdown: mpsc::UnboundedReceiver<oneshot::Sender<()>>,
    adapter_service: Arc<adapter::Service>,
    rx_config: broadcast::Receiver<Arc<Config>>,
) {
    let task_token = CancellationToken::new();
    let tasks = TaskTracker::new();

    let (tx_rumble, rx_rumble) = tokio::sync::mpsc::unbounded_channel();

    tasks.spawn(rumble_task(
        task_token.clone(),
        rx_rumble,
        adapter_service.clone(),
    ));

    for port in Port::all() {
        let rx_inputs = adapter_service.subscribe_input(*port);

        tasks.spawn(driver_task(
            task_token.clone(),
            *port,
            rx_inputs,
            tx_rumble.clone(),
            rx_config.resubscribe(),
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

fn fill_layers(profile: &Profile, layers: &mut Vec<Box<dyn Layer>>) {
    layers.clear();
    layers.push(Box::new(layers::CenterCalibration::default()));

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
}

async fn driver_task(
    token: CancellationToken,
    port: Port,
    mut rx_inputs: tokio::sync::broadcast::Receiver<Option<Input>>,
    tx_rumble: tokio::sync::mpsc::UnboundedSender<(Port, u8)>,
    mut rx_config: broadcast::Receiver<Arc<Config>>,
) {
    let mut driver: Option<Box<dyn Driver>> = None;
    let mut layers: Vec<Box<dyn Layer>> = Vec::new();
    let mut rumble_enabled = false;

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
            config = rx_config.recv() => {
                let Ok(config) = config else { continue; };

                drop(driver.take());

                let profile = config.profile.selected(port).cloned().unwrap_or_default();

                info!(
                    "Using profile \"{}\" for port {:?}",
                    config.profile.selected[port.index()],
                    port,
                );

                driver = match profile.driver.create(port, &profile) {
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

                fill_layers(&profile, &mut layers);

                rumble_enabled = profile.rumble == RumbleSetting::On;
            }
            raw_input = rx_inputs.recv() => {
                let Ok(raw_input) = raw_input else { continue; };
                let Some(driver) = &driver else { continue; };

                let input = layers
                    .iter_mut()
                    .fold(raw_input, |input, layer| layer.apply(input));

                let driver_name = driver.name();
                if let Err(err) = driver.feed(&input).await {
                    warn!("Error feeding with {} driver: {err}", driver_name);
                }
            }
            strength = recv_rumble_strength => {
                let Ok(strength) = strength else { continue; };

                if rumble_enabled {
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
    let mut rumblers: [_; Port::COUNT] = std::array::from_fn(|_| PatternRumbler::new());

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
            rumble = rx_rumble.recv() => {
                let Some((port, strength)) = rumble else { continue; };

                rumblers[port.index()].update_strength(strength);
            }
        }
    }
}

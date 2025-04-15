use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use gcfeeder_core::adapter::Port;
use gcfeeder_core::layers::{CenterCalibration, Layer};
use gcinput::Input;
use tokio::net::UdpSocket;
use tokio::sync::{broadcast, oneshot, watch};
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use tracing::{debug, info, warn};

use super::{adapter, config};
use crate::config::Config;

pub struct Service {
    tx_shutdown: oneshot::Sender<oneshot::Sender<()>>,
}

impl Service {
    pub async fn stop(self) {
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
    let (tx_shutdown, rx_shutdown) = oneshot::channel();
    let rx_config = config_service.subscribe_config();

    task_tracker.spawn(run(rx_shutdown, rx_config, adapter_service));

    Service { tx_shutdown }
}

async fn run(
    rx_shutdown: oneshot::Receiver<oneshot::Sender<()>>,
    rx_config: broadcast::Receiver<Arc<Config>>,
    adapter_service: Arc<adapter::Service>,
) {
    let task_token = CancellationToken::new();
    let tasks = TaskTracker::new();

    for port in Port::all() {
        tasks.spawn(server_task(
            task_token.clone(),
            *port,
            adapter_service.watch_input(*port),
            rx_config.resubscribe(),
        ));
    }

    tasks.close();

    let tx = rx_shutdown.await;

    debug!("Shutting down input server tasks...");
    task_token.cancel();
    tasks.wait().await;
    debug!("Input server tasks finished!");

    if let Ok(tx) = tx {
        tx.send(()).expect("sending shutdown signal");
    }

    info!("Input server service finished");
}

async fn server_task(
    token: CancellationToken,
    port: Port,
    mut rx_inputs: watch::Receiver<Option<Input>>,
    mut rx_config: broadcast::Receiver<Arc<Config>>,
) {
    let mut socket: Option<UdpSocket> = None;
    let mut clients: HashMap<SocketAddr, Instant> = HashMap::new();
    let mut heartbeat_interval = tokio::time::interval(Duration::from_secs(10));

    let mut center_calibration = CenterCalibration::default();

    loop {
        let heartbeat_fut = async {
            if let Some(socket) = &socket {
                socket.recv_from(&mut []).await
            } else {
                std::future::pending().await
            }
        };

        tokio::select! {
            _ = token.cancelled() => {
                break;
            }
            res = heartbeat_fut => {
                match res {
                    Ok((_, src_addr)) => {
                        if !clients.contains_key(&src_addr) {
                            info!("Client {src_addr} connected!");
                        }

                        clients.insert(src_addr, Instant::now());
                    }
                    Err(err) => {
                        warn!("Failed to receive heartbeat: {err}");
                    }
                }
            }
            deadline = heartbeat_interval.tick() => {
                clients.retain(|addr, heartbeat| {
                    let is_deadline_met =
                        deadline.duration_since(*heartbeat) <= heartbeat_interval.period();
                    if !is_deadline_met {
                        info!("Client {addr} disconnected.")
                    }

                    is_deadline_met
                });
            }
            res = rx_inputs.changed() => {
                let input = if res.is_ok() {
                    *rx_inputs.borrow_and_update()
                } else {
                    continue
                };

                // Center calibrate the input so it looks nice on the other end.
                let input = center_calibration.apply(input);

                if let Some(socket) = &socket {
                    for client_addr in clients.keys() {
                        let Ok(message) = bincode::serialize(&input) else {
                            warn!("Failed to serialize input: {input:?}");
                            continue;
                        };

                        if let Err(err) = socket.send_to(&message, *client_addr).await {
                            warn!("Failed to send input message: {err}");
                            continue;
                        }
                    }
                }
            }
            config = rx_config.recv() => {
                if let Ok(config) = config {
                    // FUTURE(Sirius902) Log when server is destroyed?
                    socket = None;
                    clients.clear();

                    // FUTURE(Sirius902) If we fail to connect, store the config update and try
                    // again on an interval.
                    let server_config = &config.input_server[port.index()];
                    if server_config.enabled {
                        match UdpSocket::bind(("127.0.0.1", server_config.port)).await {
                            Ok(s) => {
                                info!(
                                    "Created input server on localhost:{} for port {port:?}!",
                                    server_config.port,
                                );
                                socket = Some(s);
                            }
                            Err(err) => {
                                warn!(
                                    "Failed to bind input server on localhost:{} for port {port:?}: {err}",
                                    server_config.port,
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

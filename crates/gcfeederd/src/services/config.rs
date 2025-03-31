use std::sync::Arc;

use tokio::sync::{broadcast, mpsc, oneshot};
use tokio_util::task::TaskTracker;
use tracing::{debug, info, warn};

use crate::config::Config;

pub struct Service {
    tx_shutdown: mpsc::UnboundedSender<oneshot::Sender<()>>,
    rx_config: broadcast::Receiver<Arc<Config>>,
    tx_reload: mpsc::Sender<()>,
}

impl Service {
    pub async fn stop(&self) {
        let (tx, rx) = oneshot::channel();
        self.tx_shutdown.send(tx).expect("sending shutdown signal");
        rx.await.expect("waiting for shutdown");
    }

    pub fn subscribe_config(&self) -> broadcast::Receiver<Arc<Config>> {
        self.rx_config.resubscribe()
    }

    pub fn reload_config(&self) {
        match self.tx_reload.try_send(()) {
            Ok(()) | Err(mpsc::error::TrySendError::Full(())) => {}
            err => err.expect("reload config"),
        }
    }
}

pub fn start(task_tracker: &TaskTracker) -> Service {
    let (tx_shutdown, rx_shutdown) = mpsc::unbounded_channel();
    let (tx_config, rx_config) = broadcast::channel(1);
    let (tx_reload, rx_reload) = mpsc::channel(1);

    task_tracker.spawn(run(rx_shutdown, tx_config, rx_reload));

    Service {
        tx_shutdown,
        rx_config,
        tx_reload,
    }
}

async fn run(
    mut rx_shutdown: mpsc::UnboundedReceiver<oneshot::Sender<()>>,
    tx_config: broadcast::Sender<Arc<Config>>,
    mut rx_reload: mpsc::Receiver<()>,
) {
    let tx = loop {
        tokio::select! {
            tx = rx_shutdown.recv() => {
                break tx;
            }
            reload = rx_reload.recv() => {
                if reload != Some(()) {
                    continue;
                }

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

                if let Err(err) = tx_config.send(Arc::new(config)) {
                    warn!("Failed to send config: {err}");
                }
            }
        }
    };

    if let Some(tx) = tx {
        tx.send(()).expect("sending shutdown signal");
    }
    info!("Config service finished");
}

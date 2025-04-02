use std::path::Path;
use std::sync::Arc;

use tokio::sync::{broadcast, mpsc, oneshot};
use tokio_util::task::TaskTracker;
use tracing::{debug, info, warn};

use crate::config::Config;

pub type ConfigVisitor = dyn FnOnce(&mut Config) + Send;

#[derive(Debug)]
pub struct Service {
    tx_shutdown: mpsc::UnboundedSender<oneshot::Sender<()>>,
    rx_config: broadcast::Receiver<Arc<Config>>,
    tx_reload: mpsc::Sender<()>,
    tx_modify: mpsc::UnboundedSender<Box<ConfigVisitor>>,
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

    pub fn modify_config(&self, f: Box<ConfigVisitor>) {
        self.tx_modify.send(f).expect("send modify");
    }
}

pub fn start(task_tracker: &TaskTracker) -> Service {
    let (tx_shutdown, rx_shutdown) = mpsc::unbounded_channel();
    let (tx_config, rx_config) = broadcast::channel(1);
    let (tx_reload, rx_reload) = mpsc::channel(1);
    let (tx_modify, rx_modify) = mpsc::unbounded_channel();

    task_tracker.spawn(run(rx_shutdown, tx_config, rx_reload, rx_modify));

    Service {
        tx_shutdown,
        rx_config,
        tx_reload,
        tx_modify,
    }
}

async fn load_config(path: Option<impl AsRef<Path>>) -> Config {
    let config = if let Some(config_file_path) = path {
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

    config
}

async fn save_config(path: Option<impl AsRef<Path>>, config: &Config) {
    let Some(config_file_path) = path else {
        warn!("Failed to get config directory");
        return;
    };
    let config_file_path = config_file_path.as_ref();

    let config_path = config_file_path
        .parent()
        .expect("config file path has parent");

    let config = match toml::to_string(config) {
        Ok(config) => config,
        Err(err) => {
            warn!("Failed to serialize config: {err}");
            return;
        }
    };

    let file = match tempfile::NamedTempFile::new_in(config_path) {
        Ok(file) => file,
        Err(err) => {
            warn!("Failed to create temp file: {err}");
            return;
        }
    };

    if let Err(err) = tokio::fs::write(file.path(), config).await {
        warn!(
            "Failed to save config to \"{}\": {err}",
            file.path().display()
        );
        return;
    }

    if let Err(err) = std::fs::rename(file.path(), config_file_path) {
        warn!(
            "Failed to move config from \"{}\" to \"{}\": {err}",
            file.path().display(),
            config_path.display(),
        );
        return;
    }

    info!("Config saved!");
}

async fn run(
    mut rx_shutdown: mpsc::UnboundedReceiver<oneshot::Sender<()>>,
    tx_config: broadcast::Sender<Arc<Config>>,
    mut rx_reload: mpsc::Receiver<()>,
    mut rx_modify: mpsc::UnboundedReceiver<Box<ConfigVisitor>>,
) {
    let config_dir =
        directories::BaseDirs::new().map(|dirs| dirs.config_local_dir().join("gcfeeder"));

    if let Some(config_dir) = &config_dir {
        if let Err(err) = std::fs::create_dir_all(config_dir) {
            warn!(
                "Failed to create config dir \"{}\": {err}",
                config_dir.display()
            );
        }
    }

    let config_file_path = config_dir.map(|p| p.join("gcfeeder.toml"));

    let mut config: Option<Config> = None;

    loop {
        tokio::select! {
            tx = rx_shutdown.recv() => {
                if let Some(tx) = tx {
                    tx.send(()).expect("sending shutdown signal");
                }
                info!("Config service finished");
                break;
            }
            reload = rx_reload.recv() => {
                if reload != Some(()) {
                    continue;
                }

                let config = config.insert(load_config(config_file_path.as_ref()).await);
                if let Err(err) = tx_config.send(Arc::new(config.clone())) {
                    warn!("Failed to send config: {err}");
                }
            }
            modify = rx_modify.recv() => {
                let Some(visitor) = modify else { continue; };

                if config.is_none() {
                    config = Some(load_config(config_file_path.as_ref()).await);
                }

                let config = config.as_mut().expect("config exists");

                visitor(config);
                save_config(config_file_path.as_ref(), config).await;

                if let Err(err) = tx_config.send(Arc::new(config.clone())) {
                    warn!("Failed to send config: {err}");
                }
            }
        }
    }
}

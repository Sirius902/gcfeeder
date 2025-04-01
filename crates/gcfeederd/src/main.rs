#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;

use gcfeederd::services;
use tokio::sync::oneshot;
use tokio_util::task::TaskTracker;
use tracing::{info, warn};
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;

fn main() {
    let _guard = setup_logging();

    let (tx_tray_service, rx_tray_service) = oneshot::channel();
    let (tx_config_service, rx_config_service) = oneshot::channel();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("build tokio runtime");

    rt.spawn(run(rx_tray_service, tx_config_service));

    services::tray::run(rx_config_service, tx_tray_service);
}

async fn run(
    rx_tray_service: oneshot::Receiver<services::tray::Service>,
    tx_config_service: oneshot::Sender<Arc<services::config::Service>>,
) {
    let task_tracker = TaskTracker::new();

    let config_service = Arc::new(services::config::start(&task_tracker));
    tx_config_service
        .send(config_service.clone())
        .expect("send config service");

    let adapter_service = Arc::new(services::adapter::start(&task_tracker));
    let driver_service = services::driver::start(
        &task_tracker,
        adapter_service.clone(),
        config_service.clone(),
    );

    task_tracker.close();

    let mut tray_service = rx_tray_service.await.expect("recv tray service");

    config_service.reload_config();

    tokio::select! {
        // FUTURE(Sirius902) Should we handle any other signals here?
        res = tokio::signal::ctrl_c() => {
            if let Err(err) = res {
                warn!("Failed to wait for ctrl+c signal: {err}");
            }
        }
        _ = tray_service.recv_quit() => {},
        _ = task_tracker.wait() => {
            info!("Stopping tray service...");
            tray_service.stop().await;
            info!("Tray service stopped!");

            return;
        },
    }

    info!("Stopping driver service...");
    driver_service.stop().await;
    info!("Driver service stopped!");

    info!("Stopping adapter service...");
    adapter_service.stop().await;
    info!("Adapter service stopped!");

    info!("Stopping config service...");
    config_service.stop().await;
    info!("Config service stopped!");

    info!("Waiting for task tracker...");
    task_tracker.wait().await;
    info!("Task tracker finished!");

    info!("Stopping tray service...");
    tray_service.stop().await;
    info!("Tray service stopped!");
}

fn setup_logging() -> Option<tracing_appender::non_blocking::WorkerGuard> {
    let builder = tracing_subscriber::registry();

    #[cfg(feature = "tokio-console")]
    let builder = builder.with(console_subscriber::spawn().with_filter({
        use tracing::level_filters::LevelFilter;

        EnvFilter::builder()
            .with_default_directive(LevelFilter::DEBUG.into())
            .parse("tokio=trace,runtime=trace")
            .expect("tokio-console env filter string parses")
    }));

    let env_filter = || {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            EnvFilter::builder()
                .parse(["gcfeederd=trace", "gcfeeder_core=trace"].join(","))
                .expect("env filter string parses")
        })
    };

    let builder = builder.with(tracing_subscriber::fmt::layer().with_filter(env_filter()));

    let file_layer = directories::BaseDirs::new()
        .map(|dirs| dirs.cache_dir().join("gcfeederd").join("logs"))
        .map(|logs_dir| {
            let file_appender = tracing_appender::rolling::daily(logs_dir, "gcfeederd.log");
            let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
            (
                tracing_subscriber::fmt::layer()
                    .with_writer(non_blocking)
                    .with_filter(env_filter()),
                guard,
            )
        });

    if let Some((file_layer, guard)) = file_layer {
        builder.with(file_layer).init();
        Some(guard)
    } else {
        builder.init();
        None
    }
}

use std::sync::Arc;

use gcfeeder_core::adapter;
use gcfeederd::services;
use tokio_util::task::TaskTracker;
use tracing::warn;
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> adapter::Result<()> {
    let _guard = setup_logging();

    let task_tracker = TaskTracker::new();

    let config_service = Arc::new(services::config::start(&task_tracker));

    let adapter_service = Arc::new(services::adapter::start(&task_tracker));
    let driver_service = services::driver::start(
        &task_tracker,
        adapter_service.clone(),
        config_service.clone(),
    );

    let mut tray_service = services::tray::start(config_service.clone());

    task_tracker.close();

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
            return Ok(());
        },
    }

    driver_service.stop().await;
    adapter_service.stop().await;

    config_service.stop().await;

    task_tracker.wait().await;

    Ok(())
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

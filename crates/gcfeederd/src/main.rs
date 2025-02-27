use std::sync::Arc;

use gcfeeder_core::adapter;
use gcfeederd::services;
use tokio_util::task::TaskTracker;
use tracing::warn;
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> adapter::Result<()> {
    init_logging();

    let task_tracker = TaskTracker::new();

    let adapter_service = Arc::new(services::adapter::start(&task_tracker));
    let driver_service = services::driver::start(&task_tracker, adapter_service.clone());

    _ = task_tracker.close();

    tokio::select! {
        // FUTURE(Sirius902) Should we handle any other signals here?
        res = tokio::signal::ctrl_c() => {
            if let Err(err) = res {
                warn!("Failed to wait for ctrl+c signal: {err}");
            }

            driver_service.stop().await;
            adapter_service.stop().await;

            task_tracker.wait().await;
        }
        _ = task_tracker.wait() => {},
    }

    Ok(())
}

fn init_logging() {
    let registry = tracing_subscriber::registry();

    #[cfg(not(feature = "tokio-console"))]
    let layered = registry;

    #[cfg(feature = "tokio-console")]
    let layered = registry.with(console_subscriber::spawn());

    layered
        .with(tracing_subscriber::fmt::layer())
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            EnvFilter::builder()
                .parse(
                    [
                        "gcfeederd=trace",
                        "gcfeeder_core=trace",
                        #[cfg(feature = "tokio-console")]
                        "tokio=trace",
                        #[cfg(feature = "tokio-console")]
                        "runtime=trace",
                    ]
                    .join(","),
                )
                .expect("env filter string is valid")
        }))
        .init();
}

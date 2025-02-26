use std::sync::Arc;

use gcfeeder_core::adapter;
use gcfeederd::services;
use tokio_util::task::TaskTracker;
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> adapter::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            EnvFilter::builder()
                .parse("gcfeederd=trace")
                .expect("env filter string is valid")
        }))
        .init();

    let task_tracker = TaskTracker::new();

    let adapter_service = Arc::new(services::adapter::start(&task_tracker));
    let driver_service = services::driver::start(&task_tracker, adapter_service.clone());

    _ = task_tracker.close();

    loop {
        tokio::select! {
            // FUTURE(Sirius902) Should we handle any other signals here?
            res = tokio::signal::ctrl_c() => {
                if let Err(err) = res {
                    tracing::warn!("Failed to wait for ctrl+c signal: {err}");
                }

                driver_service.stop().await;
                adapter_service.stop().await;

                task_tracker.wait().await;
                break;
            }
            _ = task_tracker.wait() => break,
        }
    }

    Ok(())
}

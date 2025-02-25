use gcfeeder_core::adapter::{self, Port};
use gcfeederd::services;
use gcinput::Rumble;
use tokio_util::task::TaskTracker;
use tracing_subscriber::{prelude::*, EnvFilter};

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
    let adapter_service = services::adapter::start(&task_tracker);

    _ = task_tracker.close();

    let mut rumble_interval = tokio::time::interval(std::time::Duration::from_secs(2));
    let mut rumble: Option<Rumble> = None;

    loop {
        tokio::select! {
            res = tokio::signal::ctrl_c() => {
                if let Err(err) = res {
                    tracing::warn!("Failed to wait for ctrl+c signal: {err}");
                }

                adapter_service.stop().await;
                task_tracker.wait().await;
                break;
            }
            _ = rumble_interval.tick() => {
                adapter_service.set_rumble(Port::One, rumble.unwrap_or(Rumble::Off));

                rumble = match rumble {
                    Some(Rumble::Off) | None => Some(Rumble::On),
                    Some(Rumble::On) => Some(Rumble::Off),
                };
            }
            _ = task_tracker.wait() => break,
        }
    }

    Ok(())
}

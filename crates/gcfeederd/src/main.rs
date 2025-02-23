use std::time::Duration;

use gcfeeder_core::{
    adapter::{self, poller::Poller, source::InputSource, Port},
    feeder,
};
use tracing::debug;
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

    // TODO(Sirius902) Actually read the config.
    // let config_path = directories::BaseDirs::new()
    //     .expect("Failed to get config directory")
    //     .config_dir()
    //     .join("gcfeeder")
    //     .join("gcfeeder.toml");

    let config = feeder::Config::default();

    let poller = Poller::default();
    let feeder = feeder::Feeder::new(config, poller.add_listener(Port::One).await);

    let mut stats_interval = tokio::time::interval(Duration::from_secs(1));

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                feeder.close().await;
                poller.close().await;
                break;
            }
            _ = stats_interval.tick() => {
                if let Some(feed_time) = feeder.average_feed_time().await {
                    debug!("Average feed time: {}ms", feed_time.subsec_millis());
                }
            }
        }
    }

    Ok(())
}

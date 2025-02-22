use gcinput::Rumble;
use tracing_subscriber::{prelude::*, EnvFilter};

use gcfeederd::adapter::{self, Port};

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

    let adapter = adapter::Adapter::open().await?;

    loop {
        let then = std::time::Instant::now();

        let adapter_task = async {
            let inputs = adapter.read_inputs();
            let rumble = adapter.write_rumble([Rumble::On; Port::COUNT]);
            tokio::join!(inputs, rumble)
        };

        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                adapter.reset_rumble().await?;
                break;
            },
            _ = adapter_task => {
                let now = std::time::Instant::now();
                tracing::debug!("Poll time: {}ms", now.duration_since(then).subsec_millis());
            }
        }
    }

    Ok(())
}

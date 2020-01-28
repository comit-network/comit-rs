use tracing::{info, subscriber, Level};
use tracing_subscriber::FmtSubscriber;

pub fn init_tracing() -> anyhow::Result<()> {
    let level = Level::INFO;
    let subscriber = FmtSubscriber::builder()
        .with_max_level(level.clone())
        .finish();

    subscriber::set_global_default(subscriber)?;
    info!("Initialized tracing with level: {}", level);

    Ok(())
}

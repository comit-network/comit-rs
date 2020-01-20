use tracing::{subscriber, Level, Span};
use tracing_subscriber::FmtSubscriber;

pub fn init_tracing() -> anyhow::Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish();

    subscriber::set_global_default(subscriber)?;
    tracing::info!("Initialized tracing");

    Ok(())
}

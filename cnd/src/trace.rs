use log::LevelFilter;
use tracing::{info, subscriber, Level};
use tracing_subscriber::FmtSubscriber;

pub fn init_tracing(level: log::LevelFilter) -> anyhow::Result<()> {
    let level = level_from_level_filter(level);
    let subscriber = FmtSubscriber::builder()
        .with_max_level(level.clone())
        .finish();

    subscriber::set_global_default(subscriber)?;
    info!("Initialized tracing with level: {}", level);

    Ok(())
}

fn level_from_level_filter(level: LevelFilter) -> Level {
    match level {
        LevelFilter::Off => Level::ERROR,
        LevelFilter::Error => Level::ERROR,
        LevelFilter::Warn => Level::WARN,
        LevelFilter::Info => Level::INFO,
        LevelFilter::Debug => Level::DEBUG,
        LevelFilter::Trace => Level::TRACE,
    }
}

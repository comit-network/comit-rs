use log::LevelFilter;
use tracing::{info, subscriber, Level};
use tracing_log::LogTracer;
use tracing_subscriber::FmtSubscriber;

pub fn init_tracing(level: log::LevelFilter) -> anyhow::Result<()> {
    if level == LevelFilter::Off {
        return Ok(());
    }

    // We want upstream library log messages, just only at Info level.
    LogTracer::init_with_filter(LevelFilter::Info)?;

    let subscriber = FmtSubscriber::builder()
        .with_max_level(level_from_level_filter(level))
        .with_writer(std::io::stderr)
        .finish();

    subscriber::set_global_default(subscriber)?;
    info!("Initialized tracing with level: {}", level);

    Ok(())
}

fn level_from_level_filter(level: LevelFilter) -> Level {
    match level {
        LevelFilter::Off => unreachable!(),
        LevelFilter::Error => Level::ERROR,
        LevelFilter::Warn => Level::WARN,
        LevelFilter::Info => Level::INFO,
        LevelFilter::Debug => Level::DEBUG,
        LevelFilter::Trace => Level::TRACE,
    }
}

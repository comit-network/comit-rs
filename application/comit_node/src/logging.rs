use crate::settings::ComitNodeSettings;
use fern::{Dispatch, FormatCallback};
use log::{LevelFilter, Record};
use std::{fmt::Arguments, io::stdout};

pub fn set_up_logging(settings: &ComitNodeSettings) {
    Dispatch::new()
        .format(move |out, message, record| formatter(out, message, record))
        .level(settings.log_level)
        .level_for("tokio_core::reactor", LevelFilter::Info)
        .level_for("tokio_reactor", LevelFilter::Info)
        .level_for("hyper", LevelFilter::Info)
        .level_for("warp", LevelFilter::Info)
        .level_for("libp2p", LevelFilter::Debug) // the libp2p library
        .level_for("sub-libp2p", LevelFilter::Debug) // the libp2p subsystem in our application
        .chain(stdout())
        .apply()
        .unwrap();
}

fn formatter(out: FormatCallback<'_>, message: &Arguments<'_>, record: &Record<'_>) {
    let line = record
        .line()
        .map(|line| format!(":{}", line))
        .unwrap_or_else(String::new);
    //    let path = record.file().unwrap_or_else(|| record.target());
    let path = record.target();

    out.finish(format_args!(
        "[{date}][{level}][{path}{line}] {message}",
        date = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
        path = path,
        line = line,
        level = record.level(),
        message = message,
    ))
}

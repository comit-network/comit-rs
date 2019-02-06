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
        .chain(stdout())
        .apply()
        .unwrap();
}

fn formatter(out: FormatCallback<'_>, message: &Arguments<'_>, record: &Record<'_>) {
    let line = match record.line() {
        Some(line) => format!(":{}", line),
        None => "".to_string(),
    };

    let path = match record.file() {
        Some(file) => file.to_string(),
        None => record.target().to_string(),
    };

    out.finish(format_args!(
        "[{date}][{level}][{path}{line}] {message}",
        date = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
        path = path,
        line = line,
        level = record.level(),
        message = message,
    ))
}

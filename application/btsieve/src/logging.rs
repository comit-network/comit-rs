use crate::settings::Settings;
use fern::{Dispatch, FormatCallback};
use log::{LevelFilter, Record};
use std::{fmt::Arguments, io::stdout};

pub fn set_up_logging(settings: &Settings) {
    Dispatch::new()
        .format(move |out, message, record| formatter(out, message, record))
        .level(settings.log_levels.btsieve)
        .level_for("warp", LevelFilter::Info)
        .chain(stdout())
        .apply()
        .unwrap();
}

fn formatter(out: FormatCallback<'_>, message: &Arguments<'_>, record: &Record<'_>) {
    let line = record
        .line()
        .map(|line| format!(":{}", line))
        .unwrap_or_else(String::new);
    let path = record.file().unwrap_or_else(|| record.target());

    out.finish(format_args!(
        "[{date}][{level}][{path}{line}] {message}",
        date = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
        path = path,
        line = line,
        level = record.level(),
        message = message,
    ))
}

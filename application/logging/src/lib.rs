extern crate chrono;
extern crate fern;
pub extern crate log;

use fern::{
    colors::{Color, ColoredLevelConfig},
    FormatCallback,
};
use log::Record;
use std::{cell::RefCell, fmt::Arguments, io::stdout};

#[allow(dead_code)]
thread_local!(static LOG_CONTEXT: RefCell<Option<String>> = RefCell::new(None) );

pub fn set_context<S: ToString>(input: &S) {
    LOG_CONTEXT.with(|context| {
        *context.borrow_mut() = Some(input.to_string());
    });
}

pub fn set_up_logging() {
    fern::Dispatch::new()
        .format(move |out, message, record| formatter(out, message, record))
        //TODO: get level from config file once implemented with #136
        .level(log::LevelFilter::Warn)
        .level_for("comit_node", log::LevelFilter::Trace)
        .level_for("bitcoin_htlc", log::LevelFilter::Trace)
        .level_for("comit_wallet", log::LevelFilter::Trace)
        .level_for("ethereum_htlc", log::LevelFilter::Trace)
        .level_for("ethereum_wallet", log::LevelFilter::Trace)
        .level_for("ganp", log::LevelFilter::Trace)
        // output to stdout
        .chain(stdout())
        .apply()
        .unwrap();
}

fn formatter(out: FormatCallback, message: &Arguments, record: &Record) {
    // configure colors for the whole line
    let colors_line = ColoredLevelConfig::new()
        .error(Color::Red)
        .warn(Color::Yellow)
        // we actually don't need to specify the color for debug and info, they are white by default
        .info(Color::White)
        .debug(Color::White)
        // depending on the terminals color scheme, this is the same as the background color
        .trace(Color::BrightBlack);

    // configure colors for the name of the level.
    // since almost all of them are the some as the color for the whole line, we
    // just clone `colors_line` and overwrite our changes
    let colors_level = colors_line.clone().info(Color::Green);

    LOG_CONTEXT.with(|context| {
        let context = {
            match *context.borrow() {
                Some(ref context) => format!("[{}] ", context),
                None => "".to_string(),
            }
        };

        out.finish(format_args!(
            "{color_line}[{date}][{target}][{level}{color_line}]{context}{message}\x1B[0m",
            color_line = format_args!(
                "\x1B[{}m",
                colors_line.get_color(&record.level()).to_fg_str()
            ),
            date = chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            target = record.target(),
            level = colors_level.color(record.level()),
            context = context,
            message = message,
        ))
    });
}

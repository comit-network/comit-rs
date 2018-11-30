use chrono;
use fern::{
    colors::{Color, ColoredLevelConfig},
    Dispatch, FormatCallback,
};
use log::{LevelFilter, Record};
use std::{cell::RefCell, fmt::Arguments, io::stdout};

#[allow(dead_code)]
thread_local!(static LOG_CONTEXT: RefCell<Option<String>> = RefCell::new(None) );

pub fn set_context<S: ToString>(input: &S) {
    LOG_CONTEXT.with(|context| {
        *context.borrow_mut() = Some(input.to_string());
    });
}

pub fn set_up_logging() {
    Dispatch::new()
        .format(move |out, message, record| formatter(out, message, record))
        // TODO: get level from config file once implemented with #136
        .level(LevelFilter::Debug)
        .level_for("comit_node", LevelFilter::Trace)
        .level_for("comit_node::ledger_query_service", LevelFilter::Info)
        .level_for("bitcoin_htlc", LevelFilter::Trace)
        .level_for("comit_wallet", LevelFilter::Trace)
        .level_for("ethereum_htlc", LevelFilter::Trace)
        .level_for("ethereum_wallet", LevelFilter::Trace)
        .level_for("transport_protocol", LevelFilter::Trace)
        .level_for("tokio_core::reactor", LevelFilter::Info)
        .level_for("tokio_reactor", LevelFilter::Info)
        .level_for("hyper", LevelFilter::Info)
        .level_for("warp", LevelFilter::Info)
        // output to stdout
        .chain(stdout())
        .apply()
        .unwrap();
}

fn formatter(out: FormatCallback, message: &Arguments, record: &Record) {
    // configure colors for the whole line
    let colors_line = ColoredLevelConfig::default()
        .info(Color::Green)
        .debug(Color::Blue)
        .trace(Color::Cyan);

    LOG_CONTEXT.with(|context| {
        let context = {
            match *context.borrow() {
                Some(ref context) => format!("[{}] ", context),
                None => "".to_string(),
            }
        };

        out.finish(format_args!(
            "[{date}][{target}][{level}] {context}{message}",
            date = chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            target = record.target(),
            level = colors_line.color(record.level()),
            context = context,
            message = message,
        ))
    });
}

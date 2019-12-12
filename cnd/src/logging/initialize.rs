use fern::{Dispatch, FormatCallback};
use log::{LevelFilter, Record};
use std::{fmt::Arguments, io::stderr};

pub fn initialize(
    base_log_level: LevelFilter,
    structured: bool,
) -> Result<(), log::SetLoggerError> {
    #![allow(clippy::print_stdout)] // We cannot use `log` before we have the config file
    eprintln!("Initializing logging with base level {}", base_log_level);

    let (max_level, log) = create_logger(base_log_level, structured, stderr());

    log::set_boxed_logger(log)?;
    log::set_max_level(max_level);

    Ok(())
}

fn create_logger<T: Into<fern::Output>>(
    base_log_level: LevelFilter,
    structured: bool,
    target: T,
) -> (LevelFilter, Box<dyn log::Log>) {
    let formatter = if structured {
        json_formatter
    } else {
        line_formatter
    };

    Dispatch::new()
        .format(formatter)
        .level(base_log_level)
        .level_for("tokio_core::reactor", LevelFilter::Info)
        .level_for("tokio_reactor", LevelFilter::Info)
        .level_for("hyper", LevelFilter::Info)
        .level_for("warp", LevelFilter::Info)
        .level_for("libp2p", LevelFilter::Debug) // the libp2p library
        .level_for("sub-libp2p", LevelFilter::Debug) // the libp2p subsystem in our application
        .level_for("http-api", LevelFilter::Debug) // the http-api of our application
        .chain(target)
        .into_log()
}

fn line_formatter(out: FormatCallback<'_>, message: &Arguments<'_>, record: &Record<'_>) {
    out.finish(format_args!(
        "[{date}][{level}][{path}{line}] {message}",
        date = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
        path = record.target(),
        line = record
            .line()
            .map(|line| format!(":{}", line))
            .unwrap_or_else(String::new),
        level = record.level(),
        message = message,
    ))
}

/// Formats each log record as a JSON statement
///
/// The implementation of this function on purpose does not use `serde_json` to
/// create the JSON string. Here is why:
/// 1. `out.finish` only accepts `std::fmt::Arguments` and hence we can only use
/// `format_args!`
///
/// 2. (1) is very likely due to the fact that we don't want to
/// unnecessarily allocate strings in this function for calls to `Log::log` with
/// a level that is not even active (think about all the calls to `log::trace`
/// in all our depenencies that we don't see)
///
/// Note that this formatter **CANNOT** handle multiline message as for example
/// produced by: `log::info!("{:#?}", my_struct);`
fn json_formatter(out: FormatCallback<'_>, message: &Arguments<'_>, record: &Record<'_>) {
    out.finish(format_args!(
        "{{\"level\":\"{level}\",\"line\":{line},\"target\":\"{target}\",\"message\":\"{message}\",\"date\":\"{date}\"}}",
        date = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
        target = record.target(),
        line = record
            .line()
            .map(|line| format!(r#""{}""#, line))
            .unwrap_or_else(|| String::from("null")),
        level = record.level(),
        message = message,
    ))
}

#[cfg(test)]
mod tests {

    use super::*;
    use log::{Level, LevelFilter, Record};
    use spectral::prelude::*;
    use std::sync::mpsc::channel;

    #[test]
    fn line_formatter_should_return_a_single_line() {
        let (sender, receiver) = channel();
        let (_, log) = create_logger(LevelFilter::Trace, false, sender);

        log.log(
            &Record::builder()
                .args(format_args!("Hello {}!", "world"))
                .level(Level::Debug)
                .target("test")
                .line(Some(10))
                .build(),
        );

        let messages = receiver.recv().unwrap();
        let messages = messages
            .split('\n')
            .filter(|m| !m.is_empty())
            .collect::<Vec<_>>();

        assert_that(&messages).has_length(1);
        let message = messages[0];

        let pattern = r#"\[[0-9\-\s:\.]+\]\[DEBUG\]\[test:10\] Hello world!"#;
        let regex = regex::Regex::new(pattern).unwrap();

        if !regex.is_match(&message) {
            panic!(
                "Log message didn't match expected pattern!\n\n\
                 TransactionPattern: {}\n\
                 Message: {}\n",
                pattern, message
            );
        }
    }

    #[derive(serde::Deserialize, PartialEq, Debug)]
    struct JsonLogRecord {
        level: String,
        line: Option<String>,
        target: String,
        message: String,
        date: String,
    }

    #[test]
    fn json_formatter_should_return_a_json_object() {
        let (sender, receiver) = channel();
        let (_, log) = create_logger(LevelFilter::Trace, true, sender);

        log.log(
            &Record::builder()
                .args(format_args!("Hello {}!", "world"))
                .level(Level::Debug)
                .target("test")
                .line(Some(10))
                .build(),
        );

        let messages = receiver.recv().unwrap();
        let messages = messages
            .split('\n')
            .filter(|m| !m.is_empty())
            .collect::<Vec<_>>();

        assert_that(&messages).has_length(1);
        let message = messages[0];

        let json_log_record = serde_json::from_str(&message);

        let JsonLogRecord {
            level,
            line,
            target,
            message,
            ..
        } = assert_that(&json_log_record).is_ok().subject;

        // can't compare the date because we cannot predict it and there is no
        // abstraction to mock it :(
        assert_that(level).is_equal_to("DEBUG".to_string());
        assert_that(line).is_some().is_equal_to("10".to_string());
        assert_that(target).is_equal_to("test".to_string());
        assert_that(message).is_equal_to("Hello world!".to_string());
    }

    #[test]
    fn json_formatter_can_handle_missing_values_on_record() {
        let (sender, receiver) = channel();
        let (_, log) = create_logger(LevelFilter::Trace, true, sender);

        log.log(&Record::builder().level(Level::Debug).build());

        let messages = receiver.recv().unwrap();
        let messages = messages
            .split('\n')
            .filter(|m| !m.is_empty())
            .collect::<Vec<_>>();

        assert_that(&messages).has_length(1);
        let message = messages[0];

        let json_log_record = serde_json::from_str(&message);

        let JsonLogRecord {
            level,
            line,
            target,
            message,
            ..
        } = assert_that(&json_log_record).is_ok().subject;

        // can't compare the date because we cannot predict it and there is no
        // abstraction to mock it :(
        assert_that(level).is_equal_to("DEBUG".to_string());
        assert_that(line).is_none();
        assert_that(target).is_equal_to("".to_string());
        assert_that(message).is_equal_to("".to_string());
    }
}

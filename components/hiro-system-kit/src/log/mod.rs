use std::io;
use std::io::Write;
use std::sync::Mutex;

use slog::{o, Drain, FnValue, Logger, PushFnValue, Record, LOG_LEVEL_NAMES};
use slog_async;
use slog_atomic::AtomicSwitch;
use slog_scope::GlobalLoggerGuard;
use slog_term::{self, CountingWriter, RecordDecorator, ThreadSafeTimestampFn};

pub fn setup_global_logger(logger: Logger) -> GlobalLoggerGuard {
    slog_scope::set_global_logger(logger)
}

pub fn setup_logger() -> Logger {
    if cfg!(feature = "release") || cfg!(feature = "release_debug") {
        let drain = if cfg!(feature = "full_log_level_prefix") {
            slog_json::Json::new(std::io::stderr()).add_key_value(o!(
                "ts" => FnValue(move |_ : &Record| {
                        time::OffsetDateTime::now_utc()
                        .format(&time::format_description::well_known::Rfc3339)
                        .ok()
                }),
                "level" => FnValue(move |rinfo : &Record| {
                    LOG_LEVEL_NAMES[rinfo.level().as_usize()]
                }),
                "msg" => PushFnValue(move |record : &Record, ser| {
                    ser.emit(record.msg())
                }),
            ))
        } else {
            slog_json::Json::new(std::io::stderr()).add_default_keys()
        };

        Logger::root(Mutex::new(drain.build()).map(slog::Fuse), slog::o!())
    } else {
        let decorator = slog_term::TermDecorator::new().build();
        let drain = Mutex::new(
            slog_term::FullFormat::new(decorator)
                .use_custom_header_print(custom_print_msg_header)
                .build(),
        )
        .fuse();
        let drain = slog_async::Async::new(drain).build().fuse();
        let drain = AtomicSwitch::new(drain);
        Logger::root(drain.fuse(), o!())
    }
}

/// Copied from `slog_term::print_msg_header` with minor adjustments.
pub fn custom_print_msg_header(
    fn_timestamp: &dyn ThreadSafeTimestampFn<Output = io::Result<()>>,
    mut rd: &mut dyn RecordDecorator,
    record: &Record,
    use_file_location: bool,
) -> io::Result<bool> {
    rd.start_timestamp()?;
    fn_timestamp(&mut rd)?;

    rd.start_whitespace()?;
    write!(rd, " ")?;

    rd.start_level()?;
    if cfg!(feature = "full_log_level_prefix") {
        write!(rd, "{}", LOG_LEVEL_NAMES[record.level().as_usize()])?;
    } else {
        write!(rd, "{}", record.level().as_short_str())?;
    }

    if use_file_location {
        rd.start_location()?;
        write!(
            rd,
            "[{}:{}:{}]",
            record.location().file,
            record.location().line,
            record.location().column
        )?;
    }

    rd.start_whitespace()?;
    write!(rd, " ")?;

    rd.start_msg()?;
    let mut count_rd = CountingWriter::new(&mut rd);
    write!(count_rd, "{}", record.msg())?;
    Ok(count_rd.count() != 0)
}

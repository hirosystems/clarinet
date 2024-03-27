use slog::{o, Drain, FnValue, Logger, Record};
use slog_async;
use slog_atomic::AtomicSwitch;
use slog_scope::GlobalLoggerGuard;
use slog_term::{self, CountingWriter, RecordDecorator, ThreadSafeTimestampFn};
use std::io::Write;
use std::{io, sync::Mutex};

pub fn setup_global_logger(logger: Logger) -> GlobalLoggerGuard {
    slog_scope::set_global_logger(logger)
}

pub fn setup_logger() -> Logger {
    if cfg!(feature = "release") || cfg!(feature = "release_debug") {
        let drain = slog_json::Json::new(std::io::stderr()).add_default_keys();

        let drain = if cfg!(feature = "full_level_prefix") {
            drain.add_key_value(o!(
                "level" => FnValue(move |rinfo : &Record| {
                    rinfo.level().as_str()
                }),
            ))
        } else {
            drain
        };

        Logger::root(Mutex::new(drain.build()).map(slog::Fuse), slog::o!())
    } else {
        let decorator = slog_term::TermDecorator::new().build();
        let drain = slog_term::FullFormat::new(decorator);
        let drain = if cfg!(feature = "full_level_prefix") {
            drain
                .use_custom_header_print(custom_print_msg_header)
                .build()
        } else {
            drain.build()
        };
        let drain = Mutex::new(drain).fuse();
        let drain = slog_async::Async::new(drain).build().fuse();
        let drain = AtomicSwitch::new(drain);
        Logger::root(drain.fuse(), o!())
    }
}

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
    write!(rd, "{}", record.level().as_str())?;

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

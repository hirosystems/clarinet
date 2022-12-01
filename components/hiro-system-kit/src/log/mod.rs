use slog::{o, Drain, Logger};
use slog_async;
use slog_atomic::AtomicSwitch;
use slog_scope::GlobalLoggerGuard;
use slog_term;
use std::sync::Mutex;

pub fn setup_global_logger(logger: Logger) -> GlobalLoggerGuard {
    slog_scope::set_global_logger(logger)
}

pub fn setup_logger() -> Logger {
    if cfg!(feature = "release") {
        Logger::root(
            Mutex::new(slog_json::Json::default(std::io::stderr())).map(slog::Fuse),
            slog::o!(),
        )
    } else {
        let decorator = slog_term::TermDecorator::new().build();
        let drain = Mutex::new(slog_term::FullFormat::new(decorator).build()).fuse();
        let drain = slog_async::Async::new(drain).build().fuse();
        let drain = AtomicSwitch::new(drain);
        Logger::root(drain.fuse(), o!())
    }
}

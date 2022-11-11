#[macro_use]
extern crate serde_json;

#[macro_use]
extern crate slog_scope;

#[macro_use]
extern crate hiro_system_kit;

#[macro_use]
extern crate serde_derive;

extern crate serde;
extern crate slog;

use slog_async;
use slog_term;

pub mod archive;
pub mod block;
mod cli;
pub mod config;

use slog::*;
use slog_atomic::*;
use std::sync::Mutex;

fn main() {
    let logger = if cfg!(feature = "release") {
        slog::Logger::root(
            Mutex::new(slog_json::Json::default(std::io::stderr())).map(slog::Fuse),
            slog::o!("version" => env!("CARGO_PKG_VERSION")),
        )
    } else {
        let decorator = slog_term::TermDecorator::new().build();
        let drain = Mutex::new(slog_term::FullFormat::new(decorator).build()).fuse();
        let drain = slog_async::Async::new(drain).build().fuse();
        let drain = AtomicSwitch::new(drain);
        Logger::root(drain.fuse(), o!())
    };

    // slog_stdlog uses the logger from slog_scope, so set a logger there
    let _guard = slog_scope::set_global_logger(logger);

    cli::main();
}

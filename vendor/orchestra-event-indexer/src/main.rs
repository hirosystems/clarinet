#[macro_use]
extern crate slog_scope;

#[macro_use]
extern crate serde;

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate serde_json;

pub mod block;
mod cli;
pub mod config;

use slog::Drain;
use std::sync::Mutex;

fn main() {
    let logger = slog::Logger::root(
        Mutex::new(slog_json::Json::default(std::io::stderr())).map(slog::Fuse),
        slog::o!("version" => env!("CARGO_PKG_VERSION")),
    );
    // slog_stdlog uses the logger from slog_scope, so set a logger there
    let _guard = slog_scope::set_global_logger(logger);

    cli::main();
}

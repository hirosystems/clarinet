#[macro_use]
extern crate serde_json;

#[macro_use]
extern crate slog_scope;

extern crate serde;
extern crate serde_derive;
extern crate slog;

use slog_term;
use slog_async;

pub mod block;
mod cli;
pub mod config;

use slog::*;
use slog_atomic::*;
use std::sync::Mutex;

fn main() {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = Mutex::new(slog_term::FullFormat::new(decorator).build()).fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    let drain = AtomicSwitch::new(drain);

    // Get a root logger that will log into a given drain.
    //
    // Note `o!` macro for more natural `OwnedKeyValue` sequence building.
    let root = Logger::root(
        drain.fuse(),
        o!("version" => env!("CARGO_PKG_VERSION")),
    );

    // slog_stdlog uses the logger from slog_scope, so set a logger there
    let _guard = slog_scope::set_global_logger(root);

    cli::main();
}

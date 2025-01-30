#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
// todo(ludo): would love to eliminate these directives at some point.

#[cfg(test)]
pub mod test_fixtures;

#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate prettytable;
#[macro_use]
extern crate hiro_system_kit;

#[macro_use]
mod uprint;

pub mod analysis;
pub mod frontend;
pub mod repl;
pub mod utils;

use frontend::Terminal;
use pico_args::Arguments;
use repl::{settings, Session, SessionSettings};
use std::env;

fn main() {
    let mut args = Arguments::from_env();
    let subcommand = args.subcommand().unwrap().unwrap_or_default();
    let code = args.subcommand().unwrap();

    let settings = SessionSettings {
        include_boot_contracts: vec!["costs".into(), "costs-2".into(), "costs-3".into()],
        ..Default::default()
    };

    match code {
        Some(code_str) => {
            let mut session = Session::new(settings);
            match session.start() {
                Ok(_) => {}
                Err(e) => {
                    println!("{}", e);
                    std::process::exit(1);
                }
            };

            let (_, output, _) = session.process_console_input(&code_str);
            for line in output {
                println!("{}", line);
            }
        }
        None => loop {
            let mut terminal = Terminal::new(settings.clone(), None);
            if !terminal.start() {
                break;
            }
        },
    }
}

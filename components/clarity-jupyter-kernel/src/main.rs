#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused_mut)]
// todo(ludo): would love to eliminate these directives at some point.

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate json;
#[macro_use]
extern crate failure;

mod jupyter;

use failure::Error;
use jupyter::{control_file, core, install};

fn run(control_file_name: &str) -> Result<(), Error> {
    let config = control_file::Control::parse_file(&control_file_name)?;
    let server = core::Server::start(&config)?;
    server.wait_for_shutdown();
    Ok(())
}

fn main() -> Result<(), Error> {
    let mut args = std::env::args();
    let bin = args.next().unwrap();
    if let Some(arg) = args.next() {
        match arg.as_str() {
            "--control_file" => {
                return run(&args
                    .next()
                    .ok_or_else(|| format_err!("Missing control file"))?);
            }
            "--install" => return install::install(),
            "--uninstall" => return install::uninstall(),
            "--help" => {}
            x => bail!("Unrecognised option {}", x),
        }
    }
    println!("To install, run:\n  {} --install", bin);
    println!("To uninstall, run:\n  {} --uninstall", bin);
    Ok(())
}

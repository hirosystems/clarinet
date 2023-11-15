use std::fs::create_dir_all;

use anyhow::Result as AnyResult;
use xtaskops::ops::{clean_files, cmd, confirm, remove_dir};

// Inspired by xtaskops main and coverage methods
// https://github.com/jondot/xtaskops/blob/95153c35af57a682ea403281b4d620a703570b10/xtaskops/src/tasks.rs#L262
// Simplified for our use case and passing extra arguments to cargo test

pub fn main() -> AnyResult<()> {
    use clap::{Arg, Command};
    let cli = clap::Command::new("xtask").subcommand(
        Command::new("coverage").arg(
            Arg::new("dev")
                .long("dev")
                .short('d')
                .num_args(0)
                .help("generate an html report"),
        ),
    );
    let matches = cli.get_matches();

    // let root = root_dir();
    let res = match matches.subcommand() {
        Some(("coverage", sm)) => coverage(sm.get_flag("dev")),
        _ => Ok(()),
    };
    res
}

pub fn coverage(devmode: bool) -> AnyResult<()> {
    remove_dir("coverage")?;
    create_dir_all("coverage")?;

    println!("=== running coverage ===");

    cmd!(
        "cargo",
        "test",
        "--workspace",
        "--locked",
        "--exclude",
        "clarinet-sdk-wasm",
        "--exclude",
        "clarity-jupyter-kernel",
        "--exclude",
        "xtask",
    )
    .env("CARGO_INCREMENTAL", "0")
    .env("RUSTFLAGS", "-Cinstrument-coverage")
    .env("LLVM_PROFILE_FILE", "cargo-test-%p-%m.profraw")
    .run()?;
    println!("ok.");

    println!("=== generating report ===");
    let (fmt, file) = if devmode {
        ("html", "coverage/html")
    } else {
        ("lcov", "lcov.info")
    };
    cmd!(
        "grcov",
        ".",
        "--binary-path",
        "./target/debug/deps/",
        "-s",
        ".",
        "-t",
        fmt,
        "--branch",
        "--ignore-not-existing",
        "--ignore",
        "../*",
        "--ignore",
        "/*",
        "--ignore",
        "xtask/*",
        "-o",
        file,
    )
    .run()?;
    println!("ok.");

    println!("=== cleaning up ===");
    clean_files("**/*.profraw")?;
    println!("ok.");
    if devmode {
        if confirm("open report folder?") {
            cmd!("open", file).run()?;
        } else {
            println!("report location: {file}");
        }
    }

    Ok(())
}

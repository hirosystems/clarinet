//! Installs clarinet.

use std::str;

use anyhow::{Context, Result};

use crate::not_bash::run;

// Latest stable, feel free to send a PR if this lags behind.
const REQUIRED_RUST_VERSION: u32 = 41;

pub struct InstallCmd {
    pub clarinet: Option<ClarinetOpt>,
}

pub struct ClarinetOpt {
    pub jemalloc: bool,
}

impl InstallCmd {
    pub fn run(self) -> Result<()> {
        if let Some(clarinet) = self.clarinet {
            install_clarinet(clarinet).context("install clarinet")?;
        }
        Ok(())
    }
}

fn install_clarinet(opts: ClarinetOpt) -> Result<()> {
    let mut old_rust = false;
    if let Ok(stdout) = run!("cargo --version") {
        if !check_version(&stdout, REQUIRED_RUST_VERSION) {
            old_rust = true;
        }
    }

    if old_rust {
        eprintln!(
            "\nWARNING: at least rust 1.{}.0 is required to compile clarinet\n",
            REQUIRED_RUST_VERSION,
        )
    }

    let jemalloc = if opts.jemalloc { "--features jemalloc" } else { "" };
    let res = run!("cargo install --path . --locked --force {}", jemalloc);

    if res.is_err() && old_rust {
        eprintln!(
            "\nWARNING: at least rust 1.{}.0 is required to compile clarinet\n",
            REQUIRED_RUST_VERSION,
        );
    }

    res.map(drop)
}

fn check_version(version_output: &str, min_minor_version: u32) -> bool {
    // Parse second the number out of
    //      cargo 1.39.0-beta (1c6ec66d5 2019-09-30)
    let minor: Option<u32> = version_output.split('.').nth(1).and_then(|it| it.parse().ok());
    match minor {
        None => true,
        Some(minor) => minor >= min_minor_version,
    }
}

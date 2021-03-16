// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

use deno_core::url::Url;
use deno_runtime::permissions::PermissionsOptions;
use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Flags {
  /// Vector of CLI arguments - these are user script arguments, all Deno
  /// specific flags are removed.
  pub argv: Vec<String>,

  pub allow_env: bool,
  pub allow_hrtime: bool,
  pub allow_net: Option<Vec<String>>,
  pub allow_plugin: bool,
  pub allow_read: Option<Vec<PathBuf>>,
  pub allow_run: bool,
  pub allow_write: Option<Vec<PathBuf>>,
  pub location: Option<Url>,
  pub cache_blocklist: Vec<String>,
  pub ca_file: Option<String>,
  pub cached_only: bool,
  pub config_path: Option<String>,
  pub coverage_dir: Option<String>,
  pub ignore: Vec<PathBuf>,
  pub import_map_path: Option<String>,
  pub inspect: Option<SocketAddr>,
  pub inspect_brk: Option<SocketAddr>,
  pub lock: Option<PathBuf>,
  pub lock_write: bool,
  pub no_check: bool,
  pub no_prompts: bool,
  pub no_remote: bool,
  pub reload: bool,
  pub repl: bool,
  pub seed: Option<u64>,
  pub unstable: bool,
  pub v8_flags: Vec<String>,
  pub version: bool,
  pub watch: bool,
}

fn join_paths(allowlist: &[PathBuf], d: &str) -> String {
  allowlist
    .iter()
    .map(|path| path.to_str().unwrap().to_string())
    .collect::<Vec<String>>()
    .join(d)
}

impl Flags {
  /// Return list of permission arguments that are equivalent
  /// to the ones used to create `self`.
  pub fn to_permission_args(&self) -> Vec<String> {
    let mut args = vec![];

    match &self.allow_read {
      Some(read_allowlist) if read_allowlist.is_empty() => {
        args.push("--allow-read".to_string());
      }
      Some(read_allowlist) => {
        let s = format!("--allow-read={}", join_paths(read_allowlist, ","));
        args.push(s);
      }
      _ => {}
    }

    match &self.allow_write {
      Some(write_allowlist) if write_allowlist.is_empty() => {
        args.push("--allow-write".to_string());
      }
      Some(write_allowlist) => {
        let s = format!("--allow-write={}", join_paths(write_allowlist, ","));
        args.push(s);
      }
      _ => {}
    }

    match &self.allow_net {
      Some(net_allowlist) if net_allowlist.is_empty() => {
        args.push("--allow-net".to_string());
      }
      Some(net_allowlist) => {
        let s = format!("--allow-net={}", net_allowlist.join(","));
        args.push(s);
      }
      _ => {}
    }

    if self.allow_env {
      args.push("--allow-env".to_string());
    }

    if self.allow_run {
      args.push("--allow-run".to_string());
    }

    if self.allow_plugin {
      args.push("--allow-plugin".to_string());
    }

    if self.allow_hrtime {
      args.push("--allow-hrtime".to_string());
    }

    args
  }
}

impl From<Flags> for PermissionsOptions {
  fn from(flags: Flags) -> Self {
    Self {
      allow_env: flags.allow_env,
      allow_hrtime: flags.allow_hrtime,
      allow_net: flags.allow_net,
      allow_plugin: flags.allow_plugin,
      allow_read: flags.allow_read,
      allow_run: flags.allow_run,
      allow_write: flags.allow_write,
    }
  }
}

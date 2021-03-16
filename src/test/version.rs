// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

pub const TYPESCRIPT: &str = "4.1.3";

pub fn deno() -> String {
  let semver = env!("CARGO_PKG_VERSION");
  option_env!("DENO_CANARY").map_or(semver.to_string(), |_| {
    format!("{}", semver)
  })
}

// allow(dead_code) because denort does not use this.
#[allow(dead_code)]
pub fn is_canary() -> bool {
  option_env!("DENO_CANARY").is_some()
}

pub fn get_user_agent() -> String {
  format!("Deno/{}", deno())
}

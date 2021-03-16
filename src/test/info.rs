// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

use super::colors;
use super::media_type::serialize_media_type;
use super::media_type::MediaType;

use deno_core::resolve_url;
use deno_core::serde::Serialize;
use deno_core::ModuleSpecifier;
use std::collections::HashSet;
use std::fmt;
use std::iter::Iterator;
use std::path::PathBuf;

const SIBLING_CONNECTOR: char = '├';
const LAST_SIBLING_CONNECTOR: char = '└';
const CHILD_DEPS_CONNECTOR: char = '┬';
const CHILD_NO_DEPS_CONNECTOR: char = '─';
const VERTICAL_CONNECTOR: char = '│';
const EMPTY_CONNECTOR: char = ' ';

#[derive(Debug, Serialize, Ord, PartialOrd, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ModuleGraphInfoDep {
  pub specifier: String,
  pub is_dynamic: bool,
  #[serde(rename = "code", skip_serializing_if = "Option::is_none")]
  pub maybe_code: Option<ModuleSpecifier>,
  #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
  pub maybe_type: Option<ModuleSpecifier>,
}

impl ModuleGraphInfoDep {
  fn write_info<S: AsRef<str> + fmt::Display + Clone>(
    &self,
    f: &mut fmt::Formatter,
    prefix: S,
    last: bool,
    modules: &[ModuleGraphInfoMod],
    seen: &mut HashSet<ModuleSpecifier>,
  ) -> fmt::Result {
    let maybe_code = self
      .maybe_code
      .as_ref()
      .and_then(|s| modules.iter().find(|m| &m.specifier == s));
    let maybe_type = self
      .maybe_type
      .as_ref()
      .and_then(|s| modules.iter().find(|m| &m.specifier == s));
    match (maybe_code, maybe_type) {
      (Some(code), Some(types)) => {
        code.write_info(f, prefix.clone(), false, false, modules, seen)?;
        types.write_info(f, prefix, last, true, modules, seen)
      }
      (Some(code), None) => {
        code.write_info(f, prefix, last, false, modules, seen)
      }
      (None, Some(types)) => {
        types.write_info(f, prefix, last, true, modules, seen)
      }
      _ => Ok(()),
    }
  }
}

#[derive(Debug, Serialize, Ord, PartialOrd, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ModuleGraphInfoMod {
  pub specifier: ModuleSpecifier,
  pub dependencies: Vec<ModuleGraphInfoDep>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub size: Option<usize>,
  #[serde(
    skip_serializing_if = "Option::is_none",
    serialize_with = "serialize_media_type"
  )]
  pub media_type: Option<MediaType>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub local: Option<PathBuf>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub checksum: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub emit: Option<PathBuf>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub map: Option<PathBuf>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub error: Option<String>,
}

impl Default for ModuleGraphInfoMod {
  fn default() -> Self {
    ModuleGraphInfoMod {
      specifier: resolve_url("https://deno.land/x/mod.ts").unwrap(),
      dependencies: Vec::new(),
      size: None,
      media_type: None,
      local: None,
      checksum: None,
      emit: None,
      map: None,
      error: None,
    }
  }
}

impl ModuleGraphInfoMod {
  fn write_info<S: AsRef<str> + fmt::Display>(
    &self,
    f: &mut fmt::Formatter,
    prefix: S,
    last: bool,
    type_dep: bool,
    modules: &[ModuleGraphInfoMod],
    seen: &mut HashSet<ModuleSpecifier>,
  ) -> fmt::Result {
    let was_seen = seen.contains(&self.specifier);
    let sibling_connector = if last {
      LAST_SIBLING_CONNECTOR
    } else {
      SIBLING_CONNECTOR
    };
    let child_connector = if self.dependencies.is_empty() || was_seen {
      CHILD_NO_DEPS_CONNECTOR
    } else {
      CHILD_DEPS_CONNECTOR
    };
    let (size, specifier) = if self.error.is_some() {
      (
        colors::red_bold(" (error)").to_string(),
        colors::red(&self.specifier).to_string(),
      )
    } else if was_seen {
      let name = if type_dep {
        colors::italic_gray(&self.specifier).to_string()
      } else {
        colors::gray(&self.specifier).to_string()
      };
      (colors::gray(" *").to_string(), name)
    } else {
      let name = if type_dep {
        colors::italic(&self.specifier).to_string()
      } else {
        self.specifier.to_string()
      };
      (
        colors::gray(format!(
          " ({})",
          human_size(self.size.unwrap_or(0) as f64)
        ))
        .to_string(),
        name,
      )
    };

    seen.insert(self.specifier.clone());

    writeln!(
      f,
      "{} {}{}",
      colors::gray(format!(
        "{}{}─{}",
        prefix, sibling_connector, child_connector
      )),
      specifier,
      size
    )?;

    if !was_seen {
      let mut prefix = prefix.to_string();
      if last {
        prefix.push(EMPTY_CONNECTOR);
      } else {
        prefix.push(VERTICAL_CONNECTOR);
      }
      prefix.push(EMPTY_CONNECTOR);
      let dep_count = self.dependencies.len();
      for (idx, dep) in self.dependencies.iter().enumerate() {
        dep.write_info(f, &prefix, idx == dep_count - 1, modules, seen)?;
      }
    }

    Ok(())
  }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleGraphInfo {
  pub root: ModuleSpecifier,
  pub modules: Vec<ModuleGraphInfoMod>,
  pub size: usize,
}

impl fmt::Display for ModuleGraphInfo {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let root = self
      .modules
      .iter()
      .find(|m| m.specifier == self.root)
      .unwrap();
    if let Some(err) = &root.error {
      writeln!(f, "{} {}", colors::red("error:"), err)
    } else {
      if let Some(local) = &root.local {
        writeln!(f, "{} {}", colors::bold("local:"), local.to_string_lossy())?;
      }
      if let Some(media_type) = &root.media_type {
        writeln!(f, "{} {}", colors::bold("type:"), media_type)?;
      }
      if let Some(emit) = &root.emit {
        writeln!(f, "{} {}", colors::bold("emit:"), emit.to_string_lossy())?;
      }
      if let Some(map) = &root.map {
        writeln!(f, "{} {}", colors::bold("map:"), map.to_string_lossy())?;
      }
      let dep_count = self.modules.len() - 1;
      writeln!(
        f,
        "{} {} unique {}",
        colors::bold("dependencies:"),
        dep_count,
        colors::gray(format!("(total {})", human_size(self.size as f64)))
      )?;
      writeln!(
        f,
        "\n{} {}",
        self.root,
        colors::gray(format!(
          "({})",
          human_size(root.size.unwrap_or(0) as f64)
        ))
      )?;
      let mut seen = HashSet::new();
      let dep_len = root.dependencies.len();
      for (idx, dep) in root.dependencies.iter().enumerate() {
        dep.write_info(f, "", idx == dep_len - 1, &self.modules, &mut seen)?;
      }
      Ok(())
    }
  }
}

/// An entry in the `ModuleInfoMap` the provides the size of the module and
/// a vector of its dependencies, which should also be available as entries
/// in the map.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleInfoMapItem {
  pub deps: Vec<ModuleSpecifier>,
  pub size: usize,
}

/// A function that converts a float to a string the represents a human
/// readable version of that number.
pub fn human_size(size: f64) -> String {
  let negative = if size.is_sign_positive() { "" } else { "-" };
  let size = size.abs();
  let units = ["B", "KB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"];
  if size < 1_f64 {
    return format!("{}{}{}", negative, size, "B");
  }
  let delimiter = 1024_f64;
  let exponent = std::cmp::min(
    (size.ln() / delimiter.ln()).floor() as i32,
    (units.len() - 1) as i32,
  );
  let pretty_bytes = format!("{:.2}", size / delimiter.powi(exponent))
    .parse::<f64>()
    .unwrap()
    * 1_f64;
  let unit = units[exponent as usize];
  format!("{}{}{}", negative, pretty_bytes, unit)
}

// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

use deno_core::serde::Serialize;
use deno_core::serde::Serializer;
use deno_core::ModuleSpecifier;
use std::fmt;
use std::path::Path;
use std::path::PathBuf;

// Warning! The values in this enum are duplicated in tsc/99_main_compiler.js
// Update carefully!
#[allow(non_camel_case_types)]
#[repr(i32)]
#[derive(Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub enum MediaType {
  JavaScript = 0,
  JSX = 1,
  TypeScript = 2,
  Dts = 3,
  TSX = 4,
  Json = 5,
  Wasm = 6,
  TsBuildInfo = 7,
  SourceMap = 8,
  Unknown = 9,
}

impl fmt::Display for MediaType {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let value = match self {
      MediaType::JavaScript => "JavaScript",
      MediaType::JSX => "JSX",
      MediaType::TypeScript => "TypeScript",
      MediaType::Dts => "Dts",
      MediaType::TSX => "TSX",
      MediaType::Json => "Json",
      MediaType::Wasm => "Wasm",
      MediaType::TsBuildInfo => "TsBuildInfo",
      MediaType::SourceMap => "SourceMap",
      MediaType::Unknown => "Unknown",
    };
    write!(f, "{}", value)
  }
}

impl<'a> From<&'a Path> for MediaType {
  fn from(path: &'a Path) -> Self {
    MediaType::from_path(path)
  }
}

impl<'a> From<&'a PathBuf> for MediaType {
  fn from(path: &'a PathBuf) -> Self {
    MediaType::from_path(path)
  }
}

impl<'a> From<&'a String> for MediaType {
  fn from(specifier: &'a String) -> Self {
    MediaType::from_path(&PathBuf::from(specifier))
  }
}

impl<'a> From<&'a ModuleSpecifier> for MediaType {
  fn from(specifier: &'a ModuleSpecifier) -> Self {
    let path = if specifier.scheme() == "file" {
      if let Ok(path) = specifier.to_file_path() {
        path
      } else {
        PathBuf::from(specifier.path())
      }
    } else {
      PathBuf::from(specifier.path())
    };
    MediaType::from_path(&path)
  }
}

impl Default for MediaType {
  fn default() -> Self {
    MediaType::Unknown
  }
}

impl MediaType {
  fn from_path(path: &Path) -> Self {
    match path.extension() {
      None => match path.file_name() {
        None => MediaType::Unknown,
        Some(os_str) => match os_str.to_str() {
          Some(".tsbuildinfo") => MediaType::TsBuildInfo,
          _ => MediaType::Unknown,
        },
      },
      Some(os_str) => match os_str.to_str() {
        Some("ts") => {
          if let Some(os_str) = path.file_stem() {
            if let Some(file_name) = os_str.to_str() {
              if file_name.ends_with(".d") {
                return MediaType::Dts;
              }
            }
          }
          MediaType::TypeScript
        }
        Some("tsx") => MediaType::TSX,
        Some("js") => MediaType::JavaScript,
        Some("jsx") => MediaType::JSX,
        Some("mjs") => MediaType::JavaScript,
        Some("cjs") => MediaType::JavaScript,
        Some("json") => MediaType::Json,
        Some("wasm") => MediaType::Wasm,
        Some("tsbuildinfo") => MediaType::TsBuildInfo,
        Some("map") => MediaType::SourceMap,
        _ => MediaType::Unknown,
      },
    }
  }

  /// Convert a MediaType to a `ts.Extension`.
  ///
  /// *NOTE* This is defined in TypeScript as a string based enum.  Changes to
  /// that enum in TypeScript should be reflected here.
  pub fn as_ts_extension(&self) -> &str {
    match self {
      MediaType::JavaScript => ".js",
      MediaType::JSX => ".jsx",
      MediaType::TypeScript => ".ts",
      MediaType::Dts => ".d.ts",
      MediaType::TSX => ".tsx",
      MediaType::Json => ".json",
      // TypeScript doesn't have an "unknown", so we will treat WASM as JS for
      // mapping purposes, though in reality, it is unlikely to ever be passed
      // to the compiler.
      MediaType::Wasm => ".js",
      MediaType::TsBuildInfo => ".tsbuildinfo",
      // TypeScript doesn't have an "source map", so we will treat SourceMap as
      // JS for mapping purposes, though in reality, it is unlikely to ever be
      // passed to the compiler.
      MediaType::SourceMap => ".js",
      // TypeScript doesn't have an "unknown", so we will treat unknowns as JS
      // for mapping purposes, though in reality, it is unlikely to ever be
      // passed to the compiler.
      MediaType::Unknown => ".js",
    }
  }

  /// Map the media type to a `ts.ScriptKind`
  pub fn as_ts_script_kind(&self) -> i32 {
    match self {
      MediaType::JavaScript => 1,
      MediaType::JSX => 2,
      MediaType::TypeScript => 3,
      MediaType::Dts => 3,
      MediaType::TSX => 4,
      MediaType::Json => 5,
      _ => 0,
    }
  }
}

impl Serialize for MediaType {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    let value = match self {
      MediaType::JavaScript => 0_i32,
      MediaType::JSX => 1_i32,
      MediaType::TypeScript => 2_i32,
      MediaType::Dts => 3_i32,
      MediaType::TSX => 4_i32,
      MediaType::Json => 5_i32,
      MediaType::Wasm => 6_i32,
      MediaType::TsBuildInfo => 7_i32,
      MediaType::SourceMap => 8_i32,
      MediaType::Unknown => 9_i32,
    };
    Serialize::serialize(&value, serializer)
  }
}

/// Serialize a `MediaType` enum into a human readable string.  The default
/// serialization for media types is and integer.
///
/// TODO(@kitsonk) remove this once we stop sending MediaType into tsc.
pub fn serialize_media_type<S>(
  mmt: &Option<MediaType>,
  s: S,
) -> Result<S::Ok, S::Error>
where
  S: Serializer,
{
  match *mmt {
    Some(ref mt) => s.serialize_some(&mt.to_string()),
    None => s.serialize_none(),
  }
}

// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

use deno_core::error::AnyError;
pub use deno_core::normalize_path;
use deno_runtime::deno_crypto::rand;
use std::env::current_dir;
use std::fs::OpenOptions;
use std::io::{Error, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn atomic_write_file<T: AsRef<[u8]>>(
  filename: &Path,
  data: T,
  mode: u32,
) -> std::io::Result<()> {
  let rand: String = (0..4)
    .map(|_| format!("{:02x}", rand::random::<u8>()))
    .collect();
  let extension = format!("{}.tmp", rand);
  let tmp_file = filename.with_extension(extension);
  write_file(&tmp_file, data, mode)?;
  std::fs::rename(tmp_file, filename)?;
  Ok(())
}

pub fn write_file<T: AsRef<[u8]>>(
  filename: &Path,
  data: T,
  mode: u32,
) -> std::io::Result<()> {
  write_file_2(filename, data, true, mode, true, false)
}

pub fn write_file_2<T: AsRef<[u8]>>(
  filename: &Path,
  data: T,
  update_mode: bool,
  mode: u32,
  is_create: bool,
  is_append: bool,
) -> std::io::Result<()> {
  let mut file = OpenOptions::new()
    .read(false)
    .write(true)
    .append(is_append)
    .truncate(!is_append)
    .create(is_create)
    .open(filename)?;

  if update_mode {
    #[cfg(unix)]
    {
      use std::os::unix::fs::PermissionsExt;
      let mode = mode & 0o777;
      let permissions = PermissionsExt::from_mode(mode);
      file.set_permissions(permissions)?;
    }
    #[cfg(not(unix))]
    let _ = mode;
  }

  file.write_all(data.as_ref())
}

/// Similar to `std::fs::canonicalize()` but strips UNC prefixes on Windows.
pub fn canonicalize_path(path: &Path) -> Result<PathBuf, Error> {
  let mut canonicalized_path = path.canonicalize()?;
  if cfg!(windows) {
    canonicalized_path = PathBuf::from(
      canonicalized_path
        .display()
        .to_string()
        .trim_start_matches("\\\\?\\"),
    );
  }
  Ok(canonicalized_path)
}

pub fn resolve_from_cwd(path: &Path) -> Result<PathBuf, AnyError> {
  let resolved_path = if path.is_absolute() {
    path.to_owned()
  } else {
    let cwd = current_dir().unwrap();
    cwd.join(path)
  };

  Ok(normalize_path(&resolved_path))
}

/// Checks if the path has extension Deno supports.
pub fn is_supported_ext(path: &Path) -> bool {
  if let Some(ext) = get_extension(path) {
    matches!(ext.as_str(), "ts" | "tsx" | "js" | "jsx" | "mjs")
  } else {
    false
  }
}

/// This function is similar to is_supported_ext but adds additional extensions
/// supported by `deno fmt`.
pub fn is_supported_ext_fmt(path: &Path) -> bool {
  if let Some(ext) = get_extension(path) {
    matches!(
      ext.as_str(),
      "ts" | "tsx" | "js" | "jsx" | "mjs" | "md" | "json" | "jsonc"
    )
  } else {
    false
  }
}

/// Get the extension of a file in lowercase.
pub fn get_extension(file_path: &Path) -> Option<String> {
  return file_path
    .extension()
    .and_then(|e| e.to_str())
    .map(|e| e.to_lowercase());
}

/// Collects file paths that satisfy the given predicate, by recursively walking `files`.
/// If the walker visits a path that is listed in `ignore`, it skips descending into the directory.
pub fn collect_files<P>(
  files: &[PathBuf],
  ignore: &[PathBuf],
  predicate: P,
) -> Result<Vec<PathBuf>, AnyError>
where
  P: Fn(&Path) -> bool,
{
  let mut target_files = Vec::new();

  // retain only the paths which exist and ignore the rest
  let canonicalized_ignore: Vec<PathBuf> = ignore
    .iter()
    .filter_map(|i| i.canonicalize().ok())
    .collect();

  let cur_dir = [std::env::current_dir()?];
  let files = if files.is_empty() { &cur_dir } else { files };

  for file in files {
    for entry in WalkDir::new(file)
      .into_iter()
      .filter_entry(|e| {
        e.path().canonicalize().map_or(false, |c| {
          !canonicalized_ignore.iter().any(|i| c.starts_with(i))
        })
      })
      .filter_map(|e| match e {
        Ok(e) if !e.file_type().is_dir() && predicate(e.path()) => Some(e),
        _ => None,
      })
    {
      target_files.push(entry.into_path().canonicalize()?)
    }
  }

  Ok(target_files)
}

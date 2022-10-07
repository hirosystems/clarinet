// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

use deno_core::anyhow::Context;
use deno_core::error::{uri_error, AnyError};
pub use deno_core::normalize_path;
use deno_core::ModuleSpecifier;
use deno_crypto::rand;
use std::borrow::Cow;
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

pub fn write_file<T: AsRef<[u8]>>(filename: &Path, data: T, mode: u32) -> std::io::Result<()> {
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
    let path = path.canonicalize()?;
    #[cfg(windows)]
    return Ok(strip_unc_prefix(path));
    #[cfg(not(windows))]
    return Ok(path);
}

#[cfg(windows)]
fn strip_unc_prefix(path: PathBuf) -> PathBuf {
    use std::path::Component;
    use std::path::Prefix;

    let mut components = path.components();
    match components.next() {
        Some(Component::Prefix(prefix)) => {
            match prefix.kind() {
                // \\?\device
                Prefix::Verbatim(device) => {
                    let mut path = PathBuf::new();
                    path.push(format!(r"\\{}\", device.to_string_lossy()));
                    path.extend(components.filter(|c| !matches!(c, Component::RootDir)));
                    path
                }
                // \\?\c:\path
                Prefix::VerbatimDisk(_) => {
                    let mut path = PathBuf::new();
                    path.push(prefix.as_os_str().to_string_lossy().replace(r"\\?\", ""));
                    path.extend(components);
                    path
                }
                // \\?\UNC\hostname\share_name\path
                Prefix::VerbatimUNC(hostname, share_name) => {
                    let mut path = PathBuf::new();
                    path.push(format!(
                        r"\\{}\{}\",
                        hostname.to_string_lossy(),
                        share_name.to_string_lossy()
                    ));
                    path.extend(components.filter(|c| !matches!(c, Component::RootDir)));
                    path
                }
                _ => path,
            }
        }
        _ => path,
    }
}

pub fn resolve_from_cwd(path: &Path) -> Result<PathBuf, AnyError> {
    let resolved_path = if path.is_absolute() {
        path.to_owned()
    } else {
        let cwd = current_dir().context("Failed to get current working directory")?;
        cwd.join(path)
    };

    Ok(normalize_path(&resolved_path))
}

/// Checks if the path has extension Deno supports.
pub fn is_supported_ext(path: &Path) -> bool {
    if let Some(ext) = get_extension(path) {
        matches!(
            ext.as_str(),
            "ts" | "tsx" | "js" | "jsx" | "mjs" | "mts" | "cjs" | "cts"
        )
    } else {
        false
    }
}

/// Checks if the path has a basename and extension Deno supports for tests.
pub fn is_supported_test_path(path: &Path) -> bool {
    if let Some(name) = path.file_stem() {
        let basename = name.to_string_lossy();
        (basename.ends_with("_test") || basename.ends_with(".test") || basename == "test")
            && is_supported_ext(path)
    } else {
        false
    }
}

/// Checks if the path has a basename and extension Deno supports for benches.
pub fn is_supported_bench_path(path: &Path) -> bool {
    if let Some(name) = path.file_stem() {
        let basename = name.to_string_lossy();
        (basename.ends_with("_bench") || basename.ends_with(".bench") || basename == "bench")
            && is_supported_ext(path)
    } else {
        false
    }
}

/// Checks if the path has an extension Deno supports for tests.
pub fn is_supported_test_ext(path: &Path) -> bool {
    if let Some(ext) = get_extension(path) {
        matches!(
            ext.as_str(),
            "ts" | "tsx"
                | "js"
                | "jsx"
                | "mjs"
                | "mts"
                | "cjs"
                | "cts"
                | "md"
                | "mkd"
                | "mkdn"
                | "mdwn"
                | "mdown"
                | "markdown"
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
        .filter_map(|i| canonicalize_path(i).ok())
        .collect();

    for file in files {
        for entry in WalkDir::new(file)
            .into_iter()
            .filter_entry(|e| {
                canonicalize_path(e.path()).map_or(false, |c| {
                    !canonicalized_ignore.iter().any(|i| c.starts_with(i))
                })
            })
            .filter_map(|e| match e {
                Ok(e) if !e.file_type().is_dir() && predicate(e.path()) => Some(e),
                _ => None,
            })
        {
            target_files.push(canonicalize_path(entry.path())?)
        }
    }

    Ok(target_files)
}

/// Collects module specifiers that satisfy the given predicate as a file path, by recursively walking `include`.
/// Specifiers that start with http and https are left intact.
pub fn collect_specifiers<P>(
    include: Vec<String>,
    ignore: &[PathBuf],
    predicate: P,
) -> Result<Vec<ModuleSpecifier>, AnyError>
where
    P: Fn(&Path) -> bool,
{
    let mut prepared = vec![];

    let root_path = current_dir()?;
    for path in include {
        let lowercase_path = path.to_lowercase();
        if lowercase_path.starts_with("http://") || lowercase_path.starts_with("https://") {
            let url = ModuleSpecifier::parse(&path)?;
            prepared.push(url);
            continue;
        }

        let p = if lowercase_path.starts_with("file://") {
            specifier_to_file_path(&ModuleSpecifier::parse(&path)?)?
        } else {
            root_path.join(path)
        };
        let p = normalize_path(&p);
        if p.is_dir() {
            let test_files = collect_files(&[p], ignore, &predicate).unwrap();
            let mut test_files_as_urls = test_files
                .iter()
                .map(|f| ModuleSpecifier::from_file_path(f).unwrap())
                .collect::<Vec<ModuleSpecifier>>();

            test_files_as_urls.sort();
            prepared.extend(test_files_as_urls);
        } else {
            let url = ModuleSpecifier::from_file_path(p).unwrap();
            prepared.push(url);
        }
    }

    Ok(prepared)
}

/// Asynchronously removes a directory and all its descendants, but does not error
/// when the directory does not exist.
pub async fn remove_dir_all_if_exists(path: &Path) -> std::io::Result<()> {
    let result = tokio::fs::remove_dir_all(path).await;
    match result {
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        _ => result,
    }
}

/// Attempts to convert a specifier to a file path. By default, uses the Url
/// crate's `to_file_path()` method, but falls back to try and resolve unix-style
/// paths on Windows.
pub fn specifier_to_file_path(specifier: &ModuleSpecifier) -> Result<PathBuf, AnyError> {
    let result = if cfg!(windows) {
        match specifier.to_file_path() {
            Ok(path) => Ok(path),
            Err(()) => {
                // This might be a unix-style path which is used in the tests even on Windows.
                // Attempt to see if we can convert it to a `PathBuf`. This code should be removed
                // once/if https://github.com/servo/rust-url/issues/730 is implemented.
                if specifier.scheme() == "file"
                    && specifier.host().is_none()
                    && specifier.port().is_none()
                    && specifier.path_segments().is_some()
                {
                    let path_str = specifier.path();
                    match String::from_utf8(
                        percent_encoding::percent_decode(path_str.as_bytes()).collect(),
                    ) {
                        Ok(path_str) => Ok(PathBuf::from(path_str)),
                        Err(_) => Err(()),
                    }
                } else {
                    Err(())
                }
            }
        }
    } else {
        specifier.to_file_path()
    };
    match result {
        Ok(path) => Ok(path),
        Err(()) => Err(uri_error(format!(
            "Invalid file path.\n  Specifier: {}",
            specifier
        ))),
    }
}

/// Ensures a specifier that will definitely be a directory has a trailing slash.
pub fn ensure_directory_specifier(mut specifier: ModuleSpecifier) -> ModuleSpecifier {
    let path = specifier.path();
    if !path.ends_with('/') {
        let new_path = format!("{}/", path);
        specifier.set_path(&new_path);
    }
    specifier
}

/// Gets the parent of this module specifier.
pub fn specifier_parent(specifier: &ModuleSpecifier) -> ModuleSpecifier {
    let mut specifier = specifier.clone();
    // don't use specifier.segments() because it will strip the leading slash
    let mut segments = specifier.path().split('/').collect::<Vec<_>>();
    if segments.iter().all(|s| s.is_empty()) {
        return specifier;
    }
    if let Some(last) = segments.last() {
        if last.is_empty() {
            segments.pop();
        }
        segments.pop();
        let new_path = format!("{}/", segments.join("/"));
        specifier.set_path(&new_path);
    }
    specifier
}

/// `from.make_relative(to)` but with fixes.
pub fn relative_specifier(from: &ModuleSpecifier, to: &ModuleSpecifier) -> Option<String> {
    let is_dir = to.path().ends_with('/');

    if is_dir && from == to {
        return Some("./".to_string());
    }

    // workaround using parent directory until https://github.com/servo/rust-url/pull/754 is merged
    let from = if !from.path().ends_with('/') {
        if let Some(end_slash) = from.path().rfind('/') {
            let mut new_from = from.clone();
            new_from.set_path(&from.path()[..end_slash + 1]);
            Cow::Owned(new_from)
        } else {
            Cow::Borrowed(from)
        }
    } else {
        Cow::Borrowed(from)
    };

    // workaround for url crate not adding a trailing slash for a directory
    // it seems to be fixed once a version greater than 2.2.2 is released
    let mut text = from.make_relative(to)?;
    if is_dir && !text.ends_with('/') && to.query().is_none() {
        text.push('/');
    }

    Some(if text.starts_with("../") || text.starts_with("./") {
        text
    } else {
        format!("./{}", text)
    })
}

/// This function checks if input path has trailing slash or not. If input path
/// has trailing slash it will return true else it will return false.
pub fn path_has_trailing_slash(path: &Path) -> bool {
    if let Some(path_str) = path.to_str() {
        if cfg!(windows) {
            path_str.ends_with('\\')
        } else {
            path_str.ends_with('/')
        }
    } else {
        false
    }
}

/// Gets a path with the specified file stem suffix.
///
/// Ex. `file.ts` with suffix `_2` returns `file_2.ts`
pub fn path_with_stem_suffix(path: &Path, suffix: &str) -> PathBuf {
    if let Some(file_name) = path.file_name().map(|f| f.to_string_lossy()) {
        if let Some(file_stem) = path.file_stem().map(|f| f.to_string_lossy()) {
            if let Some(ext) = path.extension().map(|f| f.to_string_lossy()) {
                return if file_stem.to_lowercase().ends_with(".d") {
                    path.with_file_name(format!(
                        "{}{}.{}.{}",
                        &file_stem[..file_stem.len() - ".d".len()],
                        suffix,
                        // maintain casing
                        &file_stem[file_stem.len() - "d".len()..],
                        ext
                    ))
                } else {
                    path.with_file_name(format!("{}{}.{}", file_stem, suffix, ext))
                };
            }
        }

        path.with_file_name(format!("{}{}", file_name, suffix))
    } else {
        path.with_file_name(suffix)
    }
}

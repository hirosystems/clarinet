// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

use std::collections::BTreeMap;
use std::collections::HashSet;
use std::path::PathBuf;

use deno_ast::ModuleSpecifier;
use deno_core::anyhow::anyhow;
use deno_core::error::AnyError;

use super::super::super::fs_util::path_with_stem_suffix;

/// Partitions the provided specifiers by the non-path and non-query parts of a specifier.
pub fn partition_by_root_specifiers<'a>(
    specifiers: impl Iterator<Item = &'a ModuleSpecifier>,
) -> BTreeMap<ModuleSpecifier, Vec<ModuleSpecifier>> {
    let mut root_specifiers: BTreeMap<ModuleSpecifier, Vec<ModuleSpecifier>> = Default::default();
    for remote_specifier in specifiers {
        let mut root_specifier = remote_specifier.clone();
        root_specifier.set_query(None);
        root_specifier.set_path("/");

        let specifiers = root_specifiers.entry(root_specifier).or_default();
        specifiers.push(remote_specifier.clone());
    }
    root_specifiers
}

/// Gets the directory name to use for the provided root.
pub fn dir_name_for_root(root: &ModuleSpecifier) -> PathBuf {
    let mut result = String::new();
    if let Some(domain) = root.domain() {
        result.push_str(&sanitize_segment(domain));
    }
    if let Some(port) = root.port() {
        if !result.is_empty() {
            result.push('_');
        }
        result.push_str(&port.to_string());
    }
    let mut result = PathBuf::from(result);
    if let Some(segments) = root.path_segments() {
        for segment in segments.filter(|s| !s.is_empty()) {
            result = result.join(sanitize_segment(segment));
        }
    }

    result
}

/// Gets a unique file path given the provided file path
/// and the set of existing file paths. Inserts to the
/// set when finding a unique path.
pub fn get_unique_path(mut path: PathBuf, unique_set: &mut HashSet<String>) -> PathBuf {
    let original_path = path.clone();
    let mut count = 2;
    // case insensitive comparison so the output works on case insensitive file systems
    while !unique_set.insert(path.to_string_lossy().to_lowercase()) {
        path = path_with_stem_suffix(&original_path, &format!("_{}", count));
        count += 1;
    }
    path
}

pub fn make_url_relative(
    root: &ModuleSpecifier,
    url: &ModuleSpecifier,
) -> Result<String, AnyError> {
    root.make_relative(url).ok_or_else(|| {
        anyhow!(
            "Error making url ({}) relative to root: {}",
            url.to_string(),
            root.to_string()
        )
    })
}

pub fn is_remote_specifier(specifier: &ModuleSpecifier) -> bool {
    specifier.scheme().to_lowercase().starts_with("http")
}

pub fn is_remote_specifier_text(text: &str) -> bool {
    text.trim_start().to_lowercase().starts_with("http")
}

pub fn sanitize_filepath(text: &str) -> String {
    text.chars()
        .map(|c| if is_banned_path_char(c) { '_' } else { c })
        .collect()
}

fn is_banned_path_char(c: char) -> bool {
    matches!(c, '<' | '>' | ':' | '"' | '|' | '?' | '*')
}

fn sanitize_segment(text: &str) -> String {
    text.chars()
        .map(|c| if is_banned_segment_char(c) { '_' } else { c })
        .collect()
}

fn is_banned_segment_char(c: char) -> bool {
    matches!(c, '/' | '\\') || is_banned_path_char(c)
}

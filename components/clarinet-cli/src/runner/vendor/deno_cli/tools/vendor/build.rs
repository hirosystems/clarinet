// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

use std::fmt::Write as _;
use std::path::Path;
use std::path::PathBuf;

use deno_ast::ModuleSpecifier;
use deno_core::anyhow::bail;
use deno_core::anyhow::Context;
use deno_core::error::AnyError;
use deno_graph::Module;
use deno_graph::ModuleGraph;
use deno_graph::ModuleKind;
use import_map::ImportMap;
use import_map::SpecifierMap;

use super::analyze::has_default_export;
use super::import_map::build_import_map;
use super::mappings::Mappings;
use super::mappings::ProxiedModule;
use super::specifiers::is_remote_specifier;

/// Allows substituting the environment for testing purposes.
pub trait VendorEnvironment {
  fn cwd(&self) -> Result<PathBuf, AnyError>;
  fn create_dir_all(&self, dir_path: &Path) -> Result<(), AnyError>;
  fn write_file(&self, file_path: &Path, text: &str) -> Result<(), AnyError>;
  fn path_exists(&self, path: &Path) -> bool;
}

pub struct RealVendorEnvironment;

impl VendorEnvironment for RealVendorEnvironment {
  fn cwd(&self) -> Result<PathBuf, AnyError> {
    Ok(std::env::current_dir()?)
  }

  fn create_dir_all(&self, dir_path: &Path) -> Result<(), AnyError> {
    Ok(std::fs::create_dir_all(dir_path)?)
  }

  fn write_file(&self, file_path: &Path, text: &str) -> Result<(), AnyError> {
    std::fs::write(file_path, text)
      .with_context(|| format!("Failed writing {}", file_path.display()))
  }

  fn path_exists(&self, path: &Path) -> bool {
    path.exists()
  }
}

/// Vendors remote modules and returns how many were vendored.
pub fn build(
  graph: ModuleGraph,
  output_dir: &Path,
  original_import_map: Option<&ImportMap>,
  environment: &impl VendorEnvironment,
) -> Result<usize, AnyError> {
  assert!(output_dir.is_absolute());
  let output_dir_specifier =
    ModuleSpecifier::from_directory_path(output_dir).unwrap();

  if let Some(original_im) = &original_import_map {
    validate_original_import_map(original_im, &output_dir_specifier)?;
  }

  // build the graph
  graph.lock()?;

  let graph_errors = graph.errors();
  if !graph_errors.is_empty() {
    for err in &graph_errors {
      log::error!("{}", err);
    }
    bail!("failed vendoring");
  }

  // figure out how to map remote modules to local
  let all_modules = graph.modules();
  let remote_modules = all_modules
    .iter()
    .filter(|m| is_remote_specifier(&m.specifier))
    .copied()
    .collect::<Vec<_>>();
  let mappings =
    Mappings::from_remote_modules(&graph, &remote_modules, output_dir)?;

  // write out all the files
  for module in &remote_modules {
    let source = match &module.maybe_source {
      Some(source) => source,
      None => continue,
    };
    let local_path = mappings
      .proxied_path(&module.specifier)
      .unwrap_or_else(|| mappings.local_path(&module.specifier));
    if !matches!(module.kind, ModuleKind::Esm | ModuleKind::Asserted) {
      log::warn!(
        "Unsupported module kind {:?} for {}",
        module.kind,
        module.specifier
      );
      continue;
    }
    environment.create_dir_all(local_path.parent().unwrap())?;
    environment.write_file(&local_path, source)?;
  }

  // write out the proxies
  for (specifier, proxied_module) in mappings.proxied_modules() {
    let proxy_path = mappings.local_path(specifier);
    let module = graph.get(specifier).unwrap();
    let text = build_proxy_module_source(module, proxied_module);

    environment.write_file(&proxy_path, &text)?;
  }

  // create the import map if necessary
  if !remote_modules.is_empty() {
    let import_map_path = output_dir.join("import_map.json");
    let import_map_text = build_import_map(
      &output_dir_specifier,
      &graph,
      &all_modules,
      &mappings,
      original_import_map,
    );
    environment.write_file(&import_map_path, &import_map_text)?;
  }

  Ok(remote_modules.len())
}

fn validate_original_import_map(
  import_map: &ImportMap,
  output_dir: &ModuleSpecifier,
) -> Result<(), AnyError> {
  fn validate_imports(
    imports: &SpecifierMap,
    output_dir: &ModuleSpecifier,
  ) -> Result<(), AnyError> {
    for entry in imports.entries() {
      if let Some(value) = entry.value {
        if value.as_str().starts_with(output_dir.as_str()) {
          bail!(
            "Providing an existing import map with entries for the output directory is not supported (\"{}\": \"{}\").",
            entry.raw_key,
            entry.raw_value.unwrap_or("<INVALID>"),
          );
        }
      }
    }
    Ok(())
  }

  validate_imports(import_map.imports(), output_dir)?;

  for scope in import_map.scopes() {
    if scope.key.starts_with(output_dir.as_str()) {
      bail!(
        "Providing an existing import map with a scope for the output directory is not supported (\"{}\").",
        scope.raw_key,
      );
    }
    validate_imports(scope.imports, output_dir)?;
  }

  Ok(())
}

fn build_proxy_module_source(
  module: &Module,
  proxied_module: &ProxiedModule,
) -> String {
  let mut text = String::new();
  writeln!(
    text,
    "// @deno-types=\"{}\"",
    proxied_module.declaration_specifier
  )
  .unwrap();

  let relative_specifier = format!(
    "./{}",
    proxied_module
      .output_path
      .file_name()
      .unwrap()
      .to_string_lossy()
  );

  // for simplicity, always include the `export *` statement as it won't error
  // even when the module does not contain a named export
  writeln!(text, "export * from \"{}\";", relative_specifier).unwrap();

  // add a default export if one exists in the module
  if let Some(parsed_source) = module.maybe_parsed_source.as_ref() {
    if has_default_export(parsed_source) {
      writeln!(
        text,
        "export {{ default }} from \"{}\";",
        relative_specifier
      )
      .unwrap();
    }
  }

  text
}

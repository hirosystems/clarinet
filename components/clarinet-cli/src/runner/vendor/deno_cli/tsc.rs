// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

use super::args::TsConfig;
use super::diagnostics::Diagnostics;
use super::emit;
use super::graph_util::GraphData;
use super::graph_util::ModuleEntry;

use deno_ast::MediaType;
use deno_core::anyhow::anyhow;
use deno_core::anyhow::Context;
use deno_core::error::AnyError;
use deno_core::located_script_name;
use deno_core::op;
use deno_core::parking_lot::RwLock;
use deno_core::resolve_url_or_path;
use serde::Deserialize;
use serde::Serialize;
use serde_json;
use serde_json::json;
use serde_json::Value;
use deno_core::Extension;
use deno_core::JsRuntime;
use deno_core::ModuleSpecifier;
use deno_core::OpState;
use deno_core::RuntimeOptions;
use deno_core::Snapshot;
use deno_graph::Resolved;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

// Declaration files

pub static DENO_NS_LIB: &str = include_str!("dts/lib.deno.ns.d.ts");
pub static DENO_CONSOLE_LIB: &str = include_str!(env!("DENO_CONSOLE_LIB_PATH"));
pub static DENO_URL_LIB: &str = include_str!(env!("DENO_URL_LIB_PATH"));
pub static DENO_WEB_LIB: &str = include_str!(env!("DENO_WEB_LIB_PATH"));
pub static DENO_FETCH_LIB: &str = include_str!(env!("DENO_FETCH_LIB_PATH"));
pub static DENO_WEBSOCKET_LIB: &str =
  include_str!(env!("DENO_WEBSOCKET_LIB_PATH"));
pub static DENO_WEBSTORAGE_LIB: &str =
  include_str!(env!("DENO_WEBSTORAGE_LIB_PATH"));
pub static DENO_CRYPTO_LIB: &str = include_str!(env!("DENO_CRYPTO_LIB_PATH"));
pub static DENO_BROADCAST_CHANNEL_LIB: &str =
  include_str!(env!("DENO_BROADCAST_CHANNEL_LIB_PATH"));
pub static DENO_NET_LIB: &str = include_str!(env!("DENO_NET_LIB_PATH"));
pub static SHARED_GLOBALS_LIB: &str =
  include_str!("dts/lib.deno.shared_globals.d.ts");
pub static WINDOW_LIB: &str = include_str!("dts/lib.deno.window.d.ts");
pub static UNSTABLE_NS_LIB: &str = include_str!("dts/lib.deno.unstable.d.ts");

pub static COMPILER_SNAPSHOT: Lazy<Box<[u8]>> = Lazy::new(
  #[cold]
  #[inline(never)]
  || {
    static COMPRESSED_COMPILER_SNAPSHOT: &[u8] =
      include_bytes!(concat!(env!("OUT_DIR"), "/COMPILER_SNAPSHOT.bin"));

    zstd::bulk::decompress(
      &COMPRESSED_COMPILER_SNAPSHOT[4..],
      u32::from_le_bytes(COMPRESSED_COMPILER_SNAPSHOT[0..4].try_into().unwrap())
        as usize,
    )
    .unwrap()
    .into_boxed_slice()
  },
);

pub fn compiler_snapshot() -> Snapshot {
  Snapshot::Static(&*COMPILER_SNAPSHOT)
}

macro_rules! inc {
  ($e:expr) => {
    include_str!(concat!("dts/", $e))
  };
}

/// Contains static assets that are not preloaded in the compiler snapshot.
pub static STATIC_ASSETS: Lazy<HashMap<&'static str, &'static str>> =
  Lazy::new(|| {
    (&[
      (
        "lib.dom.asynciterable.d.ts",
        inc!("lib.dom.asynciterable.d.ts"),
      ),
      ("lib.dom.d.ts", inc!("lib.dom.d.ts")),
      ("lib.dom.extras.d.ts", inc!("lib.dom.extras.d.ts")),
      ("lib.dom.iterable.d.ts", inc!("lib.dom.iterable.d.ts")),
      ("lib.es6.d.ts", inc!("lib.es6.d.ts")),
      ("lib.es2016.full.d.ts", inc!("lib.es2016.full.d.ts")),
      ("lib.es2017.full.d.ts", inc!("lib.es2017.full.d.ts")),
      ("lib.es2018.full.d.ts", inc!("lib.es2018.full.d.ts")),
      ("lib.es2019.full.d.ts", inc!("lib.es2019.full.d.ts")),
      ("lib.es2020.full.d.ts", inc!("lib.es2020.full.d.ts")),
      ("lib.es2021.full.d.ts", inc!("lib.es2021.full.d.ts")),
      ("lib.es2022.full.d.ts", inc!("lib.es2022.full.d.ts")),
      ("lib.esnext.full.d.ts", inc!("lib.esnext.full.d.ts")),
      ("lib.scripthost.d.ts", inc!("lib.scripthost.d.ts")),
      ("lib.webworker.d.ts", inc!("lib.webworker.d.ts")),
      (
        "lib.webworker.importscripts.d.ts",
        inc!("lib.webworker.importscripts.d.ts"),
      ),
      (
        "lib.webworker.iterable.d.ts",
        inc!("lib.webworker.iterable.d.ts"),
      ),
    ])
      .iter()
      .cloned()
      .collect()
  });

/// Retrieve a static asset that are included in the binary.
pub fn get_asset(asset: &str) -> Option<&'static str> {
  STATIC_ASSETS.get(asset).map(|s| s.to_owned())
}

fn get_maybe_hash(
  maybe_source: Option<&str>,
  hash_data: &[Vec<u8>],
) -> Option<String> {
  if let Some(source) = maybe_source {
    let mut data = vec![source.as_bytes().to_owned()];
    data.extend_from_slice(hash_data);
    Some(super::checksum::gen(&data))
  } else {
    None
  }
}

/// Hash the URL so it can be sent to `tsc` in a supportable way
fn hash_url(specifier: &ModuleSpecifier, media_type: &MediaType) -> String {
  let hash = super::checksum::gen(&[specifier.path().as_bytes()]);
  format!(
    "{}:///{}{}",
    specifier.scheme(),
    hash,
    media_type.as_ts_extension()
  )
}

/// If the provided URLs derivable tsc media type doesn't match the media type,
/// we will add an extension to the output.  This is to avoid issues with
/// specifiers that don't have extensions, that tsc refuses to emit because they
/// think a `.js` version exists, when it doesn't.
fn maybe_remap_specifier(
  specifier: &ModuleSpecifier,
  media_type: &MediaType,
) -> Option<String> {
  let path = if specifier.scheme() == "file" {
    if let Ok(path) = specifier.to_file_path() {
      path
    } else {
      PathBuf::from(specifier.path())
    }
  } else {
    PathBuf::from(specifier.path())
  };
  if path.extension().is_none() {
    Some(format!("{}{}", specifier, media_type.as_ts_extension()))
  } else {
    None
  }
}

/// tsc only supports `.ts`, `.tsx`, `.d.ts`, `.js`, or `.jsx` as root modules
/// and so we have to detect the apparent media type based on extensions it
/// supports.
fn get_tsc_media_type(specifier: &ModuleSpecifier) -> MediaType {
  let path = if specifier.scheme() == "file" {
    if let Ok(path) = specifier.to_file_path() {
      path
    } else {
      PathBuf::from(specifier.path())
    }
  } else {
    PathBuf::from(specifier.path())
  };
  match path.extension() {
    None => MediaType::Unknown,
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
      Some("mts") => {
        if let Some(os_str) = path.file_stem() {
          if let Some(file_name) = os_str.to_str() {
            if file_name.ends_with(".d") {
              return MediaType::Dmts;
            }
          }
        }
        MediaType::Mts
      }
      Some("cts") => {
        if let Some(os_str) = path.file_stem() {
          if let Some(file_name) = os_str.to_str() {
            if file_name.ends_with(".d") {
              return MediaType::Dcts;
            }
          }
        }
        MediaType::Cts
      }
      Some("tsx") => MediaType::Tsx,
      Some("js") => MediaType::JavaScript,
      Some("mjs") => MediaType::Mjs,
      Some("cjs") => MediaType::Cjs,
      Some("jsx") => MediaType::Jsx,
      _ => MediaType::Unknown,
    },
  }
}

#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct EmittedFile {
  pub data: String,
  pub maybe_specifiers: Option<Vec<ModuleSpecifier>>,
  pub media_type: MediaType,
}

/// A structure representing a request to be sent to the tsc runtime.
#[derive(Debug)]
pub struct Request {
  /// The TypeScript compiler options which will be serialized and sent to
  /// tsc.
  pub config: TsConfig,
  /// Indicates to the tsc runtime if debug logging should occur.
  pub debug: bool,
  pub graph_data: Arc<RwLock<GraphData>>,
  pub hash_data: Vec<Vec<u8>>,
  pub maybe_config_specifier: Option<ModuleSpecifier>,
  pub maybe_tsbuildinfo: Option<String>,
  /// A vector of strings that represent the root/entry point modules for the
  /// program.
  pub root_names: Vec<(ModuleSpecifier, MediaType)>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Response {
  /// Any diagnostics that have been returned from the checker.
  pub diagnostics: Diagnostics,
  /// If there was any build info associated with the exec request.
  pub maybe_tsbuildinfo: Option<String>,
  /// Statistics from the check.
  pub stats: emit::Stats,
}

#[derive(Debug)]
struct State {
  hash_data: Vec<Vec<u8>>,
  graph_data: Arc<RwLock<GraphData>>,
  maybe_config_specifier: Option<ModuleSpecifier>,
  maybe_tsbuildinfo: Option<String>,
  maybe_response: Option<RespondArgs>,
  remapped_specifiers: HashMap<String, ModuleSpecifier>,
  root_map: HashMap<String, ModuleSpecifier>,
}

impl State {
  pub fn new(
    graph_data: Arc<RwLock<GraphData>>,
    hash_data: Vec<Vec<u8>>,
    maybe_config_specifier: Option<ModuleSpecifier>,
    maybe_tsbuildinfo: Option<String>,
    root_map: HashMap<String, ModuleSpecifier>,
    remapped_specifiers: HashMap<String, ModuleSpecifier>,
  ) -> Self {
    State {
      hash_data,
      graph_data,
      maybe_config_specifier,
      maybe_tsbuildinfo,
      maybe_response: None,
      remapped_specifiers,
      root_map,
    }
  }
}

fn normalize_specifier(specifier: &str) -> Result<ModuleSpecifier, AnyError> {
  resolve_url_or_path(&specifier.replace(".d.ts.d.ts", ".d.ts"))
    .map_err(|err| err.into())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateHashArgs {
  /// The string data to be used to generate the hash.  This will be mixed with
  /// other state data in Deno to derive the final hash.
  data: String,
}

#[op]
fn op_create_hash(s: &mut OpState, args: Value) -> Result<Value, AnyError> {
  let state = s.borrow_mut::<State>();
  let v: CreateHashArgs = serde_json::from_value(args)
    .context("Invalid request from JavaScript for \"op_create_hash\".")?;
  let mut data = vec![v.data.as_bytes().to_owned()];
  data.extend_from_slice(&state.hash_data);
  let hash = super::checksum::gen(&data);
  Ok(json!({ "hash": hash }))
}

#[op]
fn op_cwd(s: &mut OpState) -> Result<String, AnyError> {
  let state = s.borrow_mut::<State>();
  if let Some(config_specifier) = &state.maybe_config_specifier {
    let cwd = config_specifier.join("./")?;
    Ok(cwd.to_string())
  } else {
    Ok("cache:///".to_string())
  }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EmitArgs {
  /// The text data/contents of the file.
  data: String,
  /// The _internal_ filename for the file.  This will be used to determine how
  /// the file is cached and stored.
  file_name: String,
}

#[op]
fn op_emit(state: &mut OpState, args: EmitArgs) -> bool {
  let state = state.borrow_mut::<State>();
  match args.file_name.as_ref() {
    "deno:///.tsbuildinfo" => state.maybe_tsbuildinfo = Some(args.data),
    _ => {
      if cfg!(debug_assertions) {
        panic!("Unhandled emit write: {}", args.file_name);
      }
    }
  }

  true
}

#[derive(Debug, Deserialize)]
struct ExistsArgs {
  /// The fully qualified specifier that should be loaded.
  specifier: String,
}

#[op]
fn op_exists(state: &mut OpState, args: ExistsArgs) -> bool {
  let state = state.borrow_mut::<State>();
  let graph_data = state.graph_data.read();
  if let Ok(specifier) = normalize_specifier(&args.specifier) {
    if specifier.scheme() == "asset" || specifier.scheme() == "data" {
      true
    } else {
      matches!(
        graph_data.get(&graph_data.follow_redirect(&specifier)),
        Some(ModuleEntry::Module { .. })
      )
    }
  } else {
    false
  }
}

#[derive(Debug, Deserialize)]
struct LoadArgs {
  /// The fully qualified specifier that should be loaded.
  specifier: String,
}

pub fn as_ts_script_kind(media_type: &MediaType) -> i32 {
  match media_type {
    MediaType::JavaScript => 1,
    MediaType::Jsx => 2,
    MediaType::Mjs => 1,
    MediaType::Cjs => 1,
    MediaType::TypeScript => 3,
    MediaType::Mts => 3,
    MediaType::Cts => 3,
    MediaType::Dts => 3,
    MediaType::Dmts => 3,
    MediaType::Dcts => 3,
    MediaType::Tsx => 4,
    MediaType::Json => 6,
    _ => 0,
  }
}

#[op]
fn op_load(state: &mut OpState, args: Value) -> Result<Value, AnyError> {
  let state = state.borrow_mut::<State>();
  let v: LoadArgs = serde_json::from_value(args)
    .context("Invalid request from JavaScript for \"op_load\".")?;
  let specifier = normalize_specifier(&v.specifier)
    .context("Error converting a string module specifier for \"op_load\".")?;
  let mut hash: Option<String> = None;
  let mut media_type = MediaType::Unknown;
  let graph_data = state.graph_data.read();
  let data = if &v.specifier == "deno:///.tsbuildinfo" {
    state.maybe_tsbuildinfo.as_deref()
  // in certain situations we return a "blank" module to tsc and we need to
  // handle the request for that module here.
  } else if &v.specifier == "deno:///missing_dependency.d.ts" {
    hash = Some("1".to_string());
    media_type = MediaType::Dts;
    Some("declare const __: any;\nexport = __;\n")
  } else if v.specifier.starts_with("asset:///") {
    let name = v.specifier.replace("asset:///", "");
    let maybe_source = get_asset(&name);
    hash = get_maybe_hash(maybe_source, &state.hash_data);
    media_type = MediaType::from(&v.specifier);
    maybe_source
  } else {
    let specifier = if let Some(remapped_specifier) =
      state.remapped_specifiers.get(&v.specifier)
    {
      remapped_specifier.clone()
    } else if let Some(remapped_specifier) = state.root_map.get(&v.specifier) {
      remapped_specifier.clone()
    } else {
      specifier
    };
    let maybe_source = if let Some(ModuleEntry::Module {
      code,
      media_type: mt,
      ..
    }) =
      graph_data.get(&graph_data.follow_redirect(&specifier))
    {
      media_type = *mt;
      Some(code as &str)
    } else {
      media_type = MediaType::Unknown;
      None
    };
    hash = get_maybe_hash(maybe_source, &state.hash_data);
    maybe_source
  };

  Ok(
    json!({ "data": data, "version": hash, "scriptKind": as_ts_script_kind(&media_type) }),
  )
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveArgs {
  /// The base specifier that the supplied specifier strings should be resolved
  /// relative to.
  pub base: String,
  /// A list of specifiers that should be resolved.
  pub specifiers: Vec<String>,
}

#[op]
fn op_resolve(
  state: &mut OpState,
  args: ResolveArgs,
) -> Result<Vec<(String, String)>, AnyError> {
  let state = state.borrow_mut::<State>();
  let mut resolved: Vec<(String, String)> = Vec::new();
  let referrer = if let Some(remapped_specifier) =
    state.remapped_specifiers.get(&args.base)
  {
    remapped_specifier.clone()
  } else if let Some(remapped_base) = state.root_map.get(&args.base) {
    remapped_base.clone()
  } else {
    normalize_specifier(&args.base).context(
      "Error converting a string module specifier for \"op_resolve\".",
    )?
  };
  for specifier in &args.specifiers {
    if specifier.starts_with("asset:///") {
      resolved.push((
        specifier.clone(),
        MediaType::from(specifier).as_ts_extension().to_string(),
      ));
    } else {
      let graph_data = state.graph_data.read();
      let referrer = graph_data.follow_redirect(&referrer);
      let resolved_dep = match graph_data.get(&referrer) {
        Some(ModuleEntry::Module { dependencies, .. }) => {
          dependencies.get(specifier).map(|d| {
            if matches!(d.maybe_type, Resolved::Ok { .. }) {
              &d.maybe_type
            } else {
              &d.maybe_code
            }
          })
        }
        Some(ModuleEntry::Configuration { dependencies }) => {
          dependencies.get(specifier)
        }
        _ => None,
      };
      let maybe_result = match resolved_dep {
        Some(Resolved::Ok { specifier, .. }) => {
          let specifier = graph_data.follow_redirect(specifier);
          match graph_data.get(&specifier) {
            Some(ModuleEntry::Module {
              media_type,
              maybe_types,
              ..
            }) => match maybe_types {
              Some(Resolved::Ok { specifier, .. }) => {
                let types = graph_data.follow_redirect(specifier);
                match graph_data.get(&types) {
                  Some(ModuleEntry::Module { media_type, .. }) => {
                    Some((types, media_type))
                  }
                  _ => None,
                }
              }
              _ => Some((specifier, media_type)),
            },
            _ => None,
          }
        }
        _ => None,
      };
      let result = match maybe_result {
        Some((specifier, media_type)) => {
          let specifier_str = match specifier.scheme() {
            "data" | "blob" => {
              let specifier_str = hash_url(&specifier, media_type);
              state
                .remapped_specifiers
                .insert(specifier_str.clone(), specifier);
              specifier_str
            }
            _ => {
              if let Some(specifier_str) =
                maybe_remap_specifier(&specifier, media_type)
              {
                state
                  .remapped_specifiers
                  .insert(specifier_str.clone(), specifier);
                specifier_str
              } else {
                specifier.to_string()
              }
            }
          };
          (specifier_str, media_type.as_ts_extension().into())
        }
        None => (
          "deno:///missing_dependency.d.ts".to_string(),
          ".d.ts".to_string(),
        ),
      };
      resolved.push(result);
    }
  }

  Ok(resolved)
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
struct RespondArgs {
  pub diagnostics: Diagnostics,
  pub stats: emit::Stats,
}

#[op]
fn op_respond(state: &mut OpState, args: Value) -> Result<Value, AnyError> {
  let state = state.borrow_mut::<State>();
  let v: RespondArgs = serde_json::from_value(args)
    .context("Error converting the result for \"op_respond\".")?;
  state.maybe_response = Some(v);
  Ok(json!(true))
}

/// Execute a request on the supplied snapshot, returning a response which
/// contains information, like any emitted files, diagnostics, statistics and
/// optionally an updated TypeScript build info.
pub fn exec(request: Request) -> Result<Response, AnyError> {
  // tsc cannot handle root specifiers that don't have one of the "acceptable"
  // extensions.  Therefore, we have to check the root modules against their
  // extensions and remap any that are unacceptable to tsc and add them to the
  // op state so when requested, we can remap to the original specifier.
  let mut root_map = HashMap::new();
  let mut remapped_specifiers = HashMap::new();
  let root_names: Vec<String> = request
    .root_names
    .iter()
    .map(|(s, mt)| match s.scheme() {
      "data" | "blob" => {
        let specifier_str = hash_url(s, mt);
        remapped_specifiers.insert(specifier_str.clone(), s.clone());
        specifier_str
      }
      _ => {
        let ext_media_type = get_tsc_media_type(s);
        if mt != &ext_media_type {
          let new_specifier = format!("{}{}", s, mt.as_ts_extension());
          root_map.insert(new_specifier.clone(), s.clone());
          new_specifier
        } else {
          s.as_str().to_owned()
        }
      }
    })
    .collect();
  let mut runtime = JsRuntime::new(RuntimeOptions {
    startup_snapshot: Some(compiler_snapshot()),
    extensions: vec![Extension::builder()
      .ops(vec![
        op_cwd::decl(),
        op_create_hash::decl(),
        op_emit::decl(),
        op_exists::decl(),
        op_load::decl(),
        op_resolve::decl(),
        op_respond::decl(),
      ])
      .state(move |state| {
        state.put(State::new(
          request.graph_data.clone(),
          request.hash_data.clone(),
          request.maybe_config_specifier.clone(),
          request.maybe_tsbuildinfo.clone(),
          root_map.clone(),
          remapped_specifiers.clone(),
        ));
        Ok(())
      })
      .build()],
    ..Default::default()
  });

  let startup_source = "globalThis.startup({ legacyFlag: false })";
  let request_value = json!({
    "config": request.config,
    "debug": request.debug,
    "rootNames": root_names,
  });
  let request_str = request_value.to_string();
  let exec_source = format!("globalThis.exec({})", request_str);

  runtime
    .execute_script(&located_script_name!(), startup_source)
    .context("Could not properly start the compiler runtime.")?;
  runtime.execute_script(&located_script_name!(), &exec_source)?;

  let op_state = runtime.op_state();
  let mut op_state = op_state.borrow_mut();
  let state = op_state.take::<State>();

  if let Some(response) = state.maybe_response {
    let diagnostics = response.diagnostics;
    let maybe_tsbuildinfo = state.maybe_tsbuildinfo;
    let stats = response.stats;

    Ok(Response {
      diagnostics,
      maybe_tsbuildinfo,
      stats,
    })
  } else {
    Err(anyhow!("The response for the exec request was not set."))
  }
}

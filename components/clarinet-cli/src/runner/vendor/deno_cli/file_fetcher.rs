// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

use super::auth_tokens::AuthTokens;
use super::super::deno_runtime::colors;
use super::http_cache::HttpCache;
use super::http_util::fetch_once;
use super::http_util::CacheSemantics;
use super::http_util::FetchOnceArgs;
use super::http_util::FetchOnceResult;
use super::text_encoding;
use super::version::get_user_agent;

use data_url::DataUrl;
use deno_ast::MediaType;
use deno_core::anyhow::anyhow;
use deno_core::error::custom_error;
use deno_core::error::generic_error;
use deno_core::error::uri_error;
use deno_core::error::AnyError;
use deno_core::futures;
use deno_core::futures::future::FutureExt;
use deno_core::parking_lot::Mutex;
use deno_core::ModuleSpecifier;
use deno_fetch::create_http_client;
use deno_fetch::reqwest;
use deno_tls::rustls;
use deno_tls::rustls::RootCertStore;
use deno_tls::rustls_native_certs::load_native_certs;
use deno_tls::rustls_pemfile;
use deno_tls::webpki_roots;
use deno_web::BlobStore;
use super::super::deno_runtime::permissions::Permissions;
use log::debug;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::future::Future;
use std::io::BufReader;
use std::io::Read;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::time::SystemTime;

pub const SUPPORTED_SCHEMES: [&str; 5] =
  ["data", "blob", "file", "http", "https"];

/// A structure representing a source file.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct File {
  /// The path to the local version of the source file.  For local files this
  /// will be the direct path to that file.  For remote files, it will be the
  /// path to the file in the HTTP cache.
  pub local: PathBuf,
  /// For remote files, if there was an `X-TypeScript-Type` header, the parsed
  /// out value of that header.
  pub maybe_types: Option<String>,
  /// The resolved media type for the file.
  pub media_type: MediaType,
  /// The source of the file as a string.
  pub source: Arc<str>,
  /// The _final_ specifier for the file.  The requested specifier and the final
  /// specifier maybe different for remote files that have been redirected.
  pub specifier: ModuleSpecifier,

  pub maybe_headers: Option<HashMap<String, String>>,
}

/// Simple struct implementing in-process caching to prevent multiple
/// fs reads/net fetches for same file.
#[derive(Debug, Clone, Default)]
struct FileCache(Arc<Mutex<HashMap<ModuleSpecifier, File>>>);

impl FileCache {
  pub fn get(&self, specifier: &ModuleSpecifier) -> Option<File> {
    let cache = self.0.lock();
    cache.get(specifier).cloned()
  }

  pub fn insert(&self, specifier: ModuleSpecifier, file: File) -> Option<File> {
    let mut cache = self.0.lock();
    cache.insert(specifier, file)
  }
}

/// Indicates how cached source files should be handled.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum CacheSetting {
  /// Only the cached files should be used.  Any files not in the cache will
  /// error.  This is the equivalent of `--cached-only` in the CLI.
  Only,
  /// No cached source files should be used, and all files should be reloaded.
  /// This is the equivalent of `--reload` in the CLI.
  ReloadAll,
  /// Only some cached resources should be used.  This is the equivalent of
  /// `--reload=https://deno.land/std` or
  /// `--reload=https://deno.land/std,https://deno.land/x/example`.
  ReloadSome(Vec<String>),
  /// The usability of a cached value is determined by analyzing the cached
  /// headers and other metadata associated with a cached response, reloading
  /// any cached "non-fresh" cached responses.
  RespectHeaders,
  /// The cached source files should be used for local modules.  This is the
  /// default behavior of the CLI.
  Use,
}

impl CacheSetting {
  /// Returns if the cache should be used for a given specifier.
  pub fn should_use(
    &self,
    specifier: &ModuleSpecifier,
    http_cache: &HttpCache,
  ) -> bool {
    match self {
      CacheSetting::ReloadAll => false,
      CacheSetting::Use | CacheSetting::Only => true,
      CacheSetting::RespectHeaders => {
        if let Ok((_, headers, cache_time)) = http_cache.get(specifier) {
          let cache_semantics =
            CacheSemantics::new(headers, cache_time, SystemTime::now());
          cache_semantics.should_use()
        } else {
          false
        }
      }
      CacheSetting::ReloadSome(list) => {
        let mut url = specifier.clone();
        url.set_fragment(None);
        if list.contains(&url.as_str().to_string()) {
          return false;
        }
        url.set_query(None);
        let mut path = PathBuf::from(url.as_str());
        loop {
          if list.contains(&path.to_str().unwrap().to_string()) {
            return false;
          }
          if !path.pop() {
            break;
          }
        }
        true
      }
    }
  }
}

/// Fetch a source file from the local file system.
fn fetch_local(specifier: &ModuleSpecifier) -> Result<File, AnyError> {
  let local = specifier.to_file_path().map_err(|_| {
    uri_error(format!("Invalid file path.\n  Specifier: {}", specifier))
  })?;
  let bytes = fs::read(local.clone())?;
  let charset = text_encoding::detect_charset(&bytes).to_string();
  let source = get_source_from_bytes(bytes, Some(charset))?;
  let media_type = MediaType::from(specifier);

  Ok(File {
    local,
    maybe_types: None,
    media_type,
    source: source.into(),
    specifier: specifier.clone(),
    maybe_headers: None,
  })
}

/// Create and populate a root cert store based on the passed options and
/// environment.
pub fn get_root_cert_store(
  maybe_root_path: Option<PathBuf>,
  maybe_ca_stores: Option<Vec<String>>,
  maybe_ca_file: Option<String>,
) -> Result<RootCertStore, AnyError> {
  let mut root_cert_store = RootCertStore::empty();
  let ca_stores: Vec<String> = maybe_ca_stores
    .or_else(|| {
      let env_ca_store = env::var("DENO_TLS_CA_STORE").ok()?;
      Some(
        env_ca_store
          .split(',')
          .map(|s| s.trim().to_string())
          .filter(|s| !s.is_empty())
          .collect(),
      )
    })
    .unwrap_or_else(|| vec!["mozilla".to_string()]);

  for store in ca_stores.iter() {
    match store.as_str() {
      "mozilla" => {
        root_cert_store.add_server_trust_anchors(
          webpki_roots::TLS_SERVER_ROOTS.0.iter().map(|ta| {
            rustls::OwnedTrustAnchor::from_subject_spki_name_constraints(
              ta.subject,
              ta.spki,
              ta.name_constraints,
            )
          }),
        );
      }
      "system" => {
        let roots = load_native_certs().expect("could not load platform certs");
        for root in roots {
          root_cert_store
            .add(&rustls::Certificate(root.0))
            .expect("Failed to add platform cert to root cert store");
        }
      }
      _ => {
        return Err(anyhow!("Unknown certificate store \"{}\" specified (allowed: \"system,mozilla\")", store));
      }
    }
  }

  let ca_file = maybe_ca_file.or_else(|| env::var("DENO_CERT").ok());
  if let Some(ca_file) = ca_file {
    let ca_file = if let Some(root) = &maybe_root_path {
      root.join(&ca_file)
    } else {
      PathBuf::from(ca_file)
    };
    let certfile = fs::File::open(&ca_file)?;
    let mut reader = BufReader::new(certfile);

    match rustls_pemfile::certs(&mut reader) {
      Ok(certs) => {
        root_cert_store.add_parsable_certificates(&certs);
      }
      Err(e) => {
        return Err(anyhow!(
          "Unable to add pem file to certificate store: {}",
          e
        ));
      }
    }
  }

  Ok(root_cert_store)
}

/// Returns the decoded body and content-type of a provided
/// data URL.
pub fn get_source_from_data_url(
  specifier: &ModuleSpecifier,
) -> Result<(String, String), AnyError> {
  let data_url = DataUrl::process(specifier.as_str())
    .map_err(|e| uri_error(format!("{:?}", e)))?;
  let mime = data_url.mime_type();
  let charset = mime.get_parameter("charset").map(|v| v.to_string());
  let (bytes, _) = data_url
    .decode_to_vec()
    .map_err(|e| uri_error(format!("{:?}", e)))?;
  Ok((get_source_from_bytes(bytes, charset)?, format!("{}", mime)))
}

/// Given a vector of bytes and optionally a charset, decode the bytes to a
/// string.
pub fn get_source_from_bytes(
  bytes: Vec<u8>,
  maybe_charset: Option<String>,
) -> Result<String, AnyError> {
  let source = if let Some(charset) = maybe_charset {
    text_encoding::convert_to_utf8(&bytes, &charset)?.to_string()
  } else {
    String::from_utf8(bytes)?
  };

  Ok(source)
}

/// Return a validated scheme for a given module specifier.
fn get_validated_scheme(
  specifier: &ModuleSpecifier,
) -> Result<String, AnyError> {
  let scheme = specifier.scheme();
  if !SUPPORTED_SCHEMES.contains(&scheme) {
    Err(generic_error(format!(
      "Unsupported scheme \"{}\" for module \"{}\". Supported schemes: {:#?}",
      scheme, specifier, SUPPORTED_SCHEMES
    )))
  } else {
    Ok(scheme.to_string())
  }
}

/// Resolve a media type and optionally the charset from a module specifier and
/// the value of a content type header.
pub fn map_content_type(
  specifier: &ModuleSpecifier,
  maybe_content_type: Option<String>,
) -> (MediaType, Option<String>) {
  if let Some(content_type) = maybe_content_type {
    let mut content_types = content_type.split(';');
    let content_type = content_types.next().unwrap();
    let media_type = MediaType::from_content_type(specifier, content_type);
    let charset = content_types
      .map(str::trim)
      .find_map(|s| s.strip_prefix("charset="))
      .map(String::from);

    (media_type, charset)
  } else {
    (MediaType::from(specifier), None)
  }
}

/// A structure for resolving, fetching and caching source files.
#[derive(Debug, Clone)]
pub struct FileFetcher {
  auth_tokens: AuthTokens,
  allow_remote: bool,
  cache: FileCache,
  cache_setting: CacheSetting,
  pub http_cache: HttpCache,
  http_client: reqwest::Client,
  blob_store: BlobStore,
  download_log_level: log::Level,
}

impl FileFetcher {
  pub fn new(
    http_cache: HttpCache,
    cache_setting: CacheSetting,
    allow_remote: bool,
    root_cert_store: Option<RootCertStore>,
    blob_store: BlobStore,
    unsafely_ignore_certificate_errors: Option<Vec<String>>,
  ) -> Result<Self, AnyError> {
    Ok(Self {
      auth_tokens: AuthTokens::new(env::var("DENO_AUTH_TOKENS").ok()),
      allow_remote,
      cache: Default::default(),
      cache_setting,
      http_cache,
      http_client: create_http_client(
        get_user_agent(),
        root_cert_store,
        vec![],
        None,
        unsafely_ignore_certificate_errors,
        None,
      )?,
      blob_store,
      download_log_level: log::Level::Info,
    })
  }

  /// Sets the log level to use when outputting the download message.
  pub fn set_download_log_level(&mut self, level: log::Level) {
    self.download_log_level = level;
  }

  /// Creates a `File` structure for a remote file.
  fn build_remote_file(
    &self,
    specifier: &ModuleSpecifier,
    bytes: Vec<u8>,
    headers: &HashMap<String, String>,
  ) -> Result<File, AnyError> {
    let local =
      self
        .http_cache
        .get_cache_filename(specifier)
        .ok_or_else(|| {
          generic_error("Cannot convert specifier to cached filename.")
        })?;
    let maybe_content_type = headers.get("content-type").cloned();
    let (media_type, maybe_charset) =
      map_content_type(specifier, maybe_content_type);
    let source = get_source_from_bytes(bytes, maybe_charset)?;
    let maybe_types = match media_type {
      MediaType::JavaScript
      | MediaType::Cjs
      | MediaType::Mjs
      | MediaType::Jsx => headers.get("x-typescript-types").cloned(),
      _ => None,
    };

    Ok(File {
      local,
      maybe_types,
      media_type,
      source: source.into(),
      specifier: specifier.clone(),
      maybe_headers: Some(headers.clone()),
    })
  }

  /// Fetch cached remote file.
  ///
  /// This is a recursive operation if source file has redirections.
  pub fn fetch_cached(
    &self,
    specifier: &ModuleSpecifier,
    redirect_limit: i64,
  ) -> Result<Option<File>, AnyError> {
    debug!("FileFetcher::fetch_cached - specifier: {}", specifier);
    if redirect_limit < 0 {
      return Err(custom_error("Http", "Too many redirects."));
    }

    let (mut source_file, headers, _) = match self.http_cache.get(specifier) {
      Err(err) => {
        if let Some(err) = err.downcast_ref::<std::io::Error>() {
          if err.kind() == std::io::ErrorKind::NotFound {
            return Ok(None);
          }
        }
        return Err(err);
      }
      Ok(cache) => cache,
    };
    if let Some(redirect_to) = headers.get("location") {
      let redirect =
        deno_core::resolve_import(redirect_to, specifier.as_str())?;
      return self.fetch_cached(&redirect, redirect_limit - 1);
    }
    let mut bytes = Vec::new();
    source_file.read_to_end(&mut bytes)?;
    let file = self.build_remote_file(specifier, bytes, &headers)?;

    Ok(Some(file))
  }

  /// Convert a data URL into a file, resulting in an error if the URL is
  /// invalid.
  fn fetch_data_url(
    &self,
    specifier: &ModuleSpecifier,
  ) -> Result<File, AnyError> {
    debug!("FileFetcher::fetch_data_url() - specifier: {}", specifier);
    match self.fetch_cached(specifier, 0) {
      Ok(Some(file)) => return Ok(file),
      Ok(None) => {}
      Err(err) => return Err(err),
    }

    if self.cache_setting == CacheSetting::Only {
      return Err(custom_error(
        "NotCached",
        format!(
          "Specifier not found in cache: \"{}\", --cached-only is specified.",
          specifier
        ),
      ));
    }

    let (source, content_type) = get_source_from_data_url(specifier)?;
    let (media_type, _) =
      map_content_type(specifier, Some(content_type.clone()));

    let local =
      self
        .http_cache
        .get_cache_filename(specifier)
        .ok_or_else(|| {
          generic_error("Cannot convert specifier to cached filename.")
        })?;
    let mut headers = HashMap::new();
    headers.insert("content-type".to_string(), content_type);
    self
      .http_cache
      .set(specifier, headers.clone(), source.as_bytes())?;

    Ok(File {
      local,
      maybe_types: None,
      media_type,
      source: source.into(),
      specifier: specifier.clone(),
      maybe_headers: Some(headers),
    })
  }

  /// Get a blob URL.
  async fn fetch_blob_url(
    &self,
    specifier: &ModuleSpecifier,
  ) -> Result<File, AnyError> {
    debug!("FileFetcher::fetch_blob_url() - specifier: {}", specifier);
    match self.fetch_cached(specifier, 0) {
      Ok(Some(file)) => return Ok(file),
      Ok(None) => {}
      Err(err) => return Err(err),
    }

    if self.cache_setting == CacheSetting::Only {
      return Err(custom_error(
        "NotCached",
        format!(
          "Specifier not found in cache: \"{}\", --cached-only is specified.",
          specifier
        ),
      ));
    }

    let blob = {
      let blob_store = self.blob_store.borrow();
      blob_store
        .get_object_url(specifier.clone())?
        .ok_or_else(|| {
          custom_error(
            "NotFound",
            format!("Blob URL not found: \"{}\".", specifier),
          )
        })?
    };

    let content_type = blob.media_type.clone();
    let bytes = blob.read_all().await?;

    let (media_type, maybe_charset) =
      map_content_type(specifier, Some(content_type.clone()));
    let source = get_source_from_bytes(bytes, maybe_charset)?;

    let local =
      self
        .http_cache
        .get_cache_filename(specifier)
        .ok_or_else(|| {
          generic_error("Cannot convert specifier to cached filename.")
        })?;
    let mut headers = HashMap::new();
    headers.insert("content-type".to_string(), content_type);
    self
      .http_cache
      .set(specifier, headers.clone(), source.as_bytes())?;

    Ok(File {
      local,
      maybe_types: None,
      media_type,
      source: source.into(),
      specifier: specifier.clone(),
      maybe_headers: Some(headers),
    })
  }

  /// Asynchronously fetch remote source file specified by the URL following
  /// redirects.
  ///
  /// **Note** this is a recursive method so it can't be "async", but needs to
  /// return a `Pin<Box<..>>`.
  fn fetch_remote(
    &self,
    specifier: &ModuleSpecifier,
    permissions: &mut Permissions,
    redirect_limit: i64,
    maybe_accept: Option<String>,
  ) -> Pin<Box<dyn Future<Output = Result<File, AnyError>> + Send>> {
    debug!("FileFetcher::fetch_remote() - specifier: {}", specifier);
    if redirect_limit < 0 {
      return futures::future::err(custom_error("Http", "Too many redirects."))
        .boxed();
    }

    if let Err(err) = permissions.check_specifier(specifier) {
      return futures::future::err(err).boxed();
    }

    if self.cache_setting.should_use(specifier, &self.http_cache) {
      match self.fetch_cached(specifier, redirect_limit) {
        Ok(Some(file)) => {
          return futures::future::ok(file).boxed();
        }
        Ok(None) => {}
        Err(err) => {
          return futures::future::err(err).boxed();
        }
      }
    }

    if self.cache_setting == CacheSetting::Only {
      return futures::future::err(custom_error(
        "NotCached",
        format!(
          "Specifier not found in cache: \"{}\", --cached-only is specified.",
          specifier
        ),
      ))
      .boxed();
    }

    log::log!(
      self.download_log_level,
      "{} {}",
      colors::green("Download"),
      specifier
    );

    let maybe_etag = match self.http_cache.get(specifier) {
      Ok((_, headers, _)) => headers.get("etag").cloned(),
      _ => None,
    };
    let maybe_auth_token = self.auth_tokens.get(specifier);
    let specifier = specifier.clone();
    let mut permissions = permissions.clone();
    let client = self.http_client.clone();
    let file_fetcher = self.clone();
    // A single pass of fetch either yields code or yields a redirect.
    async move {
      match fetch_once(FetchOnceArgs {
        client,
        url: specifier.clone(),
        maybe_accept: maybe_accept.clone(),
        maybe_etag,
        maybe_auth_token,
      })
      .await?
      {
        FetchOnceResult::NotModified => {
          let file = file_fetcher.fetch_cached(&specifier, 10)?.unwrap();
          Ok(file)
        }
        FetchOnceResult::Redirect(redirect_url, headers) => {
          file_fetcher.http_cache.set(&specifier, headers, &[])?;
          file_fetcher
            .fetch_remote(
              &redirect_url,
              &mut permissions,
              redirect_limit - 1,
              maybe_accept,
            )
            .await
        }
        FetchOnceResult::Code(bytes, headers) => {
          file_fetcher
            .http_cache
            .set(&specifier, headers.clone(), &bytes)?;
          let file =
            file_fetcher.build_remote_file(&specifier, bytes, &headers)?;
          Ok(file)
        }
      }
    }
    .boxed()
  }

  /// Fetch a source file and asynchronously return it.
  pub async fn fetch(
    &self,
    specifier: &ModuleSpecifier,
    permissions: &mut Permissions,
  ) -> Result<File, AnyError> {
    debug!("FileFetcher::fetch() - specifier: {}", specifier);
    self.fetch_with_accept(specifier, permissions, None).await
  }

  pub async fn fetch_with_accept(
    &self,
    specifier: &ModuleSpecifier,
    permissions: &mut Permissions,
    maybe_accept: Option<&str>,
  ) -> Result<File, AnyError> {
    let scheme = get_validated_scheme(specifier)?;
    permissions.check_specifier(specifier)?;
    if let Some(file) = self.cache.get(specifier) {
      Ok(file)
    } else if scheme == "file" {
      // we do not in memory cache files, as this would prevent files on the
      // disk changing effecting things like workers and dynamic imports.
      fetch_local(specifier)
    } else if scheme == "data" {
      let result = self.fetch_data_url(specifier);
      if let Ok(file) = &result {
        self.cache.insert(specifier.clone(), file.clone());
      }
      result
    } else if scheme == "blob" {
      let result = self.fetch_blob_url(specifier).await;
      if let Ok(file) = &result {
        self.cache.insert(specifier.clone(), file.clone());
      }
      result
    } else if !self.allow_remote {
      Err(custom_error(
        "NoRemote",
        format!("A remote specifier was requested: \"{}\", but --no-remote is specified.", specifier),
      ))
    } else {
      let result = self
        .fetch_remote(
          specifier,
          permissions,
          10,
          maybe_accept.map(String::from),
        )
        .await;
      if let Ok(file) = &result {
        self.cache.insert(specifier.clone(), file.clone());
      }
      result
    }
  }

  pub fn get_local_path(&self, specifier: &ModuleSpecifier) -> Option<PathBuf> {
    // TODO(@kitsonk) fix when deno_graph does not query cache for synthetic
    // modules
    if specifier.scheme() == "flags" {
      None
    } else if specifier.scheme() == "file" {
      specifier.to_file_path().ok()
    } else {
      self.http_cache.get_cache_filename(specifier)
    }
  }

  /// Get the location of the current HTTP cache associated with the fetcher.
  pub fn get_http_cache_location(&self) -> PathBuf {
    self.http_cache.location.clone()
  }

  /// A synchronous way to retrieve a source file, where if the file has already
  /// been cached in memory it will be returned, otherwise for local files will
  /// be read from disk.
  pub fn get_source(&self, specifier: &ModuleSpecifier) -> Option<File> {
    let maybe_file = self.cache.get(specifier);
    if maybe_file.is_none() {
      let is_local = specifier.scheme() == "file";
      if is_local {
        if let Ok(file) = fetch_local(specifier) {
          return Some(file);
        }
      }
      None
    } else {
      maybe_file
    }
  }

  /// Insert a temporary module into the in memory cache for the file fetcher.
  pub fn insert_cached(&self, file: File) -> Option<File> {
    self.cache.insert(file.specifier.clone(), file)
  }
}

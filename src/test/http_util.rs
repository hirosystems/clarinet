// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

use super::auth_tokens::AuthToken;

use deno_core::error::generic_error;
use deno_core::error::AnyError;
use deno_core::url::Url;
use deno_runtime::deno_fetch::reqwest;
use deno_runtime::deno_fetch::reqwest::header::HeaderMap;
use deno_runtime::deno_fetch::reqwest::header::HeaderValue;
use deno_runtime::deno_fetch::reqwest::header::AUTHORIZATION;
use deno_runtime::deno_fetch::reqwest::header::IF_NONE_MATCH;
use deno_runtime::deno_fetch::reqwest::header::LOCATION;
use deno_runtime::deno_fetch::reqwest::header::USER_AGENT;
use deno_runtime::deno_fetch::reqwest::redirect::Policy;
use deno_runtime::deno_fetch::reqwest::Client;
use deno_runtime::deno_fetch::reqwest::StatusCode;
use std::collections::HashMap;

/// Create new instance of async reqwest::Client. This client supports
/// proxies and doesn't follow redirects.
pub fn create_http_client(
  user_agent: String,
  ca_data: Option<Vec<u8>>,
) -> Result<Client, AnyError> {
  let mut headers = HeaderMap::new();
  headers.insert(USER_AGENT, user_agent.parse().unwrap());
  let mut builder = Client::builder()
    .redirect(Policy::none())
    .default_headers(headers)
    .use_rustls_tls();

  if let Some(ca_data) = ca_data {
    let cert = reqwest::Certificate::from_pem(&ca_data)?;
    builder = builder.add_root_certificate(cert);
  }

  builder
    .build()
    .map_err(|e| generic_error(format!("Unable to build http client: {}", e)))
}

/// Construct the next uri based on base uri and location header fragment
/// See <https://tools.ietf.org/html/rfc3986#section-4.2>
fn resolve_url_from_location(base_url: &Url, location: &str) -> Url {
  if location.starts_with("http://") || location.starts_with("https://") {
    // absolute uri
    Url::parse(location).expect("provided redirect url should be a valid url")
  } else if location.starts_with("//") {
    // "//" authority path-abempty
    Url::parse(&format!("{}:{}", base_url.scheme(), location))
      .expect("provided redirect url should be a valid url")
  } else if location.starts_with('/') {
    // path-absolute
    base_url
      .join(location)
      .expect("provided redirect url should be a valid url")
  } else {
    // assuming path-noscheme | path-empty
    let base_url_path_str = base_url.path().to_owned();
    // Pop last part or url (after last slash)
    let segs: Vec<&str> = base_url_path_str.rsplitn(2, '/').collect();
    let new_path = format!("{}/{}", segs.last().unwrap_or(&""), location);
    base_url
      .join(&new_path)
      .expect("provided redirect url should be a valid url")
  }
}

// TODO(ry) HTTP headers are not unique key, value pairs. There may be more than
// one header line with the same key. This should be changed to something like
// Vec<(String, String)>
pub type HeadersMap = HashMap<String, String>;

#[derive(Debug, PartialEq)]
pub enum FetchOnceResult {
  Code(Vec<u8>, HeadersMap),
  NotModified,
  Redirect(Url, HeadersMap),
}

#[derive(Debug)]
pub struct FetchOnceArgs {
  pub client: Client,
  pub url: Url,
  pub maybe_etag: Option<String>,
  pub maybe_auth_token: Option<AuthToken>,
}

/// Asynchronously fetches the given HTTP URL one pass only.
/// If no redirect is present and no error occurs,
/// yields Code(ResultPayload).
/// If redirect occurs, does not follow and
/// yields Redirect(url).
pub async fn fetch_once(
  args: FetchOnceArgs,
) -> Result<FetchOnceResult, AnyError> {
  let mut request = args.client.get(args.url.clone());

  if let Some(etag) = args.maybe_etag {
    let if_none_match_val = HeaderValue::from_str(&etag).unwrap();
    request = request.header(IF_NONE_MATCH, if_none_match_val);
  }
  if let Some(auth_token) = args.maybe_auth_token {
    let authorization_val =
      HeaderValue::from_str(&auth_token.to_string()).unwrap();
    request = request.header(AUTHORIZATION, authorization_val);
  }
  let response = request.send().await?;

  if response.status() == StatusCode::NOT_MODIFIED {
    return Ok(FetchOnceResult::NotModified);
  }

  let mut headers_: HashMap<String, String> = HashMap::new();
  let headers = response.headers();

  if let Some(warning) = headers.get("X-Deno-Warning") {
    eprintln!(
      "{} {}",
      super::colors::yellow("Warning"),
      warning.to_str().unwrap()
    );
  }

  for key in headers.keys() {
    let key_str = key.to_string();
    let values = headers.get_all(key);
    let values_str = values
      .iter()
      .map(|e| e.to_str().unwrap().to_string())
      .collect::<Vec<String>>()
      .join(",");
    headers_.insert(key_str, values_str);
  }

  if response.status().is_redirection() {
    if let Some(location) = response.headers().get(LOCATION) {
      let location_string = location.to_str().unwrap();
      // debug!("Redirecting to {:?}...", &location_string);
      let new_url = resolve_url_from_location(&args.url, location_string);
      return Ok(FetchOnceResult::Redirect(new_url, headers_));
    } else {
      return Err(generic_error(format!(
        "Redirection from '{}' did not provide location header",
        args.url
      )));
    }
  }

  if response.status().is_client_error() || response.status().is_server_error()
  {
    let err = generic_error(format!(
      "Import '{}' failed: {}",
      args.url,
      response.status()
    ));
    return Err(err);
  }

  let body = response.bytes().await?.to_vec();

  Ok(FetchOnceResult::Code(body, headers_))
}

use std::sync::LazyLock;
use std::time::Duration;

use reqwest::blocking::Client;
use reqwest::header::HeaderMap;
use reqwest::StatusCode;
use serde::de::DeserializeOwned;

static API_KEY: LazyLock<Option<String>> = LazyLock::new(|| std::env::var("HIRO_API_KEY").ok());

const MAX_RETRY_ATTEMPTS: u32 = 3;

fn get_uint_header_value(headers: &HeaderMap, header_name: &str) -> Option<u32> {
    headers
        .get(header_name)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.parse().ok())
}

fn handle_response<T: DeserializeOwned>(
    response: reqwest::blocking::Response,
) -> Result<T, String> {
    let status = response.status();

    if status.is_success() {
        return response.json::<T>().map_err(|e| e.to_string());
    }

    let msg = response
        .text()
        .unwrap_or("Unable to read response body".to_string());
    Err(format!("http error - status: {status} - message: {msg}"))
}

fn should_retry_rate_limit(headers: &HeaderMap) -> bool {
    let remaining = get_uint_header_value(headers, "ratelimit-remaining");
    let retry_after = get_uint_header_value(headers, "retry-after");
    // This condition matches the case where the "second" rate limit is hit
    matches!((remaining, retry_after), (Some(0), Some(1)))
}

pub fn http_request<T: DeserializeOwned>(url: &str) -> Result<T, String> {
    let client = Client::new();

    let mut attempts = 0;
    loop {
        let mut request = client
            .get(url)
            .header("x-hiro-product", "clarinet-cli")
            .header("Accept", "application/json");

        if let Some(api_key) = API_KEY.as_ref() {
            request = request.header("x-api-key", api_key);
        }

        let response = request.send().map_err(|e| e.to_string())?;
        let status = response.status();

        if status.is_success() {
            return response.json::<T>().map_err(|e| e.to_string());
        }

        if status != StatusCode::TOO_MANY_REQUESTS {
            return handle_response(response);
        }

        let headers = response.headers().clone();
        if !should_retry_rate_limit(&headers) {
            return handle_response(response);
        }

        attempts += 1;
        if attempts >= MAX_RETRY_ATTEMPTS {
            return handle_response(response);
        }

        std::thread::sleep(Duration::from_secs(1));
    }
}

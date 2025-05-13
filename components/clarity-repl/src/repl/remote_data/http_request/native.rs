use serde::de::DeserializeOwned;
use std::sync::LazyLock;

static API_KEY: LazyLock<Option<String>> = LazyLock::new(|| std::env::var("HIRO_API_KEY").ok());

pub fn http_request<T: DeserializeOwned>(url: &str) -> Result<T, String> {
    let client = reqwest::blocking::Client::new();
    let mut request = client
        .get(url)
        .header("x-hiro-product", "clarinet-cli")
        .header("Accept", "application/json");

    if let Some(api_key) = API_KEY.as_ref() {
        request = request.header("x-api-key", api_key);
        println!("request: {:#?}", request);
    }
    let response = request.send().map_err(|e| e.to_string())?;
    if response.status() != 200 {
        return Err(format!(
            "http error - status: {} - message: {}",
            response.status(),
            response.text().unwrap()
        ));
    }
    response.json::<T>().map_err(|e| e.to_string())
}

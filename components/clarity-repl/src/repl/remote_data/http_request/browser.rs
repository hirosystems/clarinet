use serde::de::DeserializeOwned;
use wasm_bindgen::prelude::*;
use web_sys::XmlHttpRequest;

fn xml_http_request(url: &str) -> Result<String, JsValue> {
    let xhr = XmlHttpRequest::new()?;
    xhr.set_request_header("x-hiro-product", "clarinet-sdk")?;
    xhr.set_request_header("Accept", "application/json")?;
    xhr.open_with_async("GET", url, false)?;
    xhr.send()?;

    Ok(xhr.response_text()?.unwrap_or_default())
}

pub fn http_request<T: DeserializeOwned>(url: &str) -> Result<T, String> {
    match xml_http_request(url) {
        Ok(body) => Ok(serde_json::from_str(&body).map_err(|_| body)),
        Err(e) => Err(e.to_string()),
    }
}

use serde::de::DeserializeOwned;
use web_sys::XmlHttpRequest;

pub fn http_request<T: DeserializeOwned>(url: &str) -> Result<T, String> {
    let xhr = XmlHttpRequest::new().map_err(|e| format!("Failed to create XHR: {e:?}"))?;

    xhr.open_with_async("GET", url, false)
        .map_err(|e| format!("Failed to open XHR: {e:?}"))?;

    xhr.set_request_header("x-hiro-product", "clarinet-sdk")
        .map_err(|e| format!("Failed to set header: {e:?}"))?;
    xhr.set_request_header("Accept", "application/json")
        .map_err(|e| format!("Failed to set header: {e:?}"))?;

    xhr.send()
        .map_err(|e| format!("Failed to send request: {e:?}"))?;

    let status = xhr
        .status()
        .map_err(|e| format!("Failed to get status: {e:?}"))?;

    if status != 200 {
        return Err(format!(
            "HTTP request failed with status {}. Response: {}",
            status,
            xhr.response_text()
                .map_err(|e| format!("Failed to get response text: {e:?}"))?
                .unwrap_or_default()
        ));
    }

    let body = xhr
        .response_text()
        .map_err(|e| format!("Failed to get response text: {e:?}"))?;

    match body {
        Some(body) => serde_json::from_str(&body).map_err(|e| format!("Failed to parse JSON: {e}")),
        None => Err("Empty response received".to_string()),
    }
}

use std::sync::LazyLock;

use js_sys::Reflect;
use serde::de::DeserializeOwned;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(module = "child_process")]
extern "C" {
    #[wasm_bindgen(js_name = execSync)]
    fn exec_sync(command: &str) -> Vec<u8>;
}

// access process.env
#[wasm_bindgen(module = "process")]
extern "C" {
    #[wasm_bindgen(thread_local_v2, js_name = "env")]
    pub static ENV: JsValue;
}

fn get_env() -> JsValue {
    ENV.with(JsValue::clone)
}

static CURL_COMMAND: LazyLock<String> = LazyLock::new(|| {
    if !cfg!(windows) {
        return "curl".to_string();
    }
    match std::panic::catch_unwind(|| exec_sync("where curl.exe")) {
        Ok(output) if !output.is_empty() => "curl.exe".to_string(),
        _ => "curl".to_string(),
    }
});

static HIRO_API_KEY: LazyLock<Option<String>> = LazyLock::new(|| {
    let env = get_env();
    Reflect::get(&env, &JsValue::from_str("HIRO_API_KEY"))
        .ok()
        .and_then(|key| key.as_string())
});

pub fn http_request<T: DeserializeOwned>(url: &str) -> Result<T, String> {
    let mut curl_command = vec![format!("{} -s -X GET", *CURL_COMMAND)];

    curl_command.push("-H \"Accept: application/json\"".to_string());
    curl_command.push("-H \"x-hiro-product: clarinet-sdk\"".to_string());
    if let Some(api_key) = &*HIRO_API_KEY {
        curl_command.push(format!("-H \"x-api-key: {}\"", api_key));
    }
    curl_command.push(format!("\"{}\"", url));
    let command = curl_command.join(" ");

    let result = std::panic::catch_unwind(|| {
        let output = exec_sync(&command);
        let body = String::from_utf8_lossy(&output).into_owned();
        serde_json::from_str(&body).map_err(|_| body)
    });

    result
        .map_err(|_| "Request failed".to_string())?
        .map_err(|e| e.to_string())
}

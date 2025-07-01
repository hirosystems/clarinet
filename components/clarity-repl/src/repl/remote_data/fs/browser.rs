use std::path::Path;

use js_sys::{Object, Reflect};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = window)]
    fn vfs(action: String, data: JsValue) -> JsValue;
}

pub fn get_file_from_cache(cache_location: &Path, name: &Path) -> Option<String> {
    let path = cache_location.join(name);
    let options = Object::new();
    Reflect::set(
        &options,
        &JsValue::from_str("path"),
        &JsValue::from_str(path.to_str().unwrap()),
    )
    .unwrap();
    let file_data = vfs("vfs/readFile".into(), options.into());
    file_data.as_string()
}

pub fn write_file_to_cache(cache_location: &Path, name: &Path, data: &[u8]) {
    let cache_dir = std::path::Path::new(cache_location);
    let path = cache_dir.join(name);
    let options = Object::new();
    Reflect::set(
        &options,
        &JsValue::from_str("path"),
        &JsValue::from_str(path.to_str().unwrap()),
    )
    .unwrap();
    let data_js_value = JsValue::from(data.to_vec());
    Reflect::set(&options, &JsValue::from_str("content"), &data_js_value).unwrap();
    vfs("vfs/writeFile".into(), options.into());
}

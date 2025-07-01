use std::path::Path;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(module = "fs")]
extern "C" {
    #[wasm_bindgen(js_name = readFileSync)]
    fn read_file_sync(path: &str) -> Vec<u8>;
    #[wasm_bindgen(js_name = existsSync)]
    fn exists_sync(path: &str) -> bool;
    #[wasm_bindgen(js_name = mkdirSync)]
    fn mkdir_sync(path: &str, options: JsValue);
    #[wasm_bindgen(js_name = writeFileSync)]
    fn write_file_sync(path: &str, data: &[u8]);
}

pub fn get_file_from_cache(cache_location: &Path, name: &Path) -> Option<String> {
    let cache_path = cache_location.join(name);
    let cache_path_str = cache_path.to_str()?;
    if !exists_sync(cache_path_str) {
        None
    } else {
        Some(String::from_utf8(read_file_sync(cache_path_str)).ok()?)
    }
}

pub fn write_file_to_cache(cache_location: &Path, name: &Path, data: &[u8]) {
    let cache_location_str = cache_location.to_str().unwrap();
    if !exists_sync(cache_location_str) {
        let options = js_sys::Object::new();
        js_sys::Reflect::set(
            &options,
            &JsValue::from_str("recursive"),
            &JsValue::from_bool(true),
        )
        .unwrap();
        mkdir_sync(cache_location_str, options.into());
    }
    let cache_path = cache_location.join(name);
    write_file_sync(cache_path.to_str().unwrap(), data);
}

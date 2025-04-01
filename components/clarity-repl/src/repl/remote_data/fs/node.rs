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

pub fn get_file_from_cache(cache_location: &str, name: &str) -> Option<String> {
    let cache_dir = std::path::Path::new(cache_location);
    let cache_path = cache_dir.join(name);
    if !exists_sync(cache_path.to_str().unwrap()) {
        None
    } else {
        Some(String::from_utf8(read_file_sync(cache_path.to_str().unwrap())).unwrap())
    }
}

pub fn write_file_to_cache(cache_location: &str, name: &str, data: &[u8]) {
    let cache_dir = std::path::Path::new(cache_location);
    if !exists_sync(cache_dir.to_str().unwrap()) {
        let options = js_sys::Object::new();
        js_sys::Reflect::set(
            &options,
            &JsValue::from_str("recursive"),
            &JsValue::from_bool(true),
        )
        .unwrap();
        mkdir_sync(cache_dir.to_str().unwrap(), options.into());
    }
    let cache_path = cache_dir.join(name);
    write_file_sync(cache_path.to_str().unwrap(), data);
}

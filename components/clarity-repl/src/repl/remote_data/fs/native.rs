pub fn get_file_from_cache(cache_location: &str, name: &str) -> Option<String> {
    let cache_dir = std::path::Path::new(cache_location);
    let cache_path = cache_dir.join(name);
    if cache_path.exists() {
        Some(std::fs::read_to_string(&cache_path).unwrap())
    } else {
        None
    }
}

pub fn write_file_to_cache(cache_location: &str, name: &str, data: &[u8]) {
    let cache_dir = std::path::Path::new(cache_location);
    if !cache_dir.exists() {
        std::fs::create_dir_all(cache_dir).unwrap();
    }
    let cache_path = cache_dir.join(name);
    std::fs::write(&cache_path, data).unwrap();
}

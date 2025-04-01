use std::path::Path;

pub fn get_file_from_cache(cache_location: &Path, name: &Path) -> Option<String> {
    let cache_path = cache_location.join(name);
    if cache_path.exists() {
        Some(std::fs::read_to_string(&cache_path).unwrap())
    } else {
        None
    }
}

pub fn write_file_to_cache(cache_location: &Path, name: &Path, data: &[u8]) {
    if !cache_location.exists() {
        std::fs::create_dir_all(cache_location).unwrap();
    }
    println!("cache_location: {:?}", cache_location);
    println!("name: {:?}", name);
    let cache_path = cache_location.join(name);
    println!("cache_path: {:?}", cache_path);
    std::fs::write(&cache_path, data).unwrap();
}

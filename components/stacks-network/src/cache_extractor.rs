use crate::DevnetEvent;
use flate2::read::GzDecoder;
use std::fs;
use std::path::Path;
use std::sync::mpsc::Sender;
use tar::Archive;

/// Extract embedded devnet cache to the specified directory
pub fn extract_embedded_cache(
    cache_dir: &Path,
    devnet_event_tx: &Sender<DevnetEvent>,
) -> Result<bool, String> {
    // Include the compressed cache data at compile time
    const DEVNET_CACHE: &[u8] = include_bytes!("../data/devnet_default_snapshot.tar.gz");

    let _ = devnet_event_tx.send(DevnetEvent::info(
        "Extracting embedded devnet cache data...".to_string(),
    ));

    // Create the cache directory if it doesn't exist
    fs::create_dir_all(cache_dir)
        .map_err(|e| format!("Failed to create cache directory: {}", e))?;

    // Decompress and extract the tar archive
    let decoder = GzDecoder::new(DEVNET_CACHE);
    let mut archive = Archive::new(decoder);

    // Extract to the cache directory, stripping the top-level "devnet" directory
    for entry in archive
        .entries()
        .map_err(|e| format!("Failed to read archive entries: {}", e))?
    {
        let mut entry = entry.map_err(|e| format!("Failed to read archive entry: {}", e))?;
        let path = entry
            .path()
            .map_err(|e| format!("Failed to get entry path: {}", e))?;

        // Strip the leading "devnet/" from the path
        if let Ok(stripped_path) = path.strip_prefix("devnet") {
            let dest_path = cache_dir.join(stripped_path);
            entry
                .unpack(&dest_path)
                .map_err(|e| format!("Failed to extract file {}: {}", dest_path.display(), e))?;
        }
    }

    // Create a marker file to indicate the cache is ready
    let marker_path = cache_dir.join("epoch_3_ready");
    if let Some(parent) = marker_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create marker directory: {}", e))?;
    }

    fs::write(&marker_path, "").map_err(|e| format!("Failed to create cache marker: {}", e))?;

    let _ = devnet_event_tx.send(DevnetEvent::success(
        "Embedded devnet cache extracted successfully".to_string(),
    ));

    Ok(true)
}

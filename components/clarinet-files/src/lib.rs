extern crate serde;

#[macro_use]
extern crate serde_derive;

pub extern crate bip39;
pub extern crate url;

mod network_manifest;
mod project_manifest;

#[cfg(feature = "wasm")]
mod wasm_fs_accessor;
#[cfg(feature = "wasm")]
pub use wasm_fs_accessor::WASMFileSystemAccessor;

use chainhook_types::StacksNetwork;
pub use network_manifest::{
    compute_addresses, AccountConfig, DevnetConfig, DevnetConfigFile, NetworkManifest,
    NetworkManifestFile, PoxStackingOrder, DEFAULT_DERIVATION_PATH,
};
pub use project_manifest::{ProjectManifest, ProjectManifestFile, RequirementConfig};
use serde::ser::{Serialize, SerializeMap, Serializer};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::{borrow::BorrowMut, path::PathBuf, str::FromStr};
use url::Url;

pub const DEFAULT_DEVNET_BALANCE: u64 = 100_000_000_000_000;

pub type FileAccessorResult<T> = Pin<Box<dyn Future<Output = Result<T, String>>>>;

pub trait FileAccessor {
    fn file_exists(&self, path: String) -> FileAccessorResult<bool>;
    fn read_file(&self, path: String) -> FileAccessorResult<String>;
    fn read_contracts_content(
        &self,
        contracts_paths: Vec<String>,
    ) -> FileAccessorResult<HashMap<String, String>>;
    fn write_file(&self, path: String, content: &[u8]) -> FileAccessorResult<()>;
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum FileLocation {
    FileSystem { path: PathBuf },
    Url { url: Url },
}

impl FileLocation {
    pub fn try_parse(
        location_string: &str,
        project_root_location_hint: Option<&FileLocation>,
    ) -> Option<FileLocation> {
        if let Ok(location) = FileLocation::from_url_string(location_string) {
            return Some(location);
        }
        if let Ok(FileLocation::FileSystem { path }) =
            FileLocation::from_path_string(location_string)
        {
            match (project_root_location_hint, path.is_relative()) {
                (None, true) => return None,
                (Some(hint), true) => {
                    let mut location = hint.clone();
                    location.append_path(location_string).ok()?;
                    return Some(location);
                }
                (_, false) => return Some(FileLocation::FileSystem { path }),
            }
        }
        None
    }

    pub fn from_path(path: PathBuf) -> FileLocation {
        FileLocation::FileSystem { path }
    }

    pub fn from_url(url: Url) -> FileLocation {
        FileLocation::Url { url }
    }

    pub fn from_url_string(url_string: &str) -> Result<FileLocation, String> {
        let url = Url::from_str(url_string)
            .map_err(|e| format!("unable to parse {} as a url\n{:?}", url_string, e))?;

        #[cfg(not(feature = "wasm"))]
        if url.scheme() == "file" {
            let path = url
                .to_file_path()
                .map_err(|_| format!("unable to conver url {} to path", url))?;
            return Ok(FileLocation::FileSystem { path });
        }

        Ok(FileLocation::Url { url })
    }

    pub fn from_path_string(path_string: &str) -> Result<FileLocation, String> {
        let path = PathBuf::from_str(path_string)
            .map_err(|e| format!("unable to parse {} as a path\n{:?}", path_string, e))?;
        Ok(FileLocation::FileSystem { path })
    }

    pub fn append_path(&mut self, path_string: &str) -> Result<(), String> {
        let path_to_append = PathBuf::from_str(path_string)
            .map_err(|e| format!("unable to read relative path {}\n{:?}", path_string, e))?;
        match self.borrow_mut() {
            FileLocation::FileSystem { path } => {
                path.extend(&path_to_append);
            }
            FileLocation::Url { url } => {
                let mut paths_segments = url
                    .path_segments_mut()
                    .map_err(|_| format!("unable to mutate url"))?;
                for component in path_to_append.components() {
                    let segment = component
                        .as_os_str()
                        .to_str()
                        .ok_or(format!("unable to format component {:?}", component))?;
                    paths_segments.push(segment);
                }
            }
        }
        Ok(())
    }

    pub fn read_content_as_utf8(&self) -> Result<String, String> {
        let content = self.read_content()?;
        let contract_as_utf8 = String::from_utf8(content).map_err(|e| {
            format!(
                "unable to read content as utf8 {}\n{:?}",
                self.to_string(),
                e
            )
        })?;
        Ok(contract_as_utf8)
    }

    fn fs_read_content(path: &PathBuf) -> Result<Vec<u8>, String> {
        use std::fs::File;
        use std::io::{BufReader, Read};
        let file = File::open(path.clone())
            .map_err(|e| format!("unable to read file {}\n{:?}", path.display(), e))?;
        let mut file_reader = BufReader::new(file);
        let mut file_buffer = vec![];
        file_reader
            .read_to_end(&mut file_buffer)
            .map_err(|e| format!("unable to read file {}\n{:?}", path.display(), e))?;
        Ok(file_buffer)
    }

    fn fs_exists(path: &PathBuf) -> bool {
        path.exists()
    }

    fn fs_write_content(file_path: &PathBuf, content: &[u8]) -> Result<(), String> {
        use std::fs::{self, File};
        use std::io::Write;
        let mut parent_directory = file_path.clone();
        parent_directory.pop();
        fs::create_dir_all(&parent_directory).map_err(|e| {
            format!(
                "unable to create parent directory {}\n{}",
                parent_directory.display(),
                e
            )
        })?;
        let mut file = File::create(&file_path)
            .map_err(|e| format!("unable to open file {}\n{}", file_path.display(), e))?;
        file.write_all(content)
            .map_err(|e| format!("unable to write file {}\n{}", file_path.display(), e))?;
        Ok(())
    }

    pub async fn get_project_manifest_location(
        &self,
        file_accessor: Option<&Box<dyn FileAccessor>>,
    ) -> Result<FileLocation, String> {
        match file_accessor {
            None => {
                let mut project_root_location = self.get_project_root_location()?;
                project_root_location.append_path("Clarinet.toml")?;
                Ok(project_root_location)
            }
            Some(file_accessor) => {
                let mut manifest_location = None;
                let mut parent_location = self.get_parent_location();
                while let Ok(ref parent) = parent_location {
                    let mut candidate = parent.clone();
                    candidate.append_path("Clarinet.toml")?;

                    if let Ok(exists) = file_accessor.file_exists(candidate.to_string()).await {
                        if exists {
                            manifest_location = Some(candidate);
                            break;
                        }
                    }
                    if &parent.get_parent_location().unwrap() == parent {
                        break;
                    }
                    parent_location = parent.get_parent_location();
                }
                match manifest_location {
                    Some(manifest_location) => Ok(manifest_location),
                    None => Err(format!(
                        "No Clarinet.toml is associated to the contract {}",
                        self.get_file_name().unwrap_or_default()
                    )),
                }
            }
        }
    }

    pub fn get_project_root_location(&self) -> Result<FileLocation, String> {
        let mut project_root_location = self.clone();
        match project_root_location.borrow_mut() {
            FileLocation::FileSystem { path } => {
                let mut manifest_found = false;
                while path.pop() {
                    path.push("Clarinet.toml");
                    if FileLocation::fs_exists(path) {
                        path.pop();
                        manifest_found = true;
                        break;
                    }
                    path.pop();
                }

                match manifest_found {
                    true => Ok(project_root_location),
                    false => Err(format!(
                        "unable to find root location from {}",
                        self.to_string()
                    )),
                }
            }
            _ => {
                unimplemented!();
            }
        }
    }

    pub fn get_parent_location(&self) -> Result<FileLocation, String> {
        let mut parent_location = self.clone();
        match &mut parent_location {
            FileLocation::FileSystem { path } => {
                let mut parent = path.clone();
                parent.pop();
                if parent.to_str() == path.to_str() {
                    return Err(String::from("reached root"));
                }
                path.pop();
            }
            FileLocation::Url { url } => {
                let mut segments = url
                    .path_segments_mut()
                    .map_err(|_| format!("unable to mutate url"))?;
                segments.pop();
            }
        }
        Ok(parent_location)
    }

    pub fn get_network_manifest_location(
        &self,
        network: &StacksNetwork,
    ) -> Result<FileLocation, String> {
        let mut network_manifest_location = self.get_project_root_location()?;
        network_manifest_location.append_path("settings")?;
        network_manifest_location.append_path(match network {
            StacksNetwork::Devnet | StacksNetwork::Simnet => "Devnet.toml",
            StacksNetwork::Testnet => "Testnet.toml",
            StacksNetwork::Mainnet => "Mainnet.toml",
        })?;
        Ok(network_manifest_location)
    }

    pub fn get_relative_path_from_base(
        &self,
        base_location: &FileLocation,
    ) -> Result<String, String> {
        let file = self.to_string();
        Ok(file[(base_location.to_string().len() + 1)..].to_string())
    }

    pub fn get_relative_location(&self) -> Result<String, String> {
        let base = self
            .get_project_root_location()
            .and_then(|l| Ok(l.to_string()))?;
        let file = self.to_string();
        Ok(file[(base.len() + 1)..].to_string())
    }

    pub fn get_file_name(&self) -> Option<String> {
        match self {
            FileLocation::FileSystem { path } => {
                path.file_name().and_then(|f| Some(f.to_str()?.to_string()))
            }
            FileLocation::Url { url } => url
                .path_segments()
                .and_then(|p| Some(p.last()?.to_string())),
        }
    }
}

impl FileLocation {
    pub fn read_content(&self) -> Result<Vec<u8>, String> {
        let bytes = match &self {
            FileLocation::FileSystem { path } => FileLocation::fs_read_content(&path),
            FileLocation::Url { url } => match url.scheme() {
                #[cfg(not(feature = "wasm"))]
                "file" => {
                    let path = url
                        .to_file_path()
                        .map_err(|e| format!("unable to convert url {} to path\n{:?}", url, e))?;
                    FileLocation::fs_read_content(&path)
                }
                "http" | "https" => {
                    unimplemented!()
                }
                _ => {
                    unimplemented!()
                }
            },
        }?;
        Ok(bytes)
    }

    pub fn exists(&self) -> bool {
        match self {
            FileLocation::FileSystem { path } => FileLocation::fs_exists(path),
            FileLocation::Url { url: _url } => unimplemented!(),
        }
    }

    pub fn write_content(&self, content: &[u8]) -> Result<(), String> {
        match self {
            FileLocation::FileSystem { path } => FileLocation::fs_write_content(path, content),
            FileLocation::Url { url: _url } => unimplemented!(),
        }
    }

    pub fn to_url_string(&self) -> Result<String, String> {
        match self {
            #[cfg(not(feature = "wasm"))]
            FileLocation::FileSystem { path } => {
                let file_path = self.to_string();
                let url = Url::from_file_path(file_path)
                    .map_err(|_| format!("unable to conver path {} to url", path.display()))?;
                Ok(url.to_string())
            }
            FileLocation::Url { url } => Ok(url.to_string()),
            #[allow(unreachable_patterns)]
            _ => unreachable!(),
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            FileLocation::FileSystem { path } => {
                format!("{}", path.display())
            }
            FileLocation::Url { url } => url.to_string(),
        }
    }
}

impl Serialize for FileLocation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(1))?;
        match self {
            FileLocation::FileSystem { path: _ } => {
                let path = match self.get_relative_location() {
                    Ok(relative_path) => relative_path, // Use relative path if possible
                    Err(_) => self.to_string(),         // Fallback on fully qualified path
                };
                map.serialize_entry("path", &path)?;
            }
            FileLocation::Url { url } => {
                map.serialize_entry("url", &url.to_string())?;
            }
        }
        map.end()
    }
}

extern crate serde;

#[macro_use]
extern crate serde_derive;

pub extern crate bip39;
pub extern crate url;

mod network_manifest;
mod project_manifest;

pub use network_manifest::{
    compute_addresses, AccountConfig, DevnetConfig, DevnetConfigFile, NetworkManifest,
    NetworkManifestFile, PoxStackingOrder, DEFAULT_DERIVATION_PATH,
};
use orchestra_types::StacksNetwork;
pub use project_manifest::{
    ContractConfig, ProjectManifest, ProjectManifestFile, RequirementConfig,
};
use std::{borrow::BorrowMut, path::PathBuf, str::FromStr};
use url::Url;

pub const DEFAULT_DEVNET_BALANCE: u64 = 100_000_000_000_000;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
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
                    let _ = location.append_relative_path(location_string);
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
            .map_err(|e| format!("unable to parse {} as a url: {:?}", url_string, e))?;

        #[cfg(not(feature = "wasm"))]
        if url.scheme() == "file" {
            let path = url
                .to_file_path()
                .map_err(|_| format!("unable to conver url {} to path", url))?;
            return Ok(FileLocation::FileSystem { path })
        }

        Ok(FileLocation::Url { url })
    }

    pub fn from_path_string(path_string: &str) -> Result<FileLocation, String> {
        let path = PathBuf::from_str(path_string)
            .map_err(|e| format!("unable to parse {} as a path: {:?}", path_string, e))?;
        Ok(FileLocation::FileSystem { path })
    }

    pub fn append_relative_path(&mut self, path_string: &str) -> Result<(), String> {
        let path_to_append = PathBuf::from_str(path_string)
            .map_err(|e| format!("unable to read relative path {}: {:?}", path_string, e))?;
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
                "unable to read content as utf8 {}: {:?}",
                self.to_string(),
                e
            )
        })?;
        Ok(contract_as_utf8)
    }

    pub fn read_content(&self) -> Result<Vec<u8>, String> {
        let bytes = match &self {
            FileLocation::FileSystem { path } => FileLocation::fs_read_content(&path),
            FileLocation::Url { url } => match url.scheme() {
                #[cfg(not(feature = "wasm"))]
                "file" => {
                    let path = url
                        .to_file_path()
                        .map_err(|e| format!("unable to convert url {} to path: {:?}", url, e))?;
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

    fn fs_read_content(path: &PathBuf) -> Result<Vec<u8>, String> {
        use std::fs::File;
        use std::io::{BufReader, Read};
        let file = File::open(path.clone())
            .map_err(|e| format!("unable to read file {}: {:?}", path.display(), e))?;
        let mut file_reader = BufReader::new(file);
        let mut file_buffer = vec![];
        file_reader
            .read_to_end(&mut file_buffer)
            .map_err(|e| format!("unable to read file {}: {:?}", path.display(), e))?;
        Ok(file_buffer)
    }

    pub fn exists(&self) -> bool {
        match self {
            FileLocation::FileSystem { path } => FileLocation::fs_exists(path),
            FileLocation::Url { url } => unimplemented!(),
        }
    }

    fn fs_exists(path: &PathBuf) -> bool {
        path.exists()
    }

    fn url_exists(path: &Url) -> bool {
        unimplemented!()
    }

    pub fn write_content(&self, content: &[u8]) -> Result<(), String> {
        match self {
            FileLocation::FileSystem { path } => FileLocation::fs_write_content(path, content),
            FileLocation::Url { url } => unimplemented!(),
        }
    }

    fn fs_write_content(file_path: &PathBuf, content: &[u8]) -> Result<(), String> {
        use std::fs::{self, File};
        use std::io::Write;
        let mut parent_directory = file_path.clone();
        parent_directory.pop();
        fs::create_dir_all(&parent_directory).map_err(|e| {
            format!(
                "unable to create parent directory {}",
                parent_directory.display()
            )
        })?;
        let mut file = File::create(&file_path)
            .map_err(|e| format!("unable to open file {}", file_path.display()))?;
        file.write_all(content)
            .map_err(|e| format!("unable to write file {}", file_path.display()))?;
        Ok(())
    }

    pub fn get_parent(&self) -> Option<FileLocation> {
        None
    }

    pub fn is_project_root_location(&self) -> bool {
        false
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
            FileLocation::Url { url } => {
                let mut manifest_found = false;

                while url.path() != "/" {
                    {
                        let mut segments = url
                            .path_segments_mut()
                            .map_err(|_| format!("unable to mutate url"))?;
                        segments.pop();
                        segments.push("Clarinet.toml");
                    }
                    if FileLocation::url_exists(url) {
                        {
                            let mut segments = url
                                .path_segments_mut()
                                .map_err(|_| format!("unable to mutate url"))?;
                            segments.pop();
                        }
                        manifest_found = true;
                        break;
                    }
                    {
                        let mut segments = url
                            .path_segments_mut()
                            .map_err(|_| format!("unable to mutate url"))?;
                        segments.pop();
                    }
                }

                match manifest_found {
                    true => Ok(project_root_location),
                    false => Err(format!(
                        "unable to find root location from {}",
                        self.to_string()
                    )),
                }
            }
        }
    }

    pub fn get_project_manifest_location(&self) -> Result<FileLocation, String> {
        let mut project_root_location = self.get_project_root_location()?;
        project_root_location.append_relative_path("Clarinet.toml")?;
        Ok(project_root_location)
    }

    pub fn get_network_manifest_location(
        &self,
        network: &StacksNetwork,
    ) -> Result<FileLocation, String> {
        let mut network_manifest_location = self.get_project_root_location()?;
        network_manifest_location.append_relative_path("settings")?;
        network_manifest_location.append_relative_path(match network {
            StacksNetwork::Devnet | StacksNetwork::Simnet => "Devnet.toml",
            StacksNetwork::Testnet => "Testnet.toml",
            StacksNetwork::Mainnet => "Mainnet.toml",
        })?;
        Ok(network_manifest_location)
    }

    pub fn get_relative_location(&self) -> Result<String, String> {
        let base = self
            .get_project_root_location()
            .and_then(|l| Ok(l.to_string()))?;
        let file = self.to_string();
        Ok(file[(base.len() + 1)..].to_string())
    }

    pub fn to_string(&self) -> String {
        match self {
            FileLocation::FileSystem { path } => {
                format!("{}", path.display())
            }
            FileLocation::Url { url } => url.to_string(),
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
            _ => unreachable!()
        }
    }
}

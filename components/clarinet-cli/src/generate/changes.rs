use std::collections::HashMap;

use clarinet_core::ClarityContract;
use clarinet_files::{FileLocation, RequirementConfig};

#[derive(Clone, Debug)]
pub struct FileCreation {
    pub comment: String,
    pub name: String,
    pub content: String,
    pub path: String,
}

#[derive(Clone, Debug)]
pub struct FileDeletion {
    pub comment: String,
    pub name: String,
    pub path: String,
}

#[derive(Clone, Debug)]
pub struct DirectoryCreation {
    pub comment: String,
    pub name: String,
    pub path: String,
}

#[derive(Clone, Debug)]
pub struct TOMLEdition {
    pub comment: String,
    pub manifest_location: FileLocation,
    pub contracts_to_add: HashMap<String, ClarityContract>,
    pub contracts_to_rm: Vec<String>,
    pub requirements_to_add: Vec<RequirementConfig>,
}

#[derive(Clone, Debug)]
pub enum Changes {
    AddFile(FileCreation),
    RemoveFile(FileDeletion),
    AddDirectory(DirectoryCreation),
    EditTOML(TOMLEdition),
}

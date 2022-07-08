use crate::FileAccessor;

use super::FileLocation;
use clarity_repl::repl;
use std::collections::BTreeMap;
use toml::value::Value;

#[derive(Serialize, Deserialize, Debug)]
pub struct ProjectManifestFile {
    project: ProjectConfigFile,
    contracts: Option<Value>,
    repl: Option<repl::SettingsFile>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProjectConfigFile {
    name: String,
    authors: Option<Vec<String>>,
    description: Option<String>,
    telemetry: Option<bool>,
    requirements: Option<Value>,
    boot_contracts: Option<Vec<String>>,

    // The fields below have been moved into repl above, but are kept here for
    // backwards compatibility.
    analysis: Option<Vec<clarity_repl::analysis::Pass>>,
    costs_version: Option<u32>,
    parser_version: Option<u32>,
    cache_dir: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct ProjectManifest {
    pub project: ProjectConfig,
    #[serde(serialize_with = "toml::ser::tables_last")]
    pub contracts: BTreeMap<String, ContractConfig>,
    #[serde(rename = "repl")]
    pub repl_settings: repl::Settings,
    #[serde(skip_serializing)]
    pub location: FileLocation,
}

#[derive(Serialize, Debug, Clone)]
pub struct ProjectConfig {
    pub name: String,
    pub authors: Vec<String>,
    pub description: String,
    pub telemetry: bool,
    pub requirements: Option<Vec<RequirementConfig>>,
    pub cache_location: FileLocation,
    pub boot_contracts: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct RequirementConfig {
    pub contract_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ContractConfig {
    pub path: String,
    pub deployer: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NotebookConfig {
    pub name: String,
    pub path: String,
}

impl ProjectManifest {
    pub async fn from_file_accessor(
        location: &FileLocation,
        file_accessor: &Box<dyn FileAccessor>,
    ) -> Result<ProjectManifest, String> {
        let perform_file_access = file_accessor.read_manifest_content(location.clone());
        let (location, content) = perform_file_access.await?;

        let project_manifest_file: ProjectManifestFile = match toml::from_slice(&content.as_bytes())
        {
            Ok(s) => s,
            Err(e) => {
                return Err(format!("Clarinet.toml file malformatted {:?}", e));
            }
        };
        ProjectManifest::from_project_manifest_file(project_manifest_file, &location)
    }

    pub fn from_location(location: &FileLocation) -> Result<ProjectManifest, String> {
        let project_manifest_file_content = location.read_content()?;
        let project_manifest_file: ProjectManifestFile =
            match toml::from_slice(&project_manifest_file_content[..]) {
                Ok(s) => s,
                Err(e) => {
                    return Err(format!("Clarinet.toml file malformatted {:?}", e));
                }
            };

        ProjectManifest::from_project_manifest_file(project_manifest_file, location)
    }

    pub fn from_project_manifest_file(
        project_manifest_file: ProjectManifestFile,
        manifest_location: &FileLocation,
    ) -> Result<ProjectManifest, String> {
        let mut repl_settings = if let Some(repl_settings) = project_manifest_file.repl {
            repl::Settings::from(repl_settings)
        } else {
            repl::Settings::default()
        };

        // Check for deprecated settings
        if let Some(passes) = project_manifest_file.project.analysis {
            repl_settings.analysis.set_passes(passes);
        }
        if let Some(costs_version) = project_manifest_file.project.costs_version {
            repl_settings.costs_version = costs_version;
        }

        let project_name = project_manifest_file.project.name;
        let mut project_root_location = manifest_location.get_parent_location().unwrap();
        let cache_location = match project_manifest_file.project.cache_dir {
            Some(ref path) => FileLocation::try_parse(path, Some(&project_root_location))
                .ok_or(format!("unable to parse path {}", path))?,
            None => {
                project_root_location.append_path(".requirements")?;
                project_root_location
            }
        };

        let project = ProjectConfig {
            name: project_name.clone(),
            requirements: None,
            description: project_manifest_file
                .project
                .description
                .unwrap_or("".into()),
            authors: project_manifest_file.project.authors.unwrap_or(vec![]),
            telemetry: project_manifest_file.project.telemetry.unwrap_or(false),
            cache_location,
            boot_contracts: project_manifest_file.project.boot_contracts.unwrap_or(vec![
                "pox".to_string(),
                format!("costs-v{}", repl_settings.costs_version),
                "bns".to_string(),
            ]),
        };

        let mut config = ProjectManifest {
            project,
            contracts: BTreeMap::new(),
            repl_settings,
            location: manifest_location.clone(),
        };
        let mut config_contracts = BTreeMap::new();
        let mut config_requirements: Vec<RequirementConfig> = Vec::new();

        match project_manifest_file.project.requirements {
            Some(Value::Array(requirements)) => {
                for link_settings in requirements.iter() {
                    match link_settings {
                        Value::Table(link_settings) => {
                            let contract_id = match link_settings.get("contract_id") {
                                Some(Value::String(contract_id)) => contract_id.to_string(),
                                _ => continue,
                            };
                            config_requirements.push(RequirementConfig { contract_id });
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        };

        match project_manifest_file.contracts {
            Some(Value::Table(contracts)) => {
                for (contract_name, contract_settings) in contracts.iter() {
                    match contract_settings {
                        Value::Table(contract_settings) => {
                            let path = match contract_settings.get("path") {
                                Some(Value::String(path)) => path.to_string(),
                                _ => continue,
                            };
                            if contract_settings.get("depends_on").is_some() {
                                // We could print a deprecation notice here if that code path
                                // was not used by the LSP.
                            }
                            let deployer = match contract_settings.get("deployer") {
                                Some(Value::String(path)) => Some(path.to_string()),
                                _ => None,
                            };
                            config_contracts.insert(
                                contract_name.to_string(),
                                ContractConfig { path, deployer },
                            );
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        };
        config.contracts = config_contracts;
        config.project.requirements = Some(config_requirements);
        Ok(config)
    }
}

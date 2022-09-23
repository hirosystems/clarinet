use crate::FileAccessor;

use super::FileLocation;
use clarity_repl::clarity::stacks_common::types::StacksEpochId;
use clarity_repl::clarity::ClarityVersion;
use clarity_repl::repl;
use clarity_repl::repl::{
    ClarityCodeSource, ClarityContract, ContractDeployer, DEFAULT_CLARITY_VERSION, DEFAULT_EPOCH,
};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::str::FromStr;
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
    pub contracts: BTreeMap<String, ClarityContract>,
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
        let content = file_accessor.read_file(location.to_string()).await?;

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

        let project_name = project_manifest_file.project.name;
        let mut project_root_location = manifest_location.get_parent_location()?;
        let cache_location = match project_manifest_file.project.cache_dir {
            Some(ref path) => FileLocation::try_parse(path, Some(&project_root_location))
                .ok_or(format!("unable to parse path {}", path))?,
            None => {
                project_root_location.append_path(".cache")?;
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
                "costs".to_string(),
                "pox".to_string(),
                "pox-2".to_string(),
                "lockup".to_string(),
                "costs-2".to_string(),
                "cost-voting".to_string(),
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
                            let code_source = match contract_settings.get("path") {
                                Some(Value::String(path)) => match PathBuf::from_str(path) {
                                    Ok(path) => ClarityCodeSource::ContractOnDisk(path),
                                    Err(e) => {
                                        return Err(format!(
                                            "unable to parse path {} ({})",
                                            path, e
                                        ))
                                    }
                                },
                                _ => continue,
                            };
                            let deployer = match contract_settings.get("deployer") {
                                Some(Value::String(path)) => {
                                    ContractDeployer::LabeledDeployer(path.clone())
                                }
                                _ => ContractDeployer::DefaultDeployer,
                            };

                            let clarity_version = match contract_settings.get("clarity_version") {
                                Some(Value::Integer(version)) => {
                                    if version.eq(&1) {
                                        ClarityVersion::Clarity1
                                    } else if version.eq(&2) {
                                        ClarityVersion::Clarity2
                                    } else {
                                        return Err(
                                            "clarity_version field invalid (value supported: 1, 2)"
                                                .to_string(),
                                        );
                                    }
                                }
                                _ => DEFAULT_CLARITY_VERSION,
                            };
                            let epoch = match contract_settings.get("epoch") {
                                Some(Value::String(epoch)) => {
                                    if epoch.eq("2.0") {
                                        StacksEpochId::Epoch20
                                    } else if epoch.eq("2.05") {
                                        StacksEpochId::Epoch2_05
                                    } else if epoch.eq("2.1") {
                                        StacksEpochId::Epoch21
                                    } else {
                                        return Err("epoch field invalid (value supported: '2.0', '2.05', '2.1')".to_string());
                                    }
                                }
                                _ => DEFAULT_EPOCH,
                            };
                            config_contracts.insert(
                                contract_name.to_string(),
                                ClarityContract {
                                    name: contract_name.clone(),
                                    code_source,
                                    deployer,
                                    clarity_version,
                                    epoch,
                                },
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

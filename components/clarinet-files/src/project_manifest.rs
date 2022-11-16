use crate::FileAccessor;

use super::FileLocation;
use clarity_repl::clarity::stacks_common::types::StacksEpochId;
use clarity_repl::clarity::ClarityVersion;
use clarity_repl::repl;
use clarity_repl::repl::{
    ClarityCodeSource, ClarityContract, ContractDeployer, DEFAULT_CLARITY_VERSION, DEFAULT_EPOCH,
};
use serde::ser::SerializeMap;
use serde::{Serialize, Serializer};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::str::FromStr;
use toml::value::Value;

const INVALID_CLARITY_VERSION: &str = "clarity_version field invalid (value supported: 1, 2)";
const INVALID_EPOCH: &str = "epoch field invalid (value supported: 2.0, 2.05, 2.1)";

#[derive(Deserialize, Debug, Clone)]
pub struct ClarityContractMetadata {
    pub name: String,
    pub deployer: ContractDeployer,
    pub clarity_version: ClarityVersion,
    pub epoch: StacksEpochId,
}

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
    #[serde(skip_serializing)]
    pub contracts_settings: HashMap<FileLocation, ClarityContractMetadata>,
}

#[derive(Debug, Clone)]
pub struct ProjectConfig {
    pub name: String,
    pub authors: Vec<String>,
    pub description: String,
    pub telemetry: bool,
    pub requirements: Option<Vec<RequirementConfig>>,
    pub cache_location: FileLocation,
    pub boot_contracts: Vec<String>,
}

impl Serialize for ProjectConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry("name", &self.name)?;
        map.serialize_entry("description", &self.description)?;
        map.serialize_entry("authors", &self.authors)?;
        map.serialize_entry("telemetry", &self.telemetry)?;
        map.serialize_entry(
            "cache_dir",
            &self
                .cache_location
                .get_relative_location()
                .expect("invalida cache_dir property"),
        )?;
        if self.requirements.is_some() {
            map.serialize_entry("requirements", &self.requirements)?;
        }
        map.end()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct RequirementConfig {
    pub contract_id: String,
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
        let project_root_location = manifest_location.get_parent_location()?;
        let cache_location = match project_manifest_file.project.cache_dir {
            Some(ref path) => FileLocation::try_parse(path, Some(&project_root_location))
                .ok_or(format!("unable to parse path {}", path))?,
            None => {
                let mut cache_location = project_root_location.clone();
                cache_location.append_path(".cache")?;
                cache_location
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
            boot_contracts: vec![
                "costs".to_string(),
                "pox".to_string(),
                "pox-2".to_string(),
                "lockup".to_string(),
                "costs-2".to_string(),
                "cost-voting".to_string(),
                "bns".to_string(),
            ],
        };

        let mut config = ProjectManifest {
            project,
            contracts: BTreeMap::new(),
            repl_settings,
            location: manifest_location.clone(),
            contracts_settings: HashMap::new(),
        };
        let mut config_contracts = BTreeMap::new();
        let mut contracts_settings = HashMap::new();
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
                            let contract_path = match contract_settings.get("path") {
                                Some(Value::String(path)) => path,
                                _ => continue,
                            };
                            let code_source = match PathBuf::from_str(contract_path) {
                                Ok(path) => ClarityCodeSource::ContractOnDisk(path),
                                Err(e) => {
                                    return Err(format!(
                                        "unable to parse path {} ({})",
                                        contract_path, e
                                    ))
                                }
                            };
                            let deployer = match contract_settings.get("deployer") {
                                Some(Value::String(path)) => {
                                    ContractDeployer::LabeledDeployer(path.clone())
                                }
                                _ => ContractDeployer::DefaultDeployer,
                            };

                            let clarity_version = match contract_settings.get("clarity_version") {
                                None => DEFAULT_CLARITY_VERSION,
                                Some(Value::Integer(version)) => {
                                    if version.eq(&1) {
                                        ClarityVersion::Clarity1
                                    } else if version.eq(&2) {
                                        ClarityVersion::Clarity2
                                    } else {
                                        return Err(INVALID_CLARITY_VERSION.into());
                                    }
                                }
                                _ => {
                                    return Err(INVALID_CLARITY_VERSION.into());
                                }
                            };
                            let epoch = match contract_settings.get("epoch") {
                                None => DEFAULT_EPOCH,
                                Some(Value::String(epoch)) => {
                                    if epoch.eq("2.0") {
                                        StacksEpochId::Epoch20
                                    } else if epoch.eq("2.05") {
                                        StacksEpochId::Epoch2_05
                                    } else if epoch.eq("2.1") {
                                        StacksEpochId::Epoch21
                                    } else {
                                        return Err(INVALID_EPOCH.into());
                                    }
                                }
                                Some(Value::Float(epoch)) => {
                                    if epoch.eq(&2.0) {
                                        StacksEpochId::Epoch20
                                    } else if epoch.eq(&2.05) {
                                        StacksEpochId::Epoch2_05
                                    } else if epoch.eq(&2.1) {
                                        StacksEpochId::Epoch21
                                    } else {
                                        return Err(INVALID_EPOCH.into());
                                    }
                                }
                                _ => {
                                    return Err(INVALID_EPOCH.into());
                                }
                            };

                            config_contracts.insert(
                                contract_name.to_string(),
                                ClarityContract {
                                    name: contract_name.to_string(),
                                    deployer: deployer.clone(),
                                    code_source,
                                    clarity_version,
                                    epoch,
                                },
                            );

                            let mut contract_location = project_root_location.clone();
                            contract_location.append_path(contract_path)?;
                            contracts_settings.insert(
                                contract_location,
                                ClarityContractMetadata {
                                    name: contract_name.to_string(),
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
        config.contracts_settings = contracts_settings;
        config.project.requirements = Some(config_requirements);
        Ok(config)
    }
}

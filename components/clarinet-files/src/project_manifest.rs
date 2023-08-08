use crate::FileAccessor;

use super::FileLocation;
use clarity_repl::clarity::stacks_common::types::StacksEpochId;
use clarity_repl::clarity::ClarityVersion;
use clarity_repl::repl;
use clarity_repl::repl::{ClarityCodeSource, ClarityContract, ContractDeployer};
use serde::ser::SerializeMap;
use serde::{Serialize, Serializer};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::str::FromStr;
use toml::value::Value;

const INVALID_CLARITY_VERSION: &str = "clarity_version field invalid (value supported: 1, 2)";
const INVALID_EPOCH: &str = "epoch field invalid (value supported: 2.0, 2.05, 2.1, 2.2, 2.3, 2.4)";

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

#[derive(Deserialize, Serialize, Debug, Clone)]
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

#[derive(Deserialize, Debug, Clone)]
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
                "pox-3".to_string(),
                "lockup".to_string(),
                "costs-2".to_string(),
                "costs-3".to_string(),
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

                            let (epoch, clarity_version) = get_epoch_and_clarity_version(
                                contract_settings.get("epoch"),
                                contract_settings.get("clarity_version"),
                            )?;

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

fn get_epoch_and_clarity_version(
    settings_epoch: Option<&Value>,
    settings_clarity_version: Option<&Value>,
) -> Result<(StacksEpochId, ClarityVersion), String> {
    // if neither epoch or version are specified in clarinet.toml use: epoch 2.05 and clarity 1
    // if epoch is specified but not version: use the default version for that epoch

    let epoch = match settings_epoch {
        None => StacksEpochId::Epoch2_05,
        Some(Value::String(epoch)) => {
            if epoch.eq("2.0") {
                StacksEpochId::Epoch20
            } else if epoch.eq("2.05") {
                StacksEpochId::Epoch2_05
            } else if epoch.eq("2.1") {
                StacksEpochId::Epoch21
            } else if epoch.eq("2.2") {
                StacksEpochId::Epoch22
            } else if epoch.eq("2.3") {
                StacksEpochId::Epoch23
            } else if epoch.eq("2.4") {
                StacksEpochId::Epoch24
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
            } else if epoch.eq(&2.2) {
                StacksEpochId::Epoch22
            } else if epoch.eq(&2.3) {
                StacksEpochId::Epoch23
            } else if epoch.eq(&2.4) {
                StacksEpochId::Epoch24
            } else {
                return Err(INVALID_EPOCH.into());
            }
        }
        _ => {
            return Err(INVALID_EPOCH.into());
        }
    };

    let clarity_version = match settings_clarity_version {
        None => match settings_epoch {
            None => ClarityVersion::Clarity1,
            Some(_) => ClarityVersion::default_for_epoch(epoch),
        },
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

    if clarity_version > ClarityVersion::default_for_epoch(epoch) {
        return Err(format!("{clarity_version} can not be used with {epoch}"));
    }

    Ok((epoch, clarity_version))
}

#[test]
fn test_get_epoch_and_clarity_version() {
    use ClarityVersion::*;
    use StacksEpochId::*;

    // no epoch, no version
    let result = get_epoch_and_clarity_version(None, None);
    assert_eq!(result, Ok((Epoch2_05, Clarity1)));

    // no version
    // epoch 2.0
    let result = get_epoch_and_clarity_version(Some(&Value::String(String::from("2.0"))), None);
    assert_eq!(result, Ok((Epoch20, Clarity1)));

    // epoch 2.05, no version
    let result = get_epoch_and_clarity_version(Some(&Value::String(String::from("2.05"))), None);
    assert_eq!(result, Ok((Epoch2_05, Clarity1)));

    // epoch 2.1, no version
    let result = get_epoch_and_clarity_version(Some(&Value::String(String::from("2.1"))), None);
    assert_eq!(result, Ok((Epoch21, Clarity2)));

    // no epoch
    // no epoch, version 1
    let result = get_epoch_and_clarity_version(None, Some(&Value::Integer(1)));
    assert_eq!(result, Ok((Epoch2_05, Clarity1)));

    // no epoch, version 2 -> error, must specify epoch
    let result = get_epoch_and_clarity_version(None, Some(&Value::Integer(2)));
    assert_eq!(result, Err("Clarity 2 can not be used with 2.05".into()));

    // epoch and clarity version
    // no epoch 2.05, version 1
    let result = get_epoch_and_clarity_version(
        Some(&Value::String(String::from("2.05"))),
        Some(&Value::Integer(1)),
    );
    assert_eq!(result, Ok((Epoch2_05, Clarity1)));

    // no epoch 2.05, version 2 -> error
    let result = get_epoch_and_clarity_version(
        Some(&Value::String(String::from("2.05"))),
        Some(&Value::Integer(2)),
    );
    assert_eq!(result, Err("Clarity 2 can not be used with 2.05".into()));

    // no epoch 2.05, version 1
    let result = get_epoch_and_clarity_version(
        Some(&Value::String(String::from("2.1"))),
        Some(&Value::Integer(1)),
    );
    assert_eq!(result, Ok((Epoch21, Clarity1)));

    // no epoch 2.05, version 2 -> error
    let result = get_epoch_and_clarity_version(
        Some(&Value::String(String::from("2.1"))),
        Some(&Value::Integer(2)),
    );
    assert_eq!(result, Ok((Epoch21, Clarity2)));
}
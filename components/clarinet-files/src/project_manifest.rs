use crate::FileAccessor;

use super::FileLocation;
use clarity::types::StacksEpochId;
use clarity::vm::ClarityVersion;
use clarity_repl::repl;
use clarity_repl::repl::{ClarityCodeSource, ClarityContract, ContractDeployer};
use serde::ser::SerializeMap;
use serde::{Deserializer, Serialize, Serializer};
use serde_json::Value as JsonValue;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::str::FromStr;
use toml::Value as TomlValue;

pub const INVALID_CLARITY_VERSION: &str =
    "clarity_version field invalid (value supported: 1, 2, 3)";
const INVALID_EPOCH: &str =
    "epoch field invalid (value supported: 2.0, 2.05, 2.1, 2.2, 2.3, 2.4, 3.0)";

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
    contracts: Option<TomlValue>,
    repl: Option<repl::SettingsFile>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProjectConfigFile {
    name: String,
    authors: Option<Vec<String>>,
    description: Option<String>,
    telemetry: Option<bool>,
    requirements: Option<TomlValue>,
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
    #[serde(deserialize_with = "contracts_deserializer")]
    pub contracts: BTreeMap<String, ClarityContract>,
    #[serde(rename = "repl")]
    pub repl_settings: repl::Settings,
    #[serde(skip_serializing)]
    #[serde(default = "default_location")]
    pub location: FileLocation,
    #[serde(skip_serializing, skip_deserializing)]
    pub contracts_settings: HashMap<FileLocation, ClarityContractMetadata>,
}

impl ProjectManifest {
    pub fn default_project_manifest(
        enable_telemetry: bool,
        project_root_location: FileLocation,
    ) -> ProjectManifest {
        ProjectManifest {
            project: ProjectManifest::default_project_config(
                enable_telemetry,
                project_root_location,
            ),
            contracts: BTreeMap::new(),
            repl_settings: repl::Settings::default(),
            location: default_location(),
            contracts_settings: HashMap::new(),
        }
    }

    fn default_project_config(
        enable_telemetry: bool,
        project_root_location: FileLocation,
    ) -> ProjectConfig {
        ProjectConfig {
            name: String::new(),
            authors: Vec::new(),
            description: String::new(),
            telemetry: enable_telemetry,
            requirements: None,
            cache_location: ProjectManifest::new_cache_location(project_root_location),
            boot_contracts: Vec::new(),
        }
    }

    fn new_cache_location(project_root_location: FileLocation) -> FileLocation {
        let mut cache_location = project_root_location.clone();
        let _ = cache_location.append_path(".cache");
        cache_location
    }
}

fn default_location() -> FileLocation {
    let path = std::env::temp_dir();
    FileLocation::from_path(path)
}

fn contracts_deserializer<'de, D>(des: D) -> Result<BTreeMap<String, ClarityContract>, D::Error>
where
    D: Deserializer<'de>,
{
    let mut map: BTreeMap<String, ClarityContract> = BTreeMap::new();

    let container: HashMap<String, HashMap<String, JsonValue>> =
        serde::Deserialize::deserialize(des)?;

    for (contract_name, contract_settings) in container {
        let contract_path = match contract_settings.get("path") {
            Some(JsonValue::String(path)) => path,
            _ => continue,
        };

        let code_source = match PathBuf::from_str(contract_path) {
            Ok(path) => ClarityCodeSource::ContractOnDisk(path),
            Err(e) => {
                return Err(serde::de::Error::custom(format!(
                    "unable to parse path {} ({})",
                    contract_path, e
                )))
            }
        };

        let deployer = match contract_settings.get("deployer") {
            Some(JsonValue::String(path)) => ContractDeployer::LabeledDeployer(path.clone()),
            _ => ContractDeployer::DefaultDeployer,
        };

        let parsed_settings_epoch = match contract_settings.get("epoch") {
            None => None,
            Some(JsonValue::String(epoch)) => Some(epoch.as_str()),
            Some(JsonValue::Number(epoch)) => Some(epoch.as_str()),
            _ => return Err(serde::de::Error::custom(INVALID_EPOCH)),
        };

        let parsed_clarity_version = match contract_settings.get("clarity_version") {
            None => None,
            Some(JsonValue::Number(version)) => Some(version.as_str()),
            _ => return Err(serde::de::Error::custom(INVALID_CLARITY_VERSION)),
        };

        let (epoch, clarity_version) =
            get_epoch_and_clarity_version(parsed_settings_epoch, parsed_clarity_version)
                .map_err(serde::de::Error::custom)?;

        let cc = ClarityContract {
            code_source,
            name: contract_name.clone(),
            deployer,
            clarity_version,
            epoch,
        };

        map.insert(contract_name, cc);
    }
    Ok(map)
}

#[derive(Deserialize, Debug, Clone)]
pub struct ProjectConfig {
    pub name: String,
    pub authors: Vec<String>,
    pub description: String,
    pub telemetry: bool,
    pub requirements: Option<Vec<RequirementConfig>>,
    #[serde(rename = "cache_dir")]
    #[serde(deserialize_with = "cache_location_deserializer")]
    pub cache_location: FileLocation,
    #[serde(skip_deserializing)]
    pub boot_contracts: Vec<String>,
}

fn cache_location_deserializer<'de, D>(des: D) -> Result<FileLocation, D::Error>
where
    D: Deserializer<'de>,
{
    let container: String = serde::Deserialize::deserialize(des)?;
    FileLocation::from_path_string(&container).map_err(serde::de::Error::custom)
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
                .unwrap_or(self.cache_location.to_string()),
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
        file_accessor: &dyn FileAccessor,
    ) -> Result<ProjectManifest, String> {
        let content = file_accessor.read_file(location.to_string()).await?;

        let project_manifest_file: ProjectManifestFile = match toml::from_slice(content.as_bytes())
        {
            Ok(s) => s,
            Err(e) => {
                return Err(format!("Clarinet.toml file malformatted {:?}", e));
            }
        };
        ProjectManifest::from_project_manifest_file(project_manifest_file, location)
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
            authors: project_manifest_file.project.authors.unwrap_or_default(),
            telemetry: project_manifest_file.project.telemetry.unwrap_or(false),
            cache_location,
            boot_contracts: vec![
                "costs".to_string(),
                "pox".to_string(),
                "pox-2".to_string(),
                "pox-3".to_string(),
                "pox-4".to_string(),
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

        if let Some(TomlValue::Array(requirements)) = project_manifest_file.project.requirements {
            for link_settings in requirements.iter() {
                if let TomlValue::Table(link_settings) = link_settings {
                    let contract_id = match link_settings.get("contract_id") {
                        Some(TomlValue::String(contract_id)) => contract_id.to_string(),
                        _ => continue,
                    };
                    config_requirements.push(RequirementConfig { contract_id });
                }
            }
        };
        if let Some(TomlValue::Table(contracts)) = project_manifest_file.contracts {
            for (contract_name, contract_settings) in contracts.iter() {
                if let TomlValue::Table(contract_settings) = contract_settings {
                    let contract_path = match contract_settings.get("path") {
                        Some(TomlValue::String(path)) => path,
                        _ => continue,
                    };
                    let code_source = match PathBuf::from_str(contract_path) {
                        Ok(path) => ClarityCodeSource::ContractOnDisk(path),
                        Err(e) => {
                            return Err(format!("unable to parse path {} ({})", contract_path, e))
                        }
                    };
                    let deployer = match contract_settings.get("deployer") {
                        Some(TomlValue::String(path)) => {
                            ContractDeployer::LabeledDeployer(path.clone())
                        }
                        _ => ContractDeployer::DefaultDeployer,
                    };

                    let parsed_epoch = match contract_settings.get("epoch") {
                        Some(TomlValue::String(epoch)) => Some(epoch.clone()),
                        Some(TomlValue::Float(epoch)) => Some(epoch.to_string()),
                        None => None,
                        _ => return Err(INVALID_EPOCH.into()),
                    };

                    let parsed_clarity_version = match contract_settings.get("clarity_version") {
                        Some(TomlValue::Integer(clarity_version)) => {
                            Some(clarity_version.to_string())
                        }
                        None => None,
                        _ => return Err(INVALID_CLARITY_VERSION.into()),
                    };

                    let (epoch, clarity_version) = get_epoch_and_clarity_version(
                        parsed_epoch.as_deref(),
                        parsed_clarity_version.as_deref(),
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
            }
        };
        config.contracts = config_contracts;
        config.contracts_settings = contracts_settings;
        config.project.requirements = Some(config_requirements);
        Ok(config)
    }
}

fn get_epoch_and_clarity_version(
    settings_epoch: Option<&str>,
    settings_clarity_version: Option<&str>,
) -> Result<(StacksEpochId, ClarityVersion), String> {
    // if neither epoch or version are specified in clarinet.toml use: epoch 2.05 and clarity 1
    // if epoch is specified but not version: use the default version for that epoch

    let epoch = match settings_epoch {
        None => StacksEpochId::Epoch2_05,
        Some(epoch) => match epoch {
            "2" | "2.0" => StacksEpochId::Epoch20,
            "2.05" => StacksEpochId::Epoch2_05,
            "2.1" => StacksEpochId::Epoch21,
            "2.2" => StacksEpochId::Epoch22,
            "2.3" => StacksEpochId::Epoch23,
            "2.4" => StacksEpochId::Epoch24,
            "2.5" => StacksEpochId::Epoch25,
            "3" | "3.0" => StacksEpochId::Epoch30,
            _ => return Err(INVALID_EPOCH.into()),
        },
    };

    let clarity_version = match settings_clarity_version {
        None => match settings_epoch {
            None => ClarityVersion::Clarity1,
            Some(_) => ClarityVersion::default_for_epoch(epoch),
        },
        Some(version) => ClarityVersion::from_str(&format!("clarity{version}"))
            .map_err(|_| INVALID_CLARITY_VERSION)?,
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

    // epoch 2.0, no version
    let result = get_epoch_and_clarity_version(Some("2.0"), None);
    assert_eq!(result, Ok((Epoch20, Clarity1)));

    // epoch 2.05, no version
    let result = get_epoch_and_clarity_version(Some("2.05"), None);
    assert_eq!(result, Ok((Epoch2_05, Clarity1)));

    // epoch 2.1, no version
    let result = get_epoch_and_clarity_version(Some("2.1"), None);
    assert_eq!(result, Ok((Epoch21, Clarity2)));

    // epoch 3.0, no version
    let result = get_epoch_and_clarity_version(Some("3.0"), None);
    assert_eq!(result, Ok((Epoch30, Clarity3)));

    // no epoch
    // no epoch, version 1
    let result = get_epoch_and_clarity_version(None, Some("1"));
    assert_eq!(result, Ok((Epoch2_05, Clarity1)));

    // no epoch, version 2 -> error, must specify epoch
    let result = get_epoch_and_clarity_version(None, Some("2"));
    assert_eq!(result, Err("Clarity 2 can not be used with 2.05".into()));

    // epoch and clarity version
    // no epoch 2.05, version 1
    let result = get_epoch_and_clarity_version(Some("2.05"), Some("1"));
    assert_eq!(result, Ok((Epoch2_05, Clarity1)));

    // no epoch 2.05, version 2 -> error
    let result = get_epoch_and_clarity_version(Some("2.05"), Some("2"));
    assert_eq!(result, Err("Clarity 2 can not be used with 2.05".into()));

    // no epoch 2.05, version 1
    let result = get_epoch_and_clarity_version(Some("2.1"), Some("1"));
    assert_eq!(result, Ok((Epoch21, Clarity1)));

    // no epoch 2.05, version 2 -> error
    let result = get_epoch_and_clarity_version(Some("2.1"), Some("2"));
    assert_eq!(result, Ok((Epoch21, Clarity2)));
}

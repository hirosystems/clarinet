use clarity_repl::repl;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufReader, Read};
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

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ProjectManifest {
    pub project: ProjectConfig,
    #[serde(serialize_with = "toml::ser::tables_last")]
    pub contracts: BTreeMap<String, ContractConfig>,
    #[serde(rename = "repl")]
    pub repl_settings: repl::Settings,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ProjectConfig {
    pub name: String,
    pub authors: Vec<String>,
    pub description: String,
    pub telemetry: bool,
    pub requirements: Option<Vec<RequirementConfig>>,
    pub cache_dir: String,
    #[serde(skip_serializing, skip_deserializing)]
    pub cache_dir_relative: bool,
    pub boot_contracts: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct RequirementConfig {
    pub contract_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ContractConfig {
    pub path: String,
    pub depends_on: Vec<String>,
    pub deployer: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NotebookConfig {
    pub name: String,
    pub path: String,
}

impl ProjectManifest {
    pub fn from_path(path: &PathBuf) -> ProjectManifest {
        let file = match File::open(path) {
            Ok(path) => path,
            Err(_e) => {
                println!("Error: unable to locate Clarinet.toml in current directory");
                std::process::exit(1);
            }
        };
        let mut project_manifest_file_reader = BufReader::new(file);
        let mut project_manifest_file_buffer = vec![];
        project_manifest_file_reader
            .read_to_end(&mut project_manifest_file_buffer)
            .unwrap();

        let project_manifest_file: ProjectManifestFile =
            match toml::from_slice(&project_manifest_file_buffer[..]) {
                Ok(s) => s,
                Err(e) => {
                    println!(
                        "{}: Clarinet.toml file malformatted.\n{:?}",
                        red!("error"),
                        e
                    );
                    std::process::exit(1);
                }
            };

        ProjectManifest::from_project_manifest_file(project_manifest_file, path)
    }

    pub fn from_project_manifest_file(
        project_manifest_file: ProjectManifestFile,
        manifest_path: &PathBuf,
    ) -> ProjectManifest {
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
        let (cache_path, cache_dir_relative) = match project_manifest_file.project.cache_dir {
            Some(ref path) => {
                let path = match PathBuf::from_str(path) {
                    Ok(path) => path,
                    Err(_e) => {
                        println!(
                            "{}: Clarinet.toml file malformatted, the entry cache_dir is not a valid path",
                            red!("error")
                        );
                        std::process::exit(1);
                    }
                };
                if path.is_relative() {
                    let mut absolute_path = manifest_path.clone();
                    absolute_path.pop();
                    absolute_path.extend(&path);
                    (absolute_path, true)
                } else {
                    (path, false)
                }
            }
            None => {
                let mut default_cache_path =
                    dirs::home_dir().expect("Unable to retrieve home directory");
                default_cache_path.push(".clarinet");
                default_cache_path.push("cache");
                (default_cache_path, false)
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
            cache_dir: cache_path.to_str().unwrap().to_string(),
            cache_dir_relative,
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
                            let depends_on = match contract_settings.get("depends_on") {
                                Some(Value::Array(depends_on)) => depends_on
                                    .iter()
                                    .map(|v| v.as_str().unwrap().to_string())
                                    .collect::<Vec<String>>(),
                                _ => continue,
                            };
                            let deployer = match contract_settings.get("deployer") {
                                Some(Value::String(path)) => Some(path.to_string()),
                                _ => None,
                            };
                            config_contracts.insert(
                                contract_name.to_string(),
                                ContractConfig {
                                    path,
                                    depends_on,
                                    deployer,
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
        config
    }
}

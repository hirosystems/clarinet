use toml::value::Value;
use std::io::{BufReader, Read};
use std::fs::File;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug)]
pub struct PaperConfigFile {
    project: ProjectConfig,
    contracts: Value,
    drafts: Value,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProjectConfigFile {
    name: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PaperConfig {
    pub project: ProjectConfig,
    pub contracts: Vec<ContractConfig>,
    pub drafts: Vec<DraftConfig>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProjectConfig {
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ContractConfig {
    pub name: String,
    pub path: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DraftConfig {
    pub name: String,
    pub path: String,
}

impl PaperConfig {

    pub fn from_path(path: &PathBuf) -> PaperConfig {
        let path = File::open(path).unwrap();
        let mut config_file_reader = BufReader::new(path);
        let mut config_file_buffer = vec![];
        config_file_reader.read_to_end(&mut config_file_buffer).unwrap();    
        let config_file: PaperConfigFile = toml::from_slice(&config_file_buffer[..]).unwrap();
        PaperConfig::from_config_file(config_file)
    }

    pub fn from_config_file(config_file: PaperConfigFile) -> PaperConfig {

        let mut config = PaperConfig {
            project: config_file.project,
            contracts: vec![],
            drafts: vec![],
        };

        match config_file.contracts {
            Value::Table(contracts) => {
                for (contract_name, contract_settings) in contracts.iter() {
                    match contract_settings {
                        Value::Table(contract_settings) => {
                            let contract_path = match contract_settings.get("path") {
                                Some(Value::String(path)) => path.to_string(),
                                _ => continue,
                            };
                            config.contracts.push(
                                ContractConfig {
                                    name: contract_name.to_string(),
                                    path: contract_path,
                                }
                            );
                        }
                        _ => {},
                    }
                }
            },
            _ => {},
        };

        match config_file.drafts {
            Value::Table(drafts) => {
                for (draft_name, draft_settings) in drafts.iter() {
                    match draft_settings {
                        Value::Table(draft_settings) => {
                            let draft_path = match draft_settings.get("path") {
                                Some(Value::String(path)) => path.to_string(),
                                _ => continue,
                            };
                            config.drafts.push(
                                DraftConfig {
                                    name: draft_name.to_string(),
                                    path: draft_path,
                                }
                            );
                        }
                        _ => {},
                    }
                }
            },
            _ => {},
        };

        config
    }

}
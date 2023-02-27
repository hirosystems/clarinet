use clarinet_files::{FileAccessor, FileLocation};
use clarity_repl::clarity::stacks_common::types::StacksEpochId;
use clarity_repl::clarity::{vm::types::QualifiedContractIdentifier, ClarityVersion};
use reqwest;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractMetadata {
    pub epoch: StacksEpochId,
    pub clarity_version: ClarityVersion,
}

impl Default for ContractMetadata {
    fn default() -> Self {
        ContractMetadata {
            epoch: StacksEpochId::latest(),
            clarity_version: ClarityVersion::latest(),
        }
    }
}

pub async fn retrieve_contract(
    contract_id: &QualifiedContractIdentifier,
    cache_location: &FileLocation,
    file_accessor: &Option<&Box<dyn FileAccessor>>,
) -> Result<(String, StacksEpochId, ClarityVersion, FileLocation), String> {
    let contract_deployer = contract_id.issuer.to_address();
    let contract_name = contract_id.name.to_string();

    let mut contract_location = cache_location.clone();
    contract_location.append_path("requirements")?;
    let mut metadata_location = contract_location.clone();
    contract_location.append_path(&format!("{}.{}.clar", contract_deployer, contract_name))?;
    metadata_location.append_path(&format!("{}.{}.json", contract_deployer, contract_name))?;

    let contract_source = match file_accessor {
        None => contract_location.read_content_as_utf8(),
        Some(file_accessor) => file_accessor.read_file(contract_location.to_string()).await,
    };

    if contract_source.is_ok() {
        let metadata_json = match file_accessor {
            None => metadata_location.read_content_as_utf8(),
            Some(file_accessor) => file_accessor.read_file(metadata_location.to_string()).await,
        }
        .map_err(|e| format!("Unable to read metadata file: {}", e))?;
        let metadata: ContractMetadata = serde_json::from_str(&metadata_json).unwrap_or_default();

        return Ok((
            contract_source.unwrap(),
            metadata.epoch,
            metadata.clarity_version,
            contract_location,
        ));
    }

    let is_mainnet = contract_deployer.starts_with("SP");
    let stacks_node_addr = if is_mainnet {
        "https://stacks-node-api.mainnet.stacks.co".to_string()
    } else {
        "https://stacks-node-api.testnet.stacks.co".to_string()
    };

    let request_url = format!(
        "{host}/v2/contracts/source/{addr}/{name}?proof=0",
        host = stacks_node_addr,
        addr = contract_deployer,
        name = contract_name
    );

    let contract = fetch_contract(request_url).await?;

    let clarity_version = {
        // `version` defaults to 1 because before 2.1, no version is specified
        // since Clarity 1 was the only version available.
        let version = contract.clarity_version.unwrap_or(1);
        if version.eq(&1) {
            ClarityVersion::Clarity1
        } else if version.eq(&2) {
            ClarityVersion::Clarity2
        } else {
            return Err(format!(
                "unable to parse clarity_version (can either be '1' or '2'",
            ));
        }
    };
    let epoch = epoch_for_height(is_mainnet, contract.publish_height);

    match file_accessor {
        None => {
            contract_location.write_content(contract.source.as_bytes())?;
            metadata_location.write_content(
                serde_json::to_string_pretty(&ContractMetadata {
                    epoch,
                    clarity_version,
                })
                .unwrap()
                .as_bytes(),
            )?;
        }
        Some(file_accessor) => {
            file_accessor
                .write_file(contract_location.to_string(), contract.source.as_bytes())
                .await?;
            file_accessor
                .write_file(
                    metadata_location.to_string(),
                    serde_json::to_string_pretty(&ContractMetadata {
                        epoch,
                        clarity_version,
                    })
                    .unwrap()
                    .as_bytes(),
                )
                .await?;
        }
    };

    Ok((contract.source, epoch, clarity_version, contract_location))
}

pub const MAINNET_20_START_HEIGHT: u32 = 1;
pub const MAINNET_2_05_START_HEIGHT: u32 = 40_607;
// TODO: This is estimated. Replace with exact height once 2.1 is activated.
pub const MAINNET_21_START_HEIGHT: u32 = 99_564;
pub const TESTNET_20_START_HEIGHT: u32 = 1;

pub const TESTNET_2_05_START_HEIGHT: u32 = 20_216;
// TODO: This is estimated. Replace with exact height once 2.1 is activated.
pub const TESTNET_21_START_HEIGHT: u32 = 99_253;

fn epoch_for_height(is_mainnet: bool, height: u32) -> StacksEpochId {
    if is_mainnet {
        epoch_for_mainnet_height(height)
    } else {
        epoch_for_testnet_height(height)
    }
}

fn epoch_for_mainnet_height(height: u32) -> StacksEpochId {
    if height < MAINNET_2_05_START_HEIGHT {
        StacksEpochId::Epoch20
    } else if height < MAINNET_21_START_HEIGHT {
        StacksEpochId::Epoch2_05
    } else {
        StacksEpochId::Epoch21
    }
}

fn epoch_for_testnet_height(height: u32) -> StacksEpochId {
    if height < TESTNET_2_05_START_HEIGHT {
        StacksEpochId::Epoch20
    } else if height < TESTNET_21_START_HEIGHT {
        StacksEpochId::Epoch2_05
    } else {
        StacksEpochId::Epoch21
    }
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, Default, Clone)]
struct Contract {
    source: String,
    publish_height: u32,
    clarity_version: Option<u8>,
}

async fn fetch_contract(request_url: String) -> Result<Contract, String> {
    let response = reqwest::get(&request_url)
        .await
        .map_err(|_| format!("Unable to retrieve contract {}", request_url))?;

    let contract = response
        .json()
        .await
        .map_err(|_| format!("Unable to parse contract {}", request_url))?;

    Ok(contract)
}

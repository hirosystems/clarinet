use clarinet_files::{FileAccessor, FileLocation};
use clarity_repl::{
    clarity::{vm::types::QualifiedContractIdentifier, ClarityVersion, StacksEpochId},
    repl::{DEFAULT_CLARITY_VERSION, DEFAULT_EPOCH},
};
use reqwest;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractMetadata {
    pub epoch: StacksEpochId,
    pub clarity_version: ClarityVersion,
}

impl Default for ContractMetadata {
    fn default() -> Self {
        ContractMetadata {
            epoch: DEFAULT_EPOCH,
            clarity_version: DEFAULT_CLARITY_VERSION,
        }
    }
}

pub async fn retrieve_contract(
    contract_id: &QualifiedContractIdentifier,
    cache_location: &FileLocation,
    file_accessor: &Option<&dyn FileAccessor>,
) -> Result<(String, StacksEpochId, ClarityVersion, FileLocation), String> {
    let contract_deployer = contract_id.issuer.to_address();
    let contract_name = contract_id.name.to_string();

    let mut contract_location = cache_location.clone();
    contract_location.append_path("requirements")?;
    let mut metadata_location = contract_location.clone();
    contract_location.append_path(&format!("{}.{}.clar", contract_deployer, contract_name))?;
    metadata_location.append_path(&format!("{}.{}.json", contract_deployer, contract_name))?;

    let (contract_source, metadata_json) = match file_accessor {
        None => (
            contract_location.read_content_as_utf8(),
            metadata_location.read_content_as_utf8(),
        ),
        Some(file_accessor) => (
            file_accessor.read_file(contract_location.to_string()).await,
            file_accessor.read_file(metadata_location.to_string()).await,
        ),
    };

    if let (Ok(contract_source), Ok(metadata_json)) = (contract_source, metadata_json) {
        let metadata: ContractMetadata = serde_json::from_str(&metadata_json)
            .map_err(|e| format!("Unable to parse metadata file: {}", e))?;

        return Ok((
            contract_source,
            metadata.epoch,
            metadata.clarity_version,
            contract_location,
        ));
    }

    let is_mainnet = contract_deployer.starts_with("SP") || contract_deployer.starts_with("SM");
    let stacks_node_addr = if is_mainnet {
        "https://api.hiro.so".to_string()
    } else {
        "https://api.testnet.hiro.so".to_string()
    };

    let request_url = format!(
        "{host}/v2/contracts/source/{addr}/{name}?proof=0",
        host = stacks_node_addr,
        addr = contract_deployer,
        name = contract_name
    );

    let contract = fetch_contract(request_url).await?;
    let epoch = epoch_for_height(is_mainnet, contract.publish_height);
    let clarity_version = match contract.clarity_version {
        Some(1) => ClarityVersion::Clarity1,
        Some(2) => ClarityVersion::Clarity2,
        Some(_) => {
            return Err("unable to parse clarity_version (can either be '1' or '2'".to_string())
        }
        None => ClarityVersion::default_for_epoch(epoch),
    };

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
pub const MAINNET_21_START_HEIGHT: u32 = 99_113;
pub const MAINNET_22_START_HEIGHT: u32 = 103_900;
pub const MAINNET_23_START_HEIGHT: u32 = 104_359;
pub const MAINNET_24_START_HEIGHT: u32 = 107_055;
// @TODO: set right heights once epochs are live on mainnet
pub const MAINNET_25_START_HEIGHT: u32 = 200_000;
pub const MAINNET_30_START_HEIGHT: u32 = 300_000;

pub const TESTNET_20_START_HEIGHT: u32 = 1;
pub const TESTNET_2_05_START_HEIGHT: u32 = 20_216;
pub const TESTNET_21_START_HEIGHT: u32 = 99_113;
pub const TESTNET_22_START_HEIGHT: u32 = 105_923;
pub const TESTNET_23_START_HEIGHT: u32 = 106_196;
pub const TESTNET_24_START_HEIGHT: u32 = 106_979;
// @TODO: set right heights once epochs are live on testnet
pub const TESTNET_25_START_HEIGHT: u32 = 200_000;
pub const TESTNET_30_START_HEIGHT: u32 = 300_000;

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
    } else if height < MAINNET_22_START_HEIGHT {
        StacksEpochId::Epoch21
    } else if height < MAINNET_23_START_HEIGHT {
        StacksEpochId::Epoch22
    } else if height < MAINNET_24_START_HEIGHT {
        StacksEpochId::Epoch23
    } else if height < MAINNET_25_START_HEIGHT {
        StacksEpochId::Epoch24
    } else if height < MAINNET_30_START_HEIGHT {
        StacksEpochId::Epoch25
    } else {
        StacksEpochId::Epoch30
    }
}

fn epoch_for_testnet_height(height: u32) -> StacksEpochId {
    if height < TESTNET_2_05_START_HEIGHT {
        StacksEpochId::Epoch20
    } else if height < TESTNET_21_START_HEIGHT {
        StacksEpochId::Epoch2_05
    } else if height < TESTNET_22_START_HEIGHT {
        StacksEpochId::Epoch21
    } else if height < TESTNET_23_START_HEIGHT {
        StacksEpochId::Epoch22
    } else if height < TESTNET_24_START_HEIGHT {
        StacksEpochId::Epoch23
    } else if height < TESTNET_25_START_HEIGHT {
        StacksEpochId::Epoch24
    } else if height < TESTNET_30_START_HEIGHT {
        StacksEpochId::Epoch25
    } else {
        StacksEpochId::Epoch30
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

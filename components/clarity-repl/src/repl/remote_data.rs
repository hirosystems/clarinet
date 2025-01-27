use clarity::{
    types::{
        chainstate::{
            BlockHeaderHash, BurnchainHeaderHash, ConsensusHash, SortitionId, StacksBlockId,
        },
        StacksEpochId,
    },
    vm::errors::InterpreterResult,
};
use serde::de::DeserializeOwned;

#[cfg(target_arch = "wasm32")]
use js_sys::{JsString, Uint8Array};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(module = "/js/index.js")]
extern "C" {
    #[wasm_bindgen(js_name = httpClient)]
    fn http_client(method: &JsString, path: &JsString) -> Uint8Array;
}

use crate::repl::settings::ApiUrl;

pub const MAINNET_20_START_HEIGHT: u32 = 1;
pub const MAINNET_2_05_START_HEIGHT: u32 = 40_607;
pub const MAINNET_21_START_HEIGHT: u32 = 99_113;
pub const MAINNET_22_START_HEIGHT: u32 = 103_900;
pub const MAINNET_23_START_HEIGHT: u32 = 104_359;
pub const MAINNET_24_START_HEIGHT: u32 = 107_055;
pub const MAINNET_25_START_HEIGHT: u32 = 147_290;
pub const MAINNET_30_START_HEIGHT: u32 = 171_833;
pub const MAINNET_31_START_HEIGHT: u32 = 340_555;

// the current primary testnet starts directly in epoch 2.5 (pox-4 deployment)
pub const TESTNET_20_START_HEIGHT: u32 = 1;
pub const TESTNET_2_05_START_HEIGHT: u32 = 1;
pub const TESTNET_21_START_HEIGHT: u32 = 1;
pub const TESTNET_22_START_HEIGHT: u32 = 1;
pub const TESTNET_23_START_HEIGHT: u32 = 1;
pub const TESTNET_24_START_HEIGHT: u32 = 1;
pub const TESTNET_25_START_HEIGHT: u32 = 1;
pub const TESTNET_30_START_HEIGHT: u32 = 320;
pub const TESTNET_31_START_HEIGHT: u32 = 814;

pub fn epoch_for_height(is_mainnet: bool, height: u32) -> StacksEpochId {
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
    } else if height < MAINNET_31_START_HEIGHT {
        StacksEpochId::Epoch30
    } else {
        StacksEpochId::Epoch31
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
    } else if height < TESTNET_31_START_HEIGHT {
        StacksEpochId::Epoch30
    } else {
        StacksEpochId::Epoch31
    }
}

#[derive(Deserialize)]
pub struct ClarityDataResponse {
    pub data: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Info {
    pub network_id: u32,
    pub stacks_tip_height: u32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RawSortition {
    pub burn_block_hash: String,
    pub burn_block_height: u32,
    pub consensus_hash: String,
    pub sortition_id: String,
    pub parent_sortition_id: String,
}

#[derive(Clone, Debug)]
pub struct Sortition {
    pub burn_block_hash: BurnchainHeaderHash,
    pub burn_block_height: u32,
    pub consensus_hash: ConsensusHash,
    pub sortition_id: SortitionId,
    pub parent_sortition_id: SortitionId,
}

impl Sortition {
    pub fn from(response: RawSortition) -> Self {
        let burn_block_hash =
            BurnchainHeaderHash::from_hex(&response.burn_block_hash.replacen("0x", "", 1)).unwrap();
        let consensus_hash =
            ConsensusHash::from_hex(&response.consensus_hash.replacen("0x", "", 1)).unwrap();
        let sortition_id =
            SortitionId::from_hex(&response.sortition_id.replacen("0x", "", 1)).unwrap();
        let parent_sortition_id =
            SortitionId::from_hex(&response.parent_sortition_id.replacen("0x", "", 1)).unwrap();

        Sortition {
            burn_block_hash,
            burn_block_height: response.burn_block_height,
            consensus_hash,
            sortition_id,
            parent_sortition_id,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct RawBlock {
    pub height: u32,
    pub burn_block_height: u32,
    pub tenure_height: u32,
    pub block_time: u64,
    pub burn_block_time: u64,
    pub hash: String,
    pub index_block_hash: String,
    pub burn_block_hash: String,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct Block {
    pub height: u32,
    pub burn_block_height: u32,
    pub tenure_height: u32,
    pub block_time: u64,
    pub burn_block_time: u64,
    pub hash: BlockHeaderHash,
    pub index_block_hash: StacksBlockId,
    pub burn_block_hash: BurnchainHeaderHash,
}

impl From<RawBlock> for Block {
    fn from(response: RawBlock) -> Self {
        let hash = BlockHeaderHash::from_hex(&response.hash.replacen("0x", "", 1)).unwrap();
        let index_block_hash =
            StacksBlockId::from_hex(&response.index_block_hash.replacen("0x", "", 1)).unwrap();
        let burn_block_hash =
            BurnchainHeaderHash::from_hex(&response.burn_block_hash.replacen("0x", "", 1)).unwrap();

        Block {
            height: response.height,
            burn_block_height: response.burn_block_height,
            tenure_height: response.tenure_height,
            block_time: response.block_time,
            burn_block_time: response.burn_block_time,
            hash,
            index_block_hash,
            burn_block_hash,
        }
    }
}

#[derive(Clone, Debug)]
pub struct HttpClient {
    url: ApiUrl,
    #[allow(dead_code)]
    api_key: Option<String>,
}

impl HttpClient {
    pub fn new(url: ApiUrl) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        let api_key = std::env::var("HIRO_API_KEY").ok();
        #[cfg(target_arch = "wasm32")]
        let api_key = None;

        HttpClient { url, api_key }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn get<T: DeserializeOwned>(&self, path: &str) -> Option<T> {
        let url = format!("{}{}", self.url, path);
        let client = reqwest::blocking::Client::new();
        let mut request = client.get(&url).header("x-hiro-product", "clarinet-cli");
        if let Some(ref api_key) = self.api_key {
            request = request.header("x-api-key", api_key);
        }
        match request.send() {
            Ok(response) => response.json::<T>().ok(),
            Err(_) => None,
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn get<T: DeserializeOwned>(&self, path: &str) -> Option<T> {
        let url = JsString::from(format!("{}{}", self.url, path));
        let response = http_client(&JsString::from("GET"), &url);
        let bytes = response.to_vec();
        let raw_result = std::str::from_utf8(bytes.as_slice()).unwrap();
        serde_json::from_str::<T>(raw_result).ok()
    }

    fn fetch_data<T: DeserializeOwned>(&self, path: &str) -> InterpreterResult<Option<T>> {
        Ok(self.get::<T>(path))
    }

    pub fn fetch_info(&self) -> Info {
        self.fetch_data::<Info>("/v2/info")
            .unwrap_or_else(|e| {
                panic!("unable to parse json, error: {}", e);
            })
            .unwrap_or_else(|| {
                panic!("unable to get remote info");
            })
    }

    pub fn fetch_sortition(&self, burn_block_hash: &BurnchainHeaderHash) -> Sortition {
        let url = dbg!(format!("/v3/sortitions/burn/{}", burn_block_hash));
        let sortition = self
            .fetch_data::<Vec<RawSortition>>(&url)
            .unwrap_or_else(|e| {
                panic!("unable to parse json, error: {}", e);
            })
            .unwrap_or_else(|| {
                panic!("unable to get remote sortition info");
            });

        println!("sortition {:?}", sortition);
        Sortition::from(sortition.first().unwrap().clone())
    }

    pub fn fetch_block(&self, url: &str) -> Block {
        let block = self
            .fetch_data::<RawBlock>(url)
            .unwrap_or_else(|e| {
                panic!("unable to parse json, error: {}", e);
            })
            .unwrap_or_else(|| {
                panic!("unable to get remote block info");
            });

        Block::from(block.clone())
    }

    pub fn fetch_clarity_data(&self, path: &str) -> InterpreterResult<Option<String>> {
        let data = self
            .fetch_data::<ClarityDataResponse>(path)
            .unwrap_or_else(|e| {
                panic!("unable to parse json, error: {}", e);
            });

        match data {
            Some(data) => Ok(Some(data.data.replacen("0x", "", 1))),
            None => Ok(None),
        }
    }
}

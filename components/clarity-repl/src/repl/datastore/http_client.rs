use clarity::{
    types::chainstate::{BlockHeaderHash, BurnchainHeaderHash, StacksBlockId},
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

#[derive(Deserialize)]
pub struct ClarityDataResponse {
    pub data: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Block {
    pub height: u32,
    pub burn_block_height: u32,
    pub tenure_height: u32,
    pub block_time: u64,
    pub burn_block_time: u64,
    pub hash: String,
    pub index_block_hash: String,
    pub burn_block_hash: String,
}

#[derive(Clone, Debug, Deserialize)]
#[allow(dead_code)]
pub struct ParsedBlock {
    pub height: u32,
    pub burn_block_height: u32,
    pub tenure_height: u32,
    pub block_time: u64,
    pub burn_block_time: u64,
    pub hash: BlockHeaderHash,
    pub index_block_hash: StacksBlockId,
    pub burn_block_hash: BurnchainHeaderHash,
}

impl From<Block> for ParsedBlock {
    fn from(response: Block) -> Self {
        let hash = BlockHeaderHash::from_hex(&response.hash.replacen("0x", "", 1)).unwrap();
        let index_block_hash =
            StacksBlockId::from_hex(&response.index_block_hash.replacen("0x", "", 1)).unwrap();
        let burn_block_hash =
            BurnchainHeaderHash::from_hex(&response.burn_block_hash.replacen("0x", "", 1)).unwrap();

        ParsedBlock {
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
}

impl HttpClient {
    pub fn new(url: ApiUrl) -> Self {
        HttpClient { url }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn get<T: DeserializeOwned>(&self, path: &str) -> Option<T> {
        let url = format!("{}{}", self.url, path);
        println!("fetching data from: {}", url);
        match reqwest::blocking::get(&url) {
            Ok(response) => response.json::<T>().ok(),
            Err(_) => None,
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn get<T: DeserializeOwned>(&self, path: &str) -> Option<T> {
        uprint!("fetching data from: {}", path);
        let url = JsString::from(format!("{}{}", self.url, path));
        let response = http_client(&JsString::from("GET"), &url);
        let bytes = response.to_vec();
        let raw_result = std::str::from_utf8(bytes.as_slice()).unwrap();
        serde_json::from_str::<T>(raw_result).ok()
    }

    fn fetch_data<T: DeserializeOwned>(&self, path: &str) -> InterpreterResult<Option<T>> {
        uprint!("fetching data from: {}", path);
        Ok(self.get::<T>(path))
    }

    pub fn fetch_block(&self, url: &str) -> ParsedBlock {
        let block = self
            .fetch_data::<Block>(url)
            .unwrap_or_else(|e| {
                panic!("unable to parse json, error: {}", e);
            })
            .unwrap_or_else(|| {
                panic!("unable to get remote block info");
            });

        ParsedBlock::from(block.clone())
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

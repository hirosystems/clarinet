use clarity::{
    types::chainstate::{BlockHeaderHash, BurnchainHeaderHash, StacksBlockId},
    vm::errors::InterpreterResult,
};
use serde::de::DeserializeOwned;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;
#[cfg(target_arch = "wasm32")]
use web_sys::js_sys::{Function as JsFunction, JsString, Uint8Array};

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

pub trait HttpClientTrait {
    fn fetch_data<T: DeserializeOwned>(&self, path: &str) -> InterpreterResult<Option<T>>;
    fn fetch_block(&self, url: &str) -> ParsedBlock;
    fn fetch_clarity_data(&self, path: &str) -> InterpreterResult<Option<String>>;
}

#[derive(Clone, Debug)]
pub struct HttpClient {
    #[cfg(not(target_arch = "wasm32"))]
    client: reqwest::blocking::Client,
    #[cfg(target_arch = "wasm32")]
    client: JsFunction,
    url: ApiUrl,
}

#[cfg(not(target_arch = "wasm32"))]
impl HttpClient {
    pub fn new(url: ApiUrl) -> Self {
        HttpClient {
            client: reqwest::blocking::Client::new(),
            url,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn get<T: DeserializeOwned>(&self, path: &str) -> Option<T> {
        let url = format!("{}{}", self.url, path);
        println!("fetching data from: {}", url);
        match self.client.get(&url).send() {
            Ok(response) => match response.json::<T>() {
                Ok(data) => Some(data),
                Err(_) => None,
            },
            Err(_) => None,
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl HttpClient {
    pub fn new(url: ApiUrl, client: JsFunction) -> Self {
        HttpClient {
            #[cfg(not(target_arch = "wasm32"))]
            client: reqwest::blocking::Client::new(),
            #[cfg(target_arch = "wasm32")]
            client,
            url,
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn get<T: DeserializeOwned>(&self, url: &str) -> Option<T> {
        let url = JsString::from(url);

        match self
            .client
            .call2(&JsValue::NULL, &JsString::from("GET"), &url)
        {
            Ok(response) => {
                let bytes = Uint8Array::from(response).to_vec();
                let raw_result = std::str::from_utf8(bytes.as_slice()).unwrap();
                match serde_json::from_str::<T>(raw_result) {
                    Ok(data) => Some(data),
                    _ => None,
                }
            }
            Err(_) => None,
        }
    }
}

impl HttpClientTrait for HttpClient {
    fn fetch_data<T: DeserializeOwned>(&self, path: &str) -> InterpreterResult<Option<T>> {
        Ok(self.get::<T>(path))
    }

    fn fetch_block(&self, url: &str) -> ParsedBlock {
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

    fn fetch_clarity_data(&self, path: &str) -> InterpreterResult<Option<String>> {
        let data = self
            .fetch_data::<ClarityDataResponse>(path)
            .unwrap_or_else(|e| {
                panic!("unable to parse json, error: {}", e);
            });

        match data {
            Some(data) => {
                let value = if data.data.starts_with("0x") {
                    data.data[2..].to_string()
                } else {
                    data.data
                };
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }
}

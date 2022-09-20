use super::{FileAccessor, FileAccessorResult, FileLocation};
use js_sys::{Function as JsFunction, Promise};
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::{from_value as decode_from_js, to_value as encode_to_js};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into());
    }
}

#[derive(Serialize, Deserialize)]
struct WFSRequest {
    pub path: String,
}

#[derive(Serialize, Deserialize)]
struct WFSWriteRequest<'a> {
    pub path: String,
    pub content: &'a [u8],
}

pub struct WASMFileSystemAccessor {
    client_request_tx: JsFunction,
}

impl WASMFileSystemAccessor {
    pub fn new(client_request_tx: JsFunction) -> WASMFileSystemAccessor {
        WASMFileSystemAccessor { client_request_tx }
    }

    fn get_request_promise<T: Serialize>(
        &self,
        action: String,
        data: &T,
    ) -> Result<JsValue, JsValue> {
        self.client_request_tx.call2(
            &JsValue::null(),
            &JsValue::from(action),
            &encode_to_js(data)?,
        )
    }
}

impl FileAccessor for WASMFileSystemAccessor {
    fn file_exists(&self, location: FileLocation) -> FileAccessorResult<bool> {
        log!("checking if file exists");
        let file_exists_promise = self.get_request_promise(
            "vfs/exists".into(),
            &WFSRequest {
                path: location.to_string(),
            },
        );

        Box::pin(async move {
            match file_exists_promise {
                Ok(promise) => match JsFuture::from(Promise::resolve(&promise)).await {
                    Ok(res) => Ok(res.is_truthy()),
                    Err(err) => Err(format!("error: {:?}", &err)),
                },
                Err(err) => Err(format!("error: {:?}", &err)),
            }
        })
    }

    fn read_manifest_content(&self, manifest_location: FileLocation) -> FileAccessorResult<String> {
        log!("reading manifest");
        let read_file_promise = self.get_request_promise(
            "vfs/readFile".into(),
            &WFSRequest {
                path: manifest_location.to_string(),
            },
        );

        Box::pin(async move {
            match read_file_promise {
                Ok(req) => match JsFuture::from(Promise::resolve(&req)).await {
                    Ok(manifest) => Ok(decode_from_js(manifest)
                        .map_err(|err| format!("decode_from_js error: {}", err))?),
                    Err(err) => Err(format!("error: {:?}", &err)),
                },
                Err(err) => Err(format!("error: {:?}", &err)),
            }
        })
    }

    fn read_contract_content(&self, contract_location: FileLocation) -> FileAccessorResult<String> {
        log!("reading contract");
        let req = self.get_request_promise(
            "vfs/readFile".into(),
            &WFSRequest {
                path: contract_location.to_string(),
            },
        );

        Box::pin(async move {
            match req {
                Ok(req) => match JsFuture::from(Promise::resolve(&req)).await {
                    Ok(contract) => Ok(decode_from_js(contract)
                        .map_err(|err| format!("decode_from_js error: {}", err))?),
                    Err(err) => Err(format!("error: {:?}", &err)),
                },
                Err(err) => Err(format!("error: {:?}", &err)),
            }
        })
    }

    fn write_file(&self, location: FileLocation, content: &[u8]) -> FileAccessorResult<()> {
        log!("writting contract");
        let write_file_promise = self.get_request_promise(
            "vfs/writeFile".into(),
            &WFSWriteRequest {
                path: location.to_string(),
                content,
            },
        );

        Box::pin(async move {
            match write_file_promise {
                Ok(promise) => match JsFuture::from(Promise::resolve(&promise)).await {
                    Ok(_) => Ok(()),
                    Err(err) => Err(format!("error: {:?}", &err)),
                },
                Err(err) => Err(format!("error: {:?}", &err)),
            }
        })
    }
}

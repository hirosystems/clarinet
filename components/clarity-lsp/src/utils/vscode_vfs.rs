extern crate console_error_panic_hook;
use crate::utils::log;
use clarinet_files::{FileAccessor, FileLocation, PerformFileAccess};
use js_sys::{Function as JsFunction, Promise};
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::{from_value as decode_from_js, to_value as encode_to_js};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

#[derive(Serialize, Deserialize)]
struct VFSRequest {
    pub path: String,
}
#[derive(Serialize, Deserialize)]
struct VFSWriteRequest<'a> {
    pub path: String,
    pub content: &'a [u8],
}

pub struct VscodeFilesystemAccessor {
    client_request_tx: JsFunction,
}

impl VscodeFilesystemAccessor {
    pub fn new(client_request_tx: JsFunction) -> VscodeFilesystemAccessor {
        VscodeFilesystemAccessor { client_request_tx }
    }
}

impl FileAccessor for VscodeFilesystemAccessor {
    fn read_manifest_content(&self, manifest_location: FileLocation) -> PerformFileAccess {
        log!("reading manifest");
        let path = manifest_location.to_string();
        let req = self
            .client_request_tx
            .call2(
                &JsValue::null(),
                &JsValue::from("vfs/readFile"),
                &encode_to_js(&VFSRequest { path: path.clone() }).unwrap(),
            )
            .unwrap();

        return Box::pin(async move {
            let response = JsFuture::from(Promise::resolve(&req)).await;
            match response {
                Ok(manifest) => Ok((
                    FileLocation::from_url_string(&path).unwrap(),
                    decode_from_js(manifest).unwrap(),
                )),
                Err(_) => Err("error".into()),
            }
        });
    }

    fn read_contract_content(
        &self,
        manifest_location: FileLocation,
        relative_path: String,
    ) -> PerformFileAccess {
        log!("reading contract");
        let mut contract_location = manifest_location.get_parent_location().unwrap();
        let _ = contract_location.append_path(&relative_path);

        let req = self
            .client_request_tx
            .call2(
                &JsValue::null(),
                &JsValue::from("vfs/readFile"),
                &encode_to_js(&VFSRequest {
                    path: contract_location.to_string(),
                })
                .unwrap(),
            )
            .unwrap();

        return Box::pin(async move {
            let response = JsFuture::from(Promise::resolve(&req)).await;
            match response {
                Ok(contract) => Ok((contract_location, decode_from_js(contract).unwrap())),
                Err(_) => Err("error".into()),
            }
        });
    }

    fn write_file(
        &self,
        manifest_location: FileLocation,
        relative_path: String,
        content: &[u8],
    ) -> PerformFileAccess {
        log!("writting contract");
        let mut contract_location = manifest_location.get_parent_location().unwrap();
        let _ = contract_location.append_path(&relative_path);

        let req = self
            .client_request_tx
            .call2(
                &JsValue::null(),
                &JsValue::from("vfs/writeFile"),
                &encode_to_js(&VFSWriteRequest {
                    path: relative_path,
                    content,
                })
                .unwrap(),
            )
            .unwrap();

        return Box::pin(async move {
            let response = JsFuture::from(Promise::resolve(&req)).await;
            match response {
                // TODO: add type for PerformFileWrite
                Ok(_) => Ok((contract_location, "success".to_string())),
                Err(_) => Err("error".into()),
            }
        });
    }
}

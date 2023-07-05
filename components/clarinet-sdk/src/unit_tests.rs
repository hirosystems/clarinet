use clarinet_deployments::{
    generate_default_deployment, initiate_session_from_deployment,
    update_session_with_contracts_executions, update_session_with_genesis_accounts,
};
use clarinet_files::chainhook_types::StacksNetwork;
use clarinet_files::{FileAccessor, FileLocation, ProjectManifest, WASMFileSystemAccessor};
use clarity_repl::clarity::analysis::contract_interface_builder::{
    build_contract_interface, ContractInterface, ContractInterfaceFunction,
    ContractInterfaceFunctionAccess,
};
use clarity_repl::clarity::stacks_common::types::StacksEpochId;
use clarity_repl::clarity::EvaluationResult;
use clarity_repl::repl::Session;
use js_sys::Function as JsFunction;
use serde_wasm_bindgen::to_value as encode_to_js;
use std::collections::HashMap;
use std::{panic, path::PathBuf};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

use crate::utils::{self, raw_value_to_string, serialize_event};

macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into());
    }
}

#[wasm_bindgen]
pub struct CallContractRes {
    result: String,
    events: js_sys::Array,
}

#[wasm_bindgen]
impl CallContractRes {
    #[wasm_bindgen(getter)]
    pub fn result(&self) -> String {
        self.result.clone()
    }
    #[wasm_bindgen(getter)]
    pub fn events(&self) -> js_sys::Array {
        self.events.clone()
    }
}

#[wasm_bindgen]
pub struct TestSession {
    file_accessor: Box<dyn FileAccessor>,
    session: Option<Session>,
    accounts: HashMap<String, String>,
    // @todo: contract_interfaces: HashMap<QualifiedContractIdentifier, ContractInterface>,
    contract_interfaces: HashMap<String, ContractInterface>,
}

#[wasm_bindgen]
impl TestSession {
    #[wasm_bindgen(constructor)]
    pub fn new(fs_request: JsFunction) -> Self {
        panic::set_hook(Box::new(console_error_panic_hook::hook));

        let fs = Box::new(WASMFileSystemAccessor::new(fs_request));
        Self {
            file_accessor: fs,
            session: None,
            accounts: HashMap::new(),
            contract_interfaces: HashMap::new(),
        }
    }

    #[wasm_bindgen(getter, js_name=blockHeight)]
    pub fn block_height(&mut self) -> u32 {
        let session = self.session.as_mut().unwrap();
        session.interpreter.get_block_height()
    }

    #[wasm_bindgen(js_name=initSession)]
    pub async fn init_session(
        &mut self,
        root: String,
        manifest_path: String,
    ) -> Result<(), String> {
        let mut root_path = PathBuf::new();
        root_path.push(root);
        let project_root = FileLocation::FileSystem { path: root_path };
        let manifest_location = FileLocation::try_parse(&manifest_path, Some(&project_root))
            .ok_or("failed to parse manifest location")?;
        let manifest =
            ProjectManifest::from_file_accessor(&manifest_location, &self.file_accessor).await?;

        let (deployment, artifacts) = generate_default_deployment(
            &manifest,
            &StacksNetwork::Simnet,
            false,
            Some(&self.file_accessor),
            Some(StacksEpochId::Epoch21),
        )
        .await?;

        let mut session = initiate_session_from_deployment(&manifest);

        if let Some(ref spec) = deployment.genesis {
            for wallet in spec.wallets.iter() {
                self.accounts
                    .insert(wallet.name.clone(), wallet.address.to_string());
            }
        }

        let _ = update_session_with_genesis_accounts(&mut session, &deployment);
        let results = update_session_with_contracts_executions(
            &mut session,
            &deployment,
            Some(&artifacts.asts),
            false,
            Some(StacksEpochId::Epoch21),
        );

        for (contract_id, result) in results.into_iter() {
            match result {
                Ok(execution) => {
                    if let EvaluationResult::Contract(contract_result) = execution.result {
                        let interface =
                            build_contract_interface(&contract_result.contract.analysis);
                        self.contract_interfaces
                            .insert(contract_id.name.to_string(), interface);
                    };
                }
                Err(e) => {
                    log!("unable to load deployment: {:}", e[0].message);
                    std::process::exit(1);
                }
            }
        }

        let _ = session.start_wasm();
        self.session = Some(session);

        Ok(())
    }

    #[wasm_bindgen(js_name=getAssetsMap)]
    pub fn get_assets_maps(&mut self) -> Result<JsValue, JsValue> {
        let session = &self.session.as_mut().unwrap();
        let assets_maps = session.get_assets_maps();
        Ok(encode_to_js(&assets_maps)?)
    }

    #[wasm_bindgen(js_name=getAccounts)]
    pub fn get_accounts(&mut self) -> Result<JsValue, JsValue> {
        Ok(encode_to_js(&self.accounts)?)
    }

    fn get_function_interface(
        &self,
        contract: &str,
        method: &str,
    ) -> Result<&ContractInterfaceFunction, String> {
        let contract_interface = self
            .contract_interfaces
            .get(contract)
            .ok_or("unable to get contract interface")?;

        Ok(contract_interface
            .functions
            .iter()
            .find(|func| func.name == method)
            .ok_or(format!("contract {contract} has no function {method}"))?)
    }

    fn invoke_contract_call(
        &mut self,
        contract: &str,
        method: &str,
        js_args: &Vec<js_sys::Uint8Array>,
        sender: &str,
        test_name: &str,
    ) -> CallContractRes {
        let session = self.session.as_mut().unwrap();
        let args: Vec<String> = js_args.iter().map(|a| raw_value_to_string(a)).collect();

        let (execution, _) =
            match session.invoke_contract_call(contract, method, &args, sender, test_name.into()) {
                Ok(res) => res,
                Err(diagnostics) => {
                    let mut message = format!(
                        "{}: {}::{}({})",
                        "Contract call runtime error",
                        contract,
                        method,
                        args.join(", ")
                    );
                    if let Some(diag) = diagnostics.last() {
                        message = format!("{} -> {}", message, diag.message);
                    }
                    log!("message: {}", message);
                    std::process::exit(1);
                }
            };

        let result = match execution.result {
            EvaluationResult::Snippet(result) => utils::to_raw_value(&result.result),
            _ => unreachable!("Contract value from snippet"),
        };
        let events = js_sys::Array::new_with_length(execution.events.len() as u32);
        for (i, event) in execution.events.iter().enumerate() {
            events.set(i as u32, encode_to_js(&serialize_event(event)).unwrap())
        }

        CallContractRes { result, events }
    }

    #[wasm_bindgen(js_name=callReadOnlyFn)]
    pub fn call_read_only_fn(
        &mut self,
        contract: &str,
        method: &str,
        js_args: Vec<js_sys::Uint8Array>,
        sender: &str,
    ) -> Result<CallContractRes, String> {
        let interface = self.get_function_interface(contract, method)?;
        if interface.access != ContractInterfaceFunctionAccess::read_only {
            return Err(format!("{method} is not a read-only function"));
        }

        Ok(self.invoke_contract_call(contract, method, &js_args, sender, "read-only call"))
    }

    #[wasm_bindgen(js_name = "callPublicFn")]
    pub fn call_public_fn(
        &mut self,
        contract: &str,
        method: &str,
        js_args: Vec<js_sys::Uint8Array>,
        sender: &str,
    ) -> Result<CallContractRes, String> {
        let interface = self.get_function_interface(contract, method)?;
        if interface.access != ContractInterfaceFunctionAccess::public {
            return Err(format!("{method} is not a public function"));
        }

        let res = self.invoke_contract_call(contract, method, &js_args, sender, "public call");

        let session = self.session.as_mut().unwrap();
        session.advance_chain_tip(1);

        Ok(res)
    }

    // #[wasm_bindgen(js_name=mineBlock)]
    // pub fn mine_block(&mut self, js_params: JsValue) -> JsValue {
    //     // @todo
    //     // re-implement chainhooks logic
    //     // https://github.com/hirosystems/clarinet/blob/c6d65974f489606c0a8c57a6d0f01cfb993eb79b/components/clarinet-cli/src/runner/api_v1.rs#L718-L811
    // }

    #[wasm_bindgen(js_name=mineEmptyBlock)]
    pub fn mine_empty_block(&mut self) -> u32 {
        let session = self.session.as_mut().unwrap();
        session.advance_chain_tip(1)
    }

    #[wasm_bindgen(js_name=mineEmptyBlocks)]
    pub fn mine_empty_blocks(&mut self, count: Option<u32>) -> u32 {
        let session = self.session.as_mut().unwrap();
        session.advance_chain_tip(count.unwrap_or(1))
    }

    #[wasm_bindgen(js_name=runSnippet)]
    pub fn run_snippet(&mut self, snippet: String) -> JsValue {
        let session = self.session.as_mut().unwrap();
        let (_, output) = session.handle_command(&snippet);
        let output_as_array = js_sys::Array::new_with_length(output.len() as u32);
        for string in output {
            output_as_array.push(&JsValue::from_str(&string));
        }
        // @todo: can actualyl return raw value like contract calls
        output_as_array.into()
    }
}

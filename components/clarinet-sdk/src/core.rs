use clarinet_deployments::types::{DeploymentGenerationArtifacts, DeploymentSpecification};
use clarinet_deployments::{
    generate_default_deployment, initiate_session_from_deployment,
    update_session_with_contracts_executions, update_session_with_genesis_accounts,
};
use clarinet_files::chainhook_types::StacksNetwork;
use clarinet_files::{FileAccessor, FileLocation, ProjectManifest, WASMFileSystemAccessor};
use clarity_repl::analysis::coverage::CoverageReporter;
use clarity_repl::clarity::analysis::contract_interface_builder::{
    build_contract_interface, ContractInterface, ContractInterfaceFunction,
    ContractInterfaceFunctionAccess,
};
use clarity_repl::clarity::stacks_common::types::StacksEpochId;
use clarity_repl::clarity::vm::types::QualifiedContractIdentifier;
use clarity_repl::clarity::{EvaluationResult, ExecutionResult};
use clarity_repl::repl::{
    ClarityCodeSource, ClarityContract, ContractDeployer, Session, DEFAULT_CLARITY_VERSION,
    DEFAULT_EPOCH,
};
use js_sys::Function as JsFunction;
use serde::Deserialize;
use serde_wasm_bindgen::to_value as encode_to_js;
use std::collections::HashMap;
use std::{panic, path::PathBuf};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

use crate::utils::{self, serialize_event, uint8_to_string, uint8_to_value};

#[derive(Debug, Deserialize)]
#[wasm_bindgen]
pub struct CallContractArgs {
    contract: String,
    method: String,
    args: Vec<Vec<u8>>,
}

#[wasm_bindgen]
impl CallContractArgs {
    #[wasm_bindgen(constructor)]
    pub fn new(contract: String, method: String, args: Vec<js_sys::Uint8Array>) -> Self {
        Self {
            contract,
            method,
            args: args.iter().map(|a| a.to_vec()).collect::<Vec<Vec<u8>>>(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[wasm_bindgen]
pub struct DeployContractArgs {
    name: String,
    content: String,
}

#[wasm_bindgen]
impl DeployContractArgs {
    #[wasm_bindgen(constructor)]
    pub fn new(name: String, content: String) -> Self {
        Self { name, content }
    }
}

#[derive(Debug, Deserialize)]
#[wasm_bindgen]
pub struct TransferSTXArgs {
    amount: u64,
    recipient: String,
}

#[wasm_bindgen]
impl TransferSTXArgs {
    #[wasm_bindgen(constructor)]
    pub fn new(amount: u64, recipient: String) -> Self {
        Self { amount, recipient }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[wasm_bindgen]
pub struct TxArgs {
    call_contract: Option<CallContractArgs>,
    deploy_contract: Option<DeployContractArgs>,
    transfer_stx: Option<TransferSTXArgs>,
    sender: String,
}

macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into());
    }
}

#[wasm_bindgen(getter_with_clone)]
pub struct TransactionRes {
    pub result: String,
    pub events: js_sys::Array,
}

#[wasm_bindgen(getter_with_clone)]
pub struct SessionReport {
    pub coverage: String,
}

pub fn execution_result_to_transaction_res(execution: &ExecutionResult) -> TransactionRes {
    let result = match &execution.result {
        EvaluationResult::Snippet(result) => utils::to_raw_value(&result.result),
        _ => unreachable!("Contract value from snippet"),
    };
    let events = js_sys::Array::new_with_length(execution.events.len() as u32);
    for (i, event) in execution.events.iter().enumerate() {
        events.set(i as u32, encode_to_js(&serialize_event(event)).unwrap())
    }

    TransactionRes { result, events }
}

#[wasm_bindgen(getter_with_clone)]
pub struct SDK {
    pub deployer: String,
    file_accessor: Box<dyn FileAccessor>,
    session: Option<Session>,
    accounts: HashMap<String, String>,
    contracts_locations: HashMap<QualifiedContractIdentifier, FileLocation>,
    contracts_interfaces: HashMap<QualifiedContractIdentifier, ContractInterface>,
    cache: Option<(DeploymentSpecification, DeploymentGenerationArtifacts)>,
    current_test_name: String,
}

#[wasm_bindgen]
impl SDK {
    #[wasm_bindgen(constructor)]
    pub fn new(fs_request: JsFunction) -> Self {
        panic::set_hook(Box::new(console_error_panic_hook::hook));

        let fs = Box::new(WASMFileSystemAccessor::new(fs_request));
        Self {
            deployer: String::new(),
            file_accessor: fs,
            session: None,
            accounts: HashMap::new(),
            contracts_locations: HashMap::new(),
            contracts_interfaces: HashMap::new(),
            cache: None,
            current_test_name: String::new(),
        }
    }

    fn desugar_contract_id(&self, contract: &str) -> Result<QualifiedContractIdentifier, String> {
        let contract_id = if contract.starts_with('S') {
            contract.to_string()
        } else {
            format!("{}.{}", self.deployer, contract,)
        };

        QualifiedContractIdentifier::parse(&contract_id).map_err(|e| e.to_string())
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

        let (deployment, artifacts) = match &self.cache {
            Some(cache) => cache.clone(),
            None => {
                let cache = generate_default_deployment(
                    &manifest,
                    &StacksNetwork::Simnet,
                    false,
                    Some(&self.file_accessor),
                    Some(StacksEpochId::Epoch21),
                )
                .await?;
                self.cache = Some(cache.clone());
                cache
            }
        };

        let mut session = initiate_session_from_deployment(&manifest);

        if let Some(ref spec) = deployment.genesis {
            for wallet in spec.wallets.iter() {
                if wallet.name == "deployer" {
                    self.deployer = wallet.address.to_string();
                }
                self.accounts
                    .insert(wallet.name.clone(), wallet.address.to_string());
            }
        }

        update_session_with_genesis_accounts(&mut session, &deployment);
        let results = update_session_with_contracts_executions(
            &mut session,
            &deployment,
            Some(&artifacts.asts),
            false,
            Some(StacksEpochId::Epoch21),
        );

        for (contract_id, (_, location)) in deployment.contracts {
            self.contracts_locations
                .insert(contract_id, location.clone());
        }

        for (contract_id, result) in results.into_iter() {
            match result {
                Ok(execution) => {
                    if let EvaluationResult::Contract(contract_result) = execution.result {
                        let interface =
                            build_contract_interface(&contract_result.contract.analysis);
                        self.contracts_interfaces.insert(contract_id, interface);
                    };
                }
                Err(e) => {
                    log!("unable to load deployment: {:}", e[0].message);
                    std::process::exit(1);
                }
            }
        }

        self.session = Some(session);
        Ok(())
    }

    fn get_session(&self) -> &Session {
        self.session
            .as_ref()
            .expect("Session not initialised. Call initSession() first")
    }

    fn get_session_mut(&mut self) -> &mut Session {
        self.session
            .as_mut()
            .expect("Session not initialised. Call initSession() first")
    }

    #[wasm_bindgen(getter, js_name=blockHeight)]
    pub fn block_height(&mut self) -> u32 {
        let session = self.get_session_mut();
        session.interpreter.get_block_height()
    }

    #[wasm_bindgen(js_name=getContractsInterfaces)]
    pub fn get_contracts_interfaces(&self) -> Result<JsValue, JsValue> {
        Ok(encode_to_js(&self.contracts_interfaces)?)
    }

    #[wasm_bindgen(js_name=getAssetsMap)]
    pub fn get_assets_maps(&self) -> Result<JsValue, JsValue> {
        let session = &self.get_session();
        let assets_maps = session.get_assets_maps();
        Ok(encode_to_js(&assets_maps)?)
    }

    #[wasm_bindgen(js_name=getAccounts)]
    pub fn get_accounts(&mut self) -> Result<JsValue, JsValue> {
        Ok(encode_to_js(&self.accounts)?)
    }

    #[wasm_bindgen(js_name=getDataVar)]
    pub fn get_data_var(&mut self, contract: &str, var_name: &str) -> Result<String, String> {
        let contract_id = self.desugar_contract_id(contract)?;
        let session = self.get_session_mut();
        session
            .interpreter
            .get_data_var(&contract_id, var_name)
            .ok_or("value not found".into())
    }

    #[wasm_bindgen(js_name=getMapEntry)]
    pub fn get_map_entry(
        &mut self,
        contract: &str,
        map_name: &str,
        map_key: Vec<u8>,
    ) -> Result<String, String> {
        let contract_id = self.desugar_contract_id(contract)?;
        let session = self.get_session_mut();
        session
            .interpreter
            .get_map_entry(&contract_id, map_name, &uint8_to_value(&map_key))
            .ok_or("value not found".into())
    }

    fn get_function_interface(
        &self,
        contract: &str,
        method: &str,
    ) -> Result<&ContractInterfaceFunction, String> {
        let contract_id = self.desugar_contract_id(contract)?;
        let contract_interface = self
            .contracts_interfaces
            .get(&contract_id)
            .ok_or(format!("unable to get contract interface for {contract}"))?;
        contract_interface
            .functions
            .iter()
            .find(|func| func.name == method)
            .ok_or(format!("contract {contract} has no function {method}"))
    }

    fn invoke_contract_call(
        &mut self,
        call_contract_args: &CallContractArgs,
        sender: &str,
        test_name: &str,
    ) -> Result<TransactionRes, String> {
        let CallContractArgs {
            contract,
            method,
            args,
        } = call_contract_args;

        let clarity_args: Vec<String> = args.iter().map(|a| uint8_to_string(a)).collect();

        let session = self.get_session_mut();
        let (execution, _) = match session.invoke_contract_call(
            contract,
            method,
            &clarity_args,
            sender,
            test_name.into(),
        ) {
            Ok(res) => res,
            Err(diagnostics) => {
                let mut message = format!(
                    "{}: {}::{}({})",
                    "Contract call error",
                    contract,
                    method,
                    clarity_args.join(", ")
                );
                if let Some(diag) = diagnostics.last() {
                    message = format!("{} -> {}", message, diag.message);
                }
                log!("message: {}", message);
                return Err(message);
            }
        };

        Ok(execution_result_to_transaction_res(&execution))
    }

    #[wasm_bindgen(js_name=callReadOnlyFn)]
    pub fn call_read_only_fn(
        &mut self,
        args: &CallContractArgs,
        sender: &str,
    ) -> Result<TransactionRes, String> {
        let interface = self.get_function_interface(&args.contract, &args.method)?;
        if interface.access != ContractInterfaceFunctionAccess::read_only {
            return Err(format!("{} is not a read-only function", &args.method));
        }

        self.invoke_contract_call(args, sender, &self.current_test_name.clone())
    }

    fn call_public_fn_private(
        &mut self,
        args: &CallContractArgs,
        sender: &str,
        advance_chain_tip: bool,
    ) -> Result<TransactionRes, String> {
        let interface = self.get_function_interface(&args.contract, &args.method)?;
        if interface.access != ContractInterfaceFunctionAccess::public {
            return Err(format!("{} is not a public function", &args.method));
        }

        let session = self.get_session_mut();
        if advance_chain_tip {
            session.advance_chain_tip(1);
        }

        self.invoke_contract_call(args, sender, &self.current_test_name.clone())
    }

    fn transfer_stx_private(
        &mut self,
        args: &TransferSTXArgs,
        sender: &str,
        advance_chain_tip: bool,
    ) -> Result<TransactionRes, String> {
        let session = self.get_session_mut();
        let initial_tx_sender = session.get_tx_sender();
        session.set_tx_sender(sender.to_string());

        let execution = match session.stx_transfer(args.amount, &args.recipient) {
            Ok(res) => res,
            Err(diagnostics) => {
                let mut message = format!("{}: {}", "STX transfer error", sender);
                if let Some(diag) = diagnostics.last() {
                    message = format!("{} -> {}", message, diag.message);
                }
                return Err(message);
            }
        };

        if advance_chain_tip {
            session.advance_chain_tip(1);
        }
        session.set_tx_sender(initial_tx_sender);
        Ok(execution_result_to_transaction_res(&execution))
    }

    fn deploy_contract_private(
        &mut self,
        args: &DeployContractArgs,
        sender: &str,
        advance_chain_tip: bool,
    ) -> Result<TransactionRes, String> {
        let session = self.get_session_mut();
        let contract = ClarityContract {
            code_source: ClarityCodeSource::ContractInMemory(args.content.clone()),
            name: args.name.clone(),
            deployer: ContractDeployer::Address(sender.to_string()),
            clarity_version: DEFAULT_CLARITY_VERSION,
            epoch: DEFAULT_EPOCH,
        };

        let execution = match session.deploy_contract(
            &contract,
            None,
            false,
            Some(args.name.clone()),
            &mut None,
        ) {
            Ok(res) => res,
            Err(diagnostics) => {
                let mut message = format!(
                    "{}: {}.{}",
                    "Contract deployment runtime error", sender, args.name
                );
                if let Some(diag) = diagnostics.last() {
                    message = format!("{} -> {}", message, diag.message);
                }
                return Err(message);
            }
        };

        if advance_chain_tip {
            session.advance_chain_tip(1);
        }
        Ok(execution_result_to_transaction_res(&execution))
    }

    #[wasm_bindgen(js_name=deployContract)]
    pub fn deploy_contract(
        &mut self,
        args: &DeployContractArgs,
        sender: &str,
    ) -> Result<TransactionRes, String> {
        self.deploy_contract_private(args, sender, true)
    }

    #[wasm_bindgen(js_name = "transferSTX")]
    pub fn transfer_stx(
        &mut self,
        args: &TransferSTXArgs,
        sender: &str,
    ) -> Result<TransactionRes, String> {
        self.transfer_stx_private(args, sender, true)
    }

    #[wasm_bindgen(js_name = "callPublicFn")]
    pub fn call_public_fn(
        &mut self,
        args: &CallContractArgs,
        sender: &str,
    ) -> Result<TransactionRes, String> {
        self.call_public_fn_private(args, sender, true)
    }

    #[wasm_bindgen(js_name=mineBlock)]
    pub fn mine_block_js(&mut self, txs: js_sys::Array) -> Result<(), String> {
        for js_tx in txs.to_vec() {
            let tx: TxArgs = match serde_wasm_bindgen::from_value(js_tx) {
                Ok(tx) => tx,
                Err(err) => return Err(format!("error: {}", err)),
            };
            if let Some(contract_call_args) = tx.call_contract {
                let _ = self.call_public_fn_private(&contract_call_args, &tx.sender, false);
            } else if let Some(transfer_stx_args) = tx.transfer_stx {
                let _ = self.transfer_stx_private(&transfer_stx_args, &tx.sender, false);
            } else if let Some(deploy_contract_args) = tx.deploy_contract {
                let _ = self.deploy_contract_private(&deploy_contract_args, &tx.sender, false);
            }
        }

        let session = self.get_session_mut();
        session.advance_chain_tip(1);
        Ok(())
    }

    #[wasm_bindgen(js_name=mineEmptyBlock)]
    pub fn mine_empty_block(&mut self) -> u32 {
        let session = self.get_session_mut();
        session.advance_chain_tip(1)
    }

    #[wasm_bindgen(js_name=mineEmptyBlocks)]
    pub fn mine_empty_blocks(&mut self, count: Option<u32>) -> u32 {
        let session = self.get_session_mut();
        session.advance_chain_tip(count.unwrap_or(1))
    }

    #[wasm_bindgen(js_name=runSnippet)]
    pub fn run_snippet(&mut self, snippet: String) -> JsValue {
        let session = self.get_session_mut();
        let (_, output) = session.handle_command(&snippet);
        let output_as_array = js_sys::Array::new_with_length(output.len() as u32);
        for string in output {
            output_as_array.push(&JsValue::from_str(&string));
        }
        // @todo: can actually return raw value like contract calls
        output_as_array.into()
    }

    #[wasm_bindgen(js_name=setCurrentTestName)]
    pub fn set_current_test_name(&mut self, test_name: String) {
        self.current_test_name = test_name;
    }

    #[wasm_bindgen(js_name=getReport)]
    pub fn collect_report(&mut self) -> Result<SessionReport, String> {
        let contracts_locations = self.contracts_locations.clone();
        let session = self.get_session_mut();
        let mut coverage_reporter = CoverageReporter::new();
        coverage_reporter.asts.append(&mut session.asts);
        for (contract_id, contract_location) in contracts_locations.iter() {
            coverage_reporter
                .contract_paths
                .insert(contract_id.name.to_string(), contract_location.to_string());
        }
        coverage_reporter
            .reports
            .append(&mut session.coverage_reports);
        let coverage = coverage_reporter.build_lcov_content();
        Ok(SessionReport { coverage })
    }
}

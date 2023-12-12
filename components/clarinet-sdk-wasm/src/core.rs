use clarinet_deployments::diagnostic_digest::DiagnosticsDigest;
use clarinet_deployments::types::{DeploymentGenerationArtifacts, DeploymentSpecification};
use clarinet_deployments::{
    generate_default_deployment, initiate_session_from_deployment,
    update_session_with_contracts_executions, update_session_with_genesis_accounts,
};
use clarinet_files::chainhook_types::StacksNetwork;
use clarinet_files::{FileAccessor, FileLocation, ProjectManifest, WASMFileSystemAccessor};
use clarity_repl::analysis::coverage::CoverageReporter;
use clarity_repl::clarity::analysis::contract_interface_builder::{
    ContractInterface, ContractInterfaceFunction, ContractInterfaceFunctionAccess,
};
use clarity_repl::clarity::stacks_common::types::StacksEpochId;
use clarity_repl::clarity::vm::types::QualifiedContractIdentifier;
use clarity_repl::clarity::{ClarityVersion, EvaluationResult, ExecutionResult, ParsedContract};
use clarity_repl::repl::{
    ClarityCodeSource, ClarityContract, ContractDeployer, Session, DEFAULT_CLARITY_VERSION,
    DEFAULT_EPOCH,
};
use gloo_utils::format::JsValueSerdeExt;
use js_sys::Function as JsFunction;
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_wasm_bindgen::to_value as encode_to_js;
use std::collections::HashMap;
use std::{panic, path::PathBuf};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

use crate::utils::clarity_values::{self, uint8_to_string, uint8_to_value};
use crate::utils::costs::SerializableCostsReport;
use crate::utils::events::serialize_event;

#[wasm_bindgen(typescript_custom_section)]
const SET_EPOCH: &'static str = r#"
type EpochString = "2.0" | "2.05" | "2.1" | "2.2" | "2.3" | "2.4"
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "ITextStyle")]
    pub type ITextStyle;
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "Map<string, Map<string, bigint>>")]
    pub type AssetsMap;
    #[wasm_bindgen(typescript_type = "Map<string, string>")]
    pub type Accounts;
    #[wasm_bindgen(typescript_type = "EpochString")]
    pub type EpochString;
}

macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into());
    }
}

#[derive(Debug, Deserialize)]
struct CallContractArgsJSON {
    contract: String,
    method: String,
    args_maps: Vec<HashMap<usize, u8>>,
    sender: String,
}

#[derive(Debug, Deserialize)]
#[wasm_bindgen]
pub struct CallContractArgs {
    contract: String,
    method: String,
    args: Vec<Vec<u8>>,
    sender: String,
}

#[wasm_bindgen]
impl CallContractArgs {
    #[wasm_bindgen(constructor)]
    pub fn new(
        contract: String,
        method: String,
        args: Vec<js_sys::Uint8Array>,
        sender: String,
    ) -> Self {
        Self {
            contract,
            method,
            args: args.iter().map(|a| a.to_vec()).collect::<Vec<Vec<u8>>>(),
            sender,
        }
    }

    /*
      The mineBlock method receives an JSON Array of Txs, including ContractCalls.
      Because it's JSON, the Uint8Array arguments are passed as Map<index, value> instead of Vec<u8>.
      This method transform the Map back into a Vec.
    */
    fn from_json_args(
        CallContractArgsJSON {
            contract,
            method,
            args_maps,
            sender,
        }: CallContractArgsJSON,
    ) -> Self {
        let mut args: Vec<Vec<u8>> = vec![];
        for arg in args_maps {
            let mut parsed_arg: Vec<u8> = vec![0; arg.len()];
            for (i, v) in arg.iter() {
                parsed_arg[*i] = *v;
            }
            args.push(parsed_arg);
        }
        Self {
            contract,
            method,
            args,
            sender,
        }
    }
}

#[derive(Debug, Deserialize)]
#[wasm_bindgen]
pub struct ContractOptions {
    clarity_version: ClarityVersion,
}

#[wasm_bindgen]
impl ContractOptions {
    #[wasm_bindgen(constructor)]
    pub fn new(clarity_version: Option<u32>) -> Self {
        let clarity_version = match clarity_version {
            Some(v) => match v {
                1 => ClarityVersion::Clarity1,
                2 => ClarityVersion::Clarity2,
                _ => {
                    log!("Invalid clarity version {v}. Using default version.");
                    DEFAULT_CLARITY_VERSION
                }
            },
            _ => DEFAULT_CLARITY_VERSION,
        };

        Self { clarity_version }
    }
}

#[derive(Debug, Deserialize)]
#[wasm_bindgen]
pub struct DeployContractArgs {
    name: String,
    content: String,
    options: ContractOptions,
    sender: String,
}

#[wasm_bindgen]
impl DeployContractArgs {
    #[wasm_bindgen(constructor)]
    pub fn new(name: String, content: String, options: ContractOptions, sender: String) -> Self {
        Self {
            name,
            content,
            options,
            sender,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[wasm_bindgen]
pub struct TransferSTXArgs {
    amount: u64,
    recipient: String,
    sender: String,
}

#[wasm_bindgen]
impl TransferSTXArgs {
    #[wasm_bindgen(constructor)]
    pub fn new(amount: u64, recipient: String, sender: String) -> Self {
        Self {
            amount,
            recipient,
            sender,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[wasm_bindgen]
pub struct TxArgs {
    call_public_fn: Option<CallContractArgsJSON>,
    deploy_contract: Option<DeployContractArgs>,
    #[serde(rename(serialize = "transfer_stx", deserialize = "transferSTX"))]
    transfer_stx: Option<TransferSTXArgs>,
}

#[wasm_bindgen(getter_with_clone)]
#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionRes {
    pub result: String,
    pub events: String,
}

#[derive(Serialize, Deserialize)]
pub struct TransactionResRaw {
    pub result: String,
    pub events: Vec<String>,
}

#[wasm_bindgen(getter_with_clone)]
pub struct SessionReport {
    pub coverage: String,
    pub costs: String,
}

pub fn execution_result_to_transaction_res(execution: &ExecutionResult) -> TransactionRes {
    let result = match &execution.result {
        EvaluationResult::Snippet(result) => clarity_values::to_raw_value(&result.result),
        EvaluationResult::Contract(ref contract) => {
            // contract.;
            match contract.result {
                Some(ref result) => clarity_values::to_raw_value(result),
                _ => "0x03".into(),
            }
        }
    };
    let events_as_strings = execution
        .events
        .iter()
        .map(|e| json!(serialize_event(e)).to_string())
        .collect::<Vec<String>>();

    TransactionRes {
        result,
        events: json!(events_as_strings).to_string(),
    }
}

#[wasm_bindgen(getter_with_clone)]
pub struct SDK {
    pub deployer: String,
    file_accessor: Box<dyn FileAccessor>,
    session: Option<Session>,
    accounts: HashMap<String, String>,
    contracts_locations: HashMap<QualifiedContractIdentifier, FileLocation>,
    contracts_interfaces: HashMap<QualifiedContractIdentifier, ContractInterface>,
    parsed_contracts: HashMap<QualifiedContractIdentifier, ParsedContract>,
    cache: HashMap<FileLocation, (DeploymentSpecification, DeploymentGenerationArtifacts)>,
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
            parsed_contracts: HashMap::new(),
            cache: HashMap::new(),
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
            .ok_or("Failed to parse manifest location")?;
        let manifest =
            ProjectManifest::from_file_accessor(&manifest_location, &*self.file_accessor).await?;

        let (deployment, artifacts) = match self.cache.get(&manifest_location) {
            Some(cache) => cache.clone(),
            None => {
                let cache = generate_default_deployment(
                    &manifest,
                    &StacksNetwork::Simnet,
                    false,
                    Some(&*self.file_accessor),
                    Some(StacksEpochId::Epoch21),
                )
                .await?;
                self.cache.insert(manifest_location, cache.clone());
                cache
            }
        };

        if !artifacts.success {
            let diags_digest = DiagnosticsDigest::new(&artifacts.diags, &deployment);
            if diags_digest.errors > 0 {
                return Err(diags_digest.message);
            }
        }

        let mut session = initiate_session_from_deployment(&manifest);
        update_session_with_genesis_accounts(&mut session, &deployment);
        let results = update_session_with_contracts_executions(
            &mut session,
            &deployment,
            Some(&artifacts.asts),
            false,
            Some(DEFAULT_EPOCH),
        );

        if let Some(ref spec) = deployment.genesis {
            for wallet in spec.wallets.iter() {
                if wallet.name == "deployer" {
                    self.deployer = wallet.address.to_string();
                }
                self.accounts
                    .insert(wallet.name.clone(), wallet.address.to_string());
            }
        }

        for (contract_id, (_, location)) in deployment.contracts {
            self.contracts_locations
                .insert(contract_id, location.clone());
        }

        for (_, result) in results.into_iter() {
            match result {
                Ok(execution_result) => {
                    self.add_contract(&execution_result);
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

    fn add_contract(&mut self, execution_result: &ExecutionResult) {
        if let EvaluationResult::Contract(ref result) = &execution_result.result {
            let contract_id = result.contract.analysis.contract_identifier.clone();
            if let Some(contract_interface) = &result.contract.analysis.contract_interface {
                self.contracts_interfaces
                    .insert(contract_id.clone(), contract_interface.clone());
            }
            self.parsed_contracts
                .insert(contract_id, result.contract.clone());
        };
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

    #[wasm_bindgen(getter, js_name=currentEpoch)]
    pub fn current_epoch(&mut self) -> String {
        let session = self.get_session_mut();
        session.current_epoch.to_string()
    }

    #[wasm_bindgen(js_name=setEpoch)]
    pub fn set_epoch(&mut self, epoch: EpochString) {
        let epoch = epoch.as_string().unwrap_or("2.4".into());
        let epoch = match epoch.as_str() {
            "2.0" => StacksEpochId::Epoch20,
            "2.05" => StacksEpochId::Epoch2_05,
            "2.1" => StacksEpochId::Epoch21,
            "2.2" => StacksEpochId::Epoch22,
            "2.3" => StacksEpochId::Epoch23,
            "2.4" => StacksEpochId::Epoch24,
            _ => {
                log!("Invalid epoch {epoch}. Using default epoch");
                DEFAULT_EPOCH
            }
        };

        let session = self.get_session_mut();
        session.update_epoch(epoch)
    }

    #[wasm_bindgen(js_name=getContractsInterfaces)]
    pub fn get_contracts_interfaces(&self) -> Result<JsValue, JsError> {
        let stringified_contracts_interfaces: HashMap<String, ContractInterface> = self
            .contracts_interfaces
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect();
        Ok(encode_to_js(&stringified_contracts_interfaces)?)
    }

    #[wasm_bindgen(js_name=getContractSource)]
    pub fn get_contract_source(&self, contract: &str) -> Option<String> {
        let contract_id = self.desugar_contract_id(contract).ok()?;
        let contract = self.parsed_contracts.get(&contract_id)?;
        Some(contract.code.clone())
    }

    #[wasm_bindgen(js_name=getContractAST)]
    pub fn get_contract_ast(&self, contract: &str) -> Result<JsValue, String> {
        let contract_id = self.desugar_contract_id(contract)?;
        let contract = self.parsed_contracts.get(&contract_id).ok_or("err")?;
        encode_to_js(&contract.ast).map_err(|e| e.to_string())
    }

    #[wasm_bindgen(js_name=getAssetsMap)]
    pub fn get_assets_maps(&self) -> Result<AssetsMap, JsError> {
        let session = &self.get_session();
        let assets_maps = session.get_assets_maps();
        Ok(encode_to_js(&assets_maps)?.unchecked_into::<AssetsMap>())
    }

    #[wasm_bindgen(js_name=getAccounts)]
    pub fn get_accounts(&mut self) -> Result<Accounts, JsError> {
        Ok(encode_to_js(&self.accounts)?.unchecked_into::<Accounts>())
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
    #[wasm_bindgen(js_name=getBlockTime)]
    pub fn get_block_time(&mut self) -> u64 {
        self.get_session_mut().interpreter.get_block_time()
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
        test_name: &str,
    ) -> Result<TransactionRes, String> {
        let CallContractArgs {
            contract,
            method,
            args,
            sender,
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
                return Err(message);
            }
        };

        Ok(execution_result_to_transaction_res(&execution))
    }

    #[wasm_bindgen(js_name=callReadOnlyFn)]
    pub fn call_read_only_fn(&mut self, args: &CallContractArgs) -> Result<TransactionRes, String> {
        let interface = self.get_function_interface(&args.contract, &args.method)?;
        if interface.access != ContractInterfaceFunctionAccess::read_only {
            return Err(format!("{} is not a read-only function", &args.method));
        }

        self.invoke_contract_call(args, &self.current_test_name.clone())
    }

    fn call_public_fn_private(
        &mut self,
        args: &CallContractArgs,
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

        self.invoke_contract_call(args, &self.current_test_name.clone())
    }

    fn transfer_stx_private(
        &mut self,
        args: &TransferSTXArgs,
        advance_chain_tip: bool,
    ) -> Result<TransactionRes, String> {
        let session = self.get_session_mut();
        let initial_tx_sender = session.get_tx_sender();
        session.set_tx_sender(args.sender.to_string());

        let execution = match session.stx_transfer(args.amount, &args.recipient) {
            Ok(res) => res,
            Err(diagnostics) => {
                let mut message = format!("{}: {}", "STX transfer error", args.sender);
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
        advance_chain_tip: bool,
    ) -> Result<TransactionRes, String> {
        let execution = {
            let session = self.get_session_mut();
            if advance_chain_tip {
                session.advance_chain_tip(1);
            }

            let contract = ClarityContract {
                code_source: ClarityCodeSource::ContractInMemory(args.content.clone()),
                name: args.name.clone(),
                deployer: ContractDeployer::Address(args.sender.to_string()),
                clarity_version: args.options.clarity_version,
                epoch: session.current_epoch,
            };

            match session.deploy_contract(
                &contract,
                None,
                false,
                Some(args.name.clone()),
                &mut None,
            ) {
                Ok(res) => res,
                Err(diagnostics) => {
                    let mut message = format!(
                        "Contract deployment runtime error: {}.{}",
                        args.sender, args.name
                    );
                    if let Some(diag) = diagnostics.last() {
                        message = format!("{} -> {}", message, diag.message);
                    }
                    return Err(message);
                }
            }
        };
        self.add_contract(&execution);
        Ok(execution_result_to_transaction_res(&execution))
    }

    #[wasm_bindgen(js_name=deployContract)]
    pub fn deploy_contract(&mut self, args: &DeployContractArgs) -> Result<TransactionRes, String> {
        self.deploy_contract_private(args, true)
    }

    #[wasm_bindgen(js_name = "transferSTX")]
    pub fn transfer_stx(&mut self, args: &TransferSTXArgs) -> Result<TransactionRes, String> {
        self.transfer_stx_private(args, true)
    }

    #[wasm_bindgen(js_name = "callPublicFn")]
    pub fn call_public_fn(&mut self, args: &CallContractArgs) -> Result<TransactionRes, String> {
        self.call_public_fn_private(args, true)
    }

    #[wasm_bindgen(js_name=mineBlock)]
    pub fn mine_block_js(&mut self, js_txs: js_sys::Array) -> Result<JsValue, String> {
        let mut results: Vec<TransactionRes> = vec![];

        let txs: Vec<TxArgs> = js_txs
            .into_serde()
            .map_err(|e| format!("Failed to parse js txs: {:}", e))?;

        for tx in txs {
            let result = if let Some(args) = tx.call_public_fn {
                self.call_public_fn_private(&CallContractArgs::from_json_args(args), false)
            } else if let Some(transfer_stx) = tx.transfer_stx {
                self.transfer_stx_private(&transfer_stx, false)
            } else if let Some(deploy_contract) = tx.deploy_contract {
                self.deploy_contract_private(&deploy_contract, false)
            } else {
                return Err("Invalid tx arguments".into());
            }?;
            results.push(result);
        }

        let session = self.get_session_mut();
        session.advance_chain_tip(1);

        encode_to_js(&results).map_err(|e| format!("error: {}", e))
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
    pub fn run_snippet(&mut self, snippet: String) -> js_sys::Array {
        let session = self.get_session_mut();
        let (_, output) = session.handle_command(&snippet);
        let output_as_array = js_sys::Array::new_with_length(output.len() as u32);
        for string in output {
            output_as_array.push(&JsValue::from_str(&string));
        }
        output_as_array
    }

    #[wasm_bindgen(js_name=setCurrentTestName)]
    pub fn set_current_test_name(&mut self, test_name: String) {
        self.current_test_name = test_name;
    }

    // this method empty the session costs and coverage reports
    // and returns this report
    #[wasm_bindgen(js_name=collectReport)]
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

        let mut costs_reports = Vec::new();
        costs_reports.append(&mut session.costs_reports);
        let costs_reports: Vec<SerializableCostsReport> = costs_reports
            .iter()
            .map(SerializableCostsReport::from_vm_costs_report)
            .collect();
        let costs = serde_json::to_string(&costs_reports).map_err(|e| e.to_string())?;

        Ok(SessionReport { coverage, costs })
    }
}

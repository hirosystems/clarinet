use super::boot::{STACKS_BOOT_CODE_MAINNET, STACKS_BOOT_CODE_TESTNET};
use super::diagnostic::output_diagnostic;
use super::logger_hook::LoggerHook;
use super::{ClarityCodeSource, ClarityContract, ClarityInterpreter, ContractDeployer};
use crate::analysis::coverage::CoverageHook;
use crate::repl::clarity_values::value_to_string;
use crate::repl::Settings;
use crate::utils;
use clarity::codec::StacksMessageCodec;
use clarity::types::chainstate::StacksAddress;
use clarity::types::StacksEpochId;
use clarity::vm::ast::ContractAST;
use clarity::vm::diagnostic::{Diagnostic, Level};
use clarity::vm::docs::{make_api_reference, make_define_reference, make_keyword_reference};
use clarity::vm::functions::define::DefineFunctions;
use clarity::vm::functions::NativeFunctions;
use clarity::vm::types::{
    PrincipalData, QualifiedContractIdentifier, StandardPrincipalData, Value,
};
use clarity::vm::variables::NativeVariables;
use clarity::vm::{
    ClarityVersion, CostSynthesis, EvalHook, EvaluationResult, ExecutionResult, ParsedContract,
    SymbolicExpression,
};
use colored::*;
use prettytable::{Cell, Row, Table};
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::num::ParseIntError;

#[cfg(feature = "cli")]
use clarity::vm::analysis::ContractAnalysis;

use super::SessionSettings;

pub static BOOT_TESTNET_ADDRESS: &str = "ST000000000000000000002AMW42H";
pub static BOOT_MAINNET_ADDRESS: &str = "SP000000000000000000002Q6VF78";

pub static V1_BOOT_CONTRACTS: &[&str] = &["bns"];
pub static V2_BOOT_CONTRACTS: &[&str] = &["pox-2", "costs-3"];
pub static V3_BOOT_CONTRACTS: &[&str] = &["pox-3"];
pub static V4_BOOT_CONTRACTS: &[&str] = &["pox-4"];

lazy_static! {
    static ref BOOT_TESTNET_PRINCIPAL: StandardPrincipalData =
        PrincipalData::parse_standard_principal(BOOT_TESTNET_ADDRESS).unwrap();
    static ref BOOT_MAINNET_PRINCIPAL: StandardPrincipalData =
        PrincipalData::parse_standard_principal(BOOT_MAINNET_ADDRESS).unwrap();
    pub static ref BOOT_CONTRACTS_DATA: BTreeMap<QualifiedContractIdentifier, (ClarityContract, ContractAST)> = {
        let mut result = BTreeMap::new();
        let deploy: [(&StandardPrincipalData, [(&str, &str); 13]); 2] = [
            (&*BOOT_TESTNET_PRINCIPAL, *STACKS_BOOT_CODE_TESTNET),
            (&*BOOT_MAINNET_PRINCIPAL, *STACKS_BOOT_CODE_MAINNET),
        ];

        let interpreter =
            ClarityInterpreter::new(StandardPrincipalData::transient(), Settings::default());
        for (deployer, boot_code) in deploy.iter() {
            for (name, code) in boot_code.iter() {
                let (epoch, clarity_version) = match *name {
                    "pox-4" | "signers" | "signers-voting" => {
                        (StacksEpochId::Epoch25, ClarityVersion::Clarity2)
                    }
                    "pox-3" => (StacksEpochId::Epoch24, ClarityVersion::Clarity2),
                    "pox-2" | "costs-3" => (StacksEpochId::Epoch21, ClarityVersion::Clarity2),
                    "cost-2" => (StacksEpochId::Epoch2_05, ClarityVersion::Clarity1),
                    _ => (StacksEpochId::Epoch20, ClarityVersion::Clarity1),
                };

                let boot_contract = ClarityContract {
                    code_source: ClarityCodeSource::ContractInMemory(code.to_string()),
                    deployer: ContractDeployer::Address(deployer.to_address()),
                    name: name.to_string(),
                    epoch,
                    clarity_version,
                };
                let (ast, _, _) = interpreter.build_ast(&boot_contract);
                result.insert(
                    boot_contract.expect_resolved_contract_identifier(None),
                    (boot_contract, ast),
                );
            }
        }
        result
    };
}

#[derive(Clone, Debug)]
pub struct CostsReport {
    pub test_name: String,
    pub contract_id: String,
    pub method: String,
    pub args: Vec<String>,
    pub cost_result: CostSynthesis,
}

#[derive(Clone, Debug)]
pub struct Session {
    pub settings: SessionSettings,
    pub current_epoch: StacksEpochId,
    pub contracts: BTreeMap<QualifiedContractIdentifier, ParsedContract>,
    pub interpreter: ClarityInterpreter,
    api_reference: HashMap<String, String>,
    pub show_costs: bool,
    pub executed: Vec<String>,
    keywords_reference: HashMap<String, String>,

    coverage_hook: Option<CoverageHook>,
}

impl Session {
    pub fn new(settings: SessionSettings) -> Self {
        let tx_sender = {
            let address = match settings.initial_deployer {
                Some(ref entry) => entry.address.clone(),
                None => format!("{}", StacksAddress::burn_address(false)),
            };
            PrincipalData::parse_standard_principal(&address)
                .expect("Unable to parse deployer's address")
        };

        Self {
            interpreter: ClarityInterpreter::new(tx_sender, settings.repl_settings.clone()),
            current_epoch: settings.epoch_id.unwrap_or(StacksEpochId::Epoch2_05),
            contracts: BTreeMap::new(),
            api_reference: build_api_reference(),
            show_costs: false,
            settings,
            executed: Vec::new(),
            keywords_reference: clarity_keywords(),

            coverage_hook: None,
        }
    }

    pub fn enable_coverage(&mut self) {
        self.coverage_hook = Some(CoverageHook::new());
    }

    pub fn set_test_name(&mut self, name: String) {
        if let Some(coverage_hook) = &mut self.coverage_hook {
            coverage_hook.set_current_test_name(name);
        }
    }

    pub fn collect_lcov_content(
        &mut self,
        asts: &BTreeMap<QualifiedContractIdentifier, ContractAST>,
        contract_paths: &BTreeMap<String, String>,
    ) -> String {
        if let Some(coverage_hook) = &mut self.coverage_hook {
            println!("Collecting coverage data...");
            coverage_hook.collect_lcov_content(asts, contract_paths)
        } else {
            "".to_string()
        }
    }

    pub fn load_boot_contracts(&mut self) {
        let default_tx_sender = self.interpreter.get_tx_sender();

        let boot_testnet_deployer = BOOT_TESTNET_PRINCIPAL.clone();
        self.interpreter.set_tx_sender(boot_testnet_deployer);
        self.deploy_boot_contracts(false);

        let boot_mainnet_deployer = BOOT_MAINNET_PRINCIPAL.clone();
        self.interpreter.set_tx_sender(boot_mainnet_deployer);
        self.deploy_boot_contracts(true);

        self.interpreter.set_tx_sender(default_tx_sender);
    }

    fn deploy_boot_contracts(&mut self, mainnet: bool) {
        let boot_code = if mainnet {
            *STACKS_BOOT_CODE_MAINNET
        } else {
            *STACKS_BOOT_CODE_TESTNET
        };

        let tx_sender = self.interpreter.get_tx_sender();
        let deployer = ContractDeployer::Address(tx_sender.to_address());

        for (name, code) in boot_code.iter() {
            if self
                .settings
                .include_boot_contracts
                .contains(&name.to_string())
            {
                let (epoch, clarity_version) = if (*name).eq("pox-4") {
                    (StacksEpochId::Epoch25, ClarityVersion::Clarity2)
                } else if (*name).eq("pox-3") {
                    (StacksEpochId::Epoch24, ClarityVersion::Clarity2)
                } else if (*name).eq("pox-2") || (*name).eq("costs-3") {
                    (StacksEpochId::Epoch21, ClarityVersion::Clarity2)
                } else if (*name).eq("cost-2") {
                    (StacksEpochId::Epoch2_05, ClarityVersion::Clarity1)
                } else {
                    (StacksEpochId::Epoch20, ClarityVersion::Clarity1)
                };

                let contract = ClarityContract {
                    code_source: ClarityCodeSource::ContractInMemory(code.to_string()),
                    name: name.to_string(),
                    deployer: deployer.clone(),
                    clarity_version,
                    epoch,
                };

                // Result ignored, boot contracts are trusted to be valid
                let _ = self.deploy_contract(&contract, false, None);
            }
        }
    }

    #[cfg(feature = "cli")]
    pub fn process_console_input(
        &mut self,
        command: &str,
    ) -> (
        bool,
        Vec<String>,
        Option<Result<ExecutionResult, Vec<Diagnostic>>>,
    ) {
        let mut output = Vec::<String>::new();

        let mut reload = false;
        match command {
            #[cfg(feature = "cli")]
            cmd if cmd.starts_with("::reload") => reload = true,
            #[cfg(feature = "cli")]
            cmd if cmd.starts_with("::read") => self.read(&mut output, cmd),
            #[cfg(feature = "cli")]
            cmd if cmd.starts_with("::debug") => self.debug(&mut output, cmd),
            #[cfg(feature = "cli")]
            cmd if cmd.starts_with("::trace") => self.trace(&mut output, cmd),
            #[cfg(feature = "cli")]
            cmd if cmd.starts_with("::get_costs") => self.get_costs(&mut output, cmd),

            cmd if cmd.starts_with("::") => {
                output.push(self.handle_command(cmd));
            }

            snippet => {
                let execution_result = self.run_snippet(&mut output, self.show_costs, snippet);
                return (false, output, Some(execution_result));
            }
        }

        (reload, output, None)
    }

    pub fn handle_command(&mut self, command: &str) -> String {
        match command {
            "::help" => self.display_help(),

            #[cfg(feature = "cli")]
            cmd if cmd.starts_with("::functions") => self.display_functions(),
            #[cfg(feature = "cli")]
            cmd if cmd.starts_with("::keywords") => self.keywords(),
            #[cfg(feature = "cli")]
            cmd if cmd.starts_with("::describe") => self.display_doc(cmd),
            #[cfg(feature = "cli")]
            cmd if cmd.starts_with("::toggle_costs") => self.toggle_costs(),
            #[cfg(feature = "cli")]
            cmd if cmd.starts_with("::toggle_timings") => self.toggle_timings(),

            cmd if cmd.starts_with("::mint_stx") => self.mint_stx(cmd),
            cmd if cmd.starts_with("::set_tx_sender") => self.parse_and_set_tx_sender(cmd),
            cmd if cmd.starts_with("::get_assets_maps") => {
                self.get_accounts().unwrap_or("No account found".into())
            }
            cmd if cmd.starts_with("::get_contracts") => {
                self.get_contracts().unwrap_or("No contract found".into())
            }
            cmd if cmd.starts_with("::get_burn_block_height") => self.get_burn_block_height(),
            cmd if cmd.starts_with("::get_stacks_block_height") => self.get_block_height(),
            cmd if cmd.starts_with("::get_block_height") => self.get_block_height(),
            cmd if cmd.starts_with("::advance_chain_tip") => self.parse_and_advance_chain_tip(cmd),
            cmd if cmd.starts_with("::advance_stacks_chain_tip") => {
                self.parse_and_advance_stacks_chain_tip(cmd)
            }
            cmd if cmd.starts_with("::advance_burn_chain_tip") => {
                self.parse_and_advance_burn_chain_tip(cmd)
            }
            cmd if cmd.starts_with("::get_epoch") => self.get_epoch(),
            cmd if cmd.starts_with("::set_epoch") => self.set_epoch(cmd),
            cmd if cmd.starts_with("::encode") => self.encode(cmd),
            cmd if cmd.starts_with("::decode") => self.decode(cmd),

            _ => "Invalid command. Try `::help`".yellow().to_string(),
        }
    }

    #[cfg(feature = "cli")]
    fn run_snippet(
        &mut self,
        output: &mut Vec<String>,
        cost_track: bool,
        cmd: &str,
    ) -> Result<ExecutionResult, Vec<Diagnostic>> {
        let (mut result, cost, execution_result) = match self.formatted_interpretation(
            cmd.to_string(),
            None,
            cost_track,
            None,
        ) {
            Ok((mut output, result)) => {
                if let EvaluationResult::Contract(contract_result) = result.result.clone() {
                    let snippet = format!("→ .{} contract successfully stored. Use (contract-call? ...) for invoking the public functions:", contract_result.contract.contract_identifier.clone());
                    output.push(green!(snippet));
                };
                (output, result.cost.clone(), Ok(result))
            }
            Err((err_output, diagnostics)) => (err_output, None, Err(diagnostics)),
        };

        if let Some(cost) = cost {
            let headers = [
                "".to_string(),
                "Consumed".to_string(),
                "Limit".to_string(),
                "Percentage".to_string(),
            ];
            let mut headers_cells = vec![];
            for header in headers.iter() {
                headers_cells.push(Cell::new(header));
            }
            let mut table = Table::new();
            table.add_row(Row::new(headers_cells));
            table.add_row(Row::new(vec![
                Cell::new("Runtime"),
                Cell::new(&cost.total.runtime.to_string()),
                Cell::new(&cost.limit.runtime.to_string()),
                Cell::new(&(Self::get_costs_percentage(&cost.total.runtime, &cost.limit.runtime))),
            ]));
            table.add_row(Row::new(vec![
                Cell::new("Read count"),
                Cell::new(&cost.total.read_count.to_string()),
                Cell::new(&cost.limit.read_count.to_string()),
                Cell::new(
                    &(Self::get_costs_percentage(&cost.total.read_count, &cost.limit.read_count)),
                ),
            ]));
            table.add_row(Row::new(vec![
                Cell::new("Read length (bytes)"),
                Cell::new(&cost.total.read_length.to_string()),
                Cell::new(&cost.limit.read_length.to_string()),
                Cell::new(
                    &(Self::get_costs_percentage(&cost.total.read_length, &cost.limit.read_length)),
                ),
            ]));
            table.add_row(Row::new(vec![
                Cell::new("Write count"),
                Cell::new(&cost.total.write_count.to_string()),
                Cell::new(&cost.limit.write_count.to_string()),
                Cell::new(
                    &(Self::get_costs_percentage(&cost.total.write_count, &cost.limit.write_count)),
                ),
            ]));
            table.add_row(Row::new(vec![
                Cell::new("Write length (bytes)"),
                Cell::new(&cost.total.write_length.to_string()),
                Cell::new(&cost.limit.write_length.to_string()),
                Cell::new(
                    &(Self::get_costs_percentage(
                        &cost.total.write_length,
                        &cost.limit.write_length,
                    )),
                ),
            ]));
            output.push(format!("{}", table));
        }
        output.append(&mut result);
        execution_result
    }

    #[cfg(feature = "cli")]
    fn get_costs_percentage(consumed: &u64, limit: &u64) -> String {
        let calc = (*consumed as f64 / *limit as f64) * 100_f64;

        format!("{calc:.2} %")
    }

    pub fn formatted_interpretation(
        &mut self,
        snippet: String,
        name: Option<String>,
        cost_track: bool,
        eval_hooks: Option<Vec<&mut dyn EvalHook>>,
    ) -> Result<(Vec<String>, ExecutionResult), (Vec<String>, Vec<Diagnostic>)> {
        let result = self.eval_with_hooks(snippet.to_string(), eval_hooks, cost_track);
        let mut output = Vec::<String>::new();
        let formatted_lines: Vec<String> = snippet.lines().map(|l| l.to_string()).collect();
        let contract_name = name.unwrap_or("<stdin>".to_string());

        match result {
            Ok(result) => {
                for diagnostic in &result.diagnostics {
                    output.append(&mut output_diagnostic(
                        diagnostic,
                        &contract_name,
                        &formatted_lines,
                    ));
                }
                if !result.events.is_empty() {
                    output.push(black!("Events emitted"));
                    for event in result.events.iter() {
                        output.push(black!(format!("{}", utils::serialize_event(event))));
                    }
                }
                match &result.result {
                    EvaluationResult::Contract(contract_result) => {
                        if let Some(value) = &contract_result.result {
                            output.push(format!("{}", value).green().to_string());
                        }
                    }
                    EvaluationResult::Snippet(snippet_result) => {
                        output.push(value_to_string(&snippet_result.result).green().to_string())
                    }
                }
                Ok((output, result))
            }
            Err(diagnostics) => {
                for d in &diagnostics {
                    output.append(&mut output_diagnostic(d, &contract_name, &formatted_lines));
                }
                Err((output, diagnostics))
            }
        }
    }

    #[cfg(feature = "cli")]
    pub fn debug(&mut self, output: &mut Vec<String>, cmd: &str) {
        use crate::repl::debug::cli::CLIDebugger;

        let snippet = match cmd.split_once(' ') {
            Some((_, snippet)) => snippet,
            _ => return output.push("Usage: ::debug <expr>".red().to_string()),
        };

        let mut debugger = CLIDebugger::new(&QualifiedContractIdentifier::transient(), snippet);

        let mut result = match self.formatted_interpretation(
            snippet.to_string(),
            None,
            true,
            Some(vec![&mut debugger]),
        ) {
            Ok((mut output, result)) => {
                if let EvaluationResult::Contract(contract_result) = result.result {
                    let snippet = format!("→ .{} contract successfully stored. Use (contract-call? ...) for invoking the public functions:", contract_result.contract.contract_identifier.clone());
                    output.push(snippet.green().to_string());
                };
                output
            }
            Err((result, _)) => result,
        };
        output.append(&mut result);
    }

    #[cfg(feature = "cli")]
    pub fn trace(&mut self, output: &mut Vec<String>, cmd: &str) {
        use super::tracer::Tracer;

        let snippet = match cmd.split_once(' ') {
            Some((_, snippet)) => snippet,
            _ => return output.push("Usage: ::trace <expr>".red().to_string()),
        };

        let mut tracer = Tracer::new(snippet.to_string());

        match self.eval_with_hooks(snippet.to_string(), Some(vec![&mut tracer]), false) {
            Ok(_) => (),
            Err(diagnostics) => {
                let lines = snippet.lines();
                let formatted_lines: Vec<String> = lines.map(|l| l.to_string()).collect();
                for d in diagnostics {
                    output.append(&mut output_diagnostic(&d, "<snippet>", &formatted_lines));
                }
            }
        };
    }

    #[cfg(feature = "cli")]
    pub fn start(&mut self) -> Result<(String, Vec<(ContractAnalysis, String, String)>), String> {
        let mut output_err = Vec::<String>::new();
        let output = Vec::<String>::new();
        let contracts = vec![];

        if !self.settings.include_boot_contracts.is_empty() {
            self.load_boot_contracts();
        }

        if !self.settings.initial_accounts.is_empty() {
            let mut initial_accounts = self.settings.initial_accounts.clone();
            for account in initial_accounts.drain(..) {
                let recipient = match PrincipalData::parse(&account.address) {
                    Ok(recipient) => recipient,
                    _ => {
                        output_err.push("Unable to parse address to credit".red().to_string());
                        continue;
                    }
                };

                match self
                    .interpreter
                    .mint_stx_balance(recipient, account.balance)
                {
                    Ok(_) => {}
                    Err(err) => output_err.push(err.red().to_string()),
                };
            }
        }

        match output_err.len() {
            0 => Ok((output.join("\n"), contracts)),
            _ => Err(output_err.join("\n")),
        }
    }

    #[cfg(feature = "cli")]
    pub fn read(&mut self, output: &mut Vec<String>, cmd: &str) {
        let filename = match cmd.split_once(' ') {
            Some((_, filename)) => filename,
            _ => return output.push("Usage: ::read <filename>".red().to_string()),
        };

        match std::fs::read_to_string(filename) {
            Ok(snippet) => {
                let _ = self.run_snippet(output, self.show_costs, &snippet);
            }
            Err(err) => output.push(
                format!("unable to read {}: {}", filename, err)
                    .red()
                    .to_string(),
            ),
        };
    }

    pub fn stx_transfer(
        &mut self,
        amount: u64,
        recipient: &str,
    ) -> Result<ExecutionResult, Vec<Diagnostic>> {
        let snippet = format!("(stx-transfer? u{} tx-sender '{})", amount, recipient);
        self.eval(snippet.clone(), false)
    }

    pub fn deploy_contract(
        &mut self,
        contract: &ClarityContract,
        cost_track: bool,
        ast: Option<&ContractAST>,
    ) -> Result<ExecutionResult, Vec<Diagnostic>> {
        if contract.epoch != self.current_epoch {
            let diagnostic = Diagnostic {
                level: Level::Error,
                message: format!(
                    "contract epoch ({}) does not match current epoch ({})",
                    contract.epoch, self.current_epoch
                ),
                spans: vec![],
                suggestion: None,
            };
            return Err(vec![diagnostic]);
        }

        let mut hooks: Vec<&mut dyn EvalHook> = vec![];
        if let Some(ref mut coverage_hook) = self.coverage_hook {
            hooks.push(coverage_hook);
        }

        if contract.clarity_version > ClarityVersion::default_for_epoch(contract.epoch) {
            let diagnostic = Diagnostic {
                level: Level::Error,
                message: format!(
                    "{} can not be used with {}",
                    contract.clarity_version, contract.epoch
                ),
                spans: vec![],
                suggestion: None,
            };
            return Err(vec![diagnostic]);
        }

        let contract_id =
            contract.expect_resolved_contract_identifier(Some(&self.interpreter.get_tx_sender()));

        let result = self.interpreter.run(contract, ast, cost_track, Some(hooks));

        result.inspect(|result| {
            if let EvaluationResult::Contract(contract_result) = &result.result {
                self.contracts
                    .insert(contract_id.clone(), contract_result.contract.clone());
            }
        })
    }

    pub fn call_contract_fn(
        &mut self,
        contract: &str,
        method: &str,
        args: &[SymbolicExpression],
        sender: &str,
        allow_private: bool,
        track_costs: bool,
    ) -> Result<ExecutionResult, Vec<Diagnostic>> {
        let initial_tx_sender = self.get_tx_sender();

        // Handle fully qualified contract_id and sugared syntax
        let contract_id_str = if contract.starts_with('S') {
            contract.to_string()
        } else {
            format!("{}.{}", initial_tx_sender, contract)
        };

        self.set_tx_sender(sender);

        let mut hooks: Vec<&mut dyn EvalHook> = vec![];
        if let Some(ref mut coverage_hook) = self.coverage_hook {
            hooks.push(coverage_hook);
        }

        let execution = match self.interpreter.call_contract_fn(
            &QualifiedContractIdentifier::parse(&contract_id_str).unwrap(),
            method,
            args,
            self.current_epoch,
            ClarityVersion::default_for_epoch(self.current_epoch),
            track_costs,
            allow_private,
            hooks,
        ) {
            Ok(result) => result,
            Err(e) => {
                self.set_tx_sender(&initial_tx_sender);
                return Err(vec![Diagnostic {
                    level: Level::Error,
                    message: format!("Error calling contract function: {e}"),
                    spans: vec![],
                    suggestion: None,
                }]);
            }
        };
        self.set_tx_sender(&initial_tx_sender);

        Ok(execution)
    }

    pub fn eval(
        &mut self,
        snippet: String,
        cost_track: bool,
    ) -> Result<ExecutionResult, Vec<Diagnostic>> {
        let contract = ClarityContract {
            code_source: ClarityCodeSource::ContractInMemory(snippet),
            name: format!("contract-{}", self.contracts.len()),
            deployer: ContractDeployer::DefaultDeployer,
            clarity_version: ClarityVersion::default_for_epoch(self.current_epoch),
            epoch: self.current_epoch,
        };
        let contract_identifier =
            contract.expect_resolved_contract_identifier(Some(&self.interpreter.get_tx_sender()));

        let mut hooks: Vec<&mut dyn EvalHook> = vec![];
        if let Some(ref mut coverage_hook) = self.coverage_hook {
            hooks.push(coverage_hook);
        }

        let result = self
            .interpreter
            .run(&contract.clone(), None, cost_track, Some(hooks));

        match result {
            Ok(result) => {
                if let EvaluationResult::Contract(contract_result) = &result.result {
                    self.contracts.insert(
                        contract_identifier.clone(),
                        contract_result.contract.clone(),
                    );
                };
                Ok(result)
            }
            Err(res) => Err(res),
        }
    }

    pub fn eval_with_hooks(
        &mut self,
        snippet: String,
        eval_hooks: Option<Vec<&mut dyn EvalHook>>,
        cost_track: bool,
    ) -> Result<ExecutionResult, Vec<Diagnostic>> {
        let contract = ClarityContract {
            code_source: ClarityCodeSource::ContractInMemory(snippet),
            name: format!("contract-{}", self.contracts.len()),
            deployer: ContractDeployer::DefaultDeployer,
            clarity_version: ClarityVersion::default_for_epoch(self.current_epoch),
            epoch: self.current_epoch,
        };
        let contract_identifier =
            contract.expect_resolved_contract_identifier(Some(&self.interpreter.get_tx_sender()));

        let result = self
            .interpreter
            .run(&contract.clone(), None, cost_track, eval_hooks);

        match result {
            Ok(result) => {
                if let EvaluationResult::Contract(contract_result) = &result.result {
                    self.contracts.insert(
                        contract_identifier.clone(),
                        contract_result.contract.clone(),
                    );
                };
                Ok(result)
            }
            Err(res) => Err(res),
        }
    }

    pub fn lookup_functions_or_keywords_docs(&self, exp: &str) -> Option<&String> {
        if let Some(function_doc) = self.api_reference.get(exp) {
            return Some(function_doc);
        }

        self.keywords_reference.get(exp)
    }

    pub fn get_api_reference_index(&self) -> Vec<String> {
        let mut keys = self
            .api_reference
            .keys()
            .map(|k| k.to_string())
            .collect::<Vec<String>>();
        keys.sort();
        keys
    }

    pub fn get_clarity_keywords(&self) -> Vec<String> {
        let mut keys = self
            .keywords_reference
            .keys()
            .map(|k| k.to_string())
            .collect::<Vec<String>>();
        keys.sort();
        keys
    }

    fn display_help(&self) -> String {
        let mut output: Vec<String> = vec![];

        #[cfg(feature = "cli")]
        output.push(format!(
            "{}",
            "::functions\t\t\t\tDisplay all the native functions available in clarity".yellow()
        ));
        #[cfg(feature = "cli")]
        output.push(format!(
            "{}",
            "::keywords\t\t\t\tDisplay all the native keywords available in clarity".yellow()
        ));
        #[cfg(feature = "cli")]
        output.push(format!(
            "{}",
                "::describe <function> | <keyword>\tDisplay documentation for a given native function or keyword".yellow()
        ));
        #[cfg(feature = "cli")]
        output.push(format!(
            "{}",
            "::toggle_costs\t\t\t\tDisplay cost analysis after every expression".yellow()
        ));
        #[cfg(feature = "cli")]
        output.push(format!(
            "{}",
            "::toggle_timings\t\t\tDisplay the execution duration".yellow()
        ));

        output.push(format!(
            "{}",
            "::mint_stx <principal> <amount>\t\tMint STX balance for a given principal".yellow()
        ));
        output.push(format!(
            "{}",
            "::set_tx_sender <principal>\t\tSet tx-sender variable to principal".yellow()
        ));
        output.push(format!(
            "{}",
            "::get_assets_maps\t\t\tGet assets maps for active accounts".yellow()
        ));
        output.push(format!(
            "{}",
            "::get_contracts\t\t\t\tGet contracts".yellow()
        ));
        output.push(format!(
            "{}",
            "::get_block_height\t\t\tGet current block height".yellow()
        ));
        output.push(format!(
            "{}",
            "::advance_chain_tip <count>\t\tSimulate mining of <count> blocks".yellow()
        ));
        output.push(format!(
            "{}",
            "::advance_stacks_chain_tip <count>\tSimulate mining of <count> stacks blocks".yellow()
        ));
        output.push(format!(
            "{}",
            "::advance_burn_chain_tip <count>\tSimulate mining of <count> burnchain blocks"
                .yellow()
        ));
        output.push(format!(
            "{}",
            "::set_epoch <epoch>\t\t\tUpdate the current epoch".yellow()
        ));
        output.push(format!(
            "{}",
            "::get_epoch\t\t\t\tGet current epoch".yellow()
        ));

        #[cfg(feature = "cli")]
        output.push(format!(
            "{}",
            "::debug <expr>\t\t\t\tStart an interactive debug session executing <expr>".yellow()
        ));
        #[cfg(feature = "cli")]
        output.push(format!(
            "{}",
            "::trace <expr>\t\t\t\tGenerate an execution trace for <expr>".yellow()
        ));
        #[cfg(feature = "cli")]
        output.push(format!(
            "{}",
            "::get_costs <expr>\t\t\tDisplay the cost analysis".yellow()
        ));
        #[cfg(feature = "cli")]
        output.push(format!(
            "{}",
            "::reload \t\t\t\tReload the existing contract(s) in the session".yellow()
        ));
        #[cfg(feature = "cli")]
        output.push(format!(
            "{}",
            "::read <filename>\t\t\tRead expressions from a file".yellow()
        ));

        output.push(format!(
            "{}",
            "::encode <expr>\t\t\t\tEncode an expression to a Clarity Value bytes representation"
                .yellow()
        ));
        output.push(format!(
            "{}",
            "::decode <bytes>\t\t\tDecode a Clarity Value bytes representation".yellow()
        ));

        output.join("\n")
    }

    fn parse_and_advance_chain_tip(&mut self, command: &str) -> String {
        let args: Vec<_> = command.split(' ').skip(1).collect();
        let count = match args.first().unwrap_or(&"1").parse::<u32>() {
            Ok(count) => count,
            _ => return format!("{}", "Unable to parse count".red()),
        };

        let _ = self.advance_chain_tip(count);
        format!(
            "new burn height: {}\nnew stacks height: {}",
            self.interpreter.datastore.get_current_burn_block_height(),
            self.interpreter.datastore.get_current_stacks_block_height(),
        )
        .green()
        .to_string()
    }

    fn parse_and_advance_burn_chain_tip(&mut self, command: &str) -> String {
        let args: Vec<_> = command.split(' ').skip(1).collect();
        let count = match args.first().unwrap_or(&"1").parse::<u32>() {
            Ok(count) => count,
            _ => return format!("{}", "Unable to parse count".red()),
        };

        let _ = self.advance_burn_chain_tip(count);
        format!(
            "new burn height: {}\nnew stacks height: {}",
            self.interpreter.datastore.get_current_burn_block_height(),
            self.interpreter.datastore.get_current_stacks_block_height(),
        )
        .green()
        .to_string()
    }

    fn parse_and_advance_stacks_chain_tip(&mut self, command: &str) -> String {
        let args: Vec<_> = command.split(' ').skip(1).collect();
        let count = match args.first().unwrap_or(&"1").parse::<u32>() {
            Ok(count) => count,
            _ => return format!("{}", "Unable to parse count".red()),
        };

        match self.advance_stacks_chain_tip(count) {
            Ok(_) => format!(
                "new burn height: {}\nnew stacks height: {}",
                self.interpreter.datastore.get_current_burn_block_height(),
                self.interpreter.datastore.get_current_stacks_block_height(),
            )
            .green()
            .to_string(),
            Err(_) => format!(
                "{}",
                "advance_stacks_chain_tip can't be called in epoch lower than 3.0".red()
            ),
        }
    }

    pub fn advance_chain_tip(&mut self, count: u32) -> u32 {
        let current_epoch = self.interpreter.datastore.get_current_epoch();
        if current_epoch < StacksEpochId::Epoch30 {
            self.advance_burn_chain_tip(count)
        } else {
            match self.advance_stacks_chain_tip(count) {
                Ok(count) => count,
                Err(_) => unreachable!("Epoch checked already"),
            }
        }
    }

    pub fn advance_burn_chain_tip(&mut self, count: u32) -> u32 {
        self.interpreter.advance_burn_chain_tip(count)
    }

    pub fn advance_stacks_chain_tip(&mut self, count: u32) -> Result<u32, String> {
        self.interpreter.advance_stacks_chain_tip(count)
    }

    fn parse_and_set_tx_sender(&mut self, command: &str) -> String {
        let args: Vec<_> = command.split(' ').collect();

        if args.len() != 2 {
            return format!("{}", "Usage: ::set_tx_sender <address>".red());
        }

        let tx_sender = args[1];

        match PrincipalData::parse_standard_principal(args[1]) {
            Ok(address) => {
                self.interpreter.set_tx_sender(address);
                format!("tx-sender switched to {}", tx_sender)
            }
            _ => format!("{}", "Unable to parse the address".red()),
        }
    }

    pub fn set_tx_sender(&mut self, address: &str) {
        let tx_sender =
            PrincipalData::parse_standard_principal(address).expect("Unable to parse address");
        self.interpreter.set_tx_sender(tx_sender)
    }

    pub fn get_tx_sender(&self) -> String {
        self.interpreter.get_tx_sender().to_address()
    }

    fn get_block_height(&mut self) -> String {
        let height = self.interpreter.get_block_height();
        format!("Current height: {}", height)
    }

    fn get_burn_block_height(&mut self) -> String {
        let height = self.interpreter.get_burn_block_height();
        format!("Current height: {}", height)
    }

    fn get_account_name(&self, address: &String) -> Option<&String> {
        for account in self.settings.initial_accounts.iter() {
            if &account.address == address {
                return Some(&account.name);
            }
        }
        None
    }

    pub fn get_assets_maps(&self) -> BTreeMap<String, BTreeMap<String, u128>> {
        self.interpreter.get_assets_maps()
    }

    pub fn toggle_costs(&mut self) -> String {
        self.show_costs = !self.show_costs;
        format!("Always show costs: {}", self.show_costs)
    }

    pub fn toggle_timings(&mut self) -> String {
        self.interpreter.repl_settings.show_timings = !self.interpreter.repl_settings.show_timings;
        format!(
            "Always show timings: {}",
            self.interpreter.repl_settings.show_timings
        )
        .green()
        .to_string()
    }

    pub fn get_epoch(&mut self) -> String {
        format!("Current epoch: {}", self.current_epoch)
    }

    pub fn set_epoch(&mut self, cmd: &str) -> String {
        let epoch = match cmd.split_once(' ').map(|(_, epoch)| epoch) {
            Some("2.0") => StacksEpochId::Epoch20,
            Some("2.05") => StacksEpochId::Epoch2_05,
            Some("2.1") => StacksEpochId::Epoch21,
            Some("2.2") => StacksEpochId::Epoch22,
            Some("2.3") => StacksEpochId::Epoch23,
            Some("2.4") => StacksEpochId::Epoch24,
            Some("2.5") => StacksEpochId::Epoch25,
            Some("3.0") => StacksEpochId::Epoch30,
            _ => {
                return "Usage: ::set_epoch 2.0 | 2.05 | 2.1 | 2.2 | 2.3 | 2.4 | 2.5 | 3.0"
                    .red()
                    .to_string()
            }
        };
        self.update_epoch(epoch);
        format!("Epoch updated to: {epoch}").green().to_string()
    }

    pub fn update_epoch(&mut self, epoch: StacksEpochId) {
        self.current_epoch = epoch;
        self.interpreter.set_current_epoch(epoch);
        if epoch >= StacksEpochId::Epoch30 {
            self.interpreter.set_tenure_height();
        }
    }

    pub fn encode(&mut self, cmd: &str) -> String {
        let snippet = match cmd.split_once(' ') {
            Some((_, snippet)) => snippet,
            _ => return "Usage: ::encode <expr>".red().to_string(),
        };

        let result = self.eval(snippet.to_string(), false);
        match result {
            Ok(result) => {
                let mut tx_bytes = vec![];
                let value = match result.result {
                    EvaluationResult::Contract(contract_result) => {
                        if let Some(value) = contract_result.result {
                            value
                        } else {
                            return "No value".to_string();
                        }
                    }
                    EvaluationResult::Snippet(snippet_result) => snippet_result.result,
                };
                if let Err(e) = value.consensus_serialize(&mut tx_bytes) {
                    return format!("{}", e).red().to_string();
                };
                let mut s = String::with_capacity(2 * tx_bytes.len());
                for byte in tx_bytes {
                    s = format!("{}{:02x}", s, byte);
                }
                s.green().to_string()
            }
            Err(diagnostics) => {
                let lines: Vec<String> = snippet.split('\n').map(|s| s.to_string()).collect();
                let mut output: Vec<String> = diagnostics
                    .iter()
                    .flat_map(|d| output_diagnostic(d, "encode", &lines))
                    .collect();
                output.push("encoding failed".into());
                output.join("\n")
            }
        }
    }

    pub fn decode(&mut self, cmd: &str) -> String {
        let byte_string = match cmd.split_once(' ') {
            Some((_, bytes)) => bytes,
            _ => return "Usage: ::decode <hex-bytes>".red().to_string(),
        };
        let tx_bytes = match decode_hex(byte_string) {
            Ok(tx_bytes) => tx_bytes,
            Err(e) => return format!("Parsing error: {}", e).red().to_string(),
        };

        let value = match Value::consensus_deserialize(&mut &tx_bytes[..]) {
            Ok(value) => value,
            Err(e) => return format!("{}", e).red().to_string(),
        };

        format!("{}", value_to_string(&value).green())
    }

    #[cfg(feature = "cli")]
    pub fn get_costs(&mut self, output: &mut Vec<String>, cmd: &str) {
        let expr = match cmd.split_once(' ') {
            Some((_, expr)) => expr,
            _ => return output.push("Usage: ::get_costs <expr>".red().to_string()),
        };

        let _ = self.run_snippet(output, true, expr);
    }

    pub fn get_accounts(&self) -> Option<String> {
        let accounts = self.interpreter.get_accounts();
        if accounts.is_empty() {
            return None;
        }

        let tokens = self.interpreter.get_tokens();
        let mut headers = vec!["Address".to_string()];
        for token in tokens.iter() {
            if token == "STX" {
                headers.push(String::from("uSTX"));
            } else {
                headers.push(String::from(token));
            }
        }

        let mut headers_cells = vec![];
        for header in headers.iter() {
            headers_cells.push(Cell::new(header));
        }
        let mut table = Table::new();
        table.add_row(Row::new(headers_cells));
        for account in accounts.iter() {
            let mut cells = vec![];

            if let Some(name) = self.get_account_name(account) {
                cells.push(Cell::new(&format!("{} ({})", account, name)));
            } else {
                cells.push(Cell::new(account));
            }

            for token in tokens.iter() {
                let balance = self.interpreter.get_balance_for_account(account, token);
                cells.push(Cell::new(&format!("{}", balance)));
            }
            table.add_row(Row::new(cells));
        }
        Some(format!("{}", table))
    }

    #[cfg(feature = "cli")]
    pub fn get_contracts(&self) -> Option<String> {
        if self.contracts.is_empty() {
            return None;
        }

        let mut table = Table::new();
        table.add_row(row!["Contract identifier", "Public functions"]);
        let contracts = self.contracts.clone();
        for (contract_id, contract) in contracts.iter() {
            let contract_id_str = contract_id.to_string();
            if !contract_id_str.starts_with(BOOT_TESTNET_ADDRESS)
                && !contract_id_str.starts_with(BOOT_MAINNET_ADDRESS)
            {
                let mut formatted_methods = vec![];
                for (method_name, method_args) in contract.function_args.iter() {
                    let formatted_args = if method_args.is_empty() {
                        String::new()
                    } else if method_args.len() == 1 {
                        format!(" {}", method_args.join(" "))
                    } else {
                        format!("\n    {}", method_args.join("\n    "))
                    };
                    formatted_methods.push(format!("({}{})", method_name, formatted_args));
                }
                let formatted_spec = formatted_methods.join("\n").to_string();
                table.add_row(Row::new(vec![
                    Cell::new(&contract_id_str),
                    Cell::new(&formatted_spec),
                ]));
            }
        }
        Some(format!("{}", table))
    }

    #[cfg(not(feature = "cli"))]
    fn get_contracts(&self) -> Option<String> {
        if self.contracts.is_empty() {
            return None;
        }
        Some(
            self.contracts
                .keys()
                .map(|contract_id| contract_id.to_string())
                .collect::<Vec<String>>()
                .join("\n"),
        )
    }

    fn mint_stx(&mut self, command: &str) -> String {
        let args: Vec<_> = command.split(' ').collect();

        if args.len() != 3 {
            return "Usage: ::mint_stx <recipient address> <amount>"
                .red()
                .to_string();
        }

        let recipient = match PrincipalData::parse(args[1]) {
            Ok(address) => address,
            _ => return "Unable to parse the address".red().to_string(),
        };

        let amount: u64 = match args[2].parse() {
            Ok(recipient) => recipient,
            _ => return "Unable to parse the balance".red().to_string(),
        };

        match self.interpreter.mint_stx_balance(recipient, amount) {
            Ok(msg) => msg.green().to_string(),
            Err(err) => err.red().to_string(),
        }
    }

    #[cfg(feature = "cli")]
    fn display_functions(&self) -> String {
        let api_reference_index = self.get_api_reference_index();
        format!("{}", api_reference_index.join("\n").yellow())
    }

    #[cfg(feature = "cli")]
    fn display_doc(&self, command: &str) -> String {
        let keyword = {
            let mut s = command.to_string();
            s = s.replace("::describe", "");
            s = s.replace(' ', "");
            s
        };

        match self.lookup_functions_or_keywords_docs(&keyword) {
            Some(doc) => format!("{}", doc.yellow()),
            None => format!(
                "{}",
                "It looks like there aren't matches for your search".red()
            ),
        }
    }

    #[cfg(feature = "cli")]
    fn keywords(&self) -> String {
        let keywords = self.get_clarity_keywords();
        format!("{}", keywords.join("\n").yellow())
    }
}

#[derive(Debug, PartialEq)]
enum DecodeHexError {
    ParseError(ParseIntError),
    OddLength,
}

impl fmt::Display for DecodeHexError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DecodeHexError::ParseError(e) => write!(f, "{}", e),
            DecodeHexError::OddLength => write!(f, "odd number of hex digits"),
        }
    }
}

impl From<ParseIntError> for DecodeHexError {
    fn from(err: ParseIntError) -> Self {
        DecodeHexError::ParseError(err)
    }
}

fn decode_hex(byte_string: &str) -> Result<Vec<u8>, DecodeHexError> {
    let byte_string_filtered: String = byte_string
        .strip_prefix("0x")
        .unwrap_or(byte_string)
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    if byte_string_filtered.len() % 2 != 0 {
        return Err(DecodeHexError::OddLength);
    }
    let result: Result<Vec<u8>, ParseIntError> = (0..byte_string_filtered.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&byte_string_filtered[i..i + 2], 16))
        .collect();
    match result {
        Ok(result) => Ok(result),
        Err(e) => Err(DecodeHexError::ParseError(e)),
    }
}

fn build_api_reference() -> HashMap<String, String> {
    let mut api_reference = HashMap::new();
    for func in NativeFunctions::ALL.iter() {
        let api = make_api_reference(func);
        let description = {
            let mut s = api.description.to_string();
            s = s.replace('\n', " ");
            s
        };
        let doc = format!(
            "Usage\n{}\n\nDescription\n{}\n\nExamples\n{}",
            api.signature, description, api.example
        );
        api_reference.insert(api.name, doc);
    }

    for func in DefineFunctions::ALL.iter() {
        let api = make_define_reference(func);
        let description = {
            let mut s = api.description.to_string();
            s = s.replace('\n', " ");
            s
        };
        let doc = format!(
            "Usage\n{}\n\nDescription\n{}\n\nExamples\n{}",
            api.signature, description, api.example
        );
        api_reference.insert(api.name, doc);
    }

    api_reference
}

fn clarity_keywords() -> HashMap<String, String> {
    let mut keywords = HashMap::new();

    for func in NativeVariables::ALL.iter() {
        if let Some(key) = make_keyword_reference(func) {
            let description = {
                let mut s = key.description.to_string();
                s = s.replace('\n', " ");
                s
            };
            let doc = format!("Description\n{}\n\nExamples\n{}", description, key.example);
            keywords.insert(key.name.to_string(), doc);
        }
    }

    keywords
}

#[allow(clippy::items_after_test_module)]
#[cfg(test)]
mod tests {
    use clarity::vm::types::TupleData;

    use super::*;
    use crate::{
        repl::{settings::Account, DEFAULT_EPOCH},
        test_fixtures::clarity_contract::ClarityContractBuilder,
    };

    #[track_caller]
    fn run_session_snippet(session: &mut Session, snippet: &str) -> Value {
        let execution_res = session.eval(snippet.to_string(), false).unwrap();
        let res = match execution_res.result {
            EvaluationResult::Contract(_) => unreachable!(),
            EvaluationResult::Snippet(res) => res,
        };
        res.result
    }

    #[track_caller]
    fn assert_execution_result_value(
        result: &Result<ExecutionResult, Vec<Diagnostic>>,
        expected_value: Value,
    ) {
        assert!(result.is_ok());
        let result = result.as_ref().unwrap();
        let result = match &result.result {
            EvaluationResult::Contract(_) => unreachable!(),
            EvaluationResult::Snippet(res) => res,
        };
        assert_eq!(result.result, expected_value);
    }

    #[test]
    fn initial_accounts() {
        let address = "ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5";
        let mut session = Session::new(SessionSettings {
            initial_accounts: vec![Account {
                address: address.to_owned(),
                balance: 1000000,
                name: "wallet_1".to_owned(),
            }],
            ..Default::default()
        });
        let _ = session.start();
        let balance = session.interpreter.get_balance_for_account(address, "STX");
        assert_eq!(balance, 1000000);
    }

    #[test]
    fn epoch_switch() {
        let mut session = Session::new(SessionSettings::default());
        session.update_epoch(StacksEpochId::Epoch20);
        let diags = session
            .eval("(slice? \"blockstack\" u5 u10)".into(), false)
            .unwrap_err();
        assert_eq!(
            diags[0].message,
            format!("use of unresolved function 'slice?'",)
        );
        session.update_epoch(StacksEpochId::Epoch21);
        let res = session
            .eval("(slice? \"blockstack\" u5 u10)".into(), false)
            .unwrap();
        let res = match res.result {
            EvaluationResult::Contract(_) => unreachable!(),
            EvaluationResult::Snippet(res) => res,
        };
        assert_eq!(
            res.result,
            Value::some(Value::string_ascii_from_bytes("stack".as_bytes().to_vec()).unwrap())
                .unwrap()
        );
    }

    #[test]
    fn test_parse_and_advance_stacks_chain_tip() {
        let mut session = Session::new(SessionSettings::default());
        let result = session.handle_command("::advance_stacks_chain_tip 1");
        assert_eq!(
            result,
            "advance_stacks_chain_tip can't be called in epoch lower than 3.0"
                .to_string()
                .red()
                .to_string()
        );
        session.handle_command("::set_epoch 3.0");
        let _ = session.handle_command("::advance_stacks_chain_tip 1");
        let new_height = session.handle_command("::get_stacks_block_height");
        assert_eq!(new_height, "Current height: 2");
    }

    #[test]
    fn test_parse_and_advance_burn_chain_tip_pre_epoch3() {
        let mut session = Session::new(SessionSettings::default());
        let result = session.handle_command("::advance_burn_chain_tip 1");
        assert_eq!(
            result,
            "new burn height: 1\nnew stacks height: 1"
                .to_string()
                .green()
                .to_string()
        );
        // before epoch 3 this acts the same as burn_chain_tip
        let result = session.handle_command("::advance_chain_tip 1");
        assert_eq!(
            result,
            "new burn height: 2\nnew stacks height: 2"
                .to_string()
                .green()
                .to_string()
        );
    }

    #[test]
    fn test_parse_and_advance_burn_chain_tip_epoch3() {
        let mut session = Session::new(SessionSettings::default());
        session.handle_command("::set_epoch 3.0");
        let result = session.handle_command("::advance_burn_chain_tip 1");
        assert_eq!(
            result,
            "new burn height: 2\nnew stacks height: 2"
                .to_string()
                .green()
                .to_string()
        );
        let new_height = session.handle_command("::get_stacks_block_height");
        assert_eq!(new_height, "Current height: 2");
        // advance_chain_tip will only affect stacks height in epoch 3 or greater
        let _ = session.handle_command("::advance_chain_tip 1");
        let new_height = session.handle_command("::get_stacks_block_height");
        assert_eq!(new_height, "Current height: 3");
        let new_height = session.handle_command("::get_burn_block_height");
        assert_eq!(new_height, "Current height: 2");
    }

    #[test]
    fn set_epoch_command() {
        let mut session = Session::new(SessionSettings::default());
        let initial_block_height = session.interpreter.get_block_height();
        let initial_epoch = session.handle_command("::get_epoch");
        // initial epoch is 2.05
        assert_eq!(initial_epoch, "Current epoch: 2.05");

        // it can be lowered to 2.0
        // it's possible that in the feature we want to start from 2.0 and forbid lowering the epoch
        // this test would have to be updated
        session.handle_command("::set_epoch 2.0");
        let current_epoch = session.handle_command("::get_epoch");
        assert_eq!(current_epoch, "Current epoch: 2.0");

        session.handle_command("::set_epoch 2.4");
        let current_epoch = session.handle_command("::get_epoch");
        assert_eq!(current_epoch, "Current epoch: 2.4");

        // changing epoch in 2.x does not impact the block height
        assert_eq!(session.interpreter.get_block_height(), initial_block_height);

        session.handle_command("::set_epoch 3.0");
        let current_epoch = session.handle_command("::get_epoch");
        assert_eq!(current_epoch, "Current epoch: 3.0");

        // changing epoch in 3.x increments the block height
        assert_eq!(
            session.interpreter.get_block_height(),
            initial_block_height + 1
        );
    }

    #[test]
    fn encode_error() {
        let mut session = Session::new(SessionSettings::default());
        let result = session.encode("::encode { foo false }");
        assert_eq!(
            result,
            format_err!("Tuple literal construction expects a colon at index 1\nencoding failed")
        );

        let result = session.encode("::encode (foo 1)");
        assert_eq!(
            result.split('\n').next().unwrap(),
            format!(
                "encode:1:1: {} use of unresolved function 'foo'",
                "error:".red()
            )
        );
    }

    #[test]
    fn decode_simple() {
        let mut session = Session::new(SessionSettings::default());

        let result = session.decode("::decode 0000000000000000 0000000000000000 2a");
        assert_eq!(result, "42".green().to_string());
    }

    #[test]
    fn decode_map() {
        let mut session = Session::new(SessionSettings::default());
        let result = session.decode("::decode 0x0c00000002036261720403666f6f0d0000000568656c6c6f");
        assert_eq!(result, "{ bar: false, foo: \"hello\" }".green().to_string());
    }

    #[test]
    fn decode_error() {
        let mut session = Session::new(SessionSettings::default());
        let result = session.decode("::decode 42");
        assert_eq!(
            result,
            "Failed to decode clarity value: DeserializationError(\"Bad type prefix\")"
                .red()
                .to_string()
        );

        let result = session.decode("::decode 4g");
        assert_eq!(
            result,
            "Parsing error: invalid digit found in string"
                .red()
                .to_string()
        );
    }

    #[test]
    fn clarity_epoch_mismatch() {
        let settings = SessionSettings::default();
        let mut session = Session::new(settings);
        let snippet = "(define-data-var x uint u0)";

        // can not use ClarityContractBuilder to build an invalid contract
        let contract = ClarityContract {
            code_source: ClarityCodeSource::ContractInMemory(snippet.to_string()),
            name: "should_error".to_string(),
            deployer: ContractDeployer::Address("ST000000000000000000002AMW42H".into()),
            clarity_version: ClarityVersion::Clarity2,
            epoch: StacksEpochId::Epoch2_05,
        };

        let result = session.deploy_contract(&contract, false, None);
        assert!(result.is_err(), "Expected error for clarity mismatch");
    }

    #[test]
    fn deploy_contract_with_wrong_epoch() {
        let settings = SessionSettings::default();
        let mut session = Session::new(settings);

        session.update_epoch(StacksEpochId::Epoch24);

        let snippet = "(define-data-var x uint u0)";
        let contract = ClarityContractBuilder::new()
            .code_source(snippet.into())
            .epoch(StacksEpochId::Epoch25)
            .clarity_version(ClarityVersion::Clarity2)
            .build();

        let result = session.deploy_contract(&contract, false, None);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.len() == 1);
        assert_eq!(
            err.first().unwrap().message,
            "contract epoch (2.5) does not match current epoch (2.4)"
        );
    }

    #[test]
    fn evaluate_at_block() {
        let settings = SessionSettings {
            include_boot_contracts: vec!["costs".into(), "costs-2".into(), "costs-3".into()],
            ..Default::default()
        };

        let mut session = Session::new(settings);
        session.start().expect("session could not start");

        session.handle_command("::set_epoch 2.5");

        // setup contract state
        let snippet = "
            (define-data-var x uint u0)
            (define-read-only (get-x)
                (var-get x))
            (define-public (incr)
                (begin
                    (var-set x (+ (var-get x) u1))
                    (ok (var-get x))))";

        let contract = ClarityContract {
            code_source: ClarityCodeSource::ContractInMemory(snippet.to_string()),
            name: "contract".to_string(),
            deployer: ContractDeployer::Address("ST000000000000000000002AMW42H".into()),
            clarity_version: ClarityVersion::Clarity2,
            epoch: StacksEpochId::Epoch25,
        };

        let _ = session.deploy_contract(&contract, false, None);

        // assert data-var is set to 0
        assert_eq!(
            session
                .process_console_input("(contract-call? .contract get-x)")
                .1[0],
            "u0".green().to_string()
        );

        // advance chain tip and test at-block
        let _ = session.advance_chain_tip(10000);
        assert_eq!(
            session
                .process_console_input("(contract-call? .contract get-x)")
                .1[0],
            "u0".green().to_string()
        );
        session.process_console_input("(contract-call? .contract incr)");
        assert_eq!(
            session
                .process_console_input("(contract-call? .contract get-x)")
                .1[0],
            "u1".green().to_string()
        );
        assert_eq!(session.process_console_input("(at-block (unwrap-panic (get-block-info? id-header-hash u0)) (contract-call? .contract get-x))").1[0], "u0".green().to_string());
        assert_eq!(session.process_console_input("(at-block (unwrap-panic (get-block-info? id-header-hash u5000)) (contract-call? .contract get-x))").1[0], "u0".green().to_string());

        // advance chain tip again and test at-block
        // do this twice to make sure that the lookup table is being updated properly
        session.advance_chain_tip(10);
        session.advance_chain_tip(10);

        assert_eq!(
            session
                .process_console_input("(contract-call? .contract get-x)")
                .1[0],
            "u1".green().to_string()
        );
        session.process_console_input("(contract-call? .contract incr)");
        assert_eq!(
            session
                .process_console_input("(contract-call? .contract get-x)")
                .1[0],
            "u2".green().to_string()
        );
        assert_eq!(session.process_console_input("(at-block (unwrap-panic (get-block-info? id-header-hash u10000)) (contract-call? .contract get-x))").1[0], "u1".green().to_string());
    }

    #[test]
    fn can_deploy_a_contract() {
        let settings = SessionSettings::default();
        let mut session = Session::new(settings);
        session.start().expect("session could not start");
        session.update_epoch(DEFAULT_EPOCH);

        // deploy default contract
        let contract = ClarityContractBuilder::default().build();
        let result = session.deploy_contract(&contract, false, None);
        assert!(result.is_ok());
    }

    #[test]
    fn can_call_boot_contract_fn() {
        let settings = SessionSettings {
            include_boot_contracts: vec!["pox-4".into()],
            ..Default::default()
        };
        let mut session = Session::new(settings);
        session.update_epoch(StacksEpochId::Epoch25);
        session.load_boot_contracts();

        // call pox4 get-info
        let result = session.call_contract_fn(
            format!("{}.pox-4", BOOT_MAINNET_ADDRESS).as_str(),
            "get-pox-info",
            &[],
            BOOT_TESTNET_ADDRESS,
            false,
            false,
        );
        assert_execution_result_value(
            &result,
            Value::okay(Value::Tuple(
                TupleData::from_data(vec![
                    ("min-amount-ustx".into(), Value::UInt(0)),
                    ("reward-cycle-id".into(), Value::UInt(0)),
                    ("prepare-cycle-length".into(), Value::UInt(50)),
                    ("first-burnchain-block-height".into(), Value::UInt(0)),
                    ("reward-cycle-length".into(), Value::UInt(1050)),
                    ("total-liquid-supply-ustx".into(), Value::UInt(0)),
                ])
                .unwrap(),
            ))
            .unwrap(),
        );
    }

    #[test]
    fn can_call_public_contract_fn() {
        let settings = SessionSettings::default();
        let mut session = Session::new(settings);
        session.start().expect("session could not start");
        session.update_epoch(DEFAULT_EPOCH);

        // deploy default contract
        let contract = ClarityContractBuilder::default().build();
        let _ = session.deploy_contract(&contract, false, None);

        dbg!(&contract);

        let result = session.call_contract_fn(
            "contract",
            "incr",
            &[],
            &session.get_tx_sender(),
            false,
            false,
        );
        assert_execution_result_value(&result, Value::okay(Value::UInt(1)).unwrap());

        let result = session.call_contract_fn(
            "contract",
            "get-x",
            &[],
            &session.get_tx_sender(),
            false,
            false,
        );
        assert_execution_result_value(&result, Value::UInt(1));
    }

    #[test]
    fn current_block_info_is_none() {
        let settings = SessionSettings::default();
        let mut session = Session::new(settings);
        session.start().expect("session could not start");
        session.update_epoch(StacksEpochId::Epoch25);

        session.advance_chain_tip(5);
        let result = run_session_snippet(&mut session, "(get-block-info? time block-height)");
        assert_eq!(result, Value::none());
    }

    #[test]
    fn block_time_is_realistic_in_epoch_2_5() {
        let settings = SessionSettings::default();
        let mut session = Session::new(settings);
        session.start().expect("session could not start");
        session.update_epoch(StacksEpochId::Epoch25);

        session.advance_chain_tip(4);

        let result = run_session_snippet(&mut session, "(get-block-info? time u2)");
        let time_block_1 = match result.expect_optional() {
            Ok(Some(Value::UInt(time))) => time,
            _ => panic!("Unexpected result"),
        };

        let result = run_session_snippet(&mut session, "(get-block-info? time u3)");
        let time_block_2 = match result.expect_optional() {
            Ok(Some(Value::UInt(time))) => time,
            _ => panic!("Unexpected result"),
        };

        assert!(time_block_2 - time_block_1 == 600);
    }
}
#[cfg(test)]
mod logger_hook_tests {

    use crate::{repl::DEFAULT_EPOCH, test_fixtures::clarity_contract::ClarityContractBuilder};

    use super::*;

    #[test]
    fn can_retrieve_print_values() {
        let settings = SessionSettings::default();
        let mut session = Session::new(settings);
        session.start().expect("session could not start");
        session.update_epoch(DEFAULT_EPOCH);

        // session.deploy_contract(contract, eval_hooks, cost_track, test_name, ast)
        let snippet = [
            "(define-public (print-and-return (input (response uint uint)))",
            "  (begin",
            "    (match input x (print x) y (print y))",
            "    input",
            "  )",
            ")",
        ]
        .join("\n");

        let contract = ClarityContractBuilder::new().code_source(snippet).build();

        let _ = session.deploy_contract(&contract, false, None);
        let arg = SymbolicExpression::atom_value(Value::okay(Value::UInt(42)).unwrap());
        let res = session.call_contract_fn(
            "contract",
            "print-and-return",
            &[arg],
            &session.get_tx_sender(),
            false,
            false,
        );

        println!("{:?}", res);

        let arg = SymbolicExpression::atom_value(Value::error(Value::UInt(404)).unwrap());
        let res = session.call_contract_fn(
            "contract",
            "print-and-return",
            &[arg],
            &session.get_tx_sender(),
            false,
            false,
        );

        println!("{:?}", res);
    }
}

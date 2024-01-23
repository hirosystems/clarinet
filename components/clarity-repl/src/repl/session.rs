use super::boot::{STACKS_BOOT_CODE_MAINNET, STACKS_BOOT_CODE_TESTNET};
use super::diagnostic::output_diagnostic;
use super::{ClarityCodeSource, ClarityContract, ClarityInterpreter, ContractDeployer};
use crate::analysis::coverage::TestCoverageReport;
use crate::repl::Settings;
use crate::utils;
use ansi_term::Colour;
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
use clarity::vm::{ClarityVersion, CostSynthesis, EvalHook, EvaluationResult, ExecutionResult};
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::num::ParseIntError;

#[cfg(feature = "cli")]
use clarity::vm::analysis::ContractAnalysis;
#[cfg(feature = "cli")]
use prettytable::{Cell, Row, Table};
#[cfg(feature = "cli")]
use reqwest;

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
        let deploy: [(&StandardPrincipalData, [(&str, &str); 11]); 2] = [
            (&*BOOT_TESTNET_PRINCIPAL, *STACKS_BOOT_CODE_TESTNET),
            (&*BOOT_MAINNET_PRINCIPAL, *STACKS_BOOT_CODE_MAINNET),
        ];

        let interpreter =
            ClarityInterpreter::new(StandardPrincipalData::transient(), Settings::default());
        for (deployer, boot_code) in deploy.iter() {
            for (name, code) in boot_code.iter() {
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
    pub is_interactive: bool,
    pub settings: SessionSettings,
    pub contracts: BTreeMap<String, BTreeMap<String, Vec<String>>>,
    pub asts: BTreeMap<QualifiedContractIdentifier, ContractAST>,
    pub interpreter: ClarityInterpreter,
    api_reference: HashMap<String, String>,
    pub coverage_reports: Vec<TestCoverageReport>,
    pub costs_reports: Vec<CostsReport>,
    pub show_costs: bool,
    pub executed: Vec<String>,
    pub current_epoch: StacksEpochId,
    keywords_reference: HashMap<String, String>,
}

impl Session {
    pub fn new(settings: SessionSettings) -> Session {
        let tx_sender = {
            let address = match settings.initial_deployer {
                Some(ref entry) => entry.address.clone(),
                None => format!("{}", StacksAddress::burn_address(false)),
            };
            PrincipalData::parse_standard_principal(&address)
                .expect("Unable to parse deployer's address")
        };

        Session {
            is_interactive: false,
            interpreter: ClarityInterpreter::new(tx_sender, settings.repl_settings.clone()),
            asts: BTreeMap::new(),
            contracts: BTreeMap::new(),
            api_reference: build_api_reference(),
            coverage_reports: vec![],
            costs_reports: vec![],
            show_costs: false,
            settings,
            executed: Vec::new(),
            current_epoch: StacksEpochId::Epoch2_05,
            keywords_reference: clarity_keywords(),
        }
    }

    pub fn load_boot_contracts(&mut self) {
        let default_tx_sender = self.interpreter.get_tx_sender();

        let boot_testnet_deployer = BOOT_TESTNET_PRINCIPAL.clone();
        self.interpreter.set_tx_sender(boot_testnet_deployer);
        self.include_boot_contracts(false);

        let boot_mainnet_deployer = BOOT_MAINNET_PRINCIPAL.clone();
        self.interpreter.set_tx_sender(boot_mainnet_deployer);
        self.include_boot_contracts(true);
        self.interpreter.set_tx_sender(default_tx_sender);
    }

    pub fn include_boot_contracts(&mut self, mainnet: bool) {
        let boot_code = if mainnet {
            *STACKS_BOOT_CODE_MAINNET
        } else {
            *STACKS_BOOT_CODE_TESTNET
        };

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
                    deployer: ContractDeployer::DefaultDeployer,
                    clarity_version,
                    epoch,
                };
                let _ = self.deploy_contract(&contract, None, false, None, &mut None);
                // Result ignored, boot contracts are trusted to be valid
            }
        }
    }

    pub fn handle_command(&mut self, command: &str) -> (bool, Vec<String>) {
        let mut output = Vec::<String>::new();

        #[allow(unused_mut)]
        let mut reload = false;
        match command {
            "::help" => self.display_help(&mut output),
            "/-/" => self.easter_egg(&mut output),
            cmd if cmd.starts_with("::functions") => self.display_functions(&mut output),
            cmd if cmd.starts_with("::describe") => self.display_doc(&mut output, cmd),
            cmd if cmd.starts_with("::mint_stx") => self.mint_stx(&mut output, cmd),
            cmd if cmd.starts_with("::set_tx_sender") => {
                self.parse_and_set_tx_sender(&mut output, cmd)
            }
            cmd if cmd.starts_with("::get_assets_maps") => self.get_accounts(&mut output),
            cmd if cmd.starts_with("::get_costs") => self.get_costs(&mut output, cmd),
            cmd if cmd.starts_with("::get_contracts") => self.get_contracts(&mut output),
            cmd if cmd.starts_with("::get_block_height") => self.get_block_height(&mut output),
            cmd if cmd.starts_with("::advance_chain_tip") => {
                self.parse_and_advance_chain_tip(&mut output, cmd)
            }
            cmd if cmd.starts_with("::toggle_costs") => self.toggle_costs(&mut output),
            cmd if cmd.starts_with("::get_epoch") => self.get_epoch(&mut output),
            cmd if cmd.starts_with("::set_epoch") => self.set_epoch(&mut output, cmd),
            cmd if cmd.starts_with("::encode") => self.encode(&mut output, cmd),
            cmd if cmd.starts_with("::decode") => self.decode(&mut output, cmd),

            #[cfg(feature = "cli")]
            cmd if cmd.starts_with("::debug") => self.debug(&mut output, cmd),
            #[cfg(feature = "cli")]
            cmd if cmd.starts_with("::trace") => self.trace(&mut output, cmd),
            #[cfg(feature = "cli")]
            cmd if cmd.starts_with("::reload") => reload = true,
            #[cfg(feature = "cli")]
            cmd if cmd.starts_with("::read") => self.read(&mut output, cmd),
            cmd if cmd.starts_with("::keywords") => self.keywords(&mut output),

            snippet => self.run_snippet(&mut output, self.show_costs, snippet),
        }

        (reload, output)
    }

    #[cfg(feature = "cli")]
    fn run_snippet(&mut self, output: &mut Vec<String>, cost_track: bool, cmd: &str) {
        let (mut result, cost) = match self.formatted_interpretation(
            cmd.to_string(),
            None,
            cost_track,
            None,
            None,
        ) {
            Ok((mut output, result)) => {
                if let EvaluationResult::Contract(contract_result) = result.result {
                    let snippet = format!("→ .{} contract successfully stored. Use (contract-call? ...) for invoking the public functions:", contract_result.contract.contract_identifier.clone());
                    output.push(green!(snippet));
                };
                (output, result.cost.clone())
            }
            Err(output) => (output, None),
        };

        if let Some(cost) = cost {
            let headers = vec![
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
    }

    #[cfg(feature = "cli")]
    fn get_costs_percentage(consumed: &u64, limit: &u64) -> String {
        let calc = (*consumed as f64 / *limit as f64) * 100_f64;

        format!("{calc:.2} %")
    }

    pub fn formatted_interpretation(
        &mut self,
        // @TODO: should be &str, it implies 80+ changes in unit tests
        snippet: String,
        name: Option<String>,
        cost_track: bool,
        eval_hooks: Option<Vec<&mut dyn EvalHook>>,
        _test_name: Option<String>,
    ) -> Result<(Vec<String>, ExecutionResult), Vec<String>> {
        let result = self.eval(snippet.to_string(), eval_hooks, cost_track);
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
                            output.push(green!(format!("{}", value)));
                        }
                    }
                    EvaluationResult::Snippet(snippet_result) => {
                        output.push(green!(format!("{}", snippet_result.result)))
                    }
                }
                Ok((output, result))
            }
            Err(diagnostics) => {
                for d in diagnostics {
                    output.append(&mut output_diagnostic(&d, &contract_name, &formatted_lines));
                }
                Err(output)
            }
        }
    }

    #[cfg(feature = "cli")]
    pub fn debug(&mut self, output: &mut Vec<String>, cmd: &str) {
        use crate::repl::debug::cli::CLIDebugger;

        let snippet = match cmd.split_once(' ') {
            Some((_, snippet)) => snippet,
            _ => return output.push(red!("Usage: ::debug <expr>")),
        };

        let mut debugger = CLIDebugger::new(&QualifiedContractIdentifier::transient(), snippet);

        let mut result = match self.formatted_interpretation(
            snippet.to_string(),
            None,
            true,
            Some(vec![&mut debugger]),
            None,
        ) {
            Ok((mut output, result)) => {
                if let EvaluationResult::Contract(contract_result) = result.result {
                    let snippet = format!("→ .{} contract successfully stored. Use (contract-call? ...) for invoking the public functions:", contract_result.contract.contract_identifier.clone());
                    output.push(green!(snippet));
                };
                output
            }
            Err(result) => result,
        };
        output.append(&mut result);
    }

    #[cfg(feature = "cli")]
    pub fn trace(&mut self, output: &mut Vec<String>, cmd: &str) {
        use super::tracer::Tracer;

        let snippet = match cmd.split_once(' ') {
            Some((_, snippet)) => snippet,
            _ => return output.push(red!("Usage: ::trace <expr>")),
        };

        let mut tracer = Tracer::new(snippet.to_string());

        match self.eval(snippet.to_string(), Some(vec![&mut tracer]), false) {
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
                        output_err.push(red!("Unable to parse address to credit"));
                        continue;
                    }
                };

                match self
                    .interpreter
                    .mint_stx_balance(recipient, account.balance)
                {
                    Ok(_) => {}
                    Err(err) => output_err.push(red!(err)),
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
            _ => return output.push(red!("Usage: ::read <filename>")),
        };

        match std::fs::read_to_string(filename) {
            Ok(snippet) => self.run_snippet(output, self.show_costs, &snippet),
            Err(err) => output.push(red!(format!("unable to read {}: {}", filename, err))),
        };
    }

    pub fn stx_transfer(
        &mut self,
        amount: u64,
        recipient: &str,
    ) -> Result<ExecutionResult, Vec<Diagnostic>> {
        let snippet = format!("(stx-transfer? u{} tx-sender '{})", amount, recipient);
        self.eval(snippet.clone(), None, false)
    }

    pub fn deploy_contract(
        &mut self,
        contract: &ClarityContract,
        eval_hooks: Option<Vec<&mut dyn EvalHook>>,
        cost_track: bool,
        test_name: Option<String>,
        ast: &mut Option<ContractAST>,
    ) -> Result<ExecutionResult, Vec<Diagnostic>> {
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
        let mut hooks: Vec<&mut dyn EvalHook> = Vec::new();
        let mut coverage = test_name.map(TestCoverageReport::new);
        if let Some(coverage) = &mut coverage {
            hooks.push(coverage);
        };

        if let Some(mut in_hooks) = eval_hooks {
            for hook in in_hooks.drain(..) {
                hooks.push(hook);
            }
        }

        let contract_id =
            contract.expect_resolved_contract_identifier(Some(&self.interpreter.get_tx_sender()));

        let result = self
            .interpreter
            .run_both(contract, ast, cost_track, Some(hooks));

        match result {
            Ok(result) => {
                if let Some(ref coverage) = coverage {
                    self.coverage_reports.push(coverage.clone());
                }
                if let EvaluationResult::Contract(contract_result) = &result.result {
                    self.asts
                        .insert(contract_id.clone(), contract_result.contract.ast.clone());
                    self.contracts.insert(
                        contract_id.to_string(),
                        contract_result.contract.function_args.clone(),
                    );
                };
                Ok(result)
            }
            Err(res) => Err(res),
        }
    }

    pub fn invoke_contract_call(
        &mut self,
        contract: &str,
        method: &str,
        args: &[String],
        sender: &str,
        test_name: String,
    ) -> Result<(ExecutionResult, QualifiedContractIdentifier), Vec<Diagnostic>> {
        let initial_tx_sender = self.get_tx_sender();
        // Handle fully qualified contract_id and sugared syntax
        let contract_id = if contract.starts_with('S') {
            contract.to_string()
        } else {
            format!("{}.{}", initial_tx_sender, contract,)
        };

        let mut hooks: Vec<&mut dyn EvalHook> = vec![];
        let mut coverage = TestCoverageReport::new(test_name.clone());
        hooks.push(&mut coverage);

        let contract_call = format!(
            "(contract-call? '{} {} {})",
            contract_id,
            method,
            args.join(" ")
        );
        let contract_call = ClarityContract {
            code_source: ClarityCodeSource::ContractInMemory(contract_call),
            name: "contract-call".to_string(),
            deployer: ContractDeployer::Address(sender.to_string()),
            epoch: self.current_epoch,
            clarity_version: ClarityVersion::default_for_epoch(self.current_epoch),
        };

        self.set_tx_sender(sender.into());
        let execution =
            match self
                .interpreter
                .run_both(&contract_call, &mut None, true, Some(hooks))
            {
                Ok(result) => result,
                Err(e) => {
                    self.set_tx_sender(initial_tx_sender);
                    return Err(e);
                }
            };
        self.set_tx_sender(initial_tx_sender);
        self.coverage_reports.push(coverage);

        let contract_identifier = QualifiedContractIdentifier::parse(&contract_id).unwrap();
        if let Some(ref cost) = execution.cost {
            self.costs_reports.push(CostsReport {
                test_name,
                contract_id,
                method: method.to_string(),
                args: args.to_vec(),
                cost_result: cost.clone(),
            });
        }

        Ok((execution, contract_identifier))
    }

    pub fn eval(
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
            .run_both(&contract, &mut None, cost_track, eval_hooks);

        match result {
            Ok(result) => {
                if let EvaluationResult::Contract(contract_result) = &result.result {
                    self.asts.insert(
                        contract_identifier.clone(),
                        contract_result.contract.ast.clone(),
                    );
                    self.contracts.insert(
                        contract_result.contract.contract_identifier.clone(),
                        contract_result.contract.function_args.clone(),
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

    fn display_help(&self, output: &mut Vec<String>) {
        let help_colour = Colour::Yellow;
        output.push(format!(
            "{}",
            help_colour.paint("::help\t\t\t\t\tDisplay help")
        ));
        output.push(format!(
            "{}",
            help_colour
                .paint("::functions\t\t\t\tDisplay all the native functions available in clarity")
        ));
        output.push(format!(
            "{}",
            help_colour
                .paint("::keywords\t\t\t\tDisplay all the native keywords available in clarity")
        ));
        output.push(format!(
            "{}",
            help_colour.paint(
                "::describe <function> | <keyword>\tDisplay documentation for a given native function or keyword"
            )
        ));
        output.push(format!(
            "{}",
            help_colour
                .paint("::mint_stx <principal> <amount>\t\tMint STX balance for a given principal")
        ));
        output.push(format!(
            "{}",
            help_colour.paint("::set_tx_sender <principal>\t\tSet tx-sender variable to principal")
        ));
        output.push(format!(
            "{}",
            help_colour.paint("::get_assets_maps\t\t\tGet assets maps for active accounts")
        ));
        output.push(format!(
            "{}",
            help_colour.paint("::get_costs <expr>\t\t\tDisplay the cost analysis")
        ));
        output.push(format!(
            "{}",
            help_colour.paint("::get_contracts\t\t\t\tGet contracts")
        ));
        output.push(format!(
            "{}",
            help_colour.paint("::get_block_height\t\t\tGet current block height")
        ));
        output.push(format!(
            "{}",
            help_colour.paint("::advance_chain_tip <count>\t\tSimulate mining of <count> blocks")
        ));
        output.push(format!(
            "{}",
            help_colour.paint("::set_epoch <2.0> | <2.05> | <2.1>\tUpdate the current epoch")
        ));
        output.push(format!(
            "{}",
            help_colour.paint("::get_epoch\t\t\t\tGet current epoch")
        ));
        output.push(format!(
            "{}",
            help_colour.paint("::toggle_costs\t\t\t\tDisplay cost analysis after every expression")
        ));
        output.push(format!(
            "{}",
            help_colour
                .paint("::debug <expr>\t\t\t\tStart an interactive debug session executing <expr>")
        ));
        output.push(format!(
            "{}",
            help_colour.paint("::trace <expr>\t\t\t\tGenerate an execution trace for <expr>")
        ));
        output.push(format!(
            "{}",
            help_colour.paint("::reload \t\t\t\tReload the existing contract(s) in the session")
        ));
        output.push(format!(
            "{}",
            help_colour.paint("::read <filename>\t\t\tRead expressions from a file")
        ));
        output.push(format!(
            "{}",
            help_colour.paint("::encode <expr>\t\t\t\tEncode an expression to a Clarity Value bytes representation")
        ));
        output.push(format!(
            "{}",
            help_colour.paint("::decode <bytes>\t\t\tDecode a Clarity Value bytes representation")
        ));
    }

    #[cfg(not(feature = "wasm"))]
    fn easter_egg(&self, _output: &mut [String]) {
        let result = hiro_system_kit::nestable_block_on(fetch_message());
        let message = result.unwrap_or("You found it!".to_string());
        println!("{}", message);
    }

    #[cfg(feature = "wasm")]
    fn easter_egg(&self, _output: &mut [String]) {}

    fn parse_and_advance_chain_tip(&mut self, output: &mut Vec<String>, command: &str) {
        let args: Vec<_> = command.split(' ').collect();

        if args.len() != 2 {
            output.push(red!("Usage: ::advance_chain_tip <count>"));
            return;
        }

        let count = match args[1].parse::<u32>() {
            Ok(count) => count,
            _ => {
                output.push(red!("Unable to parse count"));
                return;
            }
        };

        let new_height = self.advance_chain_tip(count);
        output.push(green!(format!(
            "{} blocks simulated, new height: {}",
            count, new_height
        )));
    }

    pub fn advance_chain_tip(&mut self, count: u32) -> u32 {
        self.interpreter.advance_chain_tip(count)
    }

    fn parse_and_set_tx_sender(&mut self, output: &mut Vec<String>, command: &str) {
        let args: Vec<_> = command.split(' ').collect();

        if args.len() != 2 {
            output.push(red!("Usage: ::set_tx_sender <address>"));
            return;
        }

        let tx_sender = match PrincipalData::parse_standard_principal(args[1]) {
            Ok(address) => address,
            _ => {
                output.push(red!("Unable to parse the address"));
                return;
            }
        };

        self.set_tx_sender(tx_sender.to_address());
        output.push(green!(format!("tx-sender switched to {}", tx_sender)));
    }

    pub fn set_tx_sender(&mut self, address: String) {
        let tx_sender =
            PrincipalData::parse_standard_principal(&address).expect("Unable to parse address");
        self.interpreter.set_tx_sender(tx_sender)
    }

    pub fn get_tx_sender(&self) -> String {
        self.interpreter.get_tx_sender().to_address()
    }

    fn get_block_height(&mut self, output: &mut Vec<String>) {
        let height = self.interpreter.get_block_height();
        output.push(green!(format!("Current height: {}", height)));
    }

    #[cfg(feature = "cli")]
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

    pub fn toggle_costs(&mut self, output: &mut Vec<String>) {
        self.show_costs = !self.show_costs;
        output.push(green!(format!("Always show costs: {}", self.show_costs)))
    }

    pub fn get_epoch(&mut self, output: &mut Vec<String>) {
        output.push(format!("Current epoch: {}", self.current_epoch))
    }

    pub fn set_epoch(&mut self, output: &mut Vec<String>, cmd: &str) {
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
                return output.push(red!(
                    "Usage: ::set_epoch 2.0 | 2.05 | 2.1 | 2.2 | 2.3 | 2.4 | 2.5 | 3.0"
                ))
            }
        };
        self.update_epoch(epoch);
        output.push(green!(format!("Epoch updated to: {epoch}")))
    }

    pub fn update_epoch(&mut self, epoch: StacksEpochId) {
        self.current_epoch = epoch;
    }

    pub fn encode(&mut self, output: &mut Vec<String>, cmd: &str) {
        let snippet = match cmd.split_once(' ') {
            Some((_, snippet)) => snippet,
            _ => return output.push(red!("Usage: ::encode <expr>")),
        };

        let result = self.eval(snippet.to_string(), None, false);
        let value = match result {
            Ok(result) => {
                let mut tx_bytes = vec![];
                let value = match result.result {
                    EvaluationResult::Contract(contract_result) => {
                        if let Some(value) = contract_result.result {
                            value
                        } else {
                            return output.push("No value".to_string());
                        }
                    }
                    EvaluationResult::Snippet(snippet_result) => snippet_result.result,
                };
                if let Err(e) = value.consensus_serialize(&mut tx_bytes) {
                    return output.push(red!(format!("{}", e)));
                };
                let mut s = String::with_capacity(2 * tx_bytes.len());
                for byte in tx_bytes {
                    s = format!("{}{:02x}", s, byte);
                }
                green!(s)
            }
            Err(diagnostics) => {
                let lines: Vec<String> = snippet.split('\n').map(|s| s.to_string()).collect();
                for d in diagnostics {
                    output.append(&mut output_diagnostic(&d, "encode", &lines));
                }
                red!("encoding failed")
            }
        };
        output.push(value);
    }

    pub fn decode(&mut self, output: &mut Vec<String>, cmd: &str) {
        let byte_string = match cmd.split_once(' ') {
            Some((_, bytes)) => bytes,
            _ => return output.push(red!("Usage: ::decode <hex-bytes>")),
        };
        let tx_bytes = match decode_hex(byte_string) {
            Ok(tx_bytes) => tx_bytes,
            Err(e) => return output.push(red!(format!("Parsing error: {}", e))),
        };

        let value = match Value::consensus_deserialize(&mut &tx_bytes[..]) {
            Ok(value) => value,
            Err(e) => return output.push(red!(format!("{}", e))),
        };
        output.push(green!(format!("{}", crate::utils::value_to_string(&value))));
    }

    pub fn get_costs(&mut self, output: &mut Vec<String>, cmd: &str) {
        let expr = match cmd.split_once(' ') {
            Some((_, expr)) => expr,
            _ => return output.push(red!("Usage: ::get_costs <expr>")),
        };

        self.run_snippet(output, true, expr);
    }

    #[cfg(feature = "cli")]
    fn get_accounts(&self, output: &mut Vec<String>) {
        let accounts = self.interpreter.get_accounts();
        if !accounts.is_empty() {
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
            output.push(format!("{}", table));
        }
    }

    #[cfg(feature = "cli")]
    fn get_contracts(&self, output: &mut Vec<String>) {
        if !self.contracts.is_empty() {
            let mut table = Table::new();
            table.add_row(row!["Contract identifier", "Public functions"]);
            let contracts = self.contracts.clone();
            for (contract_id, methods) in contracts.iter() {
                if !contract_id.starts_with(BOOT_TESTNET_ADDRESS)
                    && !contract_id.starts_with(BOOT_MAINNET_ADDRESS)
                {
                    let mut formatted_methods = vec![];
                    for (method_name, method_args) in methods.iter() {
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
                        Cell::new(contract_id),
                        Cell::new(&formatted_spec),
                    ]));
                }
            }
            output.push(format!("{}", table));
        }
    }

    #[cfg(not(feature = "cli"))]
    fn run_snippet(&mut self, output: &mut Vec<String>, cost_track: bool, cmd: &str) {
        let (mut result, cost) =
            match self.formatted_interpretation(cmd.to_string(), None, cost_track, None, None) {
                Ok((output, result)) => (output, result.cost.clone()),
                Err(output) => (output, None),
            };

        if let Some(cost) = cost {
            output.push(format!(
                "Execution: {:?}\nLimit: {:?}",
                cost.total, cost.limit
            ));
        }
        output.append(&mut result);
    }

    #[cfg(not(feature = "cli"))]
    fn get_accounts(&self, output: &mut Vec<String>) {
        if !self.settings.initial_accounts.is_empty() {
            let mut initial_accounts = self.settings.initial_accounts.clone();
            for account in initial_accounts.drain(..) {
                output.push(format!(
                    "{}: {} ({})",
                    account.address, account.balance, account.name
                ));
            }
        }
    }

    #[cfg(not(feature = "cli"))]
    fn get_contracts(&self, output: &mut Vec<String>) {
        for (contract_id, _methods) in self.contracts.iter() {
            if !contract_id.ends_with(".pox")
                && !contract_id.ends_with(".bns")
                && !contract_id.ends_with(".costs")
            {
                output.push(contract_id.to_string());
            }
        }
    }

    fn mint_stx(&mut self, output: &mut Vec<String>, command: &str) {
        let args: Vec<_> = command.split(' ').collect();

        if args.len() != 3 {
            output.push(red!("Usage: ::mint_stx <recipient address> <amount>"));
            return;
        }

        let recipient = match PrincipalData::parse(args[1]) {
            Ok(address) => address,
            _ => {
                output.push(red!("Unable to parse the address"));
                return;
            }
        };

        let amount: u64 = match args[2].parse() {
            Ok(recipient) => recipient,
            _ => {
                output.push(red!("Unable to parse the balance"));
                return;
            }
        };

        match self.interpreter.mint_stx_balance(recipient, amount) {
            Ok(msg) => output.push(green!(msg)),
            Err(err) => output.push(red!(err)),
        };
    }

    fn display_functions(&self, output: &mut Vec<String>) {
        let help_colour = Colour::Yellow;
        let api_reference_index = self.get_api_reference_index();
        output.push(format!(
            "{}",
            help_colour.paint(api_reference_index.join("\n"))
        ));
    }

    fn display_doc(&self, output: &mut Vec<String>, command: &str) {
        let help_colour = Colour::Yellow;
        let keyword = {
            let mut s = command.to_string();
            s = s.replace("::describe", "");
            s = s.replace(' ', "");
            s
        };

        let result = match self.lookup_functions_or_keywords_docs(&keyword) {
            Some(doc) => format!("{}", help_colour.paint(doc)),
            None => format!(
                "{}",
                Colour::Red.paint("It looks like there aren't matches for your search")
            ),
        };
        output.push(result);
    }

    pub fn display_digest(&self) -> Result<String, String> {
        let mut output = vec![];
        self.get_contracts(&mut output);
        self.get_accounts(&mut output);
        Ok(output.join("\n"))
    }

    fn keywords(&self, output: &mut Vec<String>) {
        let help_colour = Colour::Yellow;
        let keywords = self.get_clarity_keywords();
        output.push(format!("{}", help_colour.paint(keywords.join("\n"))));
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

#[cfg(test)]
mod tests {
    use crate::repl::{self, settings::Account};

    use super::*;

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
    fn encode_simple() {
        let mut session = Session::new(SessionSettings::default());
        let mut output: Vec<String> = Vec::new();
        session.encode(&mut output, "::encode 42");
        assert_eq!(output.len(), 1);
        assert_eq!(output[0], green!("000000000000000000000000000000002a"));
    }

    #[test]
    fn encode_map() {
        let mut session = Session::new(SessionSettings::default());
        let mut output: Vec<String> = Vec::new();
        session.encode(&mut output, "::encode { foo: \"hello\", bar: false }");
        assert_eq!(output.len(), 1);
        assert_eq!(
            output[0],
            green!("0c00000002036261720403666f6f0d0000000568656c6c6f")
        );
    }

    #[test]
    fn epoch_switch() {
        let mut session = Session::new(SessionSettings::default());
        session.update_epoch(StacksEpochId::Epoch20);
        let diags = session
            .eval("(slice? \"blockstack\" u5 u10)".into(), None, false)
            .unwrap_err();
        assert_eq!(
            diags[0].message,
            format!("use of unresolved function 'slice?'",)
        );
        session.update_epoch(StacksEpochId::Epoch21);
        let res = session
            .eval("(slice? \"blockstack\" u5 u10)".into(), None, false)
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
    fn encode_error() {
        let mut session = Session::new(SessionSettings::default());
        let mut output: Vec<String> = Vec::new();
        session.encode(&mut output, "::encode { foo false }");
        assert_eq!(
            output[0],
            format_err!("Tuple literal construction expects a colon at index 1")
        );

        session.encode(&mut output, "::encode (foo 1)");
        assert_eq!(
            output[2],
            format!(
                "encode:1:1: {} use of unresolved function 'foo'",
                red!("error:")
            )
        );
    }

    #[test]
    fn decode_simple() {
        let mut session = Session::new(SessionSettings::default());
        let mut output: Vec<String> = Vec::new();
        session.decode(&mut output, "::decode 0000000000000000 0000000000000000 2a");
        assert_eq!(output.len(), 1);
        assert_eq!(output[0], green!("42"));
    }

    #[test]
    fn decode_map() {
        let mut session = Session::new(SessionSettings::default());
        let mut output: Vec<String> = Vec::new();
        session.decode(
            &mut output,
            "::decode 0x0c00000002036261720403666f6f0d0000000568656c6c6f",
        );
        assert_eq!(output.len(), 1);
        assert_eq!(output[0], green!("{bar: false, foo: \"hello\"}"));
    }

    #[test]
    fn decode_error() {
        let mut session = Session::new(SessionSettings::default());
        let mut output: Vec<String> = Vec::new();
        session.decode(&mut output, "::decode 42");
        assert_eq!(output.len(), 1);
        assert_eq!(
            output[0],
            red!("Failed to decode clarity value: DeserializationError(\"Bad type prefix\")")
        );

        session.decode(&mut output, "::decode 4g");
        assert_eq!(output.len(), 2);
        assert_eq!(
            output[1],
            red!("Parsing error: invalid digit found in string")
        );
    }

    #[test]
    fn clarity_epoch_mismatch() {
        let settings = SessionSettings::default();
        let mut session = Session::new(settings);
        session.start().expect("session could not start");
        let snippet = "(define-data-var x uint u0)";
        let contract = ClarityContract {
            code_source: ClarityCodeSource::ContractInMemory(snippet.to_string()),
            name: "should_error".to_string(),
            deployer: ContractDeployer::Address("ST000000000000000000002AMW42H".into()),
            clarity_version: ClarityVersion::Clarity2,
            epoch: StacksEpochId::Epoch20,
        };

        let result = session.deploy_contract(&contract, None, false, None, &mut None);
        assert!(result.is_err(), "Expected error for clarity mismatch");
    }

    #[test]
    fn evaluate_at_block() {
        let settings = SessionSettings {
            include_boot_contracts: vec!["costs".into(), "costs-2".into(), "costs-3".into()],
            ..Default::default()
        };

        let mut session = Session::new(settings);
        session.start().expect("session could not start");

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
            clarity_version: ClarityVersion::Clarity1,
            epoch: repl::DEFAULT_EPOCH,
        };

        let _ = session.deploy_contract(&contract, None, false, None, &mut None);

        // assert data-var is set to 0
        assert_eq!(
            session.handle_command("(contract-call? .contract get-x)").1[0],
            green!("u0")
        );

        // advance chain tip and test at-block
        session.advance_chain_tip(10000);
        assert_eq!(
            session.handle_command("(contract-call? .contract get-x)").1[0],
            green!("u0")
        );
        session.handle_command("(contract-call? .contract incr)");
        assert_eq!(
            session.handle_command("(contract-call? .contract get-x)").1[0],
            green!("u1")
        );
        assert_eq!(session.handle_command("(at-block (unwrap-panic (get-block-info? id-header-hash u0)) (contract-call? .contract get-x))").1[0], green!("u0"));
        assert_eq!(session.handle_command("(at-block (unwrap-panic (get-block-info? id-header-hash u5000)) (contract-call? .contract get-x))").1[0], green!("u0"));

        // advance chain tip again and test at-block
        // do this twice to make sure that the lookup table is being updated properly
        session.advance_chain_tip(10);
        session.advance_chain_tip(10);

        assert_eq!(
            session.handle_command("(contract-call? .contract get-x)").1[0],
            green!("u1")
        );
        session.handle_command("(contract-call? .contract incr)");
        assert_eq!(
            session.handle_command("(contract-call? .contract get-x)").1[0],
            green!("u2")
        );
        assert_eq!(session.handle_command("(at-block (unwrap-panic (get-block-info? id-header-hash u10000)) (contract-call? .contract get-x))").1[0], green!("u1"));
    }
}

#[cfg(not(feature = "wasm"))]
async fn fetch_message() -> Result<String, reqwest::Error> {
    let gist: &str = "https://storage.googleapis.com/hiro-public/assets/clarinet-egg.txt";
    let response = reqwest::get(gist).await?;
    let message = response.text().await?;
    Ok(message)
}

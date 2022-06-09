use super::{ClarityInterpreter, ExecutionResult};
use crate::analysis::ast_dependency_detector::{ASTDependencyDetector, Dependency};
use crate::clarity::analysis::ContractAnalysis;
use crate::clarity::ast::ContractAST;
use crate::clarity::codec::StacksMessageCodec;
use crate::clarity::coverage::{CoverageReporter, TestCoverageReport};
use crate::clarity::docs::{make_api_reference, make_define_reference, make_keyword_reference};
use crate::clarity::errors::Error;
use crate::clarity::functions::define::DefineFunctions;
use crate::clarity::functions::NativeFunctions;
use crate::clarity::types::{
    PrincipalData, QualifiedContractIdentifier, StandardPrincipalData, Value,
};
use crate::clarity::util::StacksAddress;
use crate::clarity::variables::NativeVariables;
use crate::clarity::{ContractName, EvalHook};
use crate::contracts::{BNS_CONTRACT, COSTS_V1_CONTRACT, COSTS_V2_CONTRACT, POX_CONTRACT};
use crate::repl::CostSynthesis;
use crate::{clarity::diagnostic::Diagnostic, repl::settings::InitialContract};
use ansi_term::{Colour, Style};
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::convert::TryFrom;
use std::fmt;
use std::fs::{self, File};
use std::io::Write;
use std::num::ParseIntError;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[cfg(feature = "cli")]
use prettytable::{Cell, Row, Table};

use super::SessionSettings;

static BOOT_TESTNET_ADDRESS: &str = "ST000000000000000000002AMW42H";
static BOOT_MAINNET_ADDRESS: &str = "SP000000000000000000002Q6VF78";

lazy_static! {
    static ref BOOT_TESTNET_PRINCIPAL: StandardPrincipalData =
        PrincipalData::parse_standard_principal(&BOOT_TESTNET_ADDRESS).unwrap();
    static ref BOOT_MAINNET_PRINCIPAL: StandardPrincipalData =
        PrincipalData::parse_standard_principal(&BOOT_MAINNET_ADDRESS).unwrap();
}

enum Command {
    LoadLocalContract(String),
    LoadDeployContract(String),
    UnloadContract(String),
    ExecuteSnippet(String),
    OpenSession,
    CloseSession,
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
    session_id: u32,
    started_at: u32,
    pub is_interactive: bool,
    pub settings: SessionSettings,
    pub contracts: BTreeMap<String, BTreeMap<String, Vec<String>>>,
    pub asts: BTreeMap<QualifiedContractIdentifier, ContractAST>,
    pub interpreter: ClarityInterpreter,
    api_reference: HashMap<String, String>,
    pub coverage_reports: Vec<TestCoverageReport>,
    pub costs_reports: Vec<CostsReport>,
    pub initial_contracts_analysis: Vec<(ContractAnalysis, String, String)>,
    pub show_costs: bool,
    pub executed: Vec<String>,
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
            session_id: 0,
            started_at: 0,
            is_interactive: false,
            interpreter: ClarityInterpreter::new(tx_sender, settings.repl_settings.clone()),
            asts: BTreeMap::new(),
            contracts: BTreeMap::new(),
            api_reference: build_api_reference(),
            coverage_reports: vec![],
            costs_reports: vec![],
            initial_contracts_analysis: vec![],
            show_costs: false,
            settings,
            executed: Vec::new(),
        }
    }

    pub fn load_boot_contracts(&mut self) {
        let default_tx_sender = self.interpreter.get_tx_sender();

        let boot_testnet_deployer = BOOT_TESTNET_PRINCIPAL.clone();
        self.interpreter.set_tx_sender(boot_testnet_deployer);
        self.include_boot_contracts();

        let boot_mainnet_deployer = BOOT_MAINNET_PRINCIPAL.clone();
        self.interpreter.set_tx_sender(boot_mainnet_deployer);
        self.include_boot_contracts();
        self.interpreter.set_tx_sender(default_tx_sender);
    }

    #[cfg(not(feature = "wasm"))]
    pub fn start(&mut self) -> Result<(String, Vec<(ContractAnalysis, String, String)>), String> {
        let mut output_err = Vec::<String>::new();
        let mut output = Vec::<String>::new();
        let mut contracts = vec![];

        if !self.settings.include_boot_contracts.is_empty() {
            self.load_boot_contracts();
        }

        if self.settings.initial_accounts.len() > 0 {
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

        if !self.settings.lazy_initial_contracts_interpretation {
            match self.interpret_initial_contracts() {
                Ok((ref mut res, ref mut initial_contracts)) => {
                    if self.is_interactive {
                        // If the session is interactive (clarinet console, usr/bin/clarity-repl)
                        // we will display the contracts + genesis asset map.
                        if !self.settings.initial_contracts.is_empty() {
                            output.push(blue!("Contracts"));
                            self.get_contracts(&mut output);
                        }

                        if self.settings.initial_accounts.len() > 0 {
                            output.push(blue!("Initialized balances"));
                            self.get_accounts(&mut output);
                        }
                    }
                    output.append(res);
                    contracts.append(initial_contracts);
                }
                Err(ref mut res) => {
                    output_err.append(res);
                }
            };
        }

        match output_err.len() {
            0 => Ok((output.join("\n"), contracts)),
            _ => Err(output_err.join("\n")),
        }
    }

    fn handle_initial_contracts(
        &mut self,
    ) -> Result<(Vec<String>, Vec<(ContractAnalysis, String, String)>), Vec<String>> {
        let mut output_err = vec![];
        let mut output = vec![];

        let mut contracts = vec![];
        if self.settings.initial_contracts.len() > 0 {
            let initial_contracts = self.settings.initial_contracts.clone();
            let mut initial_contracts_map = HashMap::new();
            let mut contract_asts = HashMap::new();
            let default_tx_sender = self.interpreter.get_tx_sender();

            // Parse all contracts and save the ASTs
            for contract in &initial_contracts {
                let deployer = {
                    let address = match contract.deployer {
                        Some(ref entry) => entry.clone(),
                        None => format!("{}", StacksAddress::burn_address(false)),
                    };
                    PrincipalData::parse_standard_principal(&address)
                        .expect("Unable to parse deployer's address")
                };

                self.interpreter.set_tx_sender(deployer);

                match self.build_ast(&contract.code, contract.name.as_deref()) {
                    Ok((contract_identifier, ast, mut contract_output)) => {
                        contract_asts.insert(contract_identifier.clone(), ast);
                        initial_contracts_map.insert(contract_identifier, contract);
                        output.append(&mut contract_output)
                    }
                    Err(mut output) => output_err.append(&mut output),
                }
            }

            let dependencies =
                match ASTDependencyDetector::detect_dependencies(&contract_asts, &self.asts) {
                    Ok(dependencies) => dependencies,
                    Err((dependencies, unresolved)) => {
                        for contract_id in unresolved {
                            output_err.push(format!(
                                "{}: unresolved dependency: {}",
                                red!("error"),
                                contract_id
                            ));
                        }
                        dependencies
                    }
                };
            match ASTDependencyDetector::order_contracts(&dependencies) {
                Ok(ordered_contracts) => {
                    // interpret the contract ASTs in order
                    for contract_identifier in ordered_contracts {
                        let contract = initial_contracts_map[contract_identifier];
                        let ast = contract_asts.remove(contract_identifier).unwrap();
                        match self.formatted_interpretation_ast(
                            contract.code.to_string(),
                            ast,
                            contract_identifier.clone(),
                            true,
                            None,
                            Some("Deployment".into()),
                        ) {
                            Ok((ref mut res_output, result)) => {
                                output.append(res_output);
                                if result.contract.is_none() {
                                    continue;
                                }
                                let analysis_result = result.contract.unwrap();
                                contracts.push((
                                    analysis_result.4.clone(),
                                    analysis_result.1.clone(),
                                    contract.path.clone(),
                                ))
                            }
                            Err(ref mut result) => output_err.append(result),
                        };
                    }
                    self.interpreter.set_tx_sender(default_tx_sender);
                }
                Err(e) => {
                    output_err.push(format!("{}: {}", red!("error"), e));
                }
            }
        }
        if output_err.len() > 0 {
            return Err(output_err);
        }
        Ok((output, contracts))
    }

    #[cfg(not(feature = "wasm"))]
    pub fn interpret_initial_contracts(
        &mut self,
    ) -> Result<(Vec<String>, Vec<(ContractAnalysis, String, String)>), Vec<String>> {
        if self.initial_contracts_analysis.is_empty() {
            let (output, contracts) = self.handle_initial_contracts()?;

            self.initial_contracts_analysis
                .append(&mut contracts.clone());

            Ok((output, contracts))
        } else {
            Err(vec!["Initial contracts already interpreted".into()])
        }
    }

    pub fn include_boot_contracts(&mut self) {
        if self
            .settings
            .include_boot_contracts
            .contains(&"pox".to_string())
        {
            self.formatted_interpretation(
                POX_CONTRACT.to_string(),
                Some("pox".to_string()),
                false,
                None,
                None,
            )
            .expect("Unable to deploy POX");
        }
        if self
            .settings
            .include_boot_contracts
            .contains(&"bns".to_string())
        {
            self.formatted_interpretation(
                BNS_CONTRACT.to_string(),
                Some("bns".to_string()),
                false,
                None,
                None,
            )
            .expect("Unable to deploy BNS");
        }
        if self
            .settings
            .include_boot_contracts
            .contains(&"costs-v1".to_string())
        {
            self.formatted_interpretation(
                COSTS_V1_CONTRACT.to_string(),
                Some("costs-v1".to_string()),
                false,
                None,
                None,
            )
            .expect("Unable to deploy COSTS");
        }
        if self
            .settings
            .include_boot_contracts
            .contains(&"costs-v2".to_string())
        {
            self.formatted_interpretation(
                COSTS_V2_CONTRACT.to_string(),
                Some("costs-v2".to_string()),
                false,
                None,
                None,
            )
            .expect("Unable to deploy COSTS");
        }
    }

    pub fn get_boot_contracts_asts(&self) -> BTreeMap<QualifiedContractIdentifier, ContractAST> {
        let mut asts = BTreeMap::new();
        let (pox_ast, _, _) = self.interpreter.build_ast(
            QualifiedContractIdentifier::transient(),
            POX_CONTRACT.to_string(),
            2,
        );
        let (bns_ast, _, _) = self.interpreter.build_ast(
            QualifiedContractIdentifier::transient(),
            BNS_CONTRACT.to_string(),
            2,
        );

        let bns_contract = ContractName::try_from("bns").unwrap();
        let pox_contract = ContractName::try_from("pox").unwrap();

        asts.insert(
            QualifiedContractIdentifier::new(BOOT_TESTNET_PRINCIPAL.clone(), bns_contract.clone()),
            bns_ast.clone(),
        );
        asts.insert(
            QualifiedContractIdentifier::new(BOOT_MAINNET_PRINCIPAL.clone(), bns_contract),
            bns_ast,
        );
        asts.insert(
            QualifiedContractIdentifier::new(BOOT_TESTNET_PRINCIPAL.clone(), pox_contract.clone()),
            pox_ast.clone(),
        );
        asts.insert(
            QualifiedContractIdentifier::new(BOOT_MAINNET_PRINCIPAL.clone(), pox_contract),
            pox_ast,
        );
        asts
    }

    #[cfg(feature = "wasm")]
    pub async fn start_wasm(&mut self) -> String {
        let mut output = Vec::<String>::new();

        if !self.settings.include_boot_contracts.is_empty() {
            let default_tx_sender = self.interpreter.get_tx_sender();

            let boot_testnet_deployer =
                PrincipalData::parse_standard_principal(&BOOT_TESTNET_ADDRESS)
                    .expect("Unable to parse deployer's address");
            self.interpreter.set_tx_sender(boot_testnet_deployer);
            self.include_boot_contracts();

            let boot_mainnet_deployer =
                PrincipalData::parse_standard_principal(&BOOT_MAINNET_ADDRESS)
                    .expect("Unable to parse deployer's address");
            self.interpreter.set_tx_sender(boot_mainnet_deployer);
            self.include_boot_contracts();

            self.interpreter.set_tx_sender(default_tx_sender);
        }

        // The cost of maintaining the "requirements" / "links" feature on WASM builds
        // is pretty high, and the amount of code duplicated is very important.
        // We will timeshift through git and restore this feature if we
        // can identify a concrete use case in the future

        if !self.settings.initial_contracts.is_empty() {
            output.push(blue!("Contracts"));
            self.get_contracts(&mut output);
        }

        output.join("\n")
    }

    pub fn handle_command(&mut self, command: &str) -> Vec<String> {
        let mut output = Vec::<String>::new();
        match command {
            "::help" => self.display_help(&mut output),
            cmd if cmd.starts_with("::list_functions") => self.display_functions(&mut output),
            cmd if cmd.starts_with("::describe_function") => self.display_doc(&mut output, cmd),
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
            cmd if cmd.starts_with("::encode") => self.encode(&mut output, cmd),
            cmd if cmd.starts_with("::decode") => self.decode(&mut output, cmd),
            #[cfg(feature = "cli")]
            cmd if cmd.starts_with("::debug") => self.debug(&mut output, cmd),
            #[cfg(feature = "cli")]
            cmd if cmd.starts_with("::trace") => self.trace(&mut output, cmd),
            #[cfg(feature = "cli")]
            cmd if cmd.starts_with("::reload") => self.reload(&mut output),

            snippet => {
                if self.show_costs {
                    self.get_costs(&mut output, &format!("::get_costs {}", snippet))
                } else {
                    let mut result = match self.formatted_interpretation(
                        snippet.to_string(),
                        None,
                        true,
                        None,
                        None,
                    ) {
                        Ok((mut output, result)) => {
                            if let Some((ref contract_name, _, _, _, _)) = result.contract {
                                let snippet = format!("→ .{} contract successfully stored. Use (contract-call? ...) for invoking the public functions:", contract_name.clone());
                                output.push(green!(snippet));
                            }
                            output
                        }
                        Err(result) => result,
                    };
                    output.append(&mut result);
                }
            }
        }
        output
    }

    pub fn formatted_interpretation(
        &mut self,
        snippet: String,
        name: Option<String>,
        cost_track: bool,
        eval_hooks: Option<Vec<Box<dyn EvalHook>>>,
        test_name: Option<String>,
    ) -> Result<(Vec<String>, ExecutionResult), Vec<String>> {
        let light_red = Colour::Red.bold();

        let result = self.interpret(
            snippet.to_string(),
            name.clone(),
            eval_hooks,
            cost_track,
            test_name,
            None,
        );
        let mut output = Vec::<String>::new();
        let lines = snippet.lines();
        let formatted_lines: Vec<String> = lines.map(|l| l.to_string()).collect();
        let contract_name = name.unwrap_or("<stdin>".to_string());

        match result {
            Ok(result) => {
                for diagnostic in &result.diagnostics {
                    output.append(&mut diagnostic.output(&contract_name, &formatted_lines));
                }
                if result.events.len() > 0 {
                    output.push(black!("Events emitted"));
                    for event in result.events.iter() {
                        output.push(black!(format!("{}", event)));
                    }
                }
                if let Some(ref result) = result.result {
                    output.push(green!(format!("{}", result)));
                }
                Ok((output, result))
            }
            Err(diagnostics) => {
                for d in diagnostics {
                    output.append(&mut d.output(&contract_name, &formatted_lines));
                }
                Err(output)
            }
        }
    }

    #[cfg(feature = "cli")]
    pub fn debug(&mut self, output: &mut Vec<String>, cmd: &str) {
        use crate::clarity::debug::cli::CLIDebugger;

        let snippet = match cmd.split_once(" ") {
            Some((_, snippet)) => snippet,
            _ => return output.push(red!("Usage: ::debug <expr>")),
        };

        let debugger = CLIDebugger::new(&QualifiedContractIdentifier::transient(), snippet);

        let mut result = match self.formatted_interpretation(
            snippet.to_string(),
            None,
            true,
            Some(vec![Box::new(debugger)]),
            None,
        ) {
            Ok((mut output, result)) => {
                if let Some((ref contract_name, _, _, _, _)) = result.contract {
                    let snippet = format!("→ .{} contract successfully stored. Use (contract-call? ...) for invoking the public functions:", contract_name.clone());
                    output.push(green!(snippet));
                }
                output
            }
            Err(result) => result,
        };
        output.append(&mut result);
    }

    #[cfg(feature = "cli")]
    pub fn trace(&mut self, output: &mut Vec<String>, cmd: &str) {
        use super::tracer::Tracer;

        let snippet = match cmd.split_once(" ") {
            Some((_, snippet)) => snippet,
            _ => return output.push(red!("Usage: ::trace <expr>")),
        };

        let tracer = Tracer::new(snippet.to_string());

        match self.interpret(
            snippet.to_string(),
            None,
            Some(vec![Box::new(tracer)]),
            false,
            None,
            None,
        ) {
            Ok(_) => (),
            Err(diagnostics) => {
                let lines = snippet.lines();
                let formatted_lines: Vec<String> = lines.map(|l| l.to_string()).collect();
                for d in diagnostics {
                    output.append(&mut d.output("<snippet>", &formatted_lines));
                }
            }
        };
    }

    #[cfg(feature = "cli")]
    fn reload(&mut self, output: &mut Vec<String>) {
        self.asts.clear();
        self.contracts.clear();
        self.costs_reports.clear();
        self.coverage_reports.clear();
        self.initial_contracts_analysis.clear();
        self.interpreter = ClarityInterpreter::new(
            self.interpreter.get_tx_sender(),
            self.settings.repl_settings.clone(),
        );
        let contracts = std::mem::take(&mut self.settings.initial_contracts);

        for existing in contracts {
            let reloaded = match fs::read_to_string(&existing.path) {
                Ok(code) => code,
                Err(err) => {
                    println!(
                        "{}: Unable to read {}: {}",
                        red!("error"),
                        existing.path,
                        err
                    );
                    std::process::exit(1);
                }
            };

            self.settings.initial_contracts.push(InitialContract {
                code: reloaded,
                path: existing.path,
                name: existing.name,
                deployer: existing.deployer,
            });
        }

        match self.start() {
            Ok(_) => {
                self.get_contracts(output);
                output.push("Ok, contracts reloaded.".to_string());
            }
            Err(e) => {
                println!("{}", e);
            }
        }
    }

    pub fn formatted_interpretation_ast(
        &mut self,
        snippet: String,
        ast: ContractAST,
        contract_identifier: QualifiedContractIdentifier,
        cost_track: bool,
        eval_hooks: Option<Vec<Box<dyn EvalHook>>>,
        test_name: Option<String>,
    ) -> Result<(Vec<String>, ExecutionResult), Vec<String>> {
        let light_red = Colour::Red.bold();

        let result = self.interpret_ast(
            &contract_identifier,
            ast,
            snippet.to_string(),
            eval_hooks,
            cost_track,
            test_name,
        );
        let mut output = Vec::<String>::new();
        let lines = snippet.lines();
        let formatted_lines: Vec<String> = lines.map(|l| l.to_string()).collect();
        let contract_name = contract_identifier.name.to_string();

        match result {
            Ok(result) => {
                for diagnostic in &result.diagnostics {
                    output.append(&mut diagnostic.output(&contract_name, &formatted_lines));
                }
                if result.events.len() > 0 {
                    output.push(black!("Events emitted"));
                    for event in result.events.iter() {
                        output.push(black!(format!("{}", event)));
                    }
                }
                if let Some(ref result) = result.result {
                    output.push(green!(format!("{}", result)));
                }
                Ok((output, result))
            }
            Err(diagnostics) => {
                for d in diagnostics {
                    output.append(&mut d.output(&contract_name, &formatted_lines));
                }
                Err(output)
            }
        }
    }

    pub fn invoke_contract_call(
        &mut self,
        contract: &str,
        method: &str,
        args: &Vec<String>,
        sender: &str,
        test_name: String,
    ) -> Result<ExecutionResult, Vec<Diagnostic>> {
        let initial_tx_sender = self.get_tx_sender();
        // Kludge for handling fully qualified contract_id vs sugared syntax
        let first_char = contract.chars().next().unwrap();
        let contract_id = if first_char.to_string() == "S" {
            contract.to_string()
        } else {
            format!("{}.{}", initial_tx_sender, contract,)
        };

        let snippet = format!(
            "(contract-call? '{} {} {})",
            contract_id,
            method,
            args.join(" ")
        );

        self.set_tx_sender(sender.into());
        let result = self.interpret(snippet, None, None, true, Some(test_name.clone()), None)?;
        if let Some(ref cost) = result.cost {
            self.costs_reports.push(CostsReport {
                test_name,
                contract_id,
                method: method.to_string(),
                args: args.to_vec(),
                cost_result: cost.clone(),
            });
        }
        self.set_tx_sender(initial_tx_sender);
        Ok(result)
    }

    pub fn build_ast(
        &mut self,
        snippet: &str,
        name: Option<&str>,
    ) -> Result<(QualifiedContractIdentifier, ContractAST, Vec<String>), Vec<String>> {
        let (contract_name, is_tx) = match name {
            Some(name) => (name.to_string(), false),
            None => (format!("contract-{}", self.contracts.len()), true),
        };
        let tx_sender = self.interpreter.get_tx_sender().to_address();
        let id = format!("{}.{}", tx_sender, contract_name);
        let contract_identifier = QualifiedContractIdentifier::parse(&id).unwrap();

        let (ast, diagnostics, success) = self.interpreter.build_ast(
            contract_identifier.clone(),
            snippet.to_string(),
            self.settings.repl_settings.parser_version,
        );

        let mut output = Vec::<String>::new();
        let lines = snippet.lines();
        let formatted_lines: Vec<String> = lines.map(|l| l.to_string()).collect();
        for diagnostic in &diagnostics {
            output.append(&mut diagnostic.output(&contract_name, &formatted_lines));
        }
        if success {
            Ok((contract_identifier, ast, output))
        } else {
            Err(output)
        }
    }

    pub fn interpret_ast(
        &mut self,
        contract_identifier: &QualifiedContractIdentifier,
        ast: ContractAST,
        snippet: String,
        mut eval_hooks: Option<Vec<Box<dyn EvalHook>>>,
        cost_track: bool,
        test_name: Option<String>,
    ) -> Result<ExecutionResult, Vec<Diagnostic>> {
        if let Some(test_name) = test_name {
            let coverage = TestCoverageReport::new(test_name.into());
            if let Some(hooks) = &mut eval_hooks {
                hooks.push(Box::new(coverage));
            } else {
                eval_hooks = Some(vec![Box::new(coverage)]);
            };
        }
        match self.interpreter.run_ast(
            ast,
            snippet,
            contract_identifier.clone(),
            cost_track,
            eval_hooks,
        ) {
            Ok(result) => {
                if let Some(ref coverage) = result.coverage {
                    self.coverage_reports.push(coverage.clone());
                }
                if let Some((
                    ref contract_identifier_str,
                    ref source,
                    ref contract,
                    ref ast,
                    ref analysis,
                )) = result.contract
                {
                    self.asts.insert(contract_identifier.clone(), ast.clone());
                    self.contracts
                        .insert(contract_identifier_str.clone(), contract.clone());
                }
                Ok(result)
            }
            Err(res) => Err(res),
        }
    }

    pub fn interpret(
        &mut self,
        snippet: String,
        name: Option<String>,
        mut eval_hooks: Option<Vec<Box<dyn EvalHook>>>,
        cost_track: bool,
        test_name: Option<String>,
        ast: Option<&ContractAST>,
    ) -> Result<ExecutionResult, Vec<Diagnostic>> {
        let (contract_name, is_tx) = match name {
            Some(name) => (name, false),
            None => (format!("contract-{}", self.contracts.len()), true),
        };
        let first_char = contract_name.chars().next().unwrap();

        if let Some(test_name) = test_name {
            let coverage = TestCoverageReport::new(test_name.into());
            if let Some(hooks) = &mut eval_hooks {
                hooks.push(Box::new(coverage));
            } else {
                eval_hooks = Some(vec![Box::new(coverage)]);
            };
        }

        // Kludge for handling fully qualified contract_id vs sugared syntax
        let contract_identifier = if first_char.to_string() == "S" {
            QualifiedContractIdentifier::parse(&contract_name).unwrap()
        } else {
            let tx_sender = self.interpreter.get_tx_sender().to_address();
            let id = format!("{}.{}", tx_sender, contract_name);
            QualifiedContractIdentifier::parse(&id).unwrap()
        };

        let result = if let Some(ast) = ast {
            self.interpreter.run_ast(
                ast.clone(),
                snippet,
                contract_identifier.clone(),
                cost_track,
                eval_hooks,
            )
        } else {
            self.interpreter
                .run(snippet, contract_identifier.clone(), cost_track, eval_hooks)
        };

        match result {
            Ok(result) => {
                if let Some(ref coverage) = result.coverage {
                    self.coverage_reports.push(coverage.clone());
                }
                if let Some((
                    ref contract_identifier_str,
                    ref source,
                    ref contract,
                    ref ast,
                    ref analysis,
                )) = result.contract
                {
                    self.asts.insert(contract_identifier.clone(), ast.clone());
                    self.contracts
                        .insert(contract_identifier_str.clone(), contract.clone());
                }
                Ok(result)
            }
            Err(res) => Err(res),
        }
    }

    pub fn lookup_api_reference(&self, keyword: &str) -> Option<&String> {
        self.api_reference.get(keyword)
    }

    pub fn get_api_reference_index(&self) -> Vec<String> {
        let mut keys = self
            .api_reference
            .iter()
            .map(|(k, _)| k.to_string())
            .collect::<Vec<String>>();
        keys.sort();
        keys
    }

    fn display_help(&self, output: &mut Vec<String>) {
        let help_colour = Colour::Yellow;
        let coming_soon_colour = Colour::Black.bold();
        output.push(format!(
            "{}",
            help_colour.paint("::help\t\t\t\t\tDisplay help")
        ));
        output.push(format!(
            "{}",
            help_colour.paint(
                "::list_functions\t\t\tDisplay all the native functions available in clarity"
            )
        ));
        output.push(format!(
            "{}",
            help_colour.paint(
                "::describe_function <function>\t\tDisplay documentation for a given native function fn-name"
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
    }

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

        let tx_sender = match PrincipalData::parse_standard_principal(&args[1]) {
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

    pub fn encode(&mut self, output: &mut Vec<String>, cmd: &str) {
        let snippet = match cmd.split_once(" ") {
            Some((_, snippet)) => snippet,
            _ => return output.push(red!("Usage: ::encode <expr>")),
        };

        let result = self.interpret(snippet.to_string(), None, None, false, None, None);
        let value = match result {
            Ok(result) => {
                let mut tx_bytes = vec![];
                let value = match result.result {
                    Some(value) => value,
                    None => return output.push("No value".to_string()),
                };
                match value.consensus_serialize(&mut tx_bytes) {
                    Err(e) => return output.push(red!(format!("{}", e))),
                    _ => (),
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
                    output.append(&mut d.output(&"encode".to_string(), &lines));
                }
                red!("encoding failed")
            }
        };
        output.push(value);
    }

    pub fn decode(&mut self, output: &mut Vec<String>, cmd: &str) {
        let byteString = match cmd.split_once(" ") {
            Some((_, bytes)) => bytes,
            _ => return output.push(red!("Usage: ::decode <hex-bytes>")),
        };
        let tx_bytes = match decode_hex(byteString) {
            Ok(tx_bytes) => tx_bytes,
            Err(e) => return output.push(red!(format!("Parsing error: {}", e))),
        };

        let value = match Value::consensus_deserialize(&mut &tx_bytes[..]) {
            Ok(value) => value,
            Err(e) => return output.push(red!(format!("{}", e))),
        };
        output.push(green!(format!("{}", value)));
    }

    #[cfg(feature = "cli")]
    pub fn get_costs(&mut self, output: &mut Vec<String>, cmd: &str) {
        let snippet = cmd.to_string().split_off("::get_costs ".len());
        let (mut result, cost) =
            match self.formatted_interpretation(snippet, None, true, None, None) {
                Ok((output, result)) => (output, result.cost.clone()),
                Err(output) => (output, None),
            };

        if let Some(cost) = cost {
            let headers = vec!["".to_string(), "Consumed".to_string(), "Limit".to_string()];
            let mut headers_cells = vec![];
            for header in headers.iter() {
                headers_cells.push(Cell::new(&header));
            }
            let mut table = Table::new();
            table.add_row(Row::new(headers_cells));
            table.add_row(Row::new(vec![
                Cell::new("Runtime"),
                Cell::new(&cost.total.runtime.to_string()),
                Cell::new(&cost.limit.runtime.to_string()),
            ]));
            table.add_row(Row::new(vec![
                Cell::new("Read count"),
                Cell::new(&cost.total.read_count.to_string()),
                Cell::new(&cost.limit.read_count.to_string()),
            ]));
            table.add_row(Row::new(vec![
                Cell::new("Read length (bytes)"),
                Cell::new(&cost.total.read_length.to_string()),
                Cell::new(&cost.limit.read_length.to_string()),
            ]));
            table.add_row(Row::new(vec![
                Cell::new("Write count"),
                Cell::new(&cost.total.write_count.to_string()),
                Cell::new(&cost.limit.write_count.to_string()),
            ]));
            table.add_row(Row::new(vec![
                Cell::new("Write length (bytes)"),
                Cell::new(&cost.total.write_length.to_string()),
                Cell::new(&cost.limit.write_length.to_string()),
            ]));
            output.push(format!("{}", table));
        }
        output.append(&mut result);
    }

    #[cfg(feature = "cli")]
    fn get_accounts(&self, output: &mut Vec<String>) {
        let accounts = self.interpreter.get_accounts();
        if accounts.len() > 0 {
            let tokens = self.interpreter.get_tokens();
            let mut headers = vec!["Address".to_string()];
            headers.append(&mut tokens.clone());
            let mut headers_cells = vec![];
            for header in headers.iter() {
                headers_cells.push(Cell::new(&header));
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
        if self.contracts.len() > 0 {
            let mut table = Table::new();
            table.add_row(row!["Contract identifier", "Public functions"]);
            let contracts = self.contracts.clone();
            for (contract_id, methods) in contracts.iter() {
                if !contract_id.starts_with(&BOOT_TESTNET_ADDRESS)
                    && !contract_id.starts_with(&BOOT_MAINNET_ADDRESS)
                {
                    let mut formatted_methods = vec![];
                    for (method_name, method_args) in methods.iter() {
                        let formatted_args = if method_args.len() == 0 {
                            format!("")
                        } else if method_args.len() == 1 {
                            format!(" {}", method_args.join(" "))
                        } else {
                            format!("\n    {}", method_args.join("\n    "))
                        };
                        formatted_methods.push(format!("({}{})", method_name, formatted_args));
                    }
                    let formatted_spec = format!("{}", formatted_methods.join("\n"));
                    table.add_row(Row::new(vec![
                        Cell::new(&contract_id),
                        Cell::new(&formatted_spec),
                    ]));
                }
            }
            output.push(format!("{}", table));
        }
    }

    #[cfg(not(feature = "cli"))]
    pub fn get_costs(&mut self, output: &mut Vec<String>, cmd: &str) {
        let snippet = cmd.to_string().split_off("::get_costs ".len());
        let (mut result, cost) =
            match self.formatted_interpretation(snippet, None, true, None, None) {
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
        if self.settings.initial_accounts.len() > 0 {
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
        for (contract_id, methods) in self.contracts.iter() {
            if !contract_id.ends_with(".pox")
                && !contract_id.ends_with(".bns")
                && !contract_id.ends_with(".costs")
            {
                output.push(format!("{}", contract_id));
            }
        }
    }

    fn mint_stx(&mut self, output: &mut Vec<String>, command: &str) {
        let args: Vec<_> = command.split(' ').collect();

        if args.len() != 3 {
            output.push(red!("Usage: ::mint_stx <recipient address> <amount>"));
            return;
        }

        let recipient = match PrincipalData::parse(&args[1]) {
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
        let help_accent_colour = Colour::Yellow.bold();
        let keyword = {
            let mut s = command.to_string();
            s = s.replace("::describe_function", "");
            s = s.replace(" ", "");
            s
        };
        let result = match self.lookup_api_reference(&keyword) {
            Some(doc) => format!("{}", help_colour.paint(doc)),
            None => format!("{}", help_colour.paint("Function unknown")),
        };
        output.push(result);
    }

    pub fn display_digest(&self) -> Result<String, String> {
        let mut output = vec![];
        self.get_contracts(&mut output);
        self.get_accounts(&mut output);
        Ok(output.join("\n"))
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

fn decode_hex(byteString: &str) -> Result<Vec<u8>, DecodeHexError> {
    let byteStringFiltered: String = byteString
        .strip_prefix("0x")
        .unwrap_or(byteString)
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    if byteStringFiltered.len() % 2 != 0 {
        return Err(DecodeHexError::OddLength);
    }
    let result: Result<Vec<u8>, ParseIntError> = (0..byteStringFiltered.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&byteStringFiltered[i..i + 2], 16))
        .collect();
    match result {
        Ok(result) => Ok(result),
        Err(e) => Err(DecodeHexError::ParseError(e)),
    }
}

fn build_api_reference() -> HashMap<String, String> {
    let mut api_reference = HashMap::new();
    for func in NativeFunctions::ALL.iter() {
        let api = make_api_reference(&func);
        let description = {
            let mut s = api.description.to_string();
            s = s.replace("\n", " ");
            s
        };
        let doc = format!(
            "Usage\n{}\n\nDescription\n{}\n\nExamples\n{}",
            api.signature, description, api.example
        );
        api_reference.insert(api.name, doc);
    }

    for func in DefineFunctions::ALL.iter() {
        let api = make_define_reference(&func);
        let description = {
            let mut s = api.description.to_string();
            s = s.replace("\n", " ");
            s
        };
        let doc = format!(
            "Usage\n{}\n\nDescription\n{}\n\nExamples\n{}",
            api.signature, description, api.example
        );
        api_reference.insert(api.name, doc);
    }

    for func in NativeVariables::ALL.iter() {
        let api = make_keyword_reference(&func);
        let description = {
            let mut s = api.description.to_string();
            s = s.replace("\n", " ");
            s
        };
        let doc = format!("Description\n{}\n\nExamples\n{}", description, api.example);
        api_reference.insert(api.name.to_string(), doc);
    }
    api_reference
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn encode_error() {
        let mut session = Session::new(SessionSettings::default());
        let mut output: Vec<String> = Vec::new();
        session.encode(&mut output, "::encode { foo false }");
        assert_eq!(
            output[0],
            format!(
                "encode:1:7: {}: expected ':' after key in tuple",
                red!("error")
            )
        );

        session.encode(&mut output, "::encode (foo 1)");
        assert_eq!(
            output[4],
            format!(
                "encode:1:1: {}: use of unresolved function 'foo'",
                red!("error")
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
            red!("Failed to decode clarity value: Deserialization error: Bad type prefix")
        );

        session.decode(&mut output, "::decode 4g");
        assert_eq!(output.len(), 2);
        assert_eq!(
            output[1],
            red!("Parsing error: invalid digit found in string")
        );
    }

    #[test]
    fn evaluate_at_block() {
        let mut settings = SessionSettings::default();
        settings.include_boot_contracts = vec!["costs-v1".into()];
        settings.repl_settings.costs_version = 1;

        let mut session = Session::new(settings);
        session.start().expect("session could not start");

        // setup contract state
        session.handle_command(
            "
            (define-data-var x uint u0)

            (define-read-only (get-x)
                (var-get x))
            
            (define-public (incr)
                (begin
                    (var-set x (+ (var-get x) u1))
                    (ok (var-get x))))",
        );

        // assert data-var is set to 0
        assert_eq!(
            session.handle_command("(contract-call? .contract-2 get-x)")[0],
            green!("u0")
        );

        // advance chain tip and test at-block
        session.advance_chain_tip(10000);
        assert_eq!(
            session.handle_command("(contract-call? .contract-2 get-x)")[0],
            green!("u0")
        );
        session.handle_command("(contract-call? .contract-2 incr)");
        assert_eq!(
            session.handle_command("(contract-call? .contract-2 get-x)")[0],
            green!("u1")
        );
        assert_eq!(session.handle_command("(at-block (unwrap-panic (get-block-info? id-header-hash u0)) (contract-call? .contract-2 get-x))")[0], green!("u0"));
        assert_eq!(session.handle_command("(at-block (unwrap-panic (get-block-info? id-header-hash u5000)) (contract-call? .contract-2 get-x))")[0], green!("u0"));

        // advance chain tip again and test at-block
        // do this twice to make sure that the lookup table is being updated properly
        session.advance_chain_tip(10);
        session.advance_chain_tip(10);

        assert_eq!(
            session.handle_command("(contract-call? .contract-2 get-x)")[0],
            green!("u1")
        );
        session.handle_command("(contract-call? .contract-2 incr)");
        assert_eq!(
            session.handle_command("(contract-call? .contract-2 get-x)")[0],
            green!("u2")
        );
        assert_eq!(session.handle_command("(at-block (unwrap-panic (get-block-info? id-header-hash u10000)) (contract-call? .contract-2 get-x))")[0], green!("u1"));
    }
}

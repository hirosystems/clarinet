use std::collections::{btree_map::Entry, BTreeMap, BTreeSet};

use crate::analysis::annotation::{Annotation, AnnotationKind};
use crate::analysis::ast_dependency_detector::{ASTDependencyDetector, Dependency};
use crate::analysis::{self};
use crate::repl::datastore::BurnDatastore;
use crate::repl::datastore::Datastore;
use crate::repl::Settings;
use clar2wasm::Module;
use clarity::consts::CHAIN_ID_TESTNET;
use clarity::vm::analysis::ContractAnalysis;
use clarity::vm::ast::{build_ast_with_diagnostics, ContractAST};
use clarity::vm::clarity_wasm::{call_function, initialize_contract};
use clarity::vm::contexts::{CallStack, ContractContext, Environment, GlobalContext, LocalContext};
use clarity::vm::contracts::Contract;

use clarity::vm::costs::{ExecutionCost, LimitedCostTracker};
use clarity::vm::database::{ClarityDatabase, StoreType};
use clarity::vm::diagnostic::{Diagnostic, Level};
use clarity::vm::events::*;
use clarity::vm::representations::SymbolicExpressionType::{Atom, List};
use clarity::vm::representations::{Span, SymbolicExpression};
use clarity::vm::types::{
    PrincipalData, QualifiedContractIdentifier, StandardPrincipalData, Value,
};
use clarity::vm::{analysis::AnalysisDatabase, database::ClarityBackingStore};
use clarity::vm::{eval, eval_all, EvaluationResult, SnippetEvaluationResult};
use clarity::vm::{ContractEvaluationResult, EvalHook};
use clarity::vm::{CostSynthesis, ExecutionResult, ParsedContract};

use super::datastore::StacksConstants;
use super::{ClarityContract, ContractDeployer, DEFAULT_EPOCH};

pub const BLOCK_LIMIT_MAINNET: ExecutionCost = ExecutionCost {
    write_length: 15_000_000,
    write_count: 15_000,
    read_length: 100_000_000,
    read_count: 15_000,
    runtime: 5_000_000_000,
};

#[derive(Clone, Debug)]
pub struct ClarityInterpreter {
    pub datastore: Datastore,
    pub burn_datastore: BurnDatastore,
    tx_sender: StandardPrincipalData,
    accounts: BTreeSet<String>,
    tokens: BTreeMap<String, BTreeMap<String, u128>>,
    repl_settings: Settings,
}

#[derive(Debug)]
pub struct Txid(pub [u8; 32]);

trait Equivalent {
    fn equivalent(&self, other: &Self) -> bool;
}

impl Equivalent for SymbolicExpression {
    fn equivalent(&self, other: &Self) -> bool {
        use clarity::vm::representations::SymbolicExpressionType::*;
        match (&self.expr, &other.expr) {
            (AtomValue(a), AtomValue(b)) => a == b,
            (Atom(a), Atom(b)) => a == b,
            (List(a), List(b)) => {
                if a.len() != b.len() {
                    return false;
                }
                for i in 0..a.len() {
                    if !a[i].equivalent(&b[i]) {
                        return false;
                    }
                }
                true
            }
            (LiteralValue(a), LiteralValue(b)) => a == b,
            (Field(a), Field(b)) => a == b,
            (TraitReference(a_name, a_trait), TraitReference(b_name, b_trait)) => {
                a_name == b_name && a_trait == b_trait
            }
            _ => false,
        }
    }
}

impl Equivalent for ContractAST {
    fn equivalent(&self, other: &Self) -> bool {
        if self.expressions.len() != other.expressions.len() {
            return false;
        }

        for i in 0..self.expressions.len() {
            if !self.expressions[i].equivalent(&other.expressions[i]) {
                return false;
            }
        }
        true
    }
}

impl ClarityInterpreter {
    pub fn new(tx_sender: StandardPrincipalData, repl_settings: Settings) -> Self {
        let constants = StacksConstants {
            burn_start_height: 0,
            pox_prepare_length: 0,
            pox_reward_cycle_length: 0,
            pox_rejection_fraction: 0,
            epoch_21_start_height: 0,
        };
        Self {
            tx_sender,
            repl_settings,
            datastore: Datastore::new(),
            accounts: BTreeSet::new(),
            tokens: BTreeMap::new(),
            burn_datastore: BurnDatastore::new(constants),
        }
    }

    pub fn run_both(
        &mut self,
        contract: &ClarityContract,
        ast: &mut Option<ContractAST>,
        cost_track: bool,
        eval_hooks: Option<Vec<&mut dyn EvalHook>>,
    ) -> Result<ExecutionResult, Vec<Diagnostic>> {
        let mut contract_wasm = contract.clone();
        contract_wasm.deployer =
            ContractDeployer::Address("ST3NBRSFKX28FQ2ZJ1MAKX58HKHSDGNV5N7R21XCP".into());

        let start_run = std::time::Instant::now();
        let result = self.run(contract, ast, cost_track, eval_hooks);
        #[allow(unused_variables)]
        let time_run = start_run.elapsed();

        let start_run_wasm = std::time::Instant::now();
        let result_wasm = self.run_wasm(&contract_wasm, ast, cost_track, None);
        #[allow(unused_variables)]
        let time_run_wasm = start_run_wasm.elapsed();

        // println!("time taken for run_wasm: {:?}", time_run_wasm);
        // println!("time taken for run: {:?}", time_run);

        // let ratio = time_run_wasm.as_nanos() / time_run.as_nanos();
        // if time_run_wasm < time_run {
        //     println!("run_wasm {:?}x times faster", ratio);
        // } else {
        //     println!("run_wasm {:?}x times slower", ratio);
        // }

        #[allow(clippy::single_match)]
        match (result.clone(), result_wasm) {
            (Ok(result), Ok(result_wasm)) => {
                let value = match result.result {
                    EvaluationResult::Contract(contract_result) => contract_result.result,
                    EvaluationResult::Snippet(snippet_result) => Some(snippet_result.result),
                };
                let value_wasm = match result_wasm.result {
                    EvaluationResult::Contract(contract_result) => contract_result.result,
                    EvaluationResult::Snippet(snippet_result) => Some(snippet_result.result),
                };
                if value != value_wasm {
                    println!("values do not match");
                    println!("value: {:?}", value);
                    println!("value_wasm: {:?}", value_wasm);
                };
            }
            // @TODO: handle other cases
            _ => (),
        };

        result
    }

    pub fn run(
        &mut self,
        contract: &ClarityContract,
        ast: &mut Option<ContractAST>,
        cost_track: bool,
        eval_hooks: Option<Vec<&mut dyn EvalHook>>,
    ) -> Result<ExecutionResult, Vec<Diagnostic>> {
        let (mut ast, mut diagnostics, success) = match ast {
            Some(ast) => (ast.clone(), vec![], true),
            None => self.build_ast(contract),
        };

        let code_source = contract.expect_in_memory_code_source();

        let (annotations, mut annotation_diagnostics) = self.collect_annotations(code_source);
        diagnostics.append(&mut annotation_diagnostics);

        let (analysis, mut analysis_diagnostics) =
            match self.run_analysis(contract, &mut ast, &annotations) {
                Ok((analysis, diagnostics)) => (analysis, diagnostics),
                Err(diagnostic) => {
                    diagnostics.push(diagnostic);
                    return Err(diagnostics.to_vec());
                }
            };
        diagnostics.append(&mut analysis_diagnostics);

        if !success {
            return Err(diagnostics.to_vec());
        }

        let mut result =
            match self.execute(contract, &mut ast, analysis, None, cost_track, eval_hooks) {
                Ok(result) => result,
                Err(e) => {
                    diagnostics.push(Diagnostic {
                        level: Level::Error,
                        message: format!("Runtime Error: {}", e),
                        spans: vec![],
                        suggestion: None,
                    });
                    return Err(diagnostics.to_vec());
                }
            };

        result.diagnostics = diagnostics.to_vec();

        // todo: instead of just returning the value, we should be returning:
        // - value
        // - execution cost
        // - events emitted
        Ok(result)
    }

    pub fn run_wasm(
        &mut self,
        contract: &ClarityContract,
        ast: &mut Option<ContractAST>,
        cost_track: bool,
        eval_hooks: Option<Vec<&mut dyn EvalHook>>,
    ) -> Result<ExecutionResult, Vec<Diagnostic>> {
        use clar2wasm::{compile, compile_contract, CompileError, CompileResult};

        let contract_id = contract.expect_resolved_contract_identifier(Some(&self.tx_sender));
        let source = contract.expect_in_memory_code_source();
        let mut analysis_db = AnalysisDatabase::new(&mut self.datastore);

        let (mut ast, mut diagnostics, analysis, module) = match ast.take() {
            Some(mut ast) => {
                let mut diagnostics = vec![];
                let (annotations, annotation_diagnostics) =
                    self.collect_annotations(contract.expect_in_memory_code_source());
                diagnostics.extend(annotation_diagnostics);

                let (analysis, analysis_diagnostics) =
                    match self.run_analysis(contract, &mut ast, &annotations) {
                        Ok((analysis, diagnostics)) => (analysis, diagnostics),
                        Err(diagnostic) => {
                            diagnostics.push(diagnostic);
                            return Err(diagnostics.to_vec());
                        }
                    };
                diagnostics.extend(analysis_diagnostics);

                let module = match compile_contract(analysis.clone()) {
                    Ok(res) => res,
                    Err(e) => {
                        diagnostics.push(Diagnostic {
                            level: Level::Error,
                            message: format!("Wasm Generator Error: {:?}", e),
                            spans: vec![],
                            suggestion: None,
                        });
                        return Err(diagnostics);
                    }
                };
                (ast, diagnostics, analysis, module)
            }
            None => {
                // counter.clar
                // contract2.clar
                let CompileResult {
                    mut ast,
                    mut diagnostics,
                    module,
                    contract_analysis: _,
                } = match compile(
                    source,
                    &contract_id,
                    LimitedCostTracker::new_free(),
                    contract.clarity_version,
                    contract.epoch,
                    &mut analysis_db,
                ) {
                    Ok(res) => res,
                    Err(CompileError::Generic { diagnostics, .. }) => return Err(diagnostics),
                };
                let (annotations, mut annotation_diagnostics) =
                    self.collect_annotations(contract.expect_in_memory_code_source());
                diagnostics.append(&mut annotation_diagnostics);

                let (analysis, mut analysis_diagnostics) =
                    match self.run_analysis(contract, &mut ast, &annotations) {
                        Ok((analysis, diagnostics)) => (analysis, diagnostics),
                        Err(diagnostic) => {
                            diagnostics.push(diagnostic);
                            return Err(diagnostics);
                        }
                    };
                diagnostics.append(&mut analysis_diagnostics);
                (ast, diagnostics, analysis, module)
            }
        };

        let mut result = match self.execute(
            contract,
            &mut ast,
            analysis,
            Some(module),
            cost_track,
            eval_hooks,
        ) {
            Ok(result) => result,
            Err(e) => {
                diagnostics.push(Diagnostic {
                    level: Level::Error,
                    message: format!("Wasm Runtime Error: {}", e),
                    spans: vec![],
                    suggestion: None,
                });
                return Err(diagnostics.to_vec());
            }
        };

        result.diagnostics = diagnostics;

        // todo: instead of just returning the value, we should be returning:
        // - value, execution cost, events emitted
        Ok(result)
    }

    pub fn detect_dependencies(
        &mut self,
        contract: &ClarityContract,
    ) -> Result<Vec<Dependency>, String> {
        let contract_id = contract.expect_resolved_contract_identifier(Some(&self.tx_sender));
        let (ast, _, success) = self.build_ast(contract);
        if !success {
            return Err("error parsing source".to_string());
        }

        let mut contract_map = BTreeMap::new();
        contract_map.insert(contract_id.clone(), (contract.clarity_version, ast));
        let mut all_dependencies =
            match ASTDependencyDetector::detect_dependencies(&contract_map, &BTreeMap::new()) {
                Ok(dependencies) => dependencies,
                Err((_, unresolved)) => {
                    return Err(format!(
                        "unresolved dependency(ies): {}",
                        unresolved
                            .iter()
                            .map(|contract_id| contract_id.to_string())
                            .collect::<Vec<String>>()
                            .join(",")
                    ));
                }
            };
        let mut dependencies = vec![];
        if let Some(dependencies_set) = all_dependencies.remove(&contract_id) {
            dependencies.extend(dependencies_set.set);
        }
        Ok(dependencies)
    }

    pub fn build_ast(&self, contract: &ClarityContract) -> (ContractAST, Vec<Diagnostic>, bool) {
        let source_code = contract.expect_in_memory_code_source();
        let contract_id = contract.expect_resolved_contract_identifier(Some(&self.tx_sender));
        build_ast_with_diagnostics(
            &contract_id,
            source_code,
            &mut (),
            contract.clarity_version,
            contract.epoch,
        )
    }

    pub fn collect_annotations(&self, code_source: &str) -> (Vec<Annotation>, Vec<Diagnostic>) {
        let mut annotations = vec![];
        let mut diagnostics = vec![];
        for (n, line) in code_source.lines().enumerate() {
            if let Some(comment) = line.trim().strip_prefix(";;") {
                if let Some(annotation_string) = comment.trim().strip_prefix("#[") {
                    let span = Span {
                        start_line: (n + 1) as u32,
                        start_column: (line.find('#').unwrap_or(0) + 1) as u32,
                        end_line: (n + 1) as u32,
                        end_column: line.len() as u32,
                    };
                    if let Some(annotation_string) = annotation_string.strip_suffix(']') {
                        let kind: AnnotationKind = match annotation_string.trim().parse() {
                            Ok(kind) => kind,
                            Err(e) => {
                                diagnostics.push(Diagnostic {
                                    level: Level::Warning,
                                    message: e.to_string(),
                                    spans: vec![span.clone()],
                                    suggestion: None,
                                });
                                continue;
                            }
                        };
                        annotations.push(Annotation { kind, span });
                    } else {
                        diagnostics.push(Diagnostic {
                            level: Level::Warning,
                            message: "malformed annotation".to_string(),
                            spans: vec![span],
                            suggestion: None,
                        });
                    }
                }
            }
        }
        (annotations, diagnostics)
    }

    pub fn run_analysis(
        &mut self,
        contract: &ClarityContract,
        contract_ast: &mut ContractAST,
        annotations: &Vec<Annotation>,
    ) -> Result<(ContractAnalysis, Vec<Diagnostic>), Diagnostic> {
        let mut analysis_db = AnalysisDatabase::new(&mut self.datastore);

        // Run standard clarity analyses
        let mut contract_analysis = clarity::vm::analysis::run_analysis(
            &contract.expect_resolved_contract_identifier(Some(&self.tx_sender)),
            &mut contract_ast.expressions,
            &mut analysis_db,
            false,
            LimitedCostTracker::new_free(),
            contract.epoch,
            contract.clarity_version,
        )
        .map_err(|(error, _)| error.diagnostic)?;

        // Run REPL-only analyses
        let diagnostics = analysis::run_analysis(
            &mut contract_analysis,
            &mut analysis_db,
            annotations,
            &self.repl_settings.analysis,
        )
        .map_err(|mut diagnostics| diagnostics.pop().unwrap())?;

        Ok((contract_analysis, diagnostics))
    }

    pub fn save_contract(
        &mut self,
        contract: &ClarityContract,
        contract_ast: &mut ContractAST,
        contract_analysis: ContractAnalysis,
        mainnet: bool,
    ) {
        let contract_id = contract.expect_resolved_contract_identifier(Some(&self.tx_sender));
        {
            let mut contract_context = ContractContext::new(
                contract.expect_resolved_contract_identifier(Some(&self.tx_sender)),
                contract.clarity_version,
            );

            let conn = ClarityDatabase::new(
                &mut self.datastore,
                &self.burn_datastore,
                &self.burn_datastore,
            );

            let cost_tracker = LimitedCostTracker::new_free();
            let mut global_context = GlobalContext::new(
                mainnet,
                clarity::consts::CHAIN_ID_TESTNET,
                conn,
                cost_tracker,
                contract.epoch,
            );
            global_context.begin();

            let _ = global_context
                .execute(|g| eval_all(&contract_ast.expressions, &mut contract_context, g, None));

            global_context
                .database
                .insert_contract_hash(&contract_id, contract.expect_in_memory_code_source())
                .unwrap();
            let contract = Contract { contract_context };
            global_context
                .database
                .insert_contract(&contract_id, contract);
            global_context
                .database
                .set_contract_data_size(&contract_id, 0)
                .unwrap();
            global_context.commit().unwrap();
        };

        let mut analysis_db = AnalysisDatabase::new(&mut self.datastore);
        analysis_db.begin();
        analysis_db
            .insert_contract(&contract_id, &contract_analysis)
            .unwrap();
        analysis_db.commit();
    }

    pub fn get_data_var(
        &mut self,
        contract_id: &QualifiedContractIdentifier,
        var_name: &str,
    ) -> Option<String> {
        let key = ClarityDatabase::make_key_for_trip(contract_id, StoreType::Variable, var_name);
        let value_hex = self.datastore.get(&key)?;
        Some(format!("0x{value_hex}"))
    }

    pub fn get_map_entry(
        &mut self,
        contract_id: &QualifiedContractIdentifier,
        map_name: &str,
        map_key: &Value,
    ) -> Option<String> {
        let key = ClarityDatabase::make_key_for_data_map_entry(contract_id, map_name, map_key);
        let value_hex = self.datastore.get(&key)?;
        Some(format!("0x{value_hex}"))
    }

    pub fn execute(
        &mut self,
        contract: &ClarityContract,
        contract_ast: &mut ContractAST,
        contract_analysis: ContractAnalysis,
        wasm_module: Option<Module>,
        cost_track: bool,
        eval_hooks: Option<Vec<&mut dyn EvalHook>>,
    ) -> Result<ExecutionResult, String> {
        let contract_id = contract.expect_resolved_contract_identifier(Some(&self.tx_sender));
        let snippet = contract.expect_in_memory_code_source();
        let mut contract_context =
            ContractContext::new(contract_id.clone(), contract.clarity_version);

        let mut conn = ClarityDatabase::new(
            &mut self.datastore,
            &self.burn_datastore,
            &self.burn_datastore,
        );
        let tx_sender: PrincipalData = self.tx_sender.clone().into();
        conn.begin();
        conn.set_clarity_epoch_version(contract.epoch);
        conn.commit();
        let cost_tracker = if cost_track {
            LimitedCostTracker::new(
                false,
                CHAIN_ID_TESTNET,
                BLOCK_LIMIT_MAINNET.clone(),
                &mut conn,
                contract.epoch,
            )
            .expect("failed to initialize cost tracker")
        } else {
            LimitedCostTracker::new_free()
        };
        let mut global_context =
            GlobalContext::new(false, CHAIN_ID_TESTNET, conn, cost_tracker, contract.epoch);

        if let Some(mut in_hooks) = eval_hooks {
            let mut hooks: Vec<&mut dyn EvalHook> = Vec::new();
            for hook in in_hooks.drain(..) {
                hooks.push(hook);
            }
            global_context.eval_hooks = Some(hooks);
        }

        global_context.begin();

        let result = global_context.execute(|g| {
            if contract_ast.expressions.len() == 1 && !snippet.contains("(define-") {
                let context = LocalContext::new();
                let mut call_stack = CallStack::new();
                let mut env = Environment::new(
                    g,
                    &contract_context,
                    &mut call_stack,
                    Some(tx_sender.clone()),
                    Some(tx_sender.clone()),
                    None,
                );

                let result = match contract_ast.expressions[0].expr {
                    List(ref expression) => match expression[0].expr {
                        Atom(ref name) if name.to_string() == "contract-call?" => {
                            let contract_id = match expression[1]
                                .match_literal_value()
                                .unwrap()
                                .clone()
                                .expect_principal()
                            {
                                PrincipalData::Contract(contract_id) => contract_id,
                                _ => unreachable!(),
                            };
                            let method = expression[2].match_atom().unwrap().to_string();
                            let mut args = vec![];
                            for arg in expression[3..].iter() {
                                let evaluated_arg = eval(arg, &mut env, &context)?;
                                args.push(evaluated_arg);
                            }

                            let called_contract =
                                env.global_context.database.get_contract(&contract_id)?;
                            match wasm_module {
                                Some(_) => {
                                    // CLAR2WASM
                                    let start = std::time::Instant::now();

                                    let res = match call_function(
                                        &method,
                                        &args,
                                        g,
                                        &called_contract.contract_context,
                                        &mut call_stack,
                                        Some(StandardPrincipalData::transient().into()),
                                        Some(StandardPrincipalData::transient().into()),
                                        None,
                                    ) {
                                        Ok(res) => res,
                                        Err(e) => {
                                            println!("Error while calling function: {:?}", e);
                                            return Err(e);
                                        }
                                    };
                                    println!("execute wasm: {:?}", start.elapsed());
                                    Ok(res)
                                }
                                None => {
                                    // INTERPRETER
                                    let start = std::time::Instant::now();
                                    let args: Vec<SymbolicExpression> = args
                                        .iter()
                                        .map(|a| SymbolicExpression::atom_value(a.clone()))
                                        .collect();
                                    let res =
                                        env.execute_contract(&contract_id, &method, &args, false)?;
                                    println!("execute intr: {:?}", start.elapsed());
                                    Ok(res)
                                }
                            }
                        }
                        _ => eval(&contract_ast.expressions[0], &mut env, &context),
                    },
                    _ => eval(&contract_ast.expressions[0], &mut env, &context),
                };
                result.map(Some)
            } else {
                match wasm_module {
                    Some(mut wasm_module) => {
                        contract_context.set_wasm_module(wasm_module.emit_wasm());
                        initialize_contract(g, &mut contract_context, None, &contract_analysis)
                    }
                    None => eval_all(&contract_ast.expressions, &mut contract_context, g, None),
                }
            }
        });

        let value = result.map_err(|e| {
            let err = format!("Runtime error while interpreting {}: {:?}", contract_id, e);
            if let Some(mut eval_hooks) = global_context.eval_hooks.take() {
                for hook in eval_hooks.iter_mut() {
                    hook.did_complete(Err(err.clone()));
                }
                global_context.eval_hooks = Some(eval_hooks);
            }
            err
        })?;

        let mut cost = None;
        if cost_track {
            cost = Some(CostSynthesis::from_cost_tracker(&global_context.cost_track));
        }

        let mut emitted_events = global_context
            .event_batches
            .iter()
            .flat_map(|b| b.events.clone())
            .collect::<Vec<_>>();

        let contract_saved =
            !contract_context.functions.is_empty() || !contract_context.defined_traits.is_empty();

        let eval_result = if contract_saved {
            let mut functions = BTreeMap::new();
            for (name, defined_func) in contract_context.functions.iter() {
                if !defined_func.is_public() {
                    continue;
                }

                let args: Vec<_> = defined_func
                    .get_arguments()
                    .iter()
                    .zip(defined_func.get_arg_types().iter())
                    .map(|(n, t)| format!("({} {})", n.as_str(), t))
                    .collect();

                functions.insert(name.to_string(), args);
            }
            let parsed_contract = ParsedContract {
                contract_identifier: contract_id.to_string(),
                code: snippet.to_string(),
                function_args: functions,
                ast: contract_ast.clone(),
                analysis: contract_analysis.clone(),
            };

            global_context
                .database
                .insert_contract_hash(&contract_id, snippet)
                .unwrap();
            let contract = Contract { contract_context };
            global_context
                .database
                .insert_contract(&contract_id, contract);
            global_context
                .database
                .set_contract_data_size(&contract_id, 0)
                .unwrap();

            EvaluationResult::Contract(ContractEvaluationResult {
                result: value,
                contract: parsed_contract,
            })
        } else {
            let result = value.unwrap_or(Value::none());
            EvaluationResult::Snippet(SnippetEvaluationResult { result })
        };

        global_context.commit().unwrap();

        let (events, mut accounts_to_credit, mut accounts_to_debit) =
            Self::process_events(&mut emitted_events);

        let mut execution_result = ExecutionResult {
            result: eval_result,
            events,
            cost,
            diagnostics: Vec::new(),
        };

        if let Some(mut eval_hooks) = global_context.eval_hooks {
            for hook in eval_hooks.iter_mut() {
                hook.did_complete(Ok(&mut execution_result));
            }
        }

        for (account, token, value) in accounts_to_credit.drain(..) {
            self.credit_token(account, token, value);
        }

        for (account, token, value) in accounts_to_debit.drain(..) {
            self.debit_token(account, token, value);
        }

        if contract_saved {
            let mut analysis_db = AnalysisDatabase::new(&mut self.datastore);
            analysis_db
                .execute(|db| db.insert_contract(&contract_id, &contract_analysis))
                .expect("Unable to save data");
        }

        Ok(execution_result)
    }

    fn process_events(
        emitted_events: &mut Vec<StacksTransactionEvent>,
    ) -> (
        Vec<StacksTransactionEvent>,
        Vec<(String, String, u128)>,
        Vec<(String, String, u128)>,
    ) {
        let mut events = vec![];
        let mut accounts_to_debit = vec![];
        let mut accounts_to_credit = vec![];
        for event in emitted_events.drain(..) {
            match event {
                StacksTransactionEvent::STXEvent(STXEventType::STXTransferEvent(
                    ref event_data,
                )) => {
                    accounts_to_debit.push((
                        event_data.sender.to_string(),
                        "STX".to_string(),
                        event_data.amount,
                    ));
                    accounts_to_credit.push((
                        event_data.recipient.to_string(),
                        "STX".to_string(),
                        event_data.amount,
                    ));
                }
                StacksTransactionEvent::STXEvent(STXEventType::STXMintEvent(ref event_data)) => {
                    accounts_to_credit.push((
                        event_data.recipient.to_string(),
                        "STX".to_string(),
                        event_data.amount,
                    ));
                }
                StacksTransactionEvent::STXEvent(STXEventType::STXBurnEvent(ref event_data)) => {
                    accounts_to_debit.push((
                        event_data.sender.to_string(),
                        "STX".to_string(),
                        event_data.amount,
                    ));
                }
                StacksTransactionEvent::FTEvent(FTEventType::FTTransferEvent(ref event_data)) => {
                    accounts_to_credit.push((
                        event_data.recipient.to_string(),
                        event_data.asset_identifier.sugared(),
                        event_data.amount,
                    ));
                    accounts_to_debit.push((
                        event_data.sender.to_string(),
                        event_data.asset_identifier.sugared(),
                        event_data.amount,
                    ));
                }
                StacksTransactionEvent::FTEvent(FTEventType::FTMintEvent(ref event_data)) => {
                    accounts_to_credit.push((
                        event_data.recipient.to_string(),
                        event_data.asset_identifier.sugared(),
                        event_data.amount,
                    ));
                }
                StacksTransactionEvent::FTEvent(FTEventType::FTBurnEvent(ref event_data)) => {
                    accounts_to_debit.push((
                        event_data.sender.to_string(),
                        event_data.asset_identifier.sugared(),
                        event_data.amount,
                    ));
                }
                StacksTransactionEvent::NFTEvent(NFTEventType::NFTTransferEvent(
                    ref event_data,
                )) => {
                    accounts_to_credit.push((
                        event_data.recipient.to_string(),
                        event_data.asset_identifier.sugared(),
                        1,
                    ));
                    accounts_to_debit.push((
                        event_data.sender.to_string(),
                        event_data.asset_identifier.sugared(),
                        1,
                    ));
                }
                StacksTransactionEvent::NFTEvent(NFTEventType::NFTMintEvent(ref event_data)) => {
                    accounts_to_credit.push((
                        event_data.recipient.to_string(),
                        event_data.asset_identifier.sugared(),
                        1,
                    ));
                }
                StacksTransactionEvent::NFTEvent(NFTEventType::NFTBurnEvent(ref event_data)) => {
                    accounts_to_debit.push((
                        event_data.sender.to_string(),
                        event_data.asset_identifier.sugared(),
                        1,
                    ));
                }
                _ => {}
            };
            events.push(event);
        }
        (events, accounts_to_credit, accounts_to_debit)
    }

    pub fn mint_stx_balance(
        &mut self,
        recipient: PrincipalData,
        amount: u64,
    ) -> Result<String, String> {
        let final_balance = {
            let conn = ClarityDatabase::new(
                &mut self.datastore,
                &self.burn_datastore,
                &self.burn_datastore,
            );

            let mut global_context = GlobalContext::new(
                false,
                CHAIN_ID_TESTNET,
                conn,
                LimitedCostTracker::new_free(),
                DEFAULT_EPOCH,
            );
            global_context.begin();
            let mut cur_balance = global_context.database.get_stx_balance_snapshot(&recipient);
            cur_balance.credit(amount as u128);
            let final_balance = cur_balance.get_available_balance();
            cur_balance.save();
            global_context
                .database
                .increment_ustx_liquid_supply(amount as u128)
                .unwrap();
            global_context.commit().unwrap();
            final_balance
        };
        self.credit_token(recipient.to_string(), "STX".to_string(), amount.into());
        Ok(format!("→ {}: {} µSTX", recipient, final_balance))
    }

    pub fn set_tx_sender(&mut self, tx_sender: StandardPrincipalData) {
        self.tx_sender = tx_sender;
    }

    pub fn get_tx_sender(&self) -> StandardPrincipalData {
        self.tx_sender.clone()
    }

    pub fn advance_chain_tip(&mut self, count: u32) -> u32 {
        self.burn_datastore.advance_chain_tip(count);
        self.datastore.advance_chain_tip(count)
    }

    pub fn get_block_height(&mut self) -> u32 {
        self.datastore.get_current_block_height()
    }

    fn credit_token(&mut self, account: String, token: String, value: u128) {
        self.accounts.insert(account.clone());
        match self.tokens.entry(token) {
            Entry::Occupied(balances) => {
                balances
                    .into_mut()
                    .entry(account)
                    .and_modify(|e| *e += value)
                    .or_insert(value);
            }
            Entry::Vacant(v) => {
                let mut balances = BTreeMap::new();
                balances.insert(account, value);
                v.insert(balances);
            }
        };
    }

    fn debit_token(&mut self, account: String, token: String, value: u128) {
        self.accounts.insert(account.clone());
        match self.tokens.entry(token) {
            Entry::Occupied(balances) => {
                balances
                    .into_mut()
                    .entry(account)
                    .and_modify(|e| *e -= value)
                    .or_insert(value);
            }
            Entry::Vacant(v) => {
                let mut balances = BTreeMap::new();
                balances.insert(account, value);
                v.insert(balances);
            }
        };
    }

    pub fn get_assets_maps(&self) -> BTreeMap<String, BTreeMap<String, u128>> {
        self.tokens.clone()
    }

    pub fn get_tokens(&self) -> Vec<String> {
        self.tokens.keys().cloned().collect()
    }

    pub fn get_accounts(&self) -> Vec<String> {
        self.accounts.clone().into_iter().collect::<Vec<_>>()
    }

    pub fn get_balance_for_account(&self, account: &str, token: &str) -> u128 {
        match self.tokens.get(token) {
            Some(balances) => match balances.get(account) {
                Some(value) => *value,
                _ => 0,
            },
            _ => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixtures::clarity_contract::ClarityContractBuilder;
    use clarity::{
        types::{chainstate::StacksAddress, Address},
        vm::{self},
    };

    #[test]
    fn test_get_tx_sender() {
        let mut interpreter =
            ClarityInterpreter::new(StandardPrincipalData::transient(), Settings::default());
        let tx_sender = StandardPrincipalData::transient();
        interpreter.set_tx_sender(tx_sender.clone());
        assert_eq!(interpreter.get_tx_sender(), tx_sender);
    }

    #[test]
    fn test_set_tx_sender() {
        let mut interpreter =
            ClarityInterpreter::new(StandardPrincipalData::transient(), Settings::default());

        let addr = StacksAddress::from_string("ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5").unwrap();
        let tx_sender = StandardPrincipalData::from(addr);
        interpreter.set_tx_sender(tx_sender.clone());
        assert_eq!(interpreter.get_tx_sender(), tx_sender);
    }

    #[test]
    fn test_get_block_height() {
        let mut interpreter =
            ClarityInterpreter::new(StandardPrincipalData::transient(), Settings::default());
        assert_eq!(interpreter.get_block_height(), 0);
    }

    #[test]
    fn test_advance_chain_tip() {
        let mut interpreter =
            ClarityInterpreter::new(StandardPrincipalData::transient(), Settings::default());
        let count = 5;
        let initial_block_height = interpreter.get_block_height();
        interpreter.advance_chain_tip(count);
        assert_eq!(interpreter.get_block_height(), initial_block_height + count);
    }

    #[test]
    fn test_get_assets_maps() {
        let mut interpreter =
            ClarityInterpreter::new(StandardPrincipalData::transient(), Settings::default());
        let addr = "ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5";
        let amount = 1000;
        interpreter.credit_token(addr.into(), "STX".into(), amount);

        let assets = interpreter.get_assets_maps();
        assert!(assets.contains_key("STX"));

        let stx = assets.get("STX").unwrap();
        assert!(stx.contains_key(addr));

        let balance = stx.get(addr).unwrap();
        assert_eq!(balance, &amount)
    }

    #[test]
    fn test_get_tokens() {
        let mut interpreter =
            ClarityInterpreter::new(StandardPrincipalData::transient(), Settings::default());
        let addr = "ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5";
        interpreter.credit_token(addr.into(), "STX".into(), 1000);

        let tokens = interpreter.get_tokens();
        assert_eq!(tokens, ["STX"]);
    }

    #[test]
    fn test_get_accounts() {
        let mut interpreter =
            ClarityInterpreter::new(StandardPrincipalData::transient(), Settings::default());
        let addr = "ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5";
        interpreter.credit_token(addr.into(), "STX".into(), 1000);

        let accounts = interpreter.get_accounts();
        assert_eq!(accounts, ["ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5"]);
    }

    #[test]
    fn test_get_balance_for_account() {
        let mut interpreter =
            ClarityInterpreter::new(StandardPrincipalData::transient(), Settings::default());

        let addr = "ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5";
        let amount = 1000;
        interpreter.credit_token(addr.into(), "STX".into(), amount);

        let balance = interpreter.get_balance_for_account(addr, "STX");
        assert_eq!(balance, amount);
    }

    #[test]
    fn test_credit_any_token() {
        let mut interpreter =
            ClarityInterpreter::new(StandardPrincipalData::transient(), Settings::default());

        let addr = "ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5";
        let amount = 1000;
        interpreter.credit_token(addr.into(), "MIA".into(), amount);

        let balance = interpreter.get_balance_for_account(addr, "MIA");
        assert_eq!(balance, amount);
    }

    #[test]
    fn test_mint_stx_balance() {
        let mut interpreter =
            ClarityInterpreter::new(StandardPrincipalData::transient(), Settings::default());
        let recipient = PrincipalData::Standard(StandardPrincipalData::transient());
        let amount = 1000;

        let result = interpreter.mint_stx_balance(recipient.clone(), amount);
        assert!(result.is_ok());

        let balance = interpreter.get_balance_for_account(&recipient.to_string(), "STX");
        assert_eq!(balance, amount.into());
    }

    #[test]
    fn test_run_valid_contract() {
        let mut interpreter =
            ClarityInterpreter::new(StandardPrincipalData::transient(), Settings::default());
        let contract = ClarityContract::fixture();
        let result = interpreter.run(&contract, &mut None, false, None);
        assert!(result.is_ok());
        assert!(result.unwrap().diagnostics.is_empty());
    }

    #[test]
    fn test_run_invalid_contract() {
        let mut interpreter =
            ClarityInterpreter::new(StandardPrincipalData::transient(), Settings::default());

        let snippet = "(define-public (add) (ok (+ u1 1)))";
        //                                            ^ should be uint
        let contract = ClarityContractBuilder::default()
            .code_source(snippet.into())
            .build();
        let result = interpreter.run(&contract, &mut None, false, None);
        assert!(result.is_err());
        let diagnostics = result.unwrap_err();
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_run_runtime_error() {
        let mut interpreter =
            ClarityInterpreter::new(StandardPrincipalData::transient(), Settings::default());

        let snippet = "(/ u1 u0)";
        let contract = ClarityContractBuilder::default()
            .code_source(snippet.into())
            .build();
        let result = interpreter.run(&contract, &mut None, false, None);
        assert!(result.is_err());

        let diagnostics = result.unwrap_err();
        assert_eq!(diagnostics.len(), 1);

        let message = format!("Runtime Error: Runtime error while interpreting {}.{}: Runtime(DivisionByZero, Some([FunctionIdentifier {{ identifier: \"_native_:native_div\" }}]))", StandardPrincipalData::transient().to_string(), contract.name);
        assert_eq!(
            diagnostics[0],
            Diagnostic {
                level: vm::diagnostic::Level::Error,
                message,
                spans: vec![],
                suggestion: None
            }
        );
    }

    #[test]
    fn test_build_ast() {
        let interpreter =
            ClarityInterpreter::new(StandardPrincipalData::transient(), Settings::default());
        let contract = ClarityContract::fixture();
        let (_ast, diagnostics, success) = interpreter.build_ast(&contract);
        assert!(success);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_execute() {
        let mut interpreter =
            ClarityInterpreter::new(StandardPrincipalData::transient(), Settings::default());

        let contract = ClarityContract::fixture();
        let source = contract.expect_in_memory_code_source();
        let (mut ast, ..) = interpreter.build_ast(&contract);
        let (annotations, _) = interpreter.collect_annotations(source);

        let (analysis, _) = interpreter
            .run_analysis(&contract, &mut ast, &annotations)
            .unwrap();

        let result = interpreter.execute(&contract, &mut ast, analysis, None, false, None);
        assert!(result.is_ok());
        let ExecutionResult {
            diagnostics,
            events,
            ..
        } = result.unwrap();
        assert!(diagnostics.is_empty());
        assert!(events.is_empty());
    }

    #[test]
    fn test_run_both() {
        let mut interpreter =
            ClarityInterpreter::new(StandardPrincipalData::transient(), Settings::default());

        let contract = ClarityContract::fixture();
        let _ = interpreter.run_both(&contract, &mut None, false, None);

        let call_contract = ClarityContractBuilder::default()
            .code_source("(contract-call? .contract incr)".to_owned())
            .build();
        let _ = interpreter.run_both(&call_contract, &mut None, false, None);
    }

    #[test]
    fn test_get_data_var() {
        let mut interpreter =
            ClarityInterpreter::new(StandardPrincipalData::transient(), Settings::default());
        let contract = ClarityContractBuilder::default()
            .code_source(["(define-data-var count uint u9)"].join("\n"))
            .build();
        let source = contract.expect_in_memory_code_source();
        let (mut ast, ..) = interpreter.build_ast(&contract);
        let (annotations, _) = interpreter.collect_annotations(source);
        let (analysis, _) = interpreter
            .run_analysis(&contract, &mut ast, &annotations)
            .unwrap();

        interpreter.save_contract(&contract, &mut ast, analysis, false);

        let contract_id = QualifiedContractIdentifier {
            issuer: StandardPrincipalData::transient(),
            name: "contract".into(),
        };
        let count = interpreter.get_data_var(&contract_id, &"count");

        assert_eq!(
            count,
            Some("0x0100000000000000000000000000000009".to_owned())
        )
    }

    #[test]
    fn test_get_map_entry() {
        let mut interpreter =
            ClarityInterpreter::new(StandardPrincipalData::transient(), Settings::default());
        let contract = ClarityContractBuilder::default()
            .code_source(
                [
                    "(define-map people uint (string-ascii 10))",
                    "(map-insert people u0 \"satoshi\")",
                ]
                .join("\n"),
            )
            .build();
        let source = contract.expect_in_memory_code_source();
        let (mut ast, ..) = interpreter.build_ast(&contract);
        let (annotations, _) = interpreter.collect_annotations(source);
        let (analysis, _) = interpreter
            .run_analysis(&contract, &mut ast, &annotations)
            .unwrap();

        interpreter.save_contract(&contract, &mut ast, analysis, false);

        let contract_id = QualifiedContractIdentifier {
            issuer: StandardPrincipalData::transient(),
            name: "contract".into(),
        };
        let name = interpreter.get_map_entry(&contract_id, &"people", &Value::UInt(0));
        assert_eq!(name, Some("0x0a0d000000077361746f736869".to_owned()));
        let no_name = interpreter.get_map_entry(&contract_id, &"people", &Value::UInt(404));
        assert_eq!(no_name, None);
    }

    #[test]
    fn test_execute_stx_events() {
        let mut interpreter =
            ClarityInterpreter::new(StandardPrincipalData::transient(), Settings::default());
        let account = PrincipalData::parse("S1G2081040G2081040G2081040G208105NK8PE5").unwrap();
        let _ = interpreter.mint_stx_balance(account, 100000);

        let contract = ClarityContractBuilder::default()
            .code_source(
                [
                    "(define-public (test-transfer)",
                    "  (ok (stx-transfer? u10 tx-sender (as-contract tx-sender))))",
                    "(define-public (test-burn)",
                    "  (ok (stx-burn? u10 tx-sender)))",
                    "(test-transfer)",
                    "(test-burn)",
                ]
                .join("\n"),
            )
            .build();
        let source = contract.expect_in_memory_code_source();
        let (mut ast, ..) = interpreter.build_ast(&contract);
        let (annotations, _) = interpreter.collect_annotations(source);

        let (analysis, _) = interpreter
            .run_analysis(&contract, &mut ast, &annotations)
            .unwrap();

        let account =
            PrincipalData::parse_standard_principal("S1G2081040G2081040G2081040G208105NK8PE5")
                .unwrap();
        let balance = interpreter.get_balance_for_account(&account.to_string(), "STX");
        assert_eq!(balance, 100000);

        let result = interpreter.execute(&contract, &mut ast, analysis, None, false, None);
        assert!(result.is_ok());

        let ExecutionResult {
            diagnostics,
            events,
            ..
        } = result.unwrap();
        assert!(diagnostics.is_empty());
        assert_eq!(events.len(), 2);

        let balance = interpreter.get_balance_for_account(&account.to_string(), "STX");
        assert_eq!(balance, 99980);

        assert!(matches!(
            events[0],
            StacksTransactionEvent::STXEvent(STXEventType::STXTransferEvent(_))
        ));
        assert!(matches!(
            events[1],
            StacksTransactionEvent::STXEvent(STXEventType::STXBurnEvent(_))
        ));
    }

    #[test]
    fn test_execute_ft_events() {
        let mut interpreter =
            ClarityInterpreter::new(StandardPrincipalData::transient(), Settings::default());

        let contract = ClarityContractBuilder::default()
            .code_source(
                [
                    "(define-fungible-token ctb)",
                    "(define-private (test-mint)",
                    "  (ft-mint? ctb u100 tx-sender))",
                    "(define-private (test-burn)",
                    "  (ft-burn? ctb u10 tx-sender))",
                    "(define-private (test-transfer)",
                    "  (ft-transfer? ctb u10 tx-sender (as-contract tx-sender)))",
                    "(test-mint)",
                    "(test-burn)",
                    "(test-transfer)",
                ]
                .join("\n"),
            )
            .build();
        let source = contract.expect_in_memory_code_source();
        let (mut ast, ..) = interpreter.build_ast(&contract);
        let (annotations, _) = interpreter.collect_annotations(source);

        let (analysis, _) = interpreter
            .run_analysis(&contract, &mut ast, &annotations)
            .unwrap();

        let result = interpreter.execute(&contract, &mut ast, analysis, None, false, None);
        assert!(result.is_ok());
        let ExecutionResult {
            diagnostics,
            events,
            ..
        } = result.unwrap();
        assert!(diagnostics.is_empty());
        assert_eq!(events.len(), 3);
        assert!(matches!(
            events[0],
            StacksTransactionEvent::FTEvent(FTEventType::FTMintEvent(_))
        ));
        assert!(matches!(
            events[1],
            StacksTransactionEvent::FTEvent(FTEventType::FTBurnEvent(_))
        ));
        assert!(matches!(
            events[2],
            StacksTransactionEvent::FTEvent(FTEventType::FTTransferEvent(_))
        ));
    }

    #[test]
    fn test_execute_nft_events() {
        let mut interpreter =
            ClarityInterpreter::new(StandardPrincipalData::transient(), Settings::default());

        let contract = ClarityContractBuilder::default()
            .code_source(
                [
                    "(define-non-fungible-token nftest uint)",
                    "(nft-mint? nftest u1 tx-sender)",
                    "(nft-mint? nftest u2 tx-sender)",
                    "(define-private (test-burn)",
                    "  (nft-burn? nftest u1 tx-sender))",
                    "(define-private (test-transfer)",
                    "  (nft-transfer? nftest u2 tx-sender (as-contract  tx-sender)))",
                    "(test-burn)",
                    "(test-transfer)",
                ]
                .join("\n"),
            )
            .build();
        let source = contract.expect_in_memory_code_source();
        let (mut ast, ..) = interpreter.build_ast(&contract);
        let (annotations, _) = interpreter.collect_annotations(source);

        let (analysis, _) = interpreter
            .run_analysis(&contract, &mut ast, &annotations)
            .unwrap();

        let result = interpreter.execute(&contract, &mut ast, analysis, None, false, None);
        assert!(result.is_ok());
        let ExecutionResult {
            diagnostics,
            events,
            ..
        } = result.unwrap();
        assert!(diagnostics.is_empty());
        assert_eq!(events.len(), 4);
        assert!(matches!(
            events[0],
            StacksTransactionEvent::NFTEvent(NFTEventType::NFTMintEvent(_))
        ));
        assert!(matches!(
            events[1],
            StacksTransactionEvent::NFTEvent(NFTEventType::NFTMintEvent(_))
        ));
        assert!(matches!(
            events[2],
            StacksTransactionEvent::NFTEvent(NFTEventType::NFTBurnEvent(_))
        ));
        assert!(matches!(
            events[3],
            StacksTransactionEvent::NFTEvent(NFTEventType::NFTTransferEvent(_))
        ));
    }
}

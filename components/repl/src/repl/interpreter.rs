use std::collections::HashMap;
use std::collections::{btree_map::Entry, BTreeMap, BTreeSet};

use crate::analysis::annotation::{Annotation, AnnotationKind};
use crate::analysis::ast_dependency_detector::{ASTDependencyDetector, Dependency};
use crate::analysis::{self, AnalysisPass as REPLAnalysisPass};
use crate::clarity::analysis::{types::AnalysisPass, ContractAnalysis};
use crate::clarity::ast::ContractAST;
use crate::clarity::contexts::{
    CallStack, ContractContext, Environment, GlobalContext, LocalContext,
};
use crate::clarity::contracts::Contract;
use crate::clarity::costs::{ExecutionCost, LimitedCostTracker};
use crate::clarity::coverage::TestCoverageReport;
use crate::clarity::database::{Datastore, NULL_HEADER_DB};
use crate::clarity::diagnostic::{Diagnostic, Level};
use crate::clarity::errors::Error;
use crate::clarity::events::*;
use crate::clarity::representations::SymbolicExpressionType::{Atom, List};
use crate::clarity::representations::{Span, SymbolicExpression};
use crate::clarity::types::{
    self, PrincipalData, QualifiedContractIdentifier, StandardPrincipalData, Value,
};
use crate::clarity::util::StacksAddress;
use crate::clarity::{self, EvalHook};
use crate::clarity::{analysis::AnalysisDatabase, database::ClarityBackingStore};
use crate::clarity::{eval, eval_all};
use crate::repl::ast;
use crate::repl::{self, CostSynthesis, ExecutionResult};

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
    tx_sender: StandardPrincipalData,
    accounts: BTreeSet<String>,
    tokens: BTreeMap<String, BTreeMap<String, u128>>,
    repl_settings: repl::Settings,
}

trait Equivalent {
    fn equivalent(&self, other: &Self) -> bool;
}

impl Equivalent for SymbolicExpression {
    fn equivalent(&self, other: &Self) -> bool {
        use crate::clarity::representations::SymbolicExpressionType::*;
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
    pub fn new(
        tx_sender: StandardPrincipalData,
        repl_settings: repl::Settings,
    ) -> ClarityInterpreter {
        let datastore = Datastore::new();
        let accounts = BTreeSet::new();
        let tokens = BTreeMap::new();
        ClarityInterpreter {
            datastore,
            tx_sender,
            accounts,
            tokens,
            repl_settings,
        }
    }

    pub fn run(
        &mut self,
        snippet: String,
        contract_identifier: QualifiedContractIdentifier,
        cost_track: bool,
        eval_hooks: Option<Vec<Box<dyn EvalHook>>>,
    ) -> Result<ExecutionResult, Vec<Diagnostic>> {
        let (mut ast, mut diagnostics, success) = self.build_ast(
            contract_identifier.clone(),
            snippet.clone(),
            self.repl_settings.parser_version,
        );
        let (annotations, mut annotation_diagnostics) = self.collect_annotations(&ast, &snippet);
        diagnostics.append(&mut annotation_diagnostics);

        let (analysis, mut analysis_diagnostics) =
            match self.run_analysis(contract_identifier.clone(), &mut ast, &annotations) {
                Ok((analysis, diagnostics)) => (analysis, diagnostics),
                Err((_, Some(diagnostic), _)) => {
                    diagnostics.push(diagnostic);
                    return Err(diagnostics);
                }
                Err(_) => return Err(diagnostics),
            };
        diagnostics.append(&mut analysis_diagnostics);

        // If the parser or analysis failed, return the diagnostics to the caller, else execute.
        if !success {
            return Err(diagnostics);
        }

        let mut result = match self.execute(
            contract_identifier,
            &mut ast,
            snippet,
            analysis,
            cost_track,
            eval_hooks,
        ) {
            Ok(result) => result,
            Err((_, Some(diagnostic), _)) => {
                diagnostics.push(diagnostic);
                return Err(diagnostics);
            }
            Err((e, _, _)) => {
                diagnostics.push(Diagnostic {
                    level: Level::Error,
                    message: format!("Runtime Error: {}", e),
                    spans: vec![],
                    suggestion: None,
                });
                return Err(diagnostics);
            }
        };

        result.diagnostics = diagnostics;

        // todo: instead of just returning the value, we should be returning:
        // - value
        // - execution cost
        // - events emitted
        Ok(result)
    }

    pub fn run_ast(
        &mut self,
        mut ast: ContractAST,
        snippet: String,
        contract_identifier: QualifiedContractIdentifier,
        cost_track: bool,
        eval_hooks: Option<Vec<Box<dyn EvalHook>>>,
    ) -> Result<ExecutionResult, Vec<Diagnostic>> {
        let (annotations, mut diagnostics) = self.collect_annotations(&ast, &snippet);

        let (analysis, mut analysis_diagnostics) =
            match self.run_analysis(contract_identifier.clone(), &mut ast, &annotations) {
                Ok((analysis, diagnostics)) => (analysis, diagnostics),
                Err((_, Some(diagnostic), _)) => {
                    diagnostics.push(diagnostic);
                    return Err(diagnostics);
                }
                Err(_) => return Err(diagnostics),
            };
        diagnostics.append(&mut analysis_diagnostics);

        let mut result = match self.execute(
            contract_identifier,
            &mut ast,
            snippet,
            analysis,
            cost_track,
            eval_hooks,
        ) {
            Ok(result) => result,
            Err((_, Some(diagnostic), _)) => {
                diagnostics.push(diagnostic);
                return Err(diagnostics);
            }
            Err((e, _, _)) => {
                diagnostics.push(Diagnostic {
                    level: Level::Error,
                    message: format!("Runtime Error: {}", e),
                    spans: vec![],
                    suggestion: None,
                });
                return Err(diagnostics);
            }
        };

        result.diagnostics = diagnostics;

        // todo: instead of just returning the value, we should be returning:
        // - value
        // - execution cost
        // - events emitted
        Ok(result)
    }

    pub fn detect_dependencies(
        &mut self,
        contract_id: String,
        snippet: String,
        parser_version: u32,
    ) -> Result<Vec<Dependency>, String> {
        let contract_id = QualifiedContractIdentifier::parse(&contract_id).unwrap();
        let (ast, _, success) =
            self.build_ast(contract_id.clone(), snippet.clone(), parser_version);
        if !success {
            return Err("error parsing source".to_string());
        }

        let mut contract_map = HashMap::new();
        contract_map.insert(contract_id.clone(), ast);
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
        let dependencies = match all_dependencies.remove(&contract_id) {
            Some(mut dependencies_set) => dependencies_set.drain().collect(),
            None => vec![],
        };
        Ok(dependencies)
    }

    pub fn build_ast(
        &self,
        contract_identifier: QualifiedContractIdentifier,
        snippet: String,
        parser_version: u32,
    ) -> (ContractAST, Vec<Diagnostic>, bool) {
        match parser_version {
            1 => match clarity::ast::build_ast(&contract_identifier, &snippet, &mut ()) {
                Ok(res) => (res, vec![], true),
                Err(error) => (
                    ContractAST::new(contract_identifier, vec![]),
                    vec![error.diagnostic],
                    false,
                ),
            },
            _ => {
                // While parser v2 is new, we will run both parsers, and check for equivalence.
                // If they don't match, report a warning and use v1. Once v2 is battle-tested,
                // this can be removed. If there are errors when parsing, then the ASTs are not
                // expected to match, and v2 is still used.
                let (ast, diagnostics, success) =
                    ast::build_ast(&contract_identifier, &snippet, &mut ());
                let (ast_old, mut diagnostics_old, success_old) =
                    match clarity::ast::build_ast(&contract_identifier, &snippet, &mut ()) {
                        Ok(res) => (res, vec![], true),
                        Err(error) => (
                            ContractAST::new(contract_identifier, vec![]),
                            vec![error.diagnostic],
                            false,
                        ),
                    };
                if (success && !ast.equivalent(&ast_old)) || success != success_old {
                    diagnostics_old.push(Diagnostic {
                        level: Level::Warning,
                        message: r#"conflict between parser versions, reverting to parser v1
Help improve clarinet by sharing the code that triggered this warning:
https://github.com/hirosystems/clarinet/issues/new/choose"#
                            .to_string(),
                        spans: vec![],
                        suggestion: None,
                    });
                    (ast_old, diagnostics_old, success_old)
                } else {
                    (ast, diagnostics, success)
                }
            }
        }
    }

    pub fn collect_annotations(
        &self,
        ast: &ContractAST,
        snippet: &String,
    ) -> (Vec<Annotation>, Vec<Diagnostic>) {
        let mut annotations = vec![];
        let mut diagnostics = vec![];
        let lines = snippet.lines();
        for (n, line) in lines.enumerate() {
            if let Some(comment) = line.trim().strip_prefix(";;") {
                if let Some(annotation_string) = comment.trim().strip_prefix("#[") {
                    let span = Span {
                        start_line: (n + 1) as u32,
                        start_column: (line.find('#').unwrap_or(0) + 1) as u32,
                        end_line: (n + 1) as u32,
                        end_column: line.len() as u32,
                    };
                    if let Some(annotation_string) = annotation_string.strip_suffix("]") {
                        let kind: AnnotationKind = match annotation_string.trim().parse() {
                            Ok(kind) => kind,
                            Err(e) => {
                                diagnostics.push(Diagnostic {
                                    level: Level::Warning,
                                    message: format!("{}", e),
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
        contract_identifier: QualifiedContractIdentifier,
        contract_ast: &mut ContractAST,
        annotations: &Vec<Annotation>,
    ) -> Result<(ContractAnalysis, Vec<Diagnostic>), (String, Option<Diagnostic>, Option<Error>)>
    {
        let mut analysis_db = AnalysisDatabase::new(&mut self.datastore);

        // Run standard clarity analyses
        let mut contract_analysis = match clarity::analysis::run_analysis(
            &contract_identifier,
            &mut contract_ast.expressions,
            &mut analysis_db,
            false,
            LimitedCostTracker::new_free(),
        ) {
            Ok(res) => res,
            Err((error, cost_tracker)) => {
                return Err(("Analysis".to_string(), Some(error.diagnostic), None));
            }
        };

        // Run REPL-only analyses
        match analysis::run_analysis(
            &mut contract_analysis,
            &mut analysis_db,
            annotations,
            &self.repl_settings.analysis,
        ) {
            Ok(diagnostics) => Ok((contract_analysis, diagnostics)),
            Err(mut diagnostics) => {
                // The last diagnostic should be the error
                let error = diagnostics.pop().unwrap();
                Err(("Analysis".to_string(), Some(error), None))
            }
        }
    }

    #[allow(unused_assignments)]
    pub fn save_contract(
        &mut self,
        contract_identifier: QualifiedContractIdentifier,
        contract_ast: &mut ContractAST,
        snippet: String,
        contract_analysis: ContractAnalysis,
        mainnet: bool,
    ) {
        {
            let mut contract_context = ContractContext::new(contract_identifier.clone());
            let conn = self.datastore.as_clarity_db(&NULL_HEADER_DB);
            let cost_tracker = LimitedCostTracker::new_free();
            let mut global_context = GlobalContext::new(mainnet, conn, cost_tracker);
            global_context.begin();

            let _ = global_context
                .execute(|g| eval_all(&contract_ast.expressions, &mut contract_context, g));

            global_context
                .database
                .insert_contract_hash(&contract_identifier, &snippet)
                .unwrap();
            let contract = Contract { contract_context };
            global_context
                .database
                .insert_contract(&contract_identifier, contract);
            global_context
                .database
                .set_contract_data_size(&contract_identifier, 0)
                .unwrap();
            global_context.commit().unwrap();
        };

        let mut analysis_db = AnalysisDatabase::new(&mut self.datastore);
        analysis_db.begin();
        analysis_db
            .insert_contract(&contract_identifier, &contract_analysis)
            .unwrap();
        analysis_db.commit();
    }

    #[allow(unused_assignments)]
    pub fn execute(
        &mut self,
        contract_identifier: QualifiedContractIdentifier,
        contract_ast: &mut ContractAST,
        snippet: String,
        contract_analysis: ContractAnalysis,
        cost_track: bool,
        eval_hooks: Option<Vec<Box<dyn EvalHook>>>,
    ) -> Result<ExecutionResult, (String, Option<Diagnostic>, Option<Error>)> {
        let mut execution_result = ExecutionResult::default();
        let mut contract_saved = false;
        let mut serialized_events = vec![];
        let mut accounts_to_debit = vec![];
        let mut accounts_to_credit = vec![];
        let mut contract_context = ContractContext::new(contract_identifier.clone());
        let (value, eval_hooks) = {
            let tx_sender: PrincipalData = self.tx_sender.clone().into();

            let mut conn = self.datastore.as_clarity_db(&NULL_HEADER_DB);
            let cost_tracker = if cost_track {
                LimitedCostTracker::new(
                    false,
                    BLOCK_LIMIT_MAINNET.clone(),
                    &mut conn,
                    self.repl_settings.costs_version,
                )
                .unwrap()
            } else {
                LimitedCostTracker::new_free()
            };
            let mut global_context = GlobalContext::new(false, conn, cost_tracker);
            global_context.eval_hooks = eval_hooks;
            global_context.begin();

            let result = global_context.execute(|g| {
                // If we have more than one instruction
                if contract_ast.expressions.len() == 1 && !snippet.contains("(define-") {
                    let context = LocalContext::new();
                    let mut call_stack = CallStack::new();
                    let mut env = Environment::new(
                        g,
                        &mut contract_context,
                        &mut call_stack,
                        Some(tx_sender.clone()),
                        Some(tx_sender.clone()),
                    );

                    let result = match contract_ast.expressions[0].expr {
                        List(ref expression) => match expression[0].expr {
                            Atom(ref name) if name.to_string() == "contract-call?" => {
                                let contract_identifier = match expression[1]
                                    .match_literal_value()
                                    .unwrap()
                                    .clone()
                                    .expect_principal()
                                {
                                    PrincipalData::Contract(contract_identifier) => {
                                        contract_identifier
                                    }
                                    _ => unreachable!(),
                                };
                                let method = expression[2].match_atom().unwrap().to_string();
                                let mut args = vec![];
                                for arg in expression[3..].iter() {
                                    let evaluated_arg = eval(arg, &mut env, &context)?;
                                    args.push(SymbolicExpression::atom_value(evaluated_arg));
                                }
                                let res = env.execute_contract(
                                    &contract_identifier,
                                    &method,
                                    &args,
                                    false,
                                )?;
                                res
                            }
                            _ => eval(&contract_ast.expressions[0], &mut env, &context).unwrap(),
                        },
                        _ => eval(&contract_ast.expressions[0], &mut env, &context).unwrap(),
                    };
                    Ok(Some(result))
                } else {
                    eval_all(&contract_ast.expressions, &mut contract_context, g)
                }
            });

            let value = match result {
                Ok(Some(value)) => value,
                Ok(None) => Value::none(),
                Err(e) => {
                    let err = format!(
                        "Runtime error while interpreting {}: {:?}",
                        contract_identifier, e
                    );
                    if let Some(mut eval_hooks) = global_context.eval_hooks.take() {
                        for hook in eval_hooks.iter_mut() {
                            hook.did_complete(Err(err.clone()));
                        }
                        global_context.eval_hooks = Some(eval_hooks);
                    }
                    return Err((err, None, None));
                }
            };

            if cost_track {
                execution_result.cost =
                    Some(CostSynthesis::from_cost_tracker(&global_context.cost_track));
            }

            let mut emitted_events = global_context
                .event_batches
                .iter()
                .flat_map(|b| b.events.clone())
                .collect::<Vec<_>>();

            for event in emitted_events.drain(..) {
                match event {
                    StacksTransactionEvent::STXEvent(STXEventType::STXTransferEvent(
                        ref event_data,
                    )) => {
                        accounts_to_debit.push((
                            event_data.sender.to_string(),
                            "STX".to_string(),
                            event_data.amount.clone(),
                        ));
                        accounts_to_credit.push((
                            event_data.recipient.to_string(),
                            "STX".to_string(),
                            event_data.amount.clone(),
                        ));
                    }
                    StacksTransactionEvent::STXEvent(STXEventType::STXMintEvent(
                        ref event_data,
                    )) => {
                        accounts_to_credit.push((
                            event_data.recipient.to_string(),
                            "STX".to_string(),
                            event_data.amount.clone(),
                        ));
                    }
                    StacksTransactionEvent::STXEvent(STXEventType::STXBurnEvent(
                        ref event_data,
                    )) => {
                        accounts_to_debit.push((
                            event_data.sender.to_string(),
                            "STX".to_string(),
                            event_data.amount.clone(),
                        ));
                    }
                    StacksTransactionEvent::FTEvent(FTEventType::FTTransferEvent(
                        ref event_data,
                    )) => {
                        accounts_to_credit.push((
                            event_data.recipient.to_string(),
                            event_data.asset_identifier.sugared(),
                            event_data.amount.clone(),
                        ));
                        accounts_to_debit.push((
                            event_data.sender.to_string(),
                            event_data.asset_identifier.sugared(),
                            event_data.amount.clone(),
                        ));
                    }
                    StacksTransactionEvent::FTEvent(FTEventType::FTMintEvent(ref event_data)) => {
                        accounts_to_credit.push((
                            event_data.recipient.to_string(),
                            event_data.asset_identifier.sugared(),
                            event_data.amount.clone(),
                        ));
                    }
                    StacksTransactionEvent::FTEvent(FTEventType::FTBurnEvent(ref event_data)) => {
                        accounts_to_debit.push((
                            event_data.sender.to_string(),
                            event_data.asset_identifier.sugared(),
                            event_data.amount.clone(),
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
                    StacksTransactionEvent::NFTEvent(NFTEventType::NFTMintEvent(
                        ref event_data,
                    )) => {
                        accounts_to_credit.push((
                            event_data.recipient.to_string(),
                            event_data.asset_identifier.sugared(),
                            1,
                        ));
                    }
                    StacksTransactionEvent::NFTEvent(NFTEventType::NFTBurnEvent(
                        ref event_data,
                    )) => {
                        accounts_to_debit.push((
                            event_data.sender.to_string(),
                            event_data.asset_identifier.sugared(),
                            1,
                        ));
                    }
                    // StacksTransactionEvent::SmartContractEvent(event_data) => ,
                    // StacksTransactionEvent::STXEvent(STXEventType::STXLockEvent(event_data)) => ,
                    _ => {}
                };

                serialized_events.push(event.json_serialize());
            }

            contract_saved =
                contract_context.functions.len() > 0 || contract_context.defined_traits.len() > 0;

            if contract_saved {
                let mut functions = BTreeMap::new();
                for (name, defined_func) in contract_context.functions.iter() {
                    if !defined_func.is_public() {
                        continue;
                    }

                    let args: Vec<_> = defined_func
                        .arguments
                        .iter()
                        .zip(defined_func.arg_types.iter())
                        .map(|(n, t)| format!("({} {})", n.as_str(), t))
                        .collect();

                    functions.insert(name.to_string(), args);
                }
                execution_result.contract = Some((
                    contract_identifier.to_string(),
                    snippet.clone(),
                    functions,
                    contract_ast.clone(),
                    contract_analysis.clone(),
                ));

                for defined_trait in contract_context.defined_traits.iter() {}

                global_context
                    .database
                    .insert_contract_hash(&contract_identifier, &snippet)
                    .unwrap();
                let contract = Contract { contract_context };
                global_context
                    .database
                    .insert_contract(&contract_identifier, contract);
                global_context
                    .database
                    .set_contract_data_size(&contract_identifier, 0)
                    .unwrap();
            }
            global_context.commit().unwrap();

            (value, global_context.eval_hooks)
        };

        execution_result.events = serialized_events;

        for (account, token, value) in accounts_to_credit.drain(..) {
            self.credit_token(account, token, value);
        }

        for (account, token, value) in accounts_to_debit.drain(..) {
            self.debit_token(account, token, value);
        }

        if !contract_saved {
            execution_result.result = Some(value);

            if let Some(mut eval_hooks) = eval_hooks {
                for hook in eval_hooks.iter_mut() {
                    hook.did_complete(Ok(&mut execution_result));
                }
            }

            return Ok(execution_result);
        }

        {
            let mut analysis_db = AnalysisDatabase::new(&mut self.datastore);
            let _ = analysis_db
                .execute(|db| db.insert_contract(&contract_identifier, &contract_analysis))
                .expect("Unable to save data");
        }

        if let Some(mut eval_hooks) = eval_hooks {
            for hook in eval_hooks.iter_mut() {
                hook.did_complete(Ok(&mut execution_result));
            }
        }

        Ok(execution_result)
    }

    pub fn mint_stx_balance(
        &mut self,
        recipient: PrincipalData,
        amount: u64,
    ) -> Result<String, String> {
        let final_balance = {
            let conn = self.datastore.as_clarity_db(&NULL_HEADER_DB);
            let mut global_context =
                GlobalContext::new(false, conn, LimitedCostTracker::new_free());
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
                Some(value) => value.clone(),
                _ => 0,
            },
            _ => 0,
        }
    }
}

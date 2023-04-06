use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    fs::{create_dir_all, File},
    io::{Error, ErrorKind, Write},
    mem,
    path::{Path, PathBuf},
};

use clarity::vm::ast::ContractAST;
use clarity::vm::functions::define::DefineFunctionsParsed;
use clarity::vm::representations::SymbolicExpression;
use clarity::vm::types::QualifiedContractIdentifier;
use clarity::vm::EvalHook;
use serde_json::Value as JsonValue;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct CoverageReporter {
    pub reports: Vec<TestCoverageReport>,
    pub asts: BTreeMap<QualifiedContractIdentifier, ContractAST>,
    pub contract_paths: BTreeMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct TestCoverageReport {
    pub test_name: String,
    pub contracts_coverage: HashMap<QualifiedContractIdentifier, ContractCoverageReport>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct ContractCoverageReport {
    functions_coverage: HashMap<String, u64>,
    execution_counts: HashMap<u32, u64>,
    executed_statements: BTreeSet<u64>,
}

pub fn parse_coverage_str(path: &str) -> Result<PathBuf, Error> {
    let filepath = Path::new(path);
    let path_buf = filepath.to_path_buf();
    match path_buf.extension() {
        None => Ok(path_buf.join(Path::new("coverage.lcov"))),
        Some(_) => Ok(path_buf),
    }
}

impl CoverageReporter {
    pub fn new() -> CoverageReporter {
        CoverageReporter {
            reports: vec![],
            asts: BTreeMap::new(),
            contract_paths: BTreeMap::new(),
        }
    }

    pub fn register_contract(&mut self, contract_name: String, contract_path: String) {
        self.contract_paths.insert(contract_name, contract_path);
    }

    pub fn add_asts(&mut self, asts: &BTreeMap<QualifiedContractIdentifier, ContractAST>) {
        self.asts.append(&mut asts.clone());
    }

    pub fn add_reports(&mut self, reports: &Vec<TestCoverageReport>) {
        self.reports.append(&mut reports.clone());
    }

    pub fn write_lcov_file<P: AsRef<std::path::Path> + Copy>(
        &self,
        filename: P,
    ) -> std::io::Result<()> {
        let mut filtered_asts = HashMap::new();
        for (contract_id, ast) in self.asts.iter() {
            let contract_name = contract_id.name.to_string();
            if self.contract_paths.get(&contract_name).is_some() {
                filtered_asts.insert(
                    contract_name,
                    (
                        contract_id,
                        self.retrieve_functions(&ast.expressions),
                        self.filter_executable_lines(&ast.expressions),
                    ),
                );
            }
        }

        let mut test_names = BTreeSet::new();
        for report in self.reports.iter() {
            test_names.insert(report.test_name.to_string());
        }

        let filepath = filename.as_ref().to_path_buf();
        let filepath = filepath.parent().ok_or(Error::new(
            ErrorKind::NotFound,
            "could not get directory to create coverage file",
        ))?;
        create_dir_all(filepath)?;
        let mut out = File::create(filename)?;

        for (index, test_name) in test_names.iter().enumerate() {
            for (contract_name, contract_path) in self.contract_paths.iter() {
                writeln!(out, "TN:{}", test_name)?;
                writeln!(out, "SF:{}", contract_path)?;

                if let Some((contract_id, functions, executable_lines)) =
                    filtered_asts.get(contract_name)
                {
                    for (function, line_start, line_end) in functions.iter() {
                        writeln!(out, "FN:{},{}", line_start, function)?;
                    }

                    let mut function_hits = BTreeMap::new();
                    let mut consolidated_execution_counts = BTreeMap::new();
                    for report in self.reports.iter() {
                        if &report.test_name == test_name {
                            if let Some(contract) = report.contracts_coverage.get(contract_id) {
                                let mut local_function_hits = BTreeSet::new();

                                for line in executable_lines.iter() {
                                    let count = contract.execution_counts.get(line).unwrap_or(&0);

                                    if let Some(line_count) =
                                        consolidated_execution_counts.get_mut(line)
                                    {
                                        *line_count += *count;
                                    } else {
                                        consolidated_execution_counts.insert(*line, *count);
                                    }

                                    if count == &0 {
                                        continue;
                                    }

                                    for (function, line_start, line_end) in functions.iter() {
                                        if line >= line_start && line <= line_end {
                                            local_function_hits.insert(function);
                                        }
                                    }
                                }

                                for function in local_function_hits.into_iter() {
                                    if let Some(total_hit) = function_hits.get_mut(function) {
                                        *total_hit += 1;
                                    } else {
                                        function_hits.insert(function, 1);
                                    }
                                }
                            }
                        }
                    }

                    for (function, hits) in function_hits.iter() {
                        writeln!(out, "FNDA:{},{}", hits, function)?;
                    }
                    writeln!(out, "FNF:{}", functions.len())?;
                    writeln!(out, "FNH:{}", function_hits.len())?;

                    for (line_number, count) in consolidated_execution_counts.iter() {
                        writeln!(out, "DA:{},{}", line_number, count)?;
                    }
                }
                writeln!(out, "end_of_record")?;
            }
        }

        Ok(())
    }

    fn retrieve_functions(&self, exprs: &Vec<SymbolicExpression>) -> Vec<(String, u32, u32)> {
        let mut functions = vec![];
        for cur_expr in exprs.iter() {
            if let Some(define_expr) = DefineFunctionsParsed::try_parse(cur_expr).ok().flatten() {
                match define_expr {
                    DefineFunctionsParsed::PrivateFunction { signature, body }
                    | DefineFunctionsParsed::PublicFunction { signature, body }
                    | DefineFunctionsParsed::ReadOnlyFunction { signature, body } => {
                        let expr = signature.get(0).expect("Invalid function signature");
                        let function_name = expr.match_atom().expect("Invalid function signature");

                        functions.push((
                            function_name.to_string(),
                            cur_expr.span.start_line,
                            cur_expr.span.end_line,
                        ));
                    }
                    _ => {}
                }
                continue;
            }
        }
        functions
    }

    fn filter_executable_lines(&self, exprs: &Vec<SymbolicExpression>) -> Vec<u32> {
        let mut lines = vec![];
        let mut lines_seen = HashSet::new();
        for expression in exprs.iter() {
            let mut frontier = vec![expression];
            while let Some(cur_expr) = frontier.pop() {
                // Only consider body functions
                if let Some(define_expr) = DefineFunctionsParsed::try_parse(cur_expr).ok().flatten()
                {
                    match define_expr {
                        DefineFunctionsParsed::PrivateFunction { signature: _, body }
                        | DefineFunctionsParsed::PublicFunction { signature: _, body }
                        | DefineFunctionsParsed::ReadOnlyFunction { signature: _, body } => {
                            frontier.push(body);
                        }
                        DefineFunctionsParsed::BoundedFungibleToken { .. } => {}
                        DefineFunctionsParsed::Constant { .. } => {}
                        DefineFunctionsParsed::PersistedVariable { .. } => {}
                        DefineFunctionsParsed::NonFungibleToken { .. } => {}
                        DefineFunctionsParsed::UnboundedFungibleToken { .. } => {}
                        DefineFunctionsParsed::Map { .. } => {}
                        DefineFunctionsParsed::Trait { .. } => {}
                        DefineFunctionsParsed::UseTrait { .. } => {}
                        DefineFunctionsParsed::ImplTrait { .. } => {}
                    }

                    continue;
                }

                if let Some(children) = cur_expr.match_list() {
                    // don't count list expressions as a whole, just their children
                    frontier.extend(children);
                } else {
                    let line = cur_expr.span.start_line;
                    if !lines_seen.contains(&line) {
                        lines_seen.insert(line);
                        lines.push(line);
                    }
                }
            }
        }

        lines.sort();
        lines
    }
}

impl TestCoverageReport {
    pub fn new(test_name: String) -> TestCoverageReport {
        TestCoverageReport {
            test_name,
            contracts_coverage: HashMap::new(),
        }
    }
}

impl EvalHook for TestCoverageReport {
    fn will_begin_eval(
        &mut self,
        env: &mut clarity::vm::contexts::Environment,
        context: &clarity::vm::contexts::LocalContext,
        expr: &SymbolicExpression,
    ) {
        let contract = &env.contract_context.contract_identifier;
        let mut contract_report = match self.contracts_coverage.remove(contract) {
            Some(e) => e,
            _ => ContractCoverageReport::new(),
        };
        contract_report.report_eval(expr);
        self.contracts_coverage
            .insert(contract.clone(), contract_report);
    }

    fn did_finish_eval(
        &mut self,
        _env: &mut clarity::vm::Environment,
        _context: &clarity::vm::LocalContext,
        _expr: &SymbolicExpression,
        _res: &core::result::Result<clarity::vm::Value, clarity::vm::errors::Error>,
    ) {
    }

    fn did_complete(
        &mut self,
        _result: core::result::Result<&mut clarity::vm::ExecutionResult, String>,
    ) {
    }
}

impl ContractCoverageReport {
    pub fn new() -> ContractCoverageReport {
        ContractCoverageReport {
            functions_coverage: HashMap::new(),
            execution_counts: HashMap::new(),
            executed_statements: BTreeSet::new(),
        }
    }

    pub fn report_eval(&mut self, expr: &SymbolicExpression) {
        if let Some(children) = expr.match_list() {
            // Handle the function variable, then the rest of the list will be
            // eval'ed later.
            if let Some((function_variable, rest)) = children.split_first() {
                self.report_eval(function_variable);
            }
            return;
        }

        // other sexps can only span 1 line
        let line_executed = expr.span.start_line;
        if let Some(execution_count) = self.execution_counts.get_mut(&line_executed) {
            *execution_count += 1;
        } else {
            self.execution_counts.insert(line_executed, 1);
        }
        self.executed_statements.insert(expr.id);
    }
}

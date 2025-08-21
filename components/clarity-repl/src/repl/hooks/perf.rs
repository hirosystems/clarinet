use std::collections::HashMap;
use std::fmt::Display;
use std::io::Write;

use clarity::vm::ast::{build_ast_with_diagnostics, ContractAST};
use clarity::vm::contexts::{Environment, LocalContext};
use clarity::vm::costs::ExecutionCost;
use clarity::vm::errors::Error;
use clarity::vm::types::{QualifiedContractIdentifier, Value};
use clarity::vm::{ClarityVersion, EvalHook, SymbolicExpression, SymbolicExpressionType};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CostField {
    Runtime,
    ReadLength,
    ReadCount,
    WriteLength,
    WriteCount,
}

impl CostField {
    pub fn parse_from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "runtime" => Some(CostField::Runtime),
            "read_length" | "readlength" => Some(CostField::ReadLength),
            "read_count" | "readcount" => Some(CostField::ReadCount),
            "write_length" | "writelength" => Some(CostField::WriteLength),
            "write_count" | "writecount" => Some(CostField::WriteCount),
            _ => None,
        }
    }

    pub fn get_value(&self, cost: &ExecutionCost) -> u64 {
        match self {
            CostField::Runtime => cost.runtime,
            CostField::ReadLength => cost.read_length,
            CostField::ReadCount => cost.read_count,
            CostField::WriteLength => cost.write_length,
            CostField::WriteCount => cost.write_count,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            CostField::Runtime => "runtime",
            CostField::ReadLength => "read_length",
            CostField::ReadCount => "read_count",
            CostField::WriteLength => "write_length",
            CostField::WriteCount => "write_count",
        }
    }
}

struct StackEntry {
    contract: QualifiedContractIdentifier,
    function: String,
    expr_id: u64,
    line: u32,
    column: u32,
    cost_before: ExecutionCost,
    cost_descendents: ExecutionCost,
}

impl Display for StackEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}{}:{}:{}",
            self.contract, self.function, self.line, self.column,
        )
    }
}

/// A writer that can also be read from, used for WASM mode
#[cfg(target_arch = "wasm32")]
struct ReadableWriter {
    buffer: Vec<u8>,
}

#[cfg(target_arch = "wasm32")]
impl ReadableWriter {
    fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    /// Get the written data as a string
    pub fn get_data(&self) -> Option<String> {
        String::from_utf8(self.buffer.clone()).ok()
    }
}

#[cfg(target_arch = "wasm32")]
impl Write for ReadableWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.buffer.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

enum PerfWriter {
    #[cfg(not(target_arch = "wasm32"))]
    File(std::fs::File),
    #[cfg(target_arch = "wasm32")]
    Readable(ReadableWriter),
}

impl Write for PerfWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            PerfWriter::File(file) => file.write(buf),
            #[cfg(target_arch = "wasm32")]
            PerfWriter::Readable(writer) => writer.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            PerfWriter::File(file) => file.flush(),
            #[cfg(target_arch = "wasm32")]
            PerfWriter::Readable(writer) => writer.flush(),
        }
    }
}

pub struct PerfHook {
    /// Writer for outputting performance metrics
    writer: PerfWriter,
    /// Stack of expressions
    expr_stack: Vec<StackEntry>,
    /// Specific cost field to track
    cost_field: CostField,
    // Store mapping from expression IDs to their original spans from the AST
    ast_span_mapping: HashMap<u64, (u32, u32)>,
}

impl Clone for PerfHook {
    fn clone(&self) -> Self {
        PerfHook::new(self.cost_field)
    }
}

impl PerfHook {
    pub fn new(cost_field: CostField) -> PerfHook {
        Self::new_with_filename(cost_field, "perf.data")
    }

    pub fn new_with_filename(cost_field: CostField, _filename: &str) -> PerfHook {
        let writer = {
            #[cfg(target_arch = "wasm32")]
            {
                PerfWriter::Readable(ReadableWriter::new())
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                PerfWriter::File(
                    std::fs::File::create(_filename).expect("Failed to create perf output file"),
                )
            }
        };
        PerfHook {
            writer,
            expr_stack: Vec::new(),
            cost_field,
            ast_span_mapping: HashMap::new(),
        }
    }

    /// Get the performance data buffer (WASM mode) or None (non-WASM mode)
    pub fn get_buffer_data(&self) -> Option<String> {
        match &self.writer {
            #[cfg(target_arch = "wasm32")]
            PerfWriter::Readable(writer) => writer.get_data(),
            #[cfg(not(target_arch = "wasm32"))]
            PerfWriter::File(_) => None,
        }
    }
}

impl Default for PerfHook {
    fn default() -> Self {
        Self::new(CostField::Runtime)
    }
}

impl EvalHook for PerfHook {
    fn will_begin_eval(
        &mut self,
        env: &mut Environment,
        _context: &LocalContext,
        expr: &SymbolicExpression,
    ) {
        let contract = &env.contract_context.contract_identifier;

        // Check if we need to fetch contract source
        // Only fetch if we don't have the source AND the expression doesn't have valid span info
        let needs_source_fetch = env
            .global_context
            .database
            .get_contract_src(contract)
            .is_none()
            && has_blank_span(expr);

        if needs_source_fetch {
            // Try to fetch contract source from API and cache it
            if let Err(e) = self.fetch_and_cache_contract_source(env, contract) {
                eprintln!("Failed to fetch contract source: {}", e);
            }
        }

        // Find the current function name in the call stack
        let call_stack = env.call_stack.make_stack_trace();
        let mut function = String::new();
        for identifier in call_stack.iter().rev() {
            let s = identifier.to_string();
            if s.starts_with("_native_") {
                continue;
            }

            if let Some(f) = s.strip_prefix(&format!("{contract}:")) {
                function = format!(":{f}");
            } else {
                break;
            }
        }

        // If the expression is a list, extract the function name and append it
        // to the function string
        if let SymbolicExpressionType::List(list) = &expr.expr {
            if let Some((function_name, _args)) = list.split_first() {
                if let Some(function_name) = function_name.match_atom() {
                    function = format!("{function}:{function_name}");
                }
            }
        }

        // Record the cost before evaluating this expression
        let cost_before = env.global_context.cost_track.get_total();

        // Get line and column information, with fallback to AST mapping and contract source
        let (line, column) = self.get_line_column_info(env, contract, expr);

        self.expr_stack.push(StackEntry {
            contract: contract.clone(),
            function,
            expr_id: expr.id,
            line,
            column,
            cost_before,
            cost_descendents: ExecutionCost::ZERO,
        });
    }

    fn did_finish_eval(
        &mut self,
        env: &mut Environment,
        _context: &LocalContext,
        expr: &SymbolicExpression,
        _res: &Result<Value, Error>,
    ) {
        // Get the full call stack as a string
        let call_stack = self
            .expr_stack
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join(";");

        // Pop the last entry from the expression stack
        let entry = self.expr_stack.pop().expect("expr stack underflow");
        assert_eq!(
            entry.expr_id, expr.id,
            "Expression ID mismatch: expected {}, got {}",
            entry.expr_id, expr.id
        );

        // Get the current cost
        let mut cost = env.global_context.cost_track.get_total();

        // Subtract the cost before evaluation and the cost of descendents to
        // get the cost of this expression
        cost.sub(&entry.cost_before)
            .expect("cost diff calculation failed");
        cost.sub(&entry.cost_descendents)
            .expect("cost diff calculation failed");

        // Write the performance data to the output
        writeln!(
            self.writer,
            "{call_stack} {}",
            self.cost_field.get_value(&cost)
        )
        .expect("Failed to write to perf output");

        // Add the cost of this expression and its descendents to the parent
        // expression's cost so that it is not double-counted.
        if let Some(parent) = self.expr_stack.last_mut() {
            parent
                .cost_descendents
                .add(&cost)
                .expect("cost addition failed");
            parent
                .cost_descendents
                .add(&entry.cost_descendents)
                .expect("cost addition failed");
        }
    }

    fn did_complete(
        &mut self,
        _result: core::result::Result<&mut clarity::vm::ExecutionResult, String>,
    ) {
        // No action needed after evaluation completion
    }
}

impl PerfHook {
    /// Capture the original AST and build a mapping from expression IDs to spans
    pub fn capture_ast(&mut self, contract_ast: &ContractAST) {
        self.ast_span_mapping.clear();
        self.build_span_mapping(contract_ast);
    }

    fn build_span_mapping(&mut self, contract_ast: &ContractAST) {
        for expr in &contract_ast.expressions {
            self.add_expression_to_mapping(expr);
        }
    }

    fn add_expression_to_mapping(&mut self, expr: &SymbolicExpression) {
        // Store this expression's span if its not blank
        if !has_blank_span(expr) {
            self.ast_span_mapping
                .insert(expr.id, (expr.span.start_line, expr.span.start_column));
        }

        if let SymbolicExpressionType::List(list) = &expr.expr {
            for child in list {
                self.add_expression_to_mapping(child);
            }
        }
    }

    /// Fetch contract source from API and cache it in the datastore
    fn fetch_and_cache_contract_source(
        &mut self,
        env: &mut Environment,
        contract: &QualifiedContractIdentifier,
    ) -> Result<(), String> {
        let is_mainnet = env.global_context.mainnet;

        let api_base = if is_mainnet {
            "https://api.hiro.so"
        } else {
            "https://api.testnet.hiro.so"
        };

        let contract_deployer = contract.issuer.to_address();
        let contract_name = contract.name.to_string();
        let request_url = format!(
            "{}/v2/contracts/source/{}/{}?proof=0",
            api_base, contract_deployer, contract_name
        );

        eprintln!("Fetching contract source from: {}", request_url);

        let response = self.make_http_request(&request_url)?;
        let contract_data: ContractApiResponse = serde_json::from_str(&response)
            .map_err(|e| format!("Failed to parse API response: {}", e))?;

        if let Err(e) = env
            .global_context
            .database
            .insert_contract_hash(contract, &contract_data.source)
        {
            eprintln!("Failed to cache contract source: {}", e);
        }

        Ok(())
    }

    fn make_http_request(&self, url: &str) -> Result<String, String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            use reqwest::blocking::Client;
            use std::time::Duration;

            let client = Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

            let response = client
                .get(url)
                .header("x-hiro-product", "clarinet-cli")
                .header("Accept", "application/json")
                .send()
                .map_err(|e| format!("HTTP request failed: {}", e))?;

            if !response.status().is_success() {
                return Err(format!("HTTP error: {}", response.status()));
            }

            response
                .text()
                .map_err(|e| format!("Failed to read response: {}", e))
        }

        #[cfg(target_arch = "wasm32")]
        {
            Err("HTTP requests not supported in WASM mode".to_string())
        }
    }

    /// Helper to get line and column information, with fallback to AST mapping and contract source
    fn get_line_column_info(
        &mut self,
        env: &mut Environment,
        contract: &QualifiedContractIdentifier,
        expr: &SymbolicExpression,
    ) -> (u32, u32) {
        let mut line = expr.span.start_line;
        let mut column = expr.span.start_column;

        // If the span information is zero, try to get it from our AST mapping first
        if line == 0 && column == 0 {
            if let Some(&(ast_line, ast_column)) = self.ast_span_mapping.get(&expr.id) {
                line = ast_line;
                column = ast_column;
            } else if let Some(contract_source) =
                env.global_context.database.get_contract_src(contract)
            {
                // Parse contract source into SymbolicExpressions to get accurate spans
                let contract_id = QualifiedContractIdentifier::transient();
                let (ast, _diagnostics, _success) = build_ast_with_diagnostics(
                    &contract_id,
                    &contract_source,
                    &mut (),
                    ClarityVersion::default_for_epoch(env.global_context.epoch_id),
                    env.global_context.epoch_id,
                );

                // Build span mapping from the parsed AST
                self.build_span_mapping(&ast);

                // Try to get span information from our new mapping
                if let Some(&(parsed_line, parsed_column)) = self.ast_span_mapping.get(&expr.id) {
                    line = parsed_line;
                    column = parsed_column;
                } else {
                    // Use fallback values if parsing didn't work
                    line = 1;
                    column = 1;
                }
            } else {
                line = 1;
                column = 1;
            }
        }

        (line, column)
    }
}

fn has_blank_span(expr: &SymbolicExpression) -> bool {
    expr.span.start_line == 0 && expr.span.start_column == 0
}

/// Response structure for the contract source API
#[derive(serde::Deserialize)]
struct ContractApiResponse {
    source: String,
    #[allow(dead_code)]
    publish_height: u32,
    #[allow(dead_code)]
    clarity_version: Option<u8>,
}

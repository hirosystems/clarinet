use super::SessionArtifacts;
use clarity_repl::clarity::vm::CostSynthesis;
use clarity_repl::prettytable::{color, format, Attr, Cell, Row, Table};
use clarity_repl::repl::session::CostsReport;
use std::collections::{btree_map::Entry, BTreeMap};

pub struct ExecutionCost {
    actual: u64,
    limit: u64,
}

impl ExecutionCost {
    pub fn new(actual: u64, limit: u64) -> Self {
        Self { actual, limit }
    }

    pub fn actual(&self) -> u64 {
        self.actual
    }

    pub fn tx_per_block(&self) -> u64 {
        self.limit
            .checked_div(self.actual)
            .unwrap_or_else(|| self.limit)
    }
}

struct FunctionCosts {
    pub runtime: ExecutionCost,
    pub read_count: ExecutionCost,
    pub read_length: ExecutionCost,
    pub write_count: ExecutionCost,
    pub write_length: ExecutionCost,
    pub tx_per_block: u64,
}

impl FunctionCosts {
    pub fn from_cost_report(costs: CostSynthesis) -> Self {
        let limit = costs.limit;
        let total = costs.total;

        let runtime = ExecutionCost::new(total.runtime, limit.runtime);
        let read_count = ExecutionCost::new(total.read_count, limit.read_count);
        let read_length = ExecutionCost::new(total.read_length, limit.read_length);
        let write_count = ExecutionCost::new(total.write_count, limit.write_count);
        let write_length = ExecutionCost::new(total.write_length, limit.write_length);

        let mut tx_per_block_limits = vec![
            runtime.tx_per_block(),
            read_count.tx_per_block(),
            read_length.tx_per_block(),
            write_count.tx_per_block(),
            write_length.tx_per_block(),
        ];

        tx_per_block_limits.sort_by(|a, b| a.cmp(&b));
        let tx_per_block = tx_per_block_limits.first().unwrap();

        Self {
            runtime,
            read_count,
            read_length,
            write_count,
            write_length,
            tx_per_block: *tx_per_block,
        }
    }
}

pub fn display_costs_report(sessions_artifacts: &Vec<SessionArtifacts>) {
    let mut consolidated: BTreeMap<String, BTreeMap<String, Vec<CostsReport>>> = BTreeMap::new();

    let mut contracts_costs: BTreeMap<&String, BTreeMap<&String, Vec<FunctionCosts>>> =
        BTreeMap::new();

    for artifacts in sessions_artifacts.iter() {
        for report in artifacts.costs_reports.iter() {
            let key = report.contract_id.to_string();
            match consolidated.entry(key) {
                Entry::Occupied(ref mut entry) => {
                    match entry.get_mut().entry(report.method.to_string()) {
                        Entry::Occupied(entry) => entry.into_mut().push(report.clone()),
                        Entry::Vacant(entry) => {
                            let mut reports = Vec::new();
                            reports.push(report.clone());
                            entry.insert(reports);
                        }
                    }
                }
                Entry::Vacant(entry) => {
                    let mut reports = Vec::new();
                    reports.push(report.clone());
                    let mut methods = BTreeMap::new();
                    methods.insert(report.method.to_string(), reports);
                    entry.insert(methods);
                }
            };

            let contract_costs = contracts_costs
                .entry(&report.contract_id)
                .or_insert(BTreeMap::new());
            let function_costs = contract_costs.entry(&report.method).or_insert(Vec::new());

            function_costs.push(FunctionCosts::from_cost_report(report.cost_result.clone()));
        }
    }

    println!("\nContract calls cost synthesis");
    #[allow(unused_variables)]
    let table = Table::new();
    let headers = vec![
        "".to_string(),
        "# Calls".to_string(),
        "Runtime (units)".to_string(),
        "Read Count".to_string(),
        "Read Length (bytes)".to_string(),
        "Write Count".to_string(),
        "Write Length (bytes)".to_string(),
        "Tx per Block".to_string(),
    ];
    let mut headers_cells = vec![];
    for header in headers.iter() {
        headers_cells.push(Cell::new(&header).with_style(Attr::Bold));
    }

    for (contract_name, contract_stats) in contracts_costs.iter_mut() {
        let mut table = Table::new();
        table.set_format(*format::consts::FORMAT_BOX_CHARS);

        let formatted_contract_name = &format!("\n✨  {}\n ", contract_name);
        table.add_row(Row::new(vec![Cell::new(formatted_contract_name)
            .with_style(Attr::Bold)
            .with_style(Attr::ForegroundColor(color::YELLOW))
            .with_hspan(8)]));

        table.add_row(Row::new(headers_cells.clone()));

        for (method, method_stats) in contract_stats.iter_mut() {
            method_stats.sort_by(|a, b| a.tx_per_block.cmp(&b.tx_per_block));

            let min: &FunctionCosts = method_stats.last().unwrap();
            let max: &FunctionCosts = method_stats.first().unwrap();

            let runtime_summary = execution_costs_summary(
                &min.runtime,
                &max.runtime,
                &ExecutionCost::new(
                    method_stats.iter().fold(0, |acc, x| acc + x.runtime.actual)
                        / method_stats.len() as u64,
                    max.runtime.limit,
                ),
            );

            let read_count_summary = execution_costs_summary(
                &min.read_count,
                &max.read_count,
                &ExecutionCost::new(
                    method_stats
                        .iter()
                        .fold(0, |acc, x| acc + x.read_count.actual)
                        / method_stats.len() as u64,
                    max.read_count.limit,
                ),
            );

            let read_length_summary = execution_costs_summary(
                &min.read_length,
                &max.read_length,
                &ExecutionCost::new(
                    method_stats
                        .iter()
                        .fold(0, |acc, x| acc + x.read_length.actual)
                        / method_stats.len() as u64,
                    max.read_length.limit,
                ),
            );

            let write_count_summary = execution_costs_summary(
                &min.write_count,
                &max.write_count,
                &ExecutionCost::new(
                    method_stats
                        .iter()
                        .fold(0, |acc, x| acc + x.write_count.actual)
                        / method_stats.len() as u64,
                    max.write_count.limit,
                ),
            );

            let write_length_summary = execution_costs_summary(
                &min.write_length,
                &max.write_length,
                &ExecutionCost::new(
                    method_stats
                        .iter()
                        .fold(0, |acc, x| acc + x.write_length.actual)
                        / method_stats.len() as u64,
                    max.write_length.limit,
                ),
            );

            // main row with execution costs summary
            table.add_row(Row::new(vec![
                Cell::new_align(&format!("{}", method), format::Alignment::LEFT)
                    .with_style(Attr::Bold)
                    .with_style(Attr::ForegroundColor(color::GREEN)),
                Cell::new_align(&format!("{}", method_stats.len()), format::Alignment::RIGHT),
                Cell::new_align(&runtime_summary.to_string(), format::Alignment::RIGHT),
                Cell::new_align(&read_count_summary.to_string(), format::Alignment::RIGHT),
                Cell::new_align(&read_length_summary.to_string(), format::Alignment::RIGHT),
                Cell::new_align(&write_count_summary.to_string(), format::Alignment::RIGHT),
                Cell::new_align(&write_length_summary.to_string(), format::Alignment::RIGHT),
                Cell::new_align(
                    &format!(
                        "{}\n{}\n{}",
                        min.tx_per_block,
                        max.tx_per_block,
                        method_stats.iter().fold(0, |acc, x| acc + x.tx_per_block)
                            / method_stats.len() as u64,
                    ),
                    format::Alignment::RIGHT,
                ),
            ]));
        }

        if let Some((_, method_stats)) = contract_stats.iter().next() {
            let sample = &method_stats[0];

            table.add_row(Row::new(vec![
                Cell::new("Block Limits").with_style(Attr::Bold),
                Cell::new_align("-", format::Alignment::RIGHT),
                Cell::new_align(
                    &format!("{}", sample.runtime.limit),
                    format::Alignment::RIGHT,
                ),
                Cell::new_align(
                    &format!("{}", sample.read_count.limit),
                    format::Alignment::RIGHT,
                ),
                Cell::new_align(
                    &format!("{}", sample.read_length.limit),
                    format::Alignment::RIGHT,
                ),
                Cell::new_align(
                    &format!("{}", sample.write_count.limit),
                    format::Alignment::RIGHT,
                ),
                Cell::new_align(
                    &format!("{}", sample.write_length.limit),
                    format::Alignment::RIGHT,
                ),
            ]));
        }

        table.printstd();
        println!("");
    }
}

fn execution_costs_summary(min: &ExecutionCost, max: &ExecutionCost, avg: &ExecutionCost) -> Table {
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_CLEAN);

    table.add_row(Row::new(vec![
        Cell::new_align("min:", format::Alignment::LEFT),
        Cell::new_align(&format!("{}", min.actual,), format::Alignment::RIGHT),
        Cell::new_align(
            &format!("({:.3}%)", (min.actual as f64 / min.limit as f64 * 100.0)),
            format::Alignment::RIGHT,
        ),
    ]));

    table.add_row(Row::new(vec![
        Cell::new_align("max:", format::Alignment::LEFT),
        Cell::new_align(&format!("{}", max.actual(),), format::Alignment::RIGHT),
        Cell::new_align(
            &format!("({:.3}%)", (max.actual as f64 / max.limit as f64 * 100.0)),
            format::Alignment::RIGHT,
        ),
    ]));

    table.add_row(Row::new(vec![
        Cell::new_align("avg:", format::Alignment::LEFT),
        Cell::new_align(&format!("{}", avg.actual,), format::Alignment::RIGHT),
        Cell::new_align(
            &format!("({:.3}%)", (avg.actual as f64 / avg.limit as f64 * 100.0)),
            format::Alignment::RIGHT,
        ),
    ]));

    table
}

/*
  Implements a custom CostReport struct because the clarity-vm CostSynthesis struct
  does not have the serde::Serialize macro. This is a fix to avoid modifying the VM code.
  Let's try to update it in the upcoming vm version (with epoch 3)
*/

use clarity_repl::{clarity::costs::ExecutionCost, repl::session::CostsReport};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct CostSynthesis {
    pub total: ExecutionCost,
    pub limit: ExecutionCost,
    pub memory: u64,
    pub memory_limit: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct SerializableCostsReport {
    pub test_name: String,
    pub contract_id: String,
    pub method: String,
    pub args: Vec<String>,
    pub cost_result: CostSynthesis,
}

impl SerializableCostsReport {
    pub fn from_vm_costs_report(costs_report: &CostsReport) -> Self {
        let cost_result = CostSynthesis {
            total: costs_report.cost_result.total.clone(),
            limit: costs_report.cost_result.limit.clone(),
            memory: costs_report.cost_result.memory,
            memory_limit: costs_report.cost_result.memory_limit,
        };

        SerializableCostsReport {
            test_name: costs_report.test_name.clone(),
            contract_id: costs_report.contract_id.clone(),
            method: costs_report.method.clone(),
            args: costs_report.args.clone(),
            cost_result,
        }
    }
}

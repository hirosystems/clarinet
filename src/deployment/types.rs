use clarinet_deployments::types::{DeploymentSpecification, TransactionSpecification};
use std::fmt::{Display, Formatter, Result};

pub struct DeploymentSynthesis {
    pub blocks_count: u64,
    pub total_cost: u64,
    pub content: String,
}

impl DeploymentSynthesis {
    pub fn from_deployment(deployment: &DeploymentSpecification) -> DeploymentSynthesis {
        let mut blocks_count = 0;
        let mut total_cost = 0;
        for batch in deployment.plan.batches.iter() {
            blocks_count += 1;
            for tx in batch.transactions.iter() {
                match tx {
                    TransactionSpecification::ContractCall(tx) => {
                        total_cost += tx.cost;
                    }
                    TransactionSpecification::ContractPublish(tx) => {
                        total_cost += tx.cost;
                    }
                    _ => {}
                }
            }
        }
        let file = deployment.to_specification_file();
        let content = match serde_yaml::to_string(&file) {
            Ok(res) => res,
            Err(err) => panic!("unable to serialize deployment {}", err),
        };

        return DeploymentSynthesis {
            total_cost,
            blocks_count,
            content,
        };
    }
}

impl Display for DeploymentSynthesis {
    fn fmt(&self, f: &mut Formatter) -> Result {
        let base: u64 = 10;
        let int_part = self.total_cost / base.pow(6);
        let frac_part = self.total_cost % base.pow(6);
        let formatted_total_cost = format!("{}.{:08}", int_part, frac_part);
        write!(
            f,
            "{}\n\n{}\n{}",
            green!(format!("{}", self.content)),
            blue!(format!("Total cost:\t{} STX", formatted_total_cost)),
            blue!(format!("Duration:\t{} blocks", self.blocks_count))
        )
    }
}

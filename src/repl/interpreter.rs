use crate::clarity::analysis::AnalysisDatabase;
use crate::clarity::types::QualifiedContractIdentifier;
use crate::clarity::{ast, analysis};
use crate::clarity::costs::LimitedCostTracker;

pub struct ClarityInterpreter {

}

impl ClarityInterpreter {

    pub fn new() -> ClarityInterpreter {
        ClarityInterpreter {
        }
    }

    pub fn run(&mut self, snippet: String, contract_identifier: QualifiedContractIdentifier) -> String {

        let mut contract_ast = match ast::build_ast(&contract_identifier, &snippet, &mut ()) {
            Ok(res) => res,
            Err(parse_error) => {
                println!("Parse error: {:?}", parse_error);
                return "".to_string();
            }
        };

        let mut db = AnalysisDatabase::new();
        let result = analysis::run_analysis(
            &contract_identifier, 
            &mut contract_ast.expressions,
            &mut db, 
            false,
            LimitedCostTracker::new_max_limit());
    
        format!("{:?}", result)
    }
}
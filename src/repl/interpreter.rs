use crate::clarity::analysis::AnalysisDatabase;
use crate::clarity::types::QualifiedContractIdentifier;
use crate::clarity::{ast, analysis};
use crate::clarity::ast::ContractAST;
use crate::clarity::analysis::ContractAnalysis;
use crate::clarity::costs::LimitedCostTracker;
use crate::clarity::database::Datastore;

pub struct ClarityInterpreter <'a> {
    analysis_db: AnalysisDatabase<'a> ,
}

impl <'a> ClarityInterpreter <'a> {

    pub fn new() -> ClarityInterpreter<'a>  {
        let datastore = Datastore::new();
        let analysis_db = AnalysisDatabase::new(datastore);

        ClarityInterpreter {
            analysis_db,
        }
    }

    pub fn run(&mut self, snippet: String, contract_identifier: QualifiedContractIdentifier) -> Result<String, String> {

        let ast = self.build_ast(contract_identifier, snippet)?;
        let analysis = self.run_analysis(contract_identifier, ast)?;

        Ok(format!("{:?}", analysis))
    }

    pub fn build_ast(&mut self, contract_identifier: QualifiedContractIdentifier, snippet: String) -> Result<ContractAST, String> {
        let mut contract_ast = match ast::build_ast(&contract_identifier, &snippet, &mut ()) {
            Ok(res) => res,
            Err(error) => {
                let error = format!("Parse error: {:?}", error);
                return Err(error);
            }
        };
        Ok(contract_ast)
    }

    pub fn run_analysis(&mut self, contract_identifier: QualifiedContractIdentifier, contract_ast: ContractAST) -> Result<ContractAnalysis, String> {

        let mut contract_analysis = match analysis::run_analysis(
            &contract_identifier, 
            &mut contract_ast.expressions,
            &mut self.analysis_db, 
            false,
            LimitedCostTracker::new_max_limit()) 
        {
            Ok(res) => res,
            Err(error) => {
                let error = format!("Analysis error: {:?}", error);
                return Err(error);
            }
        };
        Ok(contract_analysis)
    }

}

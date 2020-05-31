use crate::clarity::analysis::AnalysisDatabase;
use crate::clarity::types::QualifiedContractIdentifier;
use crate::clarity::{ast, analysis};
use crate::clarity::ast::ContractAST;
use crate::clarity::analysis::ContractAnalysis;
use crate::clarity::costs::LimitedCostTracker;
use crate::clarity::database::{Datastore, NULL_HEADER_DB};
use crate::clarity::contexts::{ContractContext, GlobalContext};
use crate::clarity::eval_all;
use crate::clarity::diagnostic::Diagnostic;


pub struct ClarityInterpreter {
    datastore: Datastore
}

impl ClarityInterpreter {

    pub fn new() -> ClarityInterpreter {
        let datastore = Datastore::new();

        ClarityInterpreter {
            datastore
        }
    }

    pub fn run(&mut self, snippet: String, contract_identifier: QualifiedContractIdentifier) -> Result<String, (String, Option<Diagnostic>)> {

        let mut ast = self.build_ast(contract_identifier.clone(), snippet)?;
        let analysis = self.run_analysis(contract_identifier.clone(), &mut ast)?;
        let result = self.execute(contract_identifier, &mut ast)?;

        // todo: instead of just returning the value, we should be returning:
        // - value
        // - execution cost
        // - events emitted
        Ok(format!("{}", result))
    }

    pub fn build_ast(&mut self, contract_identifier: QualifiedContractIdentifier, snippet: String) -> Result<ContractAST, (String, Option<Diagnostic>)> {
        let contract_ast = match ast::build_ast(&contract_identifier, &snippet, &mut ()) {
            Ok(res) => res,
            Err(error) => {
                let message = format!("Parsing error: {}", error.diagnostic.message);
                return Err((message, Some(error.diagnostic)));
            }
        };
        Ok(contract_ast)
    }

    pub fn run_analysis(&mut self, contract_identifier: QualifiedContractIdentifier, contract_ast: &mut ContractAST) -> Result<ContractAnalysis, (String, Option<Diagnostic>)> {

        let mut analysis_db = AnalysisDatabase::new(&mut self.datastore);

        let contract_analysis = match analysis::run_analysis(
            &contract_identifier, 
            &mut contract_ast.expressions,
            &mut analysis_db, 
            false,
            LimitedCostTracker::new_max_limit()) 
        {
            Ok(res) => res,
            Err((error, cost_tracker)) => {
                let message = format!("Analysis error: {}", error.diagnostic.message);
                return Err((message, Some(error.diagnostic)));
            }
        };
        Ok(contract_analysis)
    }

    pub fn execute(&mut self, contract_identifier: QualifiedContractIdentifier, contract_ast: &mut ContractAST) -> Result<String, (String, Option<Diagnostic>)> {
        let mut contract_context = ContractContext::new(contract_identifier.clone());
        let mut marf = Datastore::new();
        let conn = marf.as_clarity_db(&NULL_HEADER_DB);
        let mut global_context = GlobalContext::new(conn, LimitedCostTracker::new_max_limit());
        let result = global_context.execute(|g| {
            eval_all(&contract_ast.expressions, &mut contract_context, g)
        });
        match result {
            Ok(Some(value)) => Ok(format!("{}", value)),
            Ok(None) => Ok(format!("()")),
            Err(error) => {
                let error = format!("Error while running snippet: {:?}", error);
                return Err((error, None));
            }
        }
    }
}

use std::collections::HashMap;

use crate::clarity::analysis::errors::{CheckError, CheckErrors, CheckResult};
use crate::clarity::analysis::types::{AnalysisPass, ContractAnalysis};
use crate::clarity::analysis::AnalysisDatabase;
use crate::clarity::functions::define::{DefineFunctions, DefineFunctionsParsed};
use crate::clarity::functions::NativeFunctions;
use crate::clarity::representations::SymbolicExpressionType::{
    Atom, AtomValue, List, LiteralValue,
};
use crate::clarity::representations::{ClarityName, SymbolicExpression};
use crate::clarity::types::{FunctionType, TraitIdentifier, TypeSignature, Value};

pub struct TraitChecker {}

impl AnalysisPass for TraitChecker {
    fn run_pass(
        contract_analysis: &mut ContractAnalysis,
        analysis_db: &mut AnalysisDatabase,
    ) -> CheckResult<()> {
        let mut command = TraitChecker::new();
        command.run(contract_analysis, analysis_db)?;
        Ok(())
    }
}

impl TraitChecker {
    fn new() -> Self {
        Self {}
    }

    pub fn run(
        &mut self,
        contract_analysis: &mut ContractAnalysis,
        analysis_db: &mut AnalysisDatabase,
    ) -> CheckResult<()> {
        for trait_identifier in &contract_analysis.implemented_traits {
            let trait_name = trait_identifier.name.to_string();
            let contract_defining_trait = analysis_db
                .load_contract(&trait_identifier.contract_identifier)
                .ok_or(CheckErrors::TraitReferenceUnknown(
                    trait_identifier.name.to_string(),
                ))?;

            let trait_definition = contract_defining_trait
                .get_defined_trait(&trait_name)
                .ok_or(CheckErrors::TraitReferenceUnknown(
                    trait_identifier.name.to_string(),
                ))?;

            contract_analysis.check_trait_compliance(trait_identifier, trait_definition)?;
        }
        Ok(())
    }
}

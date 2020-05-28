use std::collections::{HashMap, BTreeMap};

use crate::clarity::types::{TypeSignature, FunctionType, QualifiedContractIdentifier};
use crate::clarity::types::signatures::FunctionSignature;
use crate::clarity::analysis::errors::{CheckError, CheckErrors, CheckResult};
use crate::clarity::analysis::type_checker::{ContractAnalysis};
use crate::clarity::representations::{ClarityName};
use std::marker::PhantomData;

pub struct AnalysisDatabase <'a> {
    phantom: &'a str
}

impl <'a> AnalysisDatabase <'a> {
    pub fn new() -> AnalysisDatabase<'a> {
        AnalysisDatabase {
            phantom: &"phantom"
        }
    }

    pub fn execute <F, T, E> (&mut self, f: F) -> Result<T,E> where F: FnOnce(&mut Self) -> Result<T,E>, {
        self.begin();
        let result = f(self)
            .or_else(|e| {
                self.roll_back();
                Err(e)
            })?;
        self.commit();
        Ok(result)
    }

    pub fn begin(&mut self) {
        // self.store.nest();
    }

    pub fn commit(&mut self) {
        // self.store.commit();
    }

    pub fn roll_back(&mut self) {
        // self.store.rollback();
    }

    fn storage_key() -> &'static str {
        "analysis"
    }

    pub fn load_contract(&mut self, cntract_identifier: &QualifiedContractIdentifier) -> Option<ContractAnalysis> {
        None
    }

    pub fn insert_contract(&mut self, contract_identifier: &QualifiedContractIdentifier, contract: &ContractAnalysis) -> CheckResult<()> {
        Ok(())
    }

    pub fn get_public_function_type(&mut self, contract_identifier: &QualifiedContractIdentifier, function_name: &str) -> CheckResult<Option<FunctionType>> {
        Ok(None)
    }

    pub fn get_read_only_function_type(&mut self, contract_identifier: &QualifiedContractIdentifier, function_name: &str) -> CheckResult<Option<FunctionType>> {
        Ok(None)
    }

    pub fn get_defined_trait(&mut self, contract_identifier: &QualifiedContractIdentifier, trait_name: &str) -> CheckResult<Option<BTreeMap<ClarityName, FunctionSignature>>> {
        Ok(None)
    }

    pub fn get_map_type(&mut self, contract_identifier: &QualifiedContractIdentifier, map_name: &str) -> CheckResult<(TypeSignature, TypeSignature)> {
        Ok((TypeSignature::NoType, TypeSignature::NoType))
    }
}

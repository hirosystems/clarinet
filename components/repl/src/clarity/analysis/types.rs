use crate::clarity::analysis::analysis_db::AnalysisDatabase;
use crate::clarity::analysis::contract_interface_builder::ContractInterface;
use crate::clarity::analysis::errors::{CheckErrors, CheckResult};
use crate::clarity::analysis::type_checker::contexts::TypeMap;
use crate::clarity::costs::{CostTracker, ExecutionCost, LimitedCostTracker};
use crate::clarity::types::signatures::FunctionSignature;
use crate::clarity::types::{
    FunctionType, QualifiedContractIdentifier, TraitIdentifier, TypeSignature,
};
use crate::clarity::{ClarityName, SymbolicExpression};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};

const DESERIALIZE_FAIL_MESSAGE: &str =
    "PANIC: Failed to deserialize bad database data in contract analysis.";
const SERIALIZE_FAIL_MESSAGE: &str =
    "PANIC: Failed to deserialize bad database data in contract analysis.";

pub trait AnalysisPass {
    fn run_pass(
        contract_analysis: &mut ContractAnalysis,
        analysis_db: &mut AnalysisDatabase,
    ) -> CheckResult<()>;
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ContractAnalysis {
    pub contract_identifier: QualifiedContractIdentifier,
    pub private_function_types: BTreeMap<ClarityName, FunctionType>,
    pub variable_types: BTreeMap<ClarityName, TypeSignature>,
    pub public_function_types: BTreeMap<ClarityName, FunctionType>,
    pub read_only_function_types: BTreeMap<ClarityName, FunctionType>,
    pub map_types: BTreeMap<ClarityName, (TypeSignature, TypeSignature)>,
    pub persisted_variable_types: BTreeMap<ClarityName, TypeSignature>,
    pub fungible_tokens: BTreeSet<ClarityName>,
    pub non_fungible_tokens: BTreeMap<ClarityName, TypeSignature>,
    pub defined_traits: BTreeMap<ClarityName, BTreeMap<ClarityName, FunctionSignature>>,
    pub implemented_traits: BTreeSet<TraitIdentifier>,
    pub contract_interface: Option<ContractInterface>,
    pub is_cost_contract_eligible: bool,
    pub dependencies: Vec<QualifiedContractIdentifier>,
    #[serde(skip)]
    pub expressions: Vec<SymbolicExpression>,
    #[serde(skip)]
    pub type_map: Option<TypeMap>,
    #[serde(skip)]
    pub cost_track: Option<LimitedCostTracker>,
}

impl ContractAnalysis {
    pub fn new(
        contract_identifier: QualifiedContractIdentifier,
        expressions: Vec<SymbolicExpression>,
        cost_track: LimitedCostTracker,
    ) -> ContractAnalysis {
        ContractAnalysis {
            contract_identifier,
            expressions,
            type_map: None,
            contract_interface: None,
            private_function_types: BTreeMap::new(),
            public_function_types: BTreeMap::new(),
            read_only_function_types: BTreeMap::new(),
            variable_types: BTreeMap::new(),
            map_types: BTreeMap::new(),
            persisted_variable_types: BTreeMap::new(),
            defined_traits: BTreeMap::new(),
            implemented_traits: BTreeSet::new(),
            fungible_tokens: BTreeSet::new(),
            non_fungible_tokens: BTreeMap::new(),
            cost_track: Some(cost_track),
            is_cost_contract_eligible: false,
            dependencies: Vec::new(),
        }
    }

    pub fn take_contract_cost_tracker(&mut self) -> LimitedCostTracker {
        self.cost_track
            .take()
            .expect("BUG: contract analysis attempted to take a cost tracker already claimed.")
    }

    pub fn replace_contract_cost_tracker(&mut self, cost_track: LimitedCostTracker) {
        assert!(self.cost_track.is_none());
        self.cost_track.replace(cost_track);
    }

    pub fn add_map_type(
        &mut self,
        name: ClarityName,
        key_type: TypeSignature,
        map_type: TypeSignature,
    ) {
        self.map_types.insert(name, (key_type, map_type));
    }

    pub fn add_variable_type(&mut self, name: ClarityName, variable_type: TypeSignature) {
        self.variable_types.insert(name, variable_type);
    }

    pub fn add_persisted_variable_type(
        &mut self,
        name: ClarityName,
        persisted_variable_type: TypeSignature,
    ) {
        self.persisted_variable_types
            .insert(name, persisted_variable_type);
    }

    pub fn add_read_only_function(&mut self, name: ClarityName, function_type: FunctionType) {
        self.read_only_function_types.insert(name, function_type);
    }

    pub fn add_public_function(&mut self, name: ClarityName, function_type: FunctionType) {
        self.public_function_types.insert(name, function_type);
    }

    pub fn add_private_function(&mut self, name: ClarityName, function_type: FunctionType) {
        self.private_function_types.insert(name, function_type);
    }

    pub fn add_non_fungible_token(&mut self, name: ClarityName, nft_type: TypeSignature) {
        self.non_fungible_tokens.insert(name, nft_type);
    }

    pub fn add_fungible_token(&mut self, name: ClarityName) {
        self.fungible_tokens.insert(name);
    }

    pub fn add_defined_trait(
        &mut self,
        name: ClarityName,
        function_types: BTreeMap<ClarityName, FunctionSignature>,
    ) {
        self.defined_traits.insert(name, function_types);
    }

    pub fn add_implemented_trait(&mut self, trait_identifier: TraitIdentifier) {
        self.implemented_traits.insert(trait_identifier);
    }

    pub fn add_dependency(&mut self, dependency: QualifiedContractIdentifier) {
        self.dependencies.push(dependency);
    }

    pub fn get_public_function_type(&self, name: &str) -> Option<&FunctionType> {
        self.public_function_types.get(name)
    }

    pub fn get_read_only_function_type(&self, name: &str) -> Option<&FunctionType> {
        self.read_only_function_types.get(name)
    }

    pub fn get_private_function(&self, name: &str) -> Option<&FunctionType> {
        self.private_function_types.get(name)
    }

    pub fn get_map_type(&self, name: &str) -> Option<&(TypeSignature, TypeSignature)> {
        self.map_types.get(name)
    }

    pub fn get_variable_type(&self, name: &str) -> Option<&TypeSignature> {
        self.variable_types.get(name)
    }

    pub fn get_persisted_variable_type(&self, name: &str) -> Option<&TypeSignature> {
        self.persisted_variable_types.get(name)
    }

    pub fn get_defined_trait(
        &self,
        name: &str,
    ) -> Option<&BTreeMap<ClarityName, FunctionSignature>> {
        self.defined_traits.get(name)
    }

    pub fn check_trait_compliance(
        &self,
        trait_identifier: &TraitIdentifier,
        trait_definition: &BTreeMap<ClarityName, FunctionSignature>,
    ) -> CheckResult<()> {
        let trait_name = trait_identifier.name.to_string();

        for (func_name, expected_sig) in trait_definition.iter() {
            match (
                self.get_public_function_type(func_name),
                self.get_read_only_function_type(func_name),
            ) {
                (Some(FunctionType::Fixed(func)), None)
                | (None, Some(FunctionType::Fixed(func))) => {
                    let args_sig = func.args.iter().map(|a| a.signature.clone()).collect();
                    if !expected_sig.check_args_trait_compliance(args_sig) {
                        return Err(CheckErrors::BadTraitImplementation(
                            trait_name,
                            func_name.to_string(),
                        )
                        .into());
                    }

                    if !expected_sig.returns.admits_type(&func.returns) {
                        return Err(CheckErrors::BadTraitImplementation(
                            trait_name,
                            func_name.to_string(),
                        )
                        .into());
                    }
                }
                (_, _) => {
                    return Err(CheckErrors::BadTraitImplementation(
                        trait_name,
                        func_name.to_string(),
                    )
                    .into())
                }
            }
        }
        Ok(())
    }
}

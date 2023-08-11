use crate::analysis::annotation::Annotation;
use crate::analysis::ast_visitor::{traverse, ASTVisitor};
use crate::analysis::{AnalysisPass, AnalysisResult, Settings};
use crate::repl::DEFAULT_EPOCH;
use clarity::types::StacksEpochId;
use clarity::vm::analysis::analysis_db::AnalysisDatabase;
pub use clarity::vm::analysis::types::ContractAnalysis;
use clarity::vm::analysis::{CheckErrors, CheckResult};
use clarity::vm::ast::ContractAST;
use clarity::vm::representations::{SymbolicExpression, TraitDefinition};
use clarity::vm::types::signatures::CallableSubtype;
use clarity::vm::types::{
    FixedFunction, FunctionSignature, FunctionType, PrincipalData, QualifiedContractIdentifier,
    TraitIdentifier, TypeSignature, Value,
};
use clarity::vm::{ClarityName, ClarityVersion, SymbolicExpressionType};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::convert::TryFrom;
use std::hash::{Hash, Hasher};
use std::iter::FromIterator;
use std::ops::{Deref, DerefMut};
use std::process;

use super::ast_visitor::TypedVar;

lazy_static! {
    pub static ref DEFAULT_NAME: ClarityName = ClarityName::try_from("placeholder").unwrap();
}

pub struct ASTDependencyDetector<'a> {
    dependencies: BTreeMap<QualifiedContractIdentifier, DependencySet>,
    current_clarity_version: Option<&'a ClarityVersion>,
    current_contract: Option<&'a QualifiedContractIdentifier>,
    defined_functions:
        BTreeMap<(&'a QualifiedContractIdentifier, &'a ClarityName), Vec<TypeSignature>>,
    defined_traits: BTreeMap<
        (&'a QualifiedContractIdentifier, &'a ClarityName),
        BTreeMap<ClarityName, FunctionSignature>,
    >,
    pending_function_checks: BTreeMap<
        // function identifier whose type is not yet defined
        (&'a QualifiedContractIdentifier, &'a ClarityName),
        // list of contracts that need to be checked once this function is
        // defined, together with the associated args
        Vec<(&'a QualifiedContractIdentifier, &'a [SymbolicExpression])>,
    >,
    pending_trait_checks: BTreeMap<
        // trait that is not yet defined
        &'a TraitIdentifier,
        // list of contracts that need to be checked once this trait is
        // defined, together with the function called and the associated args.
        Vec<(
            &'a QualifiedContractIdentifier,
            &'a ClarityName,
            &'a [SymbolicExpression],
        )>,
    >,
    params: Option<Vec<TypedVar<'a>>>,
    top_level: bool,
    preloaded: &'a BTreeMap<QualifiedContractIdentifier, (ClarityVersion, ContractAST)>,
}

#[derive(Clone, Debug, Eq)]
pub struct Dependency {
    pub contract_id: QualifiedContractIdentifier,
    pub required_before_publish: bool,
}

impl PartialEq for Dependency {
    fn eq(&self, other: &Self) -> bool {
        self.contract_id == other.contract_id
    }
}

impl PartialOrd for Dependency {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.contract_id.partial_cmp(&other.contract_id)
    }
}

impl Ord for Dependency {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.contract_id.cmp(&other.contract_id)
    }
}

#[derive(Debug, Clone)]
pub struct DependencySet {
    pub set: BTreeSet<Dependency>,
}

impl DependencySet {
    pub fn new() -> DependencySet {
        DependencySet {
            set: BTreeSet::new(),
        }
    }

    pub fn add_dependency(
        &mut self,
        contract_id: QualifiedContractIdentifier,
        required_before_publish: bool,
    ) {
        let dep = Dependency {
            contract_id,
            required_before_publish,
        };

        // If this dependency is required before publish, then make sure to
        // delete any existing dependency so that this overrides it.
        if required_before_publish {
            self.set.remove(&dep);
        }

        self.set.insert(dep);
    }

    pub fn has_dependency(&self, contract_id: &QualifiedContractIdentifier) -> Option<bool> {
        if let Some(dep) = self.set.get(&Dependency {
            contract_id: contract_id.clone(),
            required_before_publish: false,
        }) {
            Some(dep.required_before_publish)
        } else {
            None
        }
    }
}

impl Deref for DependencySet {
    type Target = BTreeSet<Dependency>;

    fn deref(&self) -> &Self::Target {
        &self.set
    }
}

impl DerefMut for DependencySet {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.set
    }
}

impl<'a> ASTDependencyDetector<'a> {
    pub fn detect_dependencies(
        contract_asts: &'a BTreeMap<QualifiedContractIdentifier, (ClarityVersion, ContractAST)>,
        preloaded: &'a BTreeMap<QualifiedContractIdentifier, (ClarityVersion, ContractAST)>,
    ) -> Result<
        BTreeMap<QualifiedContractIdentifier, DependencySet>,
        (
            // Dependencies detected
            BTreeMap<QualifiedContractIdentifier, DependencySet>,
            // Unresolved dependencies detected
            Vec<QualifiedContractIdentifier>,
        ),
    > {
        let mut detector = Self {
            dependencies: BTreeMap::new(),
            current_clarity_version: None,
            current_contract: None,
            defined_functions: BTreeMap::new(),
            defined_traits: BTreeMap::new(),
            pending_function_checks: BTreeMap::new(),
            pending_trait_checks: BTreeMap::new(),
            params: None,
            top_level: true,
            preloaded,
        };

        let mut preloaded_visitor = PreloadedVisitor {
            detector: &mut detector,
            current_clarity_version: None,
            current_contract: None,
        };

        for (contract_identifier, (clarity_version, ast)) in preloaded {
            preloaded_visitor.current_clarity_version = Some(clarity_version);
            preloaded_visitor.current_contract = Some(contract_identifier);
            traverse(&mut preloaded_visitor, &ast.expressions);
        }

        for (contract_identifier, (clarity_version, ast)) in contract_asts {
            detector
                .dependencies
                .insert(contract_identifier.clone(), DependencySet::new());
            detector.current_clarity_version = Some(clarity_version);
            detector.current_contract = Some(contract_identifier);
            traverse(&mut detector, &ast.expressions);
        }

        // Anything remaining in the pending_ maps indicates an unresolved dependency
        let mut unresolved: Vec<QualifiedContractIdentifier> = detector
            .pending_function_checks
            .into_keys()
            .map(|(contract_id, name)| contract_id.clone())
            .collect();
        unresolved.append(
            &mut detector
                .pending_trait_checks
                .into_keys()
                .map(|trait_id| trait_id.contract_identifier.clone())
                .collect(),
        );
        if !unresolved.is_empty() {
            Err((detector.dependencies, unresolved))
        } else {
            Ok(detector.dependencies)
        }
    }

    pub fn order_contracts<'deps>(
        dependencies: &'deps BTreeMap<QualifiedContractIdentifier, DependencySet>,
        contract_epochs: &HashMap<QualifiedContractIdentifier, StacksEpochId>,
    ) -> CheckResult<Vec<&'deps QualifiedContractIdentifier>> {
        let mut lookup = BTreeMap::new();
        let mut reverse_lookup = Vec::new();

        let mut index: usize = 0;

        if dependencies.is_empty() {
            return Ok(vec![]);
        }

        for (contract, _) in dependencies {
            lookup.insert(contract, index);
            reverse_lookup.push(contract);
            index += 1;
        }

        let mut graph = Graph::new();
        for (contract, contract_dependencies) in dependencies {
            let contract_id = lookup.get(contract).unwrap();
            // Boot contracts will not be in the contract_epochs map, so default to Epoch20
            let contract_epoch = contract_epochs
                .get(contract)
                .unwrap_or(&StacksEpochId::Epoch20);
            graph.add_node(*contract_id);
            for dep in contract_dependencies.iter() {
                let dep_epoch = contract_epochs
                    .get(&dep.contract_id)
                    .unwrap_or(&StacksEpochId::Epoch20);
                if contract_epoch < dep_epoch {
                    return Err(CheckErrors::NoSuchContract(dep.contract_id.to_string()).into());
                }
                let dep_id = match lookup.get(&dep.contract_id) {
                    Some(id) => id,
                    None => {
                        // No need to report an error here, it will be caught
                        // and reported with proper location information by the
                        // later analyses. Just skip it.
                        continue;
                    }
                };
                graph.add_directed_edge(*contract_id, *dep_id);
            }
        }

        let mut walker = GraphWalker::new();
        let sorted_indexes = walker.get_sorted_dependencies(&graph);

        let cyclic_deps = walker.get_cycling_dependencies(&graph, &sorted_indexes);
        if let Some(deps) = cyclic_deps {
            let mut contracts = vec![];
            for index in deps.iter() {
                let contract = reverse_lookup[*index];
                contracts.push(contract.name.to_string());
            }
            return Err(CheckErrors::CircularReference(contracts).into());
        }

        Ok(sorted_indexes
            .iter()
            .map(|index| reverse_lookup[*index])
            .collect())
    }

    fn add_dependency(
        &mut self,
        from: &QualifiedContractIdentifier,
        to: &QualifiedContractIdentifier,
    ) {
        if self.preloaded.contains_key(from) {
            return;
        }

        // Ignore the placeholder contract.
        if to.name.starts_with("__") {
            return;
        }

        if let Some(set) = self.dependencies.get_mut(from) {
            set.add_dependency(to.clone(), self.top_level);
        } else {
            let mut set = DependencySet::new();
            set.add_dependency(to.clone(), self.top_level);
            self.dependencies.insert(from.clone(), set);
        }
    }

    fn add_defined_function(
        &mut self,
        contract_identifier: &'a QualifiedContractIdentifier,
        name: &'a ClarityName,
        param_types: Vec<TypeSignature>,
    ) {
        if let Some(pending) = self
            .pending_function_checks
            .remove(&(contract_identifier, name))
        {
            for (caller, args) in pending {
                for dependency in self.check_callee_type(&param_types, args) {
                    self.add_dependency(caller, &dependency);
                }
            }
        }

        self.defined_functions
            .insert((contract_identifier, name), param_types);
    }

    fn add_pending_function_check(
        &mut self,
        caller: &'a QualifiedContractIdentifier,
        callee: (&'a QualifiedContractIdentifier, &'a ClarityName),
        args: &'a [SymbolicExpression],
    ) {
        if let Some(list) = self.pending_function_checks.get_mut(&callee) {
            list.push((caller, args));
        } else {
            self.pending_function_checks
                .insert(callee, vec![(caller, args)]);
        }
    }

    fn add_defined_trait(
        &mut self,
        contract_identifier: &'a QualifiedContractIdentifier,
        name: &'a ClarityName,
        trait_definition: BTreeMap<ClarityName, FunctionSignature>,
    ) {
        if let Some(pending) = self.pending_trait_checks.remove(&TraitIdentifier {
            name: name.clone(),
            contract_identifier: contract_identifier.clone(),
        }) {
            for (caller, function, args) in pending {
                for dependency in self.check_trait_dependencies(&trait_definition, function, args) {
                    self.add_dependency(caller, &dependency);
                }
            }
        }

        self.defined_traits
            .insert((contract_identifier, name), trait_definition);
    }

    fn add_pending_trait_check(
        &mut self,
        caller: &'a QualifiedContractIdentifier,
        callee: &'a TraitIdentifier,
        function: &'a ClarityName,
        args: &'a [SymbolicExpression],
    ) {
        if let Some(list) = self.pending_trait_checks.get_mut(callee) {
            list.push((caller, function, args));
        } else {
            self.pending_trait_checks
                .insert(callee, vec![(caller, function, args)]);
        }
    }

    fn check_callee_type(
        &self,
        arg_types: &Vec<TypeSignature>,
        args: &'a [SymbolicExpression],
    ) -> Vec<QualifiedContractIdentifier> {
        let mut dependencies = Vec::new();
        for (i, arg) in arg_types.iter().enumerate() {
            if matches!(arg, TypeSignature::CallableType(CallableSubtype::Trait(_)))
                | matches!(arg, TypeSignature::TraitReferenceType(_))
            {
                if args.len() > i {
                    if let Some(Value::Principal(PrincipalData::Contract(contract))) =
                        args[i].match_literal_value()
                    {
                        dependencies.push(contract.clone());
                    }
                }
            }
        }
        dependencies
    }

    fn check_trait_dependencies(
        &self,
        trait_definition: &BTreeMap<ClarityName, FunctionSignature>,
        function_name: &ClarityName,
        args: &'a [SymbolicExpression],
    ) -> Vec<QualifiedContractIdentifier> {
        // Since this may run before checkers, the function may not be valid.
        // If the key does not exist, just return an empty set and the error
        // will be reported elsewhere.
        let function_signature = match trait_definition.get(function_name) {
            Some(signature) => signature,
            None => return Vec::new(),
        };
        self.check_callee_type(&function_signature.args, args)
    }

    // A trait can only come from a parameter (cannot be a let binding or a return value), so
    // find the corresponding parameter and return it.
    fn get_param_trait(&self, name: &ClarityName) -> Option<&'a TraitIdentifier> {
        let params = match &self.params {
            None => return None,
            Some(params) => params,
        };
        for param in params {
            if param.name == name {
                if let SymbolicExpressionType::TraitReference(_, trait_def) = &param.type_expr.expr
                {
                    return match trait_def {
                        TraitDefinition::Defined(identifier) => Some(identifier),
                        TraitDefinition::Imported(identifier) => Some(identifier),
                    };
                } else {
                    return None;
                }
            }
        }
        None
    }
}

impl<'a> ASTVisitor<'a> for ASTDependencyDetector<'a> {
    // For the following traverse_define_* functions, we just want to store a
    // map of the parameter types, to be used to extract the trait type in a
    // dynamic contract call.
    fn traverse_define_private(
        &mut self,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        parameters: Option<Vec<TypedVar<'a>>>,
        body: &'a SymbolicExpression,
    ) -> bool {
        self.params = parameters.clone();
        self.top_level = false;
        let res =
            self.traverse_expr(body) && self.visit_define_private(expr, name, parameters, body);
        self.params = None;
        self.top_level = true;
        res
    }

    fn visit_define_private(
        &mut self,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        parameters: Option<Vec<TypedVar<'a>>>,
        body: &'a SymbolicExpression,
    ) -> bool {
        let param_types = match parameters {
            Some(parameters) => parameters
                .iter()
                .map(|typed_var| {
                    TypeSignature::parse_type_repr(DEFAULT_EPOCH, typed_var.type_expr, &mut ())
                        .unwrap_or(TypeSignature::BoolType)
                })
                .collect(),
            None => Vec::new(),
        };

        self.add_defined_function(self.current_contract.unwrap(), name, param_types);
        true
    }

    fn traverse_define_read_only(
        &mut self,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        parameters: Option<Vec<TypedVar<'a>>>,
        body: &'a SymbolicExpression,
    ) -> bool {
        self.params = parameters.clone();
        self.top_level = false;
        let res =
            self.traverse_expr(body) && self.visit_define_read_only(expr, name, parameters, body);
        self.params = None;
        self.top_level = true;
        res
    }

    fn visit_define_read_only(
        &mut self,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        parameters: Option<Vec<TypedVar<'a>>>,
        body: &'a SymbolicExpression,
    ) -> bool {
        let param_types = match parameters {
            Some(parameters) => parameters
                .iter()
                .map(|typed_var| {
                    TypeSignature::parse_type_repr(DEFAULT_EPOCH, typed_var.type_expr, &mut ())
                        .unwrap_or(TypeSignature::BoolType)
                })
                .collect(),
            None => Vec::new(),
        };

        self.add_defined_function(self.current_contract.unwrap(), name, param_types);
        true
    }

    fn traverse_define_public(
        &mut self,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        parameters: Option<Vec<TypedVar<'a>>>,
        body: &'a SymbolicExpression,
    ) -> bool {
        self.params = parameters.clone();
        self.top_level = false;
        let res =
            self.traverse_expr(body) && self.visit_define_public(expr, name, parameters, body);
        self.params = None;
        self.top_level = true;
        res
    }

    fn visit_define_public(
        &mut self,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        parameters: Option<Vec<TypedVar<'a>>>,
        body: &'a SymbolicExpression,
    ) -> bool {
        let param_types = match parameters {
            Some(parameters) => parameters
                .iter()
                .map(|typed_var| {
                    TypeSignature::parse_type_repr(DEFAULT_EPOCH, typed_var.type_expr, &mut ())
                        .unwrap_or(TypeSignature::BoolType)
                })
                .collect(),
            None => Vec::new(),
        };

        self.add_defined_function(self.current_contract.unwrap(), name, param_types);
        true
    }

    fn visit_define_trait(
        &mut self,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        functions: &'a [SymbolicExpression],
    ) -> bool {
        if let Ok(trait_definition) = TypeSignature::parse_trait_type_repr(
            functions,
            &mut (),
            DEFAULT_EPOCH,
            *self.current_clarity_version.unwrap(),
        ) {
            self.add_defined_trait(self.current_contract.unwrap(), name, trait_definition);
        }
        true
    }

    fn visit_static_contract_call(
        &mut self,
        expr: &'a SymbolicExpression,
        contract_identifier: &'a QualifiedContractIdentifier,
        function_name: &'a ClarityName,
        args: &'a [SymbolicExpression],
    ) -> bool {
        self.add_dependency(self.current_contract.unwrap(), contract_identifier);
        let dependencies = if let Some(arg_types) = self
            .defined_functions
            .get(&(contract_identifier, function_name))
        {
            // If we know the type of this function, check the parameters for traits
            self.check_callee_type(arg_types, args)
        } else {
            // If we do not yet know the type of this function, record it to re-analyze later
            self.add_pending_function_check(
                self.current_contract.unwrap(),
                (contract_identifier, function_name),
                args,
            );
            return true;
        };
        for dependency in dependencies {
            self.add_dependency(self.current_contract.unwrap(), &dependency);
        }
        true
    }

    fn visit_dynamic_contract_call(
        &mut self,
        expr: &'a SymbolicExpression,
        trait_ref: &'a SymbolicExpression,
        function_name: &'a ClarityName,
        args: &'a [SymbolicExpression],
    ) -> bool {
        let trait_instance = trait_ref.match_atom().unwrap_or(&DEFAULT_NAME);
        if let Some(trait_identifier) = self.get_param_trait(trait_instance) {
            let dependencies = if let Some(trait_definition) = self.defined_traits.get(&(
                &trait_identifier.contract_identifier,
                &trait_identifier.name,
            )) {
                self.check_trait_dependencies(trait_definition, function_name, args)
            } else {
                self.add_pending_trait_check(
                    &self.current_contract.unwrap(),
                    trait_identifier,
                    function_name,
                    args,
                );
                return true;
            };

            for dependency in dependencies {
                self.add_dependency(self.current_contract.unwrap(), &dependency);
            }
        }
        true
    }

    fn visit_call_user_defined(
        &mut self,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        args: &'a [SymbolicExpression],
    ) -> bool {
        if let Some(arg_types) = self
            .defined_functions
            .get(&(&self.current_contract.unwrap(), name))
        {
            for dependency in self.check_callee_type(arg_types, args) {
                self.add_dependency(self.current_contract.unwrap(), &dependency);
            }
        }

        true
    }

    fn visit_use_trait(
        &mut self,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        trait_identifier: &TraitIdentifier,
    ) -> bool {
        self.add_dependency(
            self.current_contract.unwrap(),
            &trait_identifier.contract_identifier,
        );
        true
    }

    fn visit_impl_trait(
        &mut self,
        expr: &'a SymbolicExpression,
        trait_identifier: &TraitIdentifier,
    ) -> bool {
        self.add_dependency(
            self.current_contract.unwrap(),
            &trait_identifier.contract_identifier,
        );
        true
    }
}

// Traverses the preloaded contracts and saves function signatures only

struct PreloadedVisitor<'a, 'b> {
    detector: &'b mut ASTDependencyDetector<'a>,
    current_clarity_version: Option<&'a ClarityVersion>,
    current_contract: Option<&'a QualifiedContractIdentifier>,
}
impl<'a, 'b> ASTVisitor<'a> for PreloadedVisitor<'a, 'b> {
    fn traverse_define_read_only(
        &mut self,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        parameters: Option<Vec<TypedVar<'a>>>,
        body: &'a SymbolicExpression,
    ) -> bool {
        let param_types = match parameters {
            Some(parameters) => parameters
                .iter()
                .map(|typed_var| {
                    TypeSignature::parse_type_repr(DEFAULT_EPOCH, typed_var.type_expr, &mut ())
                        .unwrap_or(TypeSignature::BoolType)
                })
                .collect(),
            None => Vec::new(),
        };

        self.detector
            .add_defined_function(self.current_contract.unwrap(), name, param_types);
        true
    }

    fn traverse_define_public(
        &mut self,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        parameters: Option<Vec<TypedVar<'a>>>,
        body: &'a SymbolicExpression,
    ) -> bool {
        let param_types = match parameters {
            Some(parameters) => parameters
                .iter()
                .map(|typed_var| {
                    TypeSignature::parse_type_repr(DEFAULT_EPOCH, typed_var.type_expr, &mut ())
                        .unwrap_or(TypeSignature::BoolType)
                })
                .collect(),
            None => Vec::new(),
        };

        self.detector
            .add_defined_function(self.current_contract.unwrap(), name, param_types);
        true
    }

    fn traverse_define_trait(
        &mut self,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        functions: &'a [SymbolicExpression],
    ) -> bool {
        if let Ok(trait_definition) = TypeSignature::parse_trait_type_repr(
            functions,
            &mut (),
            DEFAULT_EPOCH,
            *self.current_clarity_version.unwrap(),
        ) {
            self.detector
                .add_defined_trait(self.current_contract.unwrap(), name, trait_definition);
        }
        true
    }
}

struct Graph {
    pub adjacency_list: Vec<Vec<usize>>,
}

impl Graph {
    fn new() -> Self {
        Self {
            adjacency_list: Vec::new(),
        }
    }

    fn add_node(&mut self, _expr_index: usize) {
        self.adjacency_list.push(vec![]);
    }

    fn add_directed_edge(&mut self, src_expr_index: usize, dst_expr_index: usize) {
        let list = self.adjacency_list.get_mut(src_expr_index).unwrap();
        list.push(dst_expr_index);
    }

    fn get_node_descendants(&self, expr_index: usize) -> Vec<usize> {
        self.adjacency_list[expr_index].clone()
    }

    fn has_node_descendants(&self, expr_index: usize) -> bool {
        self.adjacency_list[expr_index].len() > 0
    }

    fn nodes_count(&self) -> usize {
        self.adjacency_list.len()
    }
}

struct GraphWalker {
    seen: HashSet<usize>,
}

impl GraphWalker {
    fn new() -> Self {
        Self {
            seen: HashSet::new(),
        }
    }

    /// Depth-first search producing a post-order sort
    fn get_sorted_dependencies(&mut self, graph: &Graph) -> Vec<usize> {
        let mut sorted_indexes = Vec::<usize>::new();
        for expr_index in 0..graph.nodes_count() {
            self.sort_dependencies_recursion(expr_index, graph, &mut sorted_indexes);
        }

        sorted_indexes
    }

    fn sort_dependencies_recursion(
        &mut self,
        tle_index: usize,
        graph: &Graph,
        branch: &mut Vec<usize>,
    ) {
        if self.seen.contains(&tle_index) {
            return;
        }

        self.seen.insert(tle_index);
        if let Some(list) = graph.adjacency_list.get(tle_index) {
            for neighbor in list.iter() {
                self.sort_dependencies_recursion(neighbor.clone(), graph, branch);
            }
        }
        branch.push(tle_index);
    }

    fn get_cycling_dependencies(
        &mut self,
        graph: &Graph,
        sorted_indexes: &Vec<usize>,
    ) -> Option<Vec<usize>> {
        let mut tainted: HashSet<usize> = HashSet::new();

        for node in sorted_indexes.iter() {
            let mut tainted_descendants_count = 0;
            let descendants = graph.get_node_descendants(*node);
            for descendant in descendants.iter() {
                if !graph.has_node_descendants(*descendant) || tainted.contains(descendant) {
                    tainted.insert(*descendant);
                    tainted_descendants_count += 1;
                }
            }
            if tainted_descendants_count == descendants.len() {
                tainted.insert(*node);
            }
        }

        if tainted.len() == sorted_indexes.len() {
            return None;
        }

        let nodes = HashSet::from_iter(sorted_indexes.iter().cloned());
        let deps = nodes.difference(&tainted).map(|i| *i).collect();
        Some(deps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repl::session::Session;
    use crate::repl::{
        ClarityCodeSource, ClarityContract, ContractDeployer, SessionSettings,
        DEFAULT_CLARITY_VERSION, DEFAULT_EPOCH,
    };
    use ::clarity::vm::diagnostic::Diagnostic;

    fn build_ast(
        session: &Session,
        snippet: &str,
        name: Option<&str>,
    ) -> Result<(QualifiedContractIdentifier, ContractAST, Vec<Diagnostic>), String> {
        let contract = ClarityContract {
            code_source: ClarityCodeSource::ContractInMemory(snippet.to_string()),
            name: name.unwrap_or("contract").to_string(),
            deployer: ContractDeployer::Transient,
            clarity_version: DEFAULT_CLARITY_VERSION,
            epoch: DEFAULT_EPOCH,
        };
        let (ast, diags, _) = session.interpreter.build_ast(&contract);
        Ok((
            contract.expect_resolved_contract_identifier(None),
            ast,
            diags,
        ))
    }

    #[test]
    fn no_deps() {
        let session = Session::new(SessionSettings::default());
        let snippet = "
(define-public (hello)
    (ok (print \"hello\"))
)
"
        .to_string();
        match build_ast(&session, &snippet, None) {
            Ok((contract_identifier, ast, _)) => {
                let mut contracts = BTreeMap::new();
                contracts.insert(contract_identifier.clone(), (DEFAULT_CLARITY_VERSION, ast));
                let dependencies =
                    ASTDependencyDetector::detect_dependencies(&contracts, &BTreeMap::new())
                        .unwrap();
                assert_eq!(dependencies[&contract_identifier].len(), 0);
            }
            Err(_) => panic!("expected success"),
        }
    }

    #[test]
    fn contract_call() {
        let session = Session::new(SessionSettings::default());
        let mut contracts = BTreeMap::new();
        let snippet1 = "
(define-public (hello (a int))
    (ok u0)
)"
        .to_string();
        let foo = match build_ast(&session, &snippet1, Some("foo")) {
            Ok((contract_identifier, ast, _)) => {
                contracts.insert(contract_identifier.clone(), (DEFAULT_CLARITY_VERSION, ast));
                contract_identifier
            }
            Err(_) => panic!("expected success"),
        };

        let snippet = "
(define-public (call-foo)
    (contract-call? .foo hello 4)
)
"
        .to_string();
        let test_identifier = match build_ast(&session, &snippet, Some("test")) {
            Ok((contract_identifier, ast, _)) => {
                contracts.insert(contract_identifier.clone(), (DEFAULT_CLARITY_VERSION, ast));
                contract_identifier
            }
            Err(_) => panic!("expected success"),
        };

        let dependencies =
            ASTDependencyDetector::detect_dependencies(&contracts, &BTreeMap::new()).unwrap();
        assert_eq!(dependencies[&test_identifier].len(), 1);
        assert!(!dependencies[&test_identifier].has_dependency(&foo).unwrap());
    }

    // This test is disabled because it is currently not possible to refer to a
    // trait defined in the same contract. An issue has been opened to discuss
    // whether this will be fixed or documented.
    // #[test]
    fn dynamic_contract_call_local_trait() {
        let session = Session::new(SessionSettings::default());
        let mut contracts = BTreeMap::new();
        let snippet1 = "
(define-public (hello (a int))
    (ok u0)
)"
        .to_string();
        let bar = match build_ast(&session, &snippet1, Some("bar")) {
            Ok((contract_identifier, ast, _)) => {
                contracts.insert(contract_identifier.clone(), (DEFAULT_CLARITY_VERSION, ast));
                contract_identifier
            }
            Err(_) => panic!("expected success"),
        };

        let snippet = "
(define-trait my-trait
    ((hello (int) (response uint uint)))
)
(define-trait dyn-trait
    ((call-hello (<my-trait>) (response uint uint)))
)
(define-public (call-dyn (dt <dyn-trait>))
    (contract-call? dt call-hello .bar)
)
"
        .to_string();
        let test_identifier = match build_ast(&session, &snippet, Some("test")) {
            Ok((contract_identifier, ast, _)) => {
                contracts.insert(contract_identifier.clone(), (DEFAULT_CLARITY_VERSION, ast));
                contract_identifier
            }
            Err(_) => panic!("expected success"),
        };

        let dependencies =
            ASTDependencyDetector::detect_dependencies(&contracts, &BTreeMap::new()).unwrap();
        assert_eq!(dependencies[&test_identifier].len(), 1);
        assert!(!dependencies[&test_identifier].has_dependency(&bar).unwrap());
    }

    #[test]
    fn dynamic_contract_call_remote_trait() {
        let session = Session::new(SessionSettings::default());
        let mut contracts = BTreeMap::new();
        let snippet1 = "
(define-trait my-trait
    ((hello (int) (response uint uint)))
)
(define-public (hello (a int))
    (ok u0)
)"
        .to_string();
        let bar = match build_ast(&session, &snippet1, Some("bar")) {
            Ok((contract_identifier, ast, _)) => {
                contracts.insert(contract_identifier.clone(), (DEFAULT_CLARITY_VERSION, ast));
                contract_identifier
            }
            Err(_) => panic!("expected success"),
        };

        let snippet = "
(use-trait my-trait .bar.my-trait)
(define-trait dyn-trait
    ((call-hello (<my-trait>) (response uint uint)))
)
(define-public (call-dyn (dt <dyn-trait>))
    (contract-call? dt call-hello .bar)
)
"
        .to_string();
        let test_identifier = match build_ast(&session, &snippet, Some("test")) {
            Ok((contract_identifier, ast, _)) => {
                contracts.insert(contract_identifier.clone(), (DEFAULT_CLARITY_VERSION, ast));
                contract_identifier
            }
            Err(_) => panic!("expected success"),
        };

        let dependencies =
            ASTDependencyDetector::detect_dependencies(&contracts, &BTreeMap::new()).unwrap();
        assert_eq!(dependencies[&test_identifier].len(), 1);
        println!("{:?}", dependencies[&test_identifier]);
        assert!(dependencies[&test_identifier].has_dependency(&bar).unwrap());
    }

    #[test]
    fn pass_contract_local() {
        let session = Session::new(SessionSettings::default());
        let mut contracts = BTreeMap::new();
        let snippet1 = "
(define-public (hello (a int))
    (ok u0)
)"
        .to_string();
        let bar = match build_ast(&session, &snippet1, Some("bar")) {
            Ok((contract_identifier, ast, _)) => {
                contracts.insert(contract_identifier.clone(), (DEFAULT_CLARITY_VERSION, ast));
                contract_identifier
            }
            Err(_) => panic!("expected success"),
        };

        let snippet2 = "
(define-trait my-trait
    ((hello (int) (response uint uint)))
)"
        .to_string();
        let my_trait = match build_ast(&session, &snippet2, Some("my-trait")) {
            Ok((contract_identifier, ast, _)) => {
                contracts.insert(contract_identifier.clone(), (DEFAULT_CLARITY_VERSION, ast));
                contract_identifier
            }
            Err(_) => panic!("expected success"),
        };

        let snippet = "
(use-trait my-trait .my-trait.my-trait)
(define-private (pass-trait (a <my-trait>))
    (print a)
)
(define-public (call-it)
    (ok (pass-trait .bar))
)
"
        .to_string();
        let test_identifier = match build_ast(&session, &snippet, Some("test")) {
            Ok((contract_identifier, ast, _)) => {
                contracts.insert(contract_identifier.clone(), (DEFAULT_CLARITY_VERSION, ast));
                contract_identifier
            }
            Err(_) => panic!("expected success"),
        };

        let dependencies =
            ASTDependencyDetector::detect_dependencies(&contracts, &BTreeMap::new()).unwrap();

        assert_eq!(
            dependencies[&test_identifier].has_dependency(&my_trait),
            Some(true)
        );
        assert_eq!(
            dependencies[&test_identifier].has_dependency(&bar),
            Some(false)
        );
        assert_eq!(dependencies[&test_identifier].len(), 2);
    }

    #[test]
    fn impl_trait() {
        let session = Session::new(SessionSettings::default());
        let mut contracts = BTreeMap::new();
        let snippet1 = "
(define-trait something
    ((hello (int) (response uint uint)))
)"
        .to_string();
        let other = match build_ast(&session, &snippet1, Some("other")) {
            Ok((contract_identifier, ast, _)) => {
                contracts.insert(contract_identifier.clone(), (DEFAULT_CLARITY_VERSION, ast));
                contract_identifier
            }
            Err(_) => panic!("expected success"),
        };

        let snippet = "
(impl-trait .other.something)
(define-public (hello (a int))
    (ok u0)
)
"
        .to_string();
        let test_identifier = match build_ast(&session, &snippet, Some("test")) {
            Ok((contract_identifier, ast, _)) => {
                contracts.insert(contract_identifier.clone(), (DEFAULT_CLARITY_VERSION, ast));
                contract_identifier
            }
            Err(_) => panic!("expected success"),
        };

        let dependencies =
            ASTDependencyDetector::detect_dependencies(&contracts, &BTreeMap::new()).unwrap();
        assert_eq!(dependencies[&test_identifier].len(), 1);
        assert!(dependencies[&test_identifier]
            .has_dependency(&other)
            .unwrap());
    }

    #[test]
    fn use_trait() {
        let session = Session::new(SessionSettings::default());
        let mut contracts = BTreeMap::new();
        let snippet1 = "
(define-trait something
    ((hello (int) (response uint uint)))
)"
        .to_string();
        let other = match build_ast(&session, &snippet1, Some("other")) {
            Ok((contract_identifier, ast, _)) => {
                contracts.insert(contract_identifier.clone(), (DEFAULT_CLARITY_VERSION, ast));
                contract_identifier
            }
            Err(_) => panic!("expected success"),
        };

        let snippet = "
(use-trait my-trait .other.something)
;; FIXME: If there is not a second line here, the interpreter will fail.
;; See https://github.com/hirosystems/clarity-repl/issues/109.
(define-public (foo) (ok true))
"
        .to_string();
        let test_identifier = match build_ast(&session, &snippet, Some("test")) {
            Ok((contract_identifier, ast, _)) => {
                contracts.insert(contract_identifier.clone(), (DEFAULT_CLARITY_VERSION, ast));
                contract_identifier
            }
            Err(_) => panic!("expected success"),
        };

        let dependencies =
            ASTDependencyDetector::detect_dependencies(&contracts, &BTreeMap::new()).unwrap();
        assert_eq!(dependencies[&test_identifier].len(), 1);
        assert!(dependencies[&test_identifier]
            .has_dependency(&other)
            .unwrap());
    }

    #[test]
    fn unresolved_contract_call() {
        let session = Session::new(SessionSettings::default());
        let mut contracts = BTreeMap::new();
        let snippet = "
(define-public (call-foo)
    (contract-call? .foo hello 4)
)
"
        .to_string();
        let test_identifier = match build_ast(&session, &snippet, Some("test")) {
            Ok((contract_identifier, ast, _)) => {
                contracts.insert(contract_identifier.clone(), (DEFAULT_CLARITY_VERSION, ast));
                contract_identifier
            }
            Err(_) => panic!("expected success"),
        };

        match ASTDependencyDetector::detect_dependencies(&contracts, &BTreeMap::new()) {
            Ok(_) => panic!("expected unresolved error"),
            Err((_, unresolved)) => assert_eq!(unresolved[0].name.as_str(), "foo"),
        }
    }

    #[test]
    fn dynamic_contract_call_unresolved_trait() {
        let session = Session::new(SessionSettings::default());
        let mut contracts = BTreeMap::new();
        let snippet = "
(use-trait my-trait .bar.my-trait)

(define-public (call-dyn (dt <my-trait>))
    (contract-call? dt call-hello .bar)
)
"
        .to_string();
        let test_identifier = match build_ast(&session, &snippet, Some("test")) {
            Ok((contract_identifier, ast, _)) => {
                contracts.insert(contract_identifier.clone(), (DEFAULT_CLARITY_VERSION, ast));
                contract_identifier
            }
            Err(_) => panic!("expected success"),
        };

        match ASTDependencyDetector::detect_dependencies(&contracts, &BTreeMap::new()) {
            Ok(_) => panic!("expected unresolved error"),
            Err((_, unresolved)) => assert_eq!(unresolved[0].name.as_str(), "bar"),
        }
    }

    #[test]
    fn contract_call_top_level() {
        let session = Session::new(SessionSettings::default());
        let mut contracts = BTreeMap::new();
        let snippet1 = "
(define-public (hello (a int))
    (ok u0)
)"
        .to_string();
        let foo = match build_ast(&session, &snippet1, Some("foo")) {
            Ok((contract_identifier, ast, _)) => {
                contracts.insert(contract_identifier.clone(), (DEFAULT_CLARITY_VERSION, ast));
                contract_identifier
            }
            Err(_) => panic!("expected success"),
        };

        let snippet = "(contract-call? .foo hello 4)".to_string();
        let test_identifier = match build_ast(&session, &snippet, Some("test")) {
            Ok((contract_identifier, ast, _)) => {
                contracts.insert(contract_identifier.clone(), (DEFAULT_CLARITY_VERSION, ast));
                contract_identifier
            }
            Err(_) => panic!("expected success"),
        };

        let dependencies =
            ASTDependencyDetector::detect_dependencies(&contracts, &BTreeMap::new()).unwrap();
        assert_eq!(dependencies[&test_identifier].len(), 1);
        assert!(dependencies[&test_identifier].has_dependency(&foo).unwrap());
    }

    #[test]
    fn avoid_bad_type() {
        let session = Session::new(SessionSettings::default());
        let mut contracts = BTreeMap::new();
        let snippet1 = "
(define-public (hello (a (list principal)))
    (ok u0)
)"
        .to_string();
        let foo = match build_ast(&session, &snippet1, Some("foo")) {
            Ok((contract_identifier, ast, _)) => {
                contracts.insert(contract_identifier.clone(), (DEFAULT_CLARITY_VERSION, ast));
                contract_identifier
            }
            Err(_) => panic!("expected success"),
        };

        let snippet = "(contract-call? .foo hello 4)".to_string();
        let test_identifier = match build_ast(&session, &snippet, Some("test")) {
            Ok((contract_identifier, ast, _)) => {
                contracts.insert(contract_identifier.clone(), (DEFAULT_CLARITY_VERSION, ast));
                contract_identifier
            }
            Err(_) => panic!("expected success"),
        };

        let dependencies =
            ASTDependencyDetector::detect_dependencies(&contracts, &BTreeMap::new()).unwrap();
        assert_eq!(dependencies[&test_identifier].len(), 1);
        assert!(dependencies[&test_identifier].has_dependency(&foo).unwrap());
    }
}

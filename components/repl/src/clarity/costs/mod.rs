pub mod constants;
pub mod cost_functions;

use regex::internal::Exec;
use std::convert::{TryFrom, TryInto};
use std::{cmp, fmt};

use std::collections::{BTreeMap, HashMap};

use crate::clarity::ast::ContractAST;
use crate::clarity::contexts::{ContractContext, Environment, GlobalContext, OwnedEnvironment};
use crate::clarity::costs::cost_functions::ClarityCostFunction;
use crate::clarity::database::{ClarityDatabase, NullBackingStore};
use crate::clarity::errors::{Error, InterpreterResult};
use crate::clarity::types::signatures::TupleTypeSignature;
use crate::clarity::types::{FunctionType, FunctionType::Fixed};
use crate::clarity::types::{PrincipalData, TupleData, Value::UInt};
use crate::clarity::types::{QualifiedContractIdentifier, TypeSignature, NONE};
use crate::clarity::{ast, eval_all, ClarityName, SymbolicExpression, Value};

type Result<T> = std::result::Result<T, CostErrors>;

pub const CLARITY_MEMORY_LIMIT: u64 = 100 * 1000 * 1000;

lazy_static! {
    static ref COST_TUPLE_TYPE_SIGNATURE: TypeSignature = TypeSignature::TupleType(
        TupleTypeSignature::try_from(vec![
            ("runtime".into(), TypeSignature::UIntType),
            ("write_length".into(), TypeSignature::UIntType),
            ("write_count".into(), TypeSignature::UIntType),
            ("read_count".into(), TypeSignature::UIntType),
            ("read_length".into(), TypeSignature::UIntType),
        ])
        .expect("BUG: failed to construct type signature for cost tuple")
    );
}

pub fn boot_code_id(contract_name: &str, is_mainnet: bool) -> QualifiedContractIdentifier {
    let principal = if is_mainnet {
        format!("SP000000000000000000002Q6VF78.{}", contract_name)
    } else {
        format!("ST000000000000000000002AMW42H.{}", contract_name)
    };
    QualifiedContractIdentifier::parse(&principal).unwrap()
}

pub fn runtime_cost<T: TryInto<u64>, C: CostTracker>(
    cost_function: ClarityCostFunction,
    tracker: &mut C,
    input: T,
) -> Result<()> {
    let size: u64 = input.try_into().map_err(|_| CostErrors::CostOverflow)?;
    let cost = tracker.compute_cost(cost_function, &[size])?;

    tracker.add_cost(cost)
}

macro_rules! finally_drop_memory {
    ( $env: expr, $used_mem:expr; $exec:expr ) => {{
        let result = (|| $exec)();
        $env.drop_memory($used_mem);
        result
    }};
}

pub fn analysis_typecheck_cost<T: CostTracker>(
    track: &mut T,
    t1: &TypeSignature,
    t2: &TypeSignature,
) -> Result<()> {
    let t1_size = t1.type_size().map_err(|_| CostErrors::CostOverflow)?;
    let t2_size = t2.type_size().map_err(|_| CostErrors::CostOverflow)?;
    let cost = track.compute_cost(
        ClarityCostFunction::AnalysisTypeCheck,
        &[cmp::max(t1_size, t2_size) as u64],
    )?;
    track.add_cost(cost)
}

pub trait MemoryConsumer {
    fn get_memory_use(&self) -> u64;
}

impl MemoryConsumer for Value {
    fn get_memory_use(&self) -> u64 {
        self.size().into()
    }
}

pub trait CostTracker {
    fn compute_cost(
        &mut self,
        cost_function: ClarityCostFunction,
        input: &[u64],
    ) -> Result<ExecutionCost>;
    fn add_cost(&mut self, cost: ExecutionCost) -> Result<()>;
    fn add_memory(&mut self, memory: u64) -> Result<()>;
    fn drop_memory(&mut self, memory: u64);
    fn reset_memory(&mut self);
    /// Check if the given contract-call should be short-circuited.
    ///  If so: this charges the cost to the CostTracker, and return true
    ///  If not: return false
    fn short_circuit_contract_call(
        &mut self,
        contract: &QualifiedContractIdentifier,
        function: &ClarityName,
        input: &[u64],
    ) -> Result<bool>;
}

// Don't track!
impl CostTracker for () {
    fn compute_cost(
        &mut self,
        _cost_function: ClarityCostFunction,
        _input: &[u64],
    ) -> std::result::Result<ExecutionCost, CostErrors> {
        Ok(ExecutionCost::zero())
    }
    fn add_cost(&mut self, _cost: ExecutionCost) -> std::result::Result<(), CostErrors> {
        Ok(())
    }
    fn add_memory(&mut self, _memory: u64) -> std::result::Result<(), CostErrors> {
        Ok(())
    }
    fn drop_memory(&mut self, _memory: u64) {}
    fn reset_memory(&mut self) {}
    fn short_circuit_contract_call(
        &mut self,
        _contract: &QualifiedContractIdentifier,
        _function: &ClarityName,
        _input: &[u64],
    ) -> Result<bool> {
        Ok(false)
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct ClarityCostFunctionReference {
    pub contract_id: QualifiedContractIdentifier,
    pub function_name: String,
}

impl ::std::fmt::Display for ClarityCostFunctionReference {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        write!(f, "{}.{}", &self.contract_id, &self.function_name)
    }
}

impl ClarityCostFunctionReference {
    fn new(id: QualifiedContractIdentifier, name: String) -> ClarityCostFunctionReference {
        ClarityCostFunctionReference {
            contract_id: id,
            function_name: name,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CostStateSummary {
    pub contract_call_circuits:
        HashMap<(QualifiedContractIdentifier, ClarityName), ClarityCostFunctionReference>,
    pub cost_function_references: HashMap<ClarityCostFunction, ClarityCostFunctionReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SerializedCostStateSummary {
    contract_call_circuits: Vec<(
        (QualifiedContractIdentifier, ClarityName),
        ClarityCostFunctionReference,
    )>,
    cost_function_references: Vec<(ClarityCostFunction, ClarityCostFunctionReference)>,
}

impl From<CostStateSummary> for SerializedCostStateSummary {
    fn from(other: CostStateSummary) -> SerializedCostStateSummary {
        let CostStateSummary {
            contract_call_circuits,
            cost_function_references,
        } = other;
        SerializedCostStateSummary {
            contract_call_circuits: contract_call_circuits.into_iter().collect(),
            cost_function_references: cost_function_references.into_iter().collect(),
        }
    }
}

impl From<SerializedCostStateSummary> for CostStateSummary {
    fn from(other: SerializedCostStateSummary) -> CostStateSummary {
        let SerializedCostStateSummary {
            contract_call_circuits,
            cost_function_references,
        } = other;
        CostStateSummary {
            contract_call_circuits: contract_call_circuits.into_iter().collect(),
            cost_function_references: cost_function_references.into_iter().collect(),
        }
    }
}

impl CostStateSummary {
    pub fn empty() -> CostStateSummary {
        CostStateSummary {
            contract_call_circuits: HashMap::new(),
            cost_function_references: HashMap::new(),
        }
    }
}

#[derive(Clone)]
pub struct LimitedCostTracker {
    cost_function_references: HashMap<&'static ClarityCostFunction, ClarityCostFunctionReference>,
    cost_contracts: HashMap<QualifiedContractIdentifier, ContractContext>,
    contract_call_circuits:
        HashMap<(QualifiedContractIdentifier, ClarityName), ClarityCostFunctionReference>,
    total: ExecutionCost,
    limit: ExecutionCost,
    pub memory: u64,
    pub memory_limit: u64,
    pub costs_version: u32,
    free: bool,
    mainnet: bool,
}

#[cfg(test)]
impl LimitedCostTracker {
    pub fn contract_call_circuits(
        &self,
    ) -> HashMap<(QualifiedContractIdentifier, ClarityName), ClarityCostFunctionReference> {
        self.contract_call_circuits.clone()
    }
    pub fn cost_function_references(
        &self,
    ) -> HashMap<&'static ClarityCostFunction, ClarityCostFunctionReference> {
        self.cost_function_references.clone()
    }
}

impl fmt::Debug for LimitedCostTracker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LimitedCostTracker")
            .field("total", &self.total)
            .field("limit", &self.limit)
            .field("memory", &self.memory)
            .field("memory_limit", &self.memory_limit)
            .field("free", &self.free)
            .finish()
    }
}
impl PartialEq for LimitedCostTracker {
    fn eq(&self, other: &Self) -> bool {
        self.total == other.total
            && self.limit == other.limit
            && self.memory == other.memory
            && self.memory_limit == other.memory_limit
            && self.free == other.free
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum CostErrors {
    CostComputationFailed(String),
    CostOverflow,
    CostBalanceExceeded(ExecutionCost, ExecutionCost),
    MemoryBalanceExceeded(u64, u64),
    CostContractLoadFailure,
}

fn load_state_summary(mainnet: bool, clarity_db: &mut ClarityDatabase) -> Result<CostStateSummary> {
    // let cost_voting_contract = boot_code_id("cost-voting", mainnet);

    // let last_processed_at = match clarity_db.get_value(
    //     "vm-costs::last-processed-at-height",
    //     &TypeSignature::UIntType,
    // ) {
    //     Some(v) => u32::try_from(v.expect_u128()).expect("Block height overflowed u32"),
    //     None => return Ok(CostStateSummary::empty()),
    // };

    // let metadata_result = clarity_db
    //     .fetch_metadata_manual::<String>(
    //         last_processed_at,
    //         &cost_voting_contract,
    //         "::state_summary",
    //     )
    //     .map_err(|e| CostErrors::CostComputationFailed(e.to_string()))?;
    // let serialized: SerializedCostStateSummary = match metadata_result {
    //     Some(serialized) => serde_json::from_str(&serialized).unwrap(),
    //     None => return Ok(CostStateSummary::empty()),
    // };
    Ok(CostStateSummary::empty())
}

fn store_state_summary(
    mainnet: bool,
    clarity_db: &mut ClarityDatabase,
    to_store: &CostStateSummary,
) -> Result<()> {
    let block_height = clarity_db.get_current_block_height();
    let cost_voting_contract = boot_code_id("cost-voting", mainnet);

    clarity_db.put(
        "vm-costs::last-processed-at-height",
        &Value::UInt(block_height as u128),
    );
    let serialized_summary =
        serde_json::to_string(&SerializedCostStateSummary::from(to_store.clone()))
            .expect("BUG: failure to serialize cost state summary struct");
    clarity_db.set_metadata(
        &cost_voting_contract,
        "::state_summary",
        &serialized_summary,
    );

    Ok(())
}

///
/// This method loads a cost state summary structure from the currently open stacks chain tip
///   In doing so, it reads from the cost-voting contract to find any newly confirmed proposals,
///    checks those proposals for validity, and then applies those changes to the cached set
///    of cost functions.
///
/// `apply_updates` - tells this function to look for any changes in the cost voting contract
///   which would need to be applied. if `false`, just load the last computed cost state in this
///   fork.
///
fn load_cost_functions(
    mainnet: bool,
    clarity_db: &mut ClarityDatabase,
    apply_updates: bool,
    costs_version: u32,
) -> Result<CostStateSummary> {
    if mainnet == false {
        return Ok(CostStateSummary::empty());
    }

    let last_processed_count = clarity_db
        .get_value("vm-costs::last_processed_count", &TypeSignature::UIntType)
        .unwrap_or(Value::UInt(0))
        .expect_u128();
    let cost_voting_contract = boot_code_id("cost-voting", mainnet);
    let confirmed_proposals_count = clarity_db
        .lookup_variable_unknown_descriptor(&cost_voting_contract, "confirmed-proposal-count")
        .map_err(|e| CostErrors::CostComputationFailed(e.to_string()))?
        .expect_u128();
    println!("Check cost voting contract");

    // we need to process any confirmed proposals in the range [fetch-start, fetch-end)
    let (fetch_start, fetch_end) = (last_processed_count, confirmed_proposals_count);
    let mut state_summary = load_state_summary(mainnet, clarity_db)?;
    if !apply_updates {
        return Ok(state_summary);
    }

    for confirmed_proposal in fetch_start..fetch_end {
        // fetch the proposal data
        let entry = clarity_db
            .fetch_entry_unknown_descriptor(
                &cost_voting_contract,
                "confirmed-proposals",
                &Value::from(
                    TupleData::from_data(vec![(
                        "confirmed-id".into(),
                        Value::UInt(confirmed_proposal),
                    )])
                    .expect("BUG: failed to construct simple tuple"),
                ),
            )
            .expect("BUG: Failed querying confirmed-proposals")
            .expect_optional()
            .expect("BUG: confirmed-proposal-count exceeds stored proposals")
            .expect_tuple();
        let target_contract = match entry
            .get("function-contract")
            .expect("BUG: malformed cost proposal tuple")
            .clone()
            .expect_principal()
        {
            PrincipalData::Contract(contract_id) => contract_id,
            _ => {
                println!("Confirmed cost proposal invalid: function-contract is not a contract principal");
                continue;
            }
        };
        let target_function = match ClarityName::try_from(
            entry
                .get("function-name")
                .expect("BUG: malformed cost proposal tuple")
                .clone()
                .expect_ascii(),
        ) {
            Ok(x) => x,
            Err(_) => {
                println!(
                    "Confirmed cost proposal invalid: function-name is not a valid function name"
                );
                continue;
            }
        };
        let cost_contract = match entry
            .get("cost-function-contract")
            .expect("BUG: malformed cost proposal tuple")
            .clone()
            .expect_principal()
        {
            PrincipalData::Contract(contract_id) => contract_id,
            _ => {
                println!("Confirmed cost proposal invalid: cost-function-contract is not a contract principal");
                continue;
            }
        };

        let cost_function = match ClarityName::try_from(
            entry
                .get_owned("cost-function-name")
                .expect("BUG: malformed cost proposal tuple")
                .expect_ascii(),
        ) {
            Ok(x) => x,
            Err(_) => {
                println!("Confirmed cost proposal invalid: cost-function-name is not a valid function name");
                continue;
            }
        };

        // Here is where we perform the required validity checks for a confirmed proposal:
        //  * Replaced contract-calls _must_ be `define-read-only` _or_ refer to one of the boot code
        //      cost functions
        //  * cost-function contracts must be arithmetic only

        // make sure the contract is "cost contract eligible" via the
        //  arithmetic-checking analysis pass
        let (cost_func_ref, cost_func_type) = match clarity_db
            .load_contract_analysis(&cost_contract)
        {
            Some(c) => {
                if !c.is_cost_contract_eligible {
                    println!("Confirmed cost proposal invalid: cost-function-contract uses non-arithmetic or otherwise illegal operations");
                    continue;
                }

                if let Some(FunctionType::Fixed(cost_function_type)) = c
                    .read_only_function_types
                    .get(&cost_function)
                    .or_else(|| c.private_function_types.get(&cost_function))
                {
                    if !cost_function_type.returns.eq(&COST_TUPLE_TYPE_SIGNATURE) {
                        println!("Confirmed cost proposal invalid: cost-function-name does not return a cost tuple");
                        continue;
                    }
                    if !cost_function_type.args.len() == 1
                        || cost_function_type.args[0].signature != TypeSignature::UIntType
                    {
                        println!("Confirmed cost proposal invalid: cost-function-name args should be length-1 and only uint");
                        continue;
                    }
                    (
                        ClarityCostFunctionReference {
                            contract_id: cost_contract,
                            function_name: cost_function.to_string(),
                        },
                        cost_function_type.clone(),
                    )
                } else {
                    println!("Confirmed cost proposal invalid: cost-function-name not defined");
                    continue;
                }
            }
            None => {
                println!("Confirmed cost proposal invalid: cost-function-contract is not a published contract");
                continue;
            }
        };
        let costs_contract = format!("costs-v{}", costs_version);
        if target_contract == boot_code_id(&costs_contract, mainnet) {
            // refering to one of the boot code cost functions
            let target = match ClarityCostFunction::lookup_by_name(&target_function) {
                Some(cost_func) => cost_func,
                None => {
                    println!("Confirmed cost proposal invalid: function-name does not reference a Clarity cost function");
                    continue;
                }
            };
            state_summary
                .cost_function_references
                .insert(target, cost_func_ref);
        } else {
            // referring to a user-defined function
            match clarity_db.load_contract_analysis(&target_contract) {
                Some(c) => {
                    if let Some(Fixed(tf)) = c.read_only_function_types.get(&target_function) {
                        if cost_func_type.args.len() != tf.args.len() {
                            println!("Confirmed cost proposal invalid: cost-function contains the wrong number of arguments");
                            continue;
                        }
                        for arg in &cost_func_type.args {
                            if &arg.signature != &TypeSignature::UIntType {
                                println!(
                                    "Confirmed cost proposal invalid: contains non uint argument"
                                );
                                continue;
                            }
                        }
                    } else {
                        println!("Confirmed cost proposal invalid: function-name not defined or is not read-only");
                        continue;
                    }
                }
                None => {
                    println!(
                        "Confirmed cost proposal invalid: contract-name not a published contract"
                    );
                    continue;
                }
            }
            state_summary
                .contract_call_circuits
                .insert((target_contract, target_function), cost_func_ref);
        }
    }
    if confirmed_proposals_count > last_processed_count {
        store_state_summary(mainnet, clarity_db, &state_summary)?;
        clarity_db.put(
            "vm-costs::last_processed_count",
            &Value::UInt(confirmed_proposals_count),
        );
    }

    Ok(state_summary)
}

impl LimitedCostTracker {
    pub fn new(
        mainnet: bool,
        limit: ExecutionCost,
        clarity_db: &mut ClarityDatabase,
        costs_version: u32,
    ) -> Result<LimitedCostTracker> {
        let mut cost_tracker = LimitedCostTracker {
            cost_function_references: HashMap::new(),
            cost_contracts: HashMap::new(),
            contract_call_circuits: HashMap::new(),
            limit,
            memory_limit: CLARITY_MEMORY_LIMIT,
            total: ExecutionCost::zero(),
            memory: 0,
            free: false,
            mainnet,
            costs_version,
        };
        cost_tracker.load_costs(clarity_db, true)?;
        Ok(cost_tracker)
    }

    pub fn new_mid_block(
        mainnet: bool,
        limit: ExecutionCost,
        clarity_db: &mut ClarityDatabase,
    ) -> Result<LimitedCostTracker> {
        let mut cost_tracker = LimitedCostTracker {
            cost_function_references: HashMap::new(),
            cost_contracts: HashMap::new(),
            contract_call_circuits: HashMap::new(),
            limit,
            memory_limit: CLARITY_MEMORY_LIMIT,
            total: ExecutionCost::zero(),
            memory: 0,
            free: false,
            mainnet,
            costs_version: 1,
        };
        cost_tracker.load_costs(clarity_db, false)?;
        Ok(cost_tracker)
    }

    pub fn new_free() -> LimitedCostTracker {
        LimitedCostTracker {
            cost_function_references: HashMap::new(),
            cost_contracts: HashMap::new(),
            contract_call_circuits: HashMap::new(),
            limit: ExecutionCost::max_value(),
            total: ExecutionCost::zero(),
            memory: 0,
            costs_version: 1,
            memory_limit: CLARITY_MEMORY_LIMIT,
            free: true,
            mainnet: false,
        }
    }

    /// `apply_updates` - tells this function to look for any changes in the cost voting contract
    ///   which would need to be applied. if `false`, just load the last computed cost state in this
    ///   fork.
    fn load_costs(&mut self, clarity_db: &mut ClarityDatabase, apply_updates: bool) -> Result<()> {
        let costs_contract = format!("costs-v{}", self.costs_version);
        let boot_costs_id = boot_code_id(&costs_contract, self.mainnet);

        clarity_db.begin();
        let CostStateSummary {
            contract_call_circuits,
            mut cost_function_references,
        } = load_cost_functions(self.mainnet, clarity_db, apply_updates, self.costs_version)
            .map_err(|e| {
                clarity_db.roll_back();
                e
            })?;

        self.contract_call_circuits = contract_call_circuits;

        let mut cost_contracts = HashMap::new();
        let mut m = HashMap::new();
        for f in ClarityCostFunction::ALL.iter() {
            let cost_function_ref = cost_function_references.remove(&f).unwrap_or_else(|| {
                ClarityCostFunctionReference::new(boot_costs_id.clone(), f.get_name())
            });
            if !cost_contracts.contains_key(&cost_function_ref.contract_id) {
                let contract_context = match clarity_db.get_contract(&cost_function_ref.contract_id)
                {
                    Ok(contract) => contract.contract_context,
                    Err(e) => {
                        println!("Failed to load intended Clarity cost contract");
                        clarity_db.roll_back();
                        return Err(CostErrors::CostContractLoadFailure);
                    }
                };
                cost_contracts.insert(cost_function_ref.contract_id.clone(), contract_context);
            }

            m.insert(f, cost_function_ref);
        }

        for (_, circuit_target) in self.contract_call_circuits.iter() {
            if !cost_contracts.contains_key(&circuit_target.contract_id) {
                let contract_context = match clarity_db.get_contract(&circuit_target.contract_id) {
                    Ok(contract) => contract.contract_context,
                    Err(e) => {
                        println!("Failed to load intended Clarity cost contract");
                        clarity_db.roll_back();
                        return Err(CostErrors::CostContractLoadFailure);
                    }
                };
                cost_contracts.insert(circuit_target.contract_id.clone(), contract_context);
            }
        }

        self.cost_function_references = m;
        self.cost_contracts = cost_contracts;

        if apply_updates {
            clarity_db.commit();
        } else {
            clarity_db.roll_back();
        }

        return Ok(());
    }
    pub fn get_total(&self) -> ExecutionCost {
        self.total.clone()
    }
    pub fn set_total(&mut self, total: ExecutionCost) -> () {
        // used by the miner to "undo" the cost of a transaction when trying to pack a block.
        self.total = total;
    }
    pub fn get_limit(&self) -> ExecutionCost {
        self.limit.clone()
    }
}

fn parse_cost(
    cost_function_name: &str,
    eval_result: InterpreterResult<Option<Value>>,
) -> Result<ExecutionCost> {
    match eval_result {
        Ok(Some(Value::Tuple(data))) => {
            let results = (
                data.data_map.get("write_length"),
                data.data_map.get("write_count"),
                data.data_map.get("runtime"),
                data.data_map.get("read_length"),
                data.data_map.get("read_count"),
            );

            match results {
                (
                    Some(UInt(write_length)),
                    Some(UInt(write_count)),
                    Some(UInt(runtime)),
                    Some(UInt(read_length)),
                    Some(UInt(read_count)),
                ) => Ok(ExecutionCost {
                    write_length: (*write_length as u64),
                    write_count: (*write_count as u64),
                    runtime: (*runtime as u64),
                    read_length: (*read_length as u64),
                    read_count: (*read_count as u64),
                }),
                _ => Err(CostErrors::CostComputationFailed(
                    "Execution Cost tuple does not contain only UInts".to_string(),
                )),
            }
        }
        Ok(Some(_)) => Err(CostErrors::CostComputationFailed(
            "Clarity cost function returned something other than a Cost tuple".to_string(),
        )),
        Ok(None) => Err(CostErrors::CostComputationFailed(
            "Clarity cost function returned nothing".to_string(),
        )),
        Err(e) => Err(CostErrors::CostComputationFailed(format!(
            "Error evaluating result of cost function {}: {}",
            cost_function_name, e
        ))),
    }
}

fn compute_cost(
    cost_tracker: &mut LimitedCostTracker,
    cost_function_reference: ClarityCostFunctionReference,
    input_sizes: &[u64],
) -> Result<ExecutionCost> {
    let mainnet = cost_tracker.mainnet;
    let mut null_store = NullBackingStore::new();
    let conn = null_store.as_clarity_db();
    let mut global_context = GlobalContext::new(mainnet, conn, LimitedCostTracker::new_free());

    let cost_contract = cost_tracker
        .cost_contracts
        .get_mut(&cost_function_reference.contract_id)
        .ok_or(CostErrors::CostComputationFailed(format!(
            "CostFunction not found: {}",
            &cost_function_reference
        )))?;

    let mut program = vec![SymbolicExpression::atom(
        cost_function_reference.function_name[..].into(),
    )];

    for input_size in input_sizes.iter() {
        program.push(SymbolicExpression::atom_value(Value::UInt(
            *input_size as u128,
        )));
    }

    let function_invocation = [SymbolicExpression::list(program.into_boxed_slice())];

    let eval_result = eval_all(&function_invocation, cost_contract, &mut global_context);

    parse_cost(&cost_function_reference.to_string(), eval_result)
}

fn add_cost(
    s: &mut LimitedCostTracker,
    cost: ExecutionCost,
) -> std::result::Result<(), CostErrors> {
    s.total.add(&cost)?;
    if s.total.exceeds(&s.limit) {
        Err(CostErrors::CostBalanceExceeded(
            s.total.clone(),
            s.limit.clone(),
        ))
    } else {
        Ok(())
    }
}

fn add_memory(s: &mut LimitedCostTracker, memory: u64) -> std::result::Result<(), CostErrors> {
    s.memory = s.memory.cost_overflow_add(memory)?;
    if s.memory > s.memory_limit {
        Err(CostErrors::MemoryBalanceExceeded(s.memory, s.memory_limit))
    } else {
        Ok(())
    }
}

fn drop_memory(s: &mut LimitedCostTracker, memory: u64) {
    s.memory = s
        .memory
        .checked_sub(memory)
        .expect("Underflowed dropped memory");
}

impl CostTracker for LimitedCostTracker {
    fn compute_cost(
        &mut self,
        cost_function: ClarityCostFunction,
        input: &[u64],
    ) -> std::result::Result<ExecutionCost, CostErrors> {
        if self.free {
            return Ok(ExecutionCost::zero());
        }
        let cost_function_ref = self
            .cost_function_references
            .get(&cost_function)
            .ok_or(CostErrors::CostComputationFailed(format!(
                "CostFunction not defined"
            )))?
            .clone();

        compute_cost(self, cost_function_ref, input)
    }
    fn add_cost(&mut self, cost: ExecutionCost) -> std::result::Result<(), CostErrors> {
        if self.free {
            return Ok(());
        }
        add_cost(self, cost)
    }
    fn add_memory(&mut self, memory: u64) -> std::result::Result<(), CostErrors> {
        if self.free {
            return Ok(());
        }
        add_memory(self, memory)
    }
    fn drop_memory(&mut self, memory: u64) {
        if !self.free {
            drop_memory(self, memory)
        }
    }
    fn reset_memory(&mut self) {
        if !self.free {
            self.memory = 0;
        }
    }
    fn short_circuit_contract_call(
        &mut self,
        contract: &QualifiedContractIdentifier,
        function: &ClarityName,
        input: &[u64],
    ) -> Result<bool> {
        if self.free {
            // if we're already free, no need to worry about short circuiting contract-calls
            return Ok(false);
        }
        // grr, if HashMap::get didn't require Borrow, we wouldn't need this cloning.
        let lookup_key = (contract.clone(), function.clone());
        if let Some(cost_function) = self.contract_call_circuits.get(&lookup_key).cloned() {
            compute_cost(self, cost_function, input)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

impl CostTracker for &mut LimitedCostTracker {
    fn compute_cost(
        &mut self,
        cost_function: ClarityCostFunction,
        input: &[u64],
    ) -> std::result::Result<ExecutionCost, CostErrors> {
        LimitedCostTracker::compute_cost(self, cost_function, input)
    }
    fn add_cost(&mut self, cost: ExecutionCost) -> std::result::Result<(), CostErrors> {
        LimitedCostTracker::add_cost(self, cost)
    }
    fn add_memory(&mut self, memory: u64) -> std::result::Result<(), CostErrors> {
        LimitedCostTracker::add_memory(self, memory)
    }
    fn drop_memory(&mut self, memory: u64) {
        LimitedCostTracker::drop_memory(self, memory)
    }
    fn reset_memory(&mut self) {
        LimitedCostTracker::reset_memory(self)
    }
    fn short_circuit_contract_call(
        &mut self,
        contract: &QualifiedContractIdentifier,
        function: &ClarityName,
        input: &[u64],
    ) -> Result<bool> {
        LimitedCostTracker::short_circuit_contract_call(self, contract, function, input)
    }
}

#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct ExecutionCost {
    pub write_length: u64,
    pub write_count: u64,
    pub read_length: u64,
    pub read_count: u64,
    pub runtime: u64,
}

impl fmt::Display for ExecutionCost {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{{\"runtime\": {}, \"write_len\": {}, \"write_cnt\": {}, \"read_len\": {}, \"read_cnt\": {}}}",
               self.runtime, self.write_length, self.write_count, self.read_length, self.read_count)
    }
}

pub trait CostOverflowingMath<T> {
    fn cost_overflow_mul(self, other: T) -> Result<T>;
    fn cost_overflow_add(self, other: T) -> Result<T>;
    fn cost_overflow_sub(self, other: T) -> Result<T>;
}

impl CostOverflowingMath<u64> for u64 {
    fn cost_overflow_mul(self, other: u64) -> Result<u64> {
        self.checked_mul(other)
            .ok_or_else(|| CostErrors::CostOverflow)
    }
    fn cost_overflow_add(self, other: u64) -> Result<u64> {
        self.checked_add(other)
            .ok_or_else(|| CostErrors::CostOverflow)
    }
    fn cost_overflow_sub(self, other: u64) -> Result<u64> {
        self.checked_sub(other)
            .ok_or_else(|| CostErrors::CostOverflow)
    }
}

impl ExecutionCost {
    pub fn zero() -> ExecutionCost {
        Self {
            runtime: 0,
            write_length: 0,
            read_count: 0,
            write_count: 0,
            read_length: 0,
        }
    }

    /// Returns the percentage of self consumed in `numerator`'s largest proportion dimension.
    pub fn proportion_largest_dimension(&self, numerator: &ExecutionCost) -> u64 {
        [
            numerator.runtime / cmp::max(1, self.runtime / 100),
            numerator.write_length / cmp::max(1, self.write_length / 100),
            numerator.write_count / cmp::max(1, self.write_count / 100),
            numerator.read_length / cmp::max(1, self.read_length / 100),
            numerator.read_count / cmp::max(1, self.read_count / 100),
        ]
        .iter()
        .max()
        .expect("BUG: should find maximum")
        .clone()
    }

    pub fn max_value() -> ExecutionCost {
        Self {
            runtime: u64::max_value(),
            write_length: u64::max_value(),
            read_count: u64::max_value(),
            write_count: u64::max_value(),
            read_length: u64::max_value(),
        }
    }

    pub fn runtime(runtime: u64) -> ExecutionCost {
        Self {
            runtime,
            write_length: 0,
            read_count: 0,
            write_count: 0,
            read_length: 0,
        }
    }

    pub fn add_runtime(&mut self, runtime: u64) -> Result<()> {
        self.runtime = self.runtime.cost_overflow_add(runtime)?;
        Ok(())
    }

    pub fn add(&mut self, other: &ExecutionCost) -> Result<()> {
        self.runtime = self.runtime.cost_overflow_add(other.runtime)?;
        self.read_count = self.read_count.cost_overflow_add(other.read_count)?;
        self.read_length = self.read_length.cost_overflow_add(other.read_length)?;
        self.write_length = self.write_length.cost_overflow_add(other.write_length)?;
        self.write_count = self.write_count.cost_overflow_add(other.write_count)?;
        Ok(())
    }

    pub fn sub(&mut self, other: &ExecutionCost) -> Result<()> {
        self.runtime = self.runtime.cost_overflow_sub(other.runtime)?;
        self.read_count = self.read_count.cost_overflow_sub(other.read_count)?;
        self.read_length = self.read_length.cost_overflow_sub(other.read_length)?;
        self.write_length = self.write_length.cost_overflow_sub(other.write_length)?;
        self.write_count = self.write_count.cost_overflow_sub(other.write_count)?;
        Ok(())
    }

    pub fn multiply(&mut self, times: u64) -> Result<()> {
        self.runtime = self.runtime.cost_overflow_mul(times)?;
        self.read_count = self.read_count.cost_overflow_mul(times)?;
        self.read_length = self.read_length.cost_overflow_mul(times)?;
        self.write_length = self.write_length.cost_overflow_mul(times)?;
        self.write_count = self.write_count.cost_overflow_mul(times)?;
        Ok(())
    }

    /// Returns whether or not this cost exceeds any dimension of the
    ///  other cost.
    pub fn exceeds(&self, other: &ExecutionCost) -> bool {
        self.runtime > other.runtime
            || self.write_length > other.write_length
            || self.write_count > other.write_count
            || self.read_count > other.read_count
            || self.read_length > other.read_length
    }

    pub fn max_cost(first: ExecutionCost, second: ExecutionCost) -> ExecutionCost {
        Self {
            runtime: first.runtime.max(second.runtime),
            write_length: first.write_length.max(second.write_length),
            write_count: first.write_count.max(second.write_count),
            read_count: first.read_count.max(second.read_count),
            read_length: first.read_length.max(second.read_length),
        }
    }
}

// ONLY WORKS IF INPUT IS u64
fn int_log2(input: u64) -> Option<u64> {
    63_u32.checked_sub(input.leading_zeros()).map(|floor_log| {
        if input.trailing_zeros() == floor_log {
            u64::from(floor_log)
        } else {
            u64::from(floor_log + 1)
        }
    })
}

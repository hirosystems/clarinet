use crate::indexer::{BitcoinChainEvent, StacksChainEvent};
use crate::types::{AccountIdentifier, StacksTransactionReceipt};
use clarity_repl::clarity::types::{QualifiedContractIdentifier, StandardPrincipalData};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::convert::TryInto;
use std::sync::mpsc::{channel, Receiver, Sender};

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct TriggerId {
    pub pid: ClarionPid,
    pub lambda_id: u64,
}

#[derive(Clone, Debug)]
pub struct ClarionManifest {
    project: ProjectMetadata,
    lambdas: Vec<Lambda>,
    contracts: BTreeMap<QualifiedContractIdentifier, ContractSettings>,
}

#[derive(Clone, Debug)]
pub struct ProjectMetadata {
    name: String,
    authors: Vec<String>,
    homepage: String,
    license: String,
    description: String,
}

#[derive(Clone, Debug)]
pub struct ContractSettings {
    state_explorer_enabled: bool,
    api_generator_enabled: Vec<String>,
}

pub enum ClarionInstanceCommand {
    Start,
    Stop,
    AddLambda,
}

#[derive(Debug)]
pub struct ClarionInstanceController {
    pid: ClarionPid,
    tx: Sender<ClarionInstanceCommand>,
}

impl ClarionInstanceController {
    pub fn trigger_lambda(&self, lambda_id: u64) {
        println!("Triggering lambda {}", lambda_id);
    }
}

pub struct ClarionInstance {
    pub pid: ClarionPid,
    project_id: u64,
    metadata: ProjectMetadata,
    user_lambdas: Vec<Lambda>,
    platform_lambdas: Vec<Lambda>,
    contracts_ids: Vec<QualifiedContractIdentifier>,
    rx: Receiver<ClarionInstanceCommand>,
    tx: Sender<ClarionInstanceCommand>,
}

impl ClarionInstance {
    pub fn new(manifest: ClarionManifest, pid: ClarionPid) -> ClarionInstance {
        let mut platform_lambdas = vec![];
        let (tx, rx) = channel();
        ClarionInstance {
            pid,
            project_id: 0,
            contracts_ids: vec![],
            metadata: manifest.project.clone(),
            user_lambdas: manifest.lambdas.clone(),
            platform_lambdas,
            rx,
            tx,
        }
    }

    pub fn execute_lambda(&self, lambda_id: u64) {
        println!("Executing lambda {}", lambda_id);
    }
}

#[derive(Clone, Debug)]
pub enum ClarionHypervisorCommand {
    RegisterClarionInstance(ClarionManifest),
    ProcessStacksChainEvent(StacksChainEvent),
    ProcessBitcoinChainEvent(BitcoinChainEvent),
    Exit,
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct ClarionPid(u64);

pub struct ClarionHypervisor {
    instances_pool: HashMap<ClarionPid, ClarionInstance>,
    clarion_controllers: HashMap<ClarionPid, ClarionInstanceController>,
    bitcoin_predicates: HashMap<BitcoinPredicate, Vec<TriggerId>>,
    stacks_predicates: StacksChainPredicates,
    rx: Receiver<ClarionHypervisorCommand>,
    tx: Sender<ClarionHypervisorCommand>,
    trigger_history: VecDeque<(String, HashSet<TriggerId>)>,
}

pub struct StacksChainPredicates {
    pub watching_contract_id_activity: HashMap<String, HashSet<TriggerId>>,
    pub watching_contract_data_mutation_activity: HashMap<String, HashSet<TriggerId>>,
    pub watching_principal_activity: HashMap<String, HashSet<TriggerId>>,
    pub watching_ft_move_activity: HashMap<String, HashSet<TriggerId>>,
    pub watching_nft_activity: HashMap<String, HashSet<TriggerId>>,
    pub watching_any_block_activity: HashSet<TriggerId>,
}

impl StacksChainPredicates {
    pub fn new() -> Self {
        Self {
            watching_contract_id_activity: HashMap::new(),
            watching_contract_data_mutation_activity: HashMap::new(),
            watching_principal_activity: HashMap::new(),
            watching_ft_move_activity: HashMap::new(),
            watching_nft_activity: HashMap::new(),
            watching_any_block_activity: HashSet::new(),
        }
    }
}

impl ClarionHypervisor {
    pub fn new(
        tx: Sender<ClarionHypervisorCommand>,
        rx: Receiver<ClarionHypervisorCommand>,
    ) -> Self {
        Self {
            instances_pool: HashMap::new(),
            clarion_controllers: HashMap::new(),
            bitcoin_predicates: HashMap::new(),
            stacks_predicates: StacksChainPredicates::new(),
            trigger_history: VecDeque::new(),
            tx,
            rx,
        }
    }

    pub fn run(&mut self) -> Result<(), ()> {
        let mut last_pid = 1;
        loop {
            match self.rx.recv() {
                Ok(ClarionHypervisorCommand::RegisterClarionInstance(manifest)) => {
                    self.register_clarion_instance(manifest, ClarionPid(last_pid));
                    last_pid += 1;
                }
                Ok(ClarionHypervisorCommand::ProcessStacksChainEvent(event)) => {
                    self.handle_stacks_chain_event(event);
                }
                Ok(ClarionHypervisorCommand::ProcessBitcoinChainEvent(event)) => {
                    self.handle_bitcoin_chain_event(event);
                }
                Ok(ClarionHypervisorCommand::Exit) => {
                    println!("Exiting...");
                    return Ok(());
                }
                Err(e) => {
                    println!("{}", red!(format!("{}", e)));
                }
            }
        }
    }

    pub fn register_clarion_instance(&mut self, manifest: ClarionManifest, pid: ClarionPid) {
        let instance = ClarionInstance::new(manifest, pid.clone());
        let controller = ClarionInstanceController {
            pid: pid.clone(),
            tx: instance.tx.clone(),
        };
        self.clarion_controllers.insert(pid.clone(), controller);
        self.instances_pool.insert(pid, instance);
    }

    pub fn handle_stacks_chain_event(&mut self, chain_event: StacksChainEvent) {
        match chain_event {
            StacksChainEvent::ChainUpdatedWithBlock(new_block) => {
                let mut instances_to_trigger: HashSet<&TriggerId> = HashSet::new();

                // Start by adding the predicates looking for any new block
                instances_to_trigger.extend(&self.stacks_predicates.watching_any_block_activity);

                for tx in new_block.transactions.iter() {
                    let contract_id_based_predicates = self
                        .evaluate_predicates_watching_contract_mutations_activity(
                            &tx.metadata.receipt,
                        );
                    instances_to_trigger.extend(&contract_id_based_predicates);
                }

                for trigger in instances_to_trigger {
                    if let Some(controller) = self.clarion_controllers.get(&trigger.pid) {
                        controller.trigger_lambda(trigger.lambda_id);
                    }
                }
                // todo: keep track of trigger_history.
            }
            StacksChainEvent::ChainUpdatedWithReorg(old_segment, new_segment) => {
                // TODO(lgalabru): handle
                // todo: keep track of trigger_history.
            }
        }
    }

    pub fn handle_bitcoin_chain_event(&mut self, chain_event: BitcoinChainEvent) {
        match chain_event {
            BitcoinChainEvent::ChainUpdatedWithBlock(new_block) => {}
            BitcoinChainEvent::ChainUpdatedWithReorg(old_segment, new_segment) => {
                // TODO(lgalabru): handle
            }
        }
    }

    fn evaluate_predicates_watching_contract_mutations_activity(
        &self,
        transaction_receipt: &StacksTransactionReceipt,
    ) -> HashSet<&TriggerId> {
        let mut activated_triggers = HashSet::new();

        for contract_id in transaction_receipt.contracts_execution_radius.iter() {
            if let Some(triggers) = self
                .stacks_predicates
                .watching_contract_id_activity
                .get(contract_id)
            {
                activated_triggers.extend(triggers);
            }
        }

        activated_triggers
    }
}

#[derive(Clone, Debug)]
pub struct Lambda {
    lambda_id: u64,
    name: String,
    predicate: Predicate,
    action: Action,
}

#[derive(Clone, Debug)]
pub enum Action {
    User,
    Platform,
}

pub enum User {
    HTTPPost(String),
    CodeExecution(String),
}

pub enum Platform {
    StateExplorer,
    ApiGenerator,
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub enum Predicate {
    BitcoinPredicate,
    StacksPredicate,
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub enum BitcoinPredicate {
    AnyBlock,
    AnyOperation(AccountIdentifier),
    AnyStacksOperation(CrossStacksChainOperation, AccountIdentifier),
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub enum CrossStacksChainOperation {
    Any,
    MineBlock,
    TransferSTX,
    StacksSTX,
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub enum StacksPredicate {
    BitcoinPredicate,
    StacksContractPredicate,
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub enum StacksContractBasedPredicate {
    AnyCallToContract(QualifiedContractIdentifier),
    AnyResultFromContractCall(QualifiedContractIdentifier, String),
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub enum StacksOperationPredicate {
    AnyOperation(AccountIdentifier),
}

#[test]
fn instantiate_and_terminate_hypervisor() {
    let mut contracts = BTreeMap::new();
    let test_contract_id = QualifiedContractIdentifier::new(
        StandardPrincipalData::transient(),
        "test".try_into().unwrap(),
    );
    let test_contract_settings = ContractSettings {
        state_explorer_enabled: true,
        api_generator_enabled: vec![],
    };
    contracts.insert(test_contract_id, test_contract_settings);

    let clarion_manifest = ClarionManifest {
        project: ProjectMetadata {
            name: "test".into(),
            authors: vec![],
            homepage: "".into(),
            license: "".into(),
            description: "".into(),
        },
        lambdas: vec![],
        contracts,
    };

    let (hypervisor_cmd_tx, hypervisor_cmd_rx) = channel();
    let mut hypervisor = ClarionHypervisor::new(hypervisor_cmd_tx.clone(), hypervisor_cmd_rx);

    let id = std::thread::spawn(move || match hypervisor.run() {
        Ok(_) => Ok(hypervisor),
        Err(_) => Err(()),
    });

    hypervisor_cmd_tx
        .send(ClarionHypervisorCommand::RegisterClarionInstance(
            clarion_manifest,
        ))
        .unwrap();

    hypervisor_cmd_tx
        .send(ClarionHypervisorCommand::Exit)
        .unwrap();
    let hypervisor = id.join().unwrap().unwrap();

    assert_eq!(hypervisor.clarion_controllers.len(), 1);
    assert_eq!(hypervisor.instances_pool.len(), 1);
}

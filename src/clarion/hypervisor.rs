use crate::indexer::{BitcoinChainEvent, StacksChainEvent};
use clarity_repl::clarity::types::QualifiedContractIdentifier;
use crate::types::AccountIdentifier;
use std::collections::{BTreeMap, HashMap};
use std::sync::mpsc::{channel, Sender, Receiver};

pub struct TriggerId {
    clarion_instance_id: u64,
    lambda_id: u64,
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

pub struct ClarionInstanceController {
    clarion_instance_id: u64,
    tx: Sender<ClarionInstanceCommand>,
}

pub struct ClarionInstance {
    clarion_instance_id: u64,
    project_id: u64,
    metadata: ProjectMetadata,
    user_lambdas: Vec<Lambda>,
    platform_lambdas: Vec<Lambda>,
    contracts_ids: Vec<QualifiedContractIdentifier>,
    rx: Receiver<ClarionInstanceCommand>,
    tx: Sender<ClarionInstanceCommand>,
}

impl ClarionInstance {

    pub fn new(manifest: &ClarionManifest) -> ClarionInstance {
        let mut platform_lambdas = vec![];
        let (tx, rx) = channel();
        ClarionInstance {
            clarion_instance_id: 0,
            project_id: 0,
            contracts_ids: vec![],
            metadata: manifest.project.clone(),
            user_lambdas: manifest.lambdas.clone(),
            platform_lambdas,
            rx,
            tx
        }
    }
}

#[derive(Clone, Debug)]
pub enum ClarionHypervisorCommand {
    Exit
}

pub struct ClarionHypervisor {
    clarion_instances: HashMap<u64, ClarionInstanceController>,
    bitcoin_predicates: HashMap<BitcoinPredicate, Vec<TriggerId>>,
    stacks_predicates: HashMap<StacksPredicate, Vec<TriggerId>>,
    rx: Receiver<ClarionHypervisorCommand>,
    tx: Sender<ClarionHypervisorCommand>,    
}

impl ClarionHypervisor {
    pub fn new(tx: Sender<ClarionHypervisorCommand>, rx: Receiver<ClarionHypervisorCommand>) -> ClarionHypervisor {
        ClarionHypervisor {
            clarion_instances: HashMap::new(),
            bitcoin_predicates: HashMap::new(),
            stacks_predicates: HashMap::new(),
            tx,
            rx,
        }
    }

    pub fn run(&mut self) {
        loop {
            match self.rx.recv() {
                Ok(command) => {
                    println!("{:?}", command)
                }
                Err(e) => {
                    println!("{}", red!(format!("{}", e)));
                }
            }    
        }
    }

    pub fn start_clarion_process(&mut self, clarion_manifest: &ClarionManifest) {

    }

    pub fn handle_stacks_chain_event(&self, chain_event: BitcoinChainEvent) {

    }

    pub fn handle_bitcoin_chain_event(&self, chain_event: StacksChainEvent) {

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

#[derive(Clone, Debug)]
pub enum Predicate {
    BitcoinPredicate,
    StacksPredicate,
}

#[derive(Clone, Debug)]
pub enum BitcoinPredicate {
    AnyBlock,
    AnyOperation(AccountIdentifier),
    AnyStacksOperation(CrossStacksChainOperation, AccountIdentifier),
}

#[derive(Clone, Debug)]
pub enum CrossStacksChainOperation {
    Any,
    MineBlock,
    TransferSTX,
    StacksSTX,
}

#[derive(Clone, Debug)]
pub enum StacksPredicate {
    AnyBlock,
    AnyCallToContract(QualifiedContractIdentifier),
    AnyResultFromContractCall(QualifiedContractIdentifier, String),
    AnyOperation(AccountIdentifier),
}


#[test]
fn instantiate_and_terminate_hypervisor() {
    let (hypervisor_cmd_tx, hypervisor_cmd_rx) = channel();
    let hypervisor = ClarionHypervisor::new(hypervisor_cmd_tx, hypervisor_cmd_rx);
    println!("Hello tests :)");
}
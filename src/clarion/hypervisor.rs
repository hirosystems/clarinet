use crate::indexer::{BitcoinChainEvent, StacksChainEvent};
use crate::types::AccountIdentifier;
use clarity_repl::clarity::types::{QualifiedContractIdentifier, StandardPrincipalData};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::convert::TryInto;
use std::sync::mpsc::{channel, Receiver, Sender};

pub struct TriggerId {
    pid: u64,
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

#[derive(Debug)]
pub struct ClarionInstanceController {
    pid: ClarionPid,
    tx: Sender<ClarionInstanceCommand>,
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
}

#[derive(Clone, Debug)]
pub enum ClarionHypervisorCommand {
    RegisterInstance(ClarionManifest),
    Exit,
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct ClarionPid(u64);

pub struct ClarionHypervisor {
    instances_pool: HashMap<ClarionPid, ClarionInstance>,
    clarion_controllers: HashMap<ClarionPid, ClarionInstanceController>,
    bitcoin_predicates: HashMap<BitcoinPredicate, Vec<TriggerId>>,
    stacks_predicates: HashMap<StacksPredicate, Vec<TriggerId>>,
    rx: Receiver<ClarionHypervisorCommand>,
    tx: Sender<ClarionHypervisorCommand>,
}

impl ClarionHypervisor {
    pub fn new(
        tx: Sender<ClarionHypervisorCommand>,
        rx: Receiver<ClarionHypervisorCommand>,
    ) -> ClarionHypervisor {
        ClarionHypervisor {
            instances_pool: HashMap::new(),
            clarion_controllers: HashMap::new(),
            bitcoin_predicates: HashMap::new(),
            stacks_predicates: HashMap::new(),
            tx,
            rx,
        }
    }

    pub fn run(&mut self) -> Result<(), ()> {
        let mut last_pid = 1;
        loop {
            match self.rx.recv() {
                Ok(ClarionHypervisorCommand::RegisterInstance(manifest)) => {
                    println!("Registering new instance {:?}", manifest);
                    let pid = ClarionPid(last_pid);
                    let instance = ClarionInstance::new(manifest, pid.clone());
                    let controller = ClarionInstanceController {
                        pid: pid.clone(),
                        tx: instance.tx.clone(),
                    };
                    self.clarion_controllers.insert(pid.clone(), controller);
                    self.instances_pool.insert(pid, instance);
                    last_pid += 1;
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

    pub fn handle_stacks_chain_event(&self, chain_event: BitcoinChainEvent) {}

    pub fn handle_bitcoin_chain_event(&self, chain_event: StacksChainEvent) {}
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
        .send(ClarionHypervisorCommand::RegisterInstance(clarion_manifest))
        .unwrap();

    hypervisor_cmd_tx
        .send(ClarionHypervisorCommand::Exit)
        .unwrap();
    let hypervisor = id.join().unwrap().unwrap();

    assert_eq!(hypervisor.clarion_controllers.len(), 1);
    assert_eq!(hypervisor.instances_pool.len(), 1);
}

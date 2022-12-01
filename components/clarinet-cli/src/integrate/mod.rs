use std::sync::mpsc::{self, Sender};

use crate::chainhooks::load_chainhooks;
use chainhook_event_observer::utils::Context;
use chainhook_types::{BitcoinNetwork, StacksNetwork};
use clarinet_deployments::types::DeploymentSpecification;
use stacks_network::{
    do_run_devnet, ChainsCoordinatorCommand, DevnetEvent, DevnetOrchestrator, LogData,
};

pub fn run_devnet(
    devnet: DevnetOrchestrator,
    deployment: DeploymentSpecification,
    log_tx: Option<Sender<LogData>>,
    display_dashboard: bool,
) -> Result<
    (
        Option<mpsc::Receiver<DevnetEvent>>,
        Option<mpsc::Sender<bool>>,
        Option<mpsc::Sender<ChainsCoordinatorCommand>>,
    ),
    String,
> {
    let hooks = match load_chainhooks(
        &devnet.manifest.location,
        &(BitcoinNetwork::Regtest, StacksNetwork::Devnet),
    ) {
        Ok(hooks) => hooks,
        Err(e) => {
            println!("{}", e);
            std::process::exit(1);
        }
    };

    hiro_system_kit::nestable_block_on(do_run_devnet(
        devnet,
        deployment,
        &mut Some(hooks),
        log_tx,
        display_dashboard,
        Context::empty(),
    ))
}

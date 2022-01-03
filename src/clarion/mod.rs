mod hypervisor;
mod services;

use crate::utils;
pub use hypervisor::{ClarionHypervisor, ClarionHypervisorCommand};
use std::sync::mpsc::{channel, Receiver, Sender};

pub fn run_clarion_hypervisor(
    hypervisor_cmd_tx: Sender<ClarionHypervisorCommand>,
    hypervisor_cmd_rx: Receiver<ClarionHypervisorCommand>,
) -> Result<(), String> {
    match block_on(do_run_clarion_hypervisor(
        hypervisor_cmd_tx,
        hypervisor_cmd_rx,
    )) {
        Err(_e) => std::process::exit(1),
        Ok(res) => Ok(res),
    }
}

pub fn block_on<F, R>(future: F) -> R
where
    F: std::future::Future<Output = R>,
{
    let rt = utils::create_basic_runtime();
    rt.block_on(future)
}

pub async fn do_run_clarion_hypervisor(
    hypervisor_cmd_tx: Sender<ClarionHypervisorCommand>,
    hypervisor_cmd_rx: Receiver<ClarionHypervisorCommand>,
) -> Result<(), String> {
    let mut hypervisor = ClarionHypervisor::new(hypervisor_cmd_tx, hypervisor_cmd_rx);
    hypervisor.run();
    Ok(())
}

trait Datastore {}
// InMemory Datastore
// OnDisk Datastore
// Remote Datastore

mod supervisor;
mod services;

use crate::utils;
pub use supervisor::{ClarionSupervisor, ClarionSupervisorMessage};
use std::sync::mpsc::{channel, Receiver, Sender};

use kompact::{component::AbstractComponent, prelude::*};
use std::{
    error::Error,
    fmt,
    io::{stdin, BufRead},
    sync::Arc,
};

use kompact::prelude::*;

pub fn run_clarion_hypervisor(
    hypervisor_cmd_rx: Receiver<ClarionSupervisorMessage>,
) -> Result<(), String> {
    match block_on(do_run_clarion_hypervisor(
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
    hypervisor_cmd_rx: Receiver<ClarionSupervisorMessage>,
) -> Result<(), String> {
    let system = KompactConfig::default().build().expect("system");

    let hypervisor: Arc<Component<ClarionSupervisor>> = system.create(|| ClarionSupervisor::new() );
    system.start(&hypervisor);
    let hypervisor_ref = hypervisor.actor_ref();

    std::thread::spawn(move || {

        while let Ok(msg) = hypervisor_cmd_rx.recv() {
            hypervisor_ref.tell(msg);
        }
    });
    system.await_termination();
    Ok(())
}

trait Datastore {}
// InMemory Datastore
// OnDisk Datastore
// Remote Datastore

#[test]
fn spawn_integrated_hypervisor() {

    let (tx, rx) = channel();
    
    let handle = std::thread::spawn(|| {
        run_clarion_hypervisor(rx)
    });

    tx.send(ClarionSupervisorMessage::Exit).unwrap();

    let _res = handle.join().unwrap();
}

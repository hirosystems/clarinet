use crate::types::{ChainConfig, MainConfig};

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};
use bollard::Docker;
use bollard::container::{Config, KillContainerOptions, CreateContainerOptions, LogsOptions, NetworkingConfig};
use bollard::models::{HostConfig, Network, PortBinding};
use bollard::network::{ConnectNetworkOptions, CreateNetworkOptions};
// use bollard::exec::{CreateExecOptions, StartExecResults};
use bollard::image::CreateImageOptions;
use deno_core::futures::TryStreamExt;

// pub const STACKS_BLOCKCHAIN_IMAGE: &str = "blockstack/stacks-blockchain:latest";
// pub const STACKS_BLOCKCHAIN_API_IMAGE: &str = "blockstack/stacks-blockchain-api:latest";
// pub const STACKS_EXPLORER_IMAGE: &str = "blockstack/explorer:latest";
pub const BITCOIN_BLOCKCHAIN_IMAGE: &str  = "blockstack/bitcoind:puppet-chain";
// pub const BITCOIN_EXPLORER_IMAGE: &str  = "blockstack/bitcoind:puppet-chain";
// pub const POSTGRES_IMAGE: &str = "postgres:alpine";

pub fn run_devnet(devnet: &mut DevnetOrchestrator) {


    match block_on(do_run_devnet(devnet)) {
        Err(_e) => std::process::exit(1),
        _ => {}
    };
}

pub fn create_basic_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .max_blocking_threads(32)
        .build()
        .unwrap()
}

pub fn block_on<F, R>(future: F) -> R
where
    F: std::future::Future<Output = R>,
{
    let rt = create_basic_runtime();
    rt.block_on(future)
}

pub async fn do_run_devnet(
    devnet: &mut DevnetOrchestrator,
) -> Result<bool, String> {

    let event_tx = devnet.event_tx.clone().unwrap();
    let (termination_success_tx, termination_success_rx) = channel();
    devnet.termination_success_tx = Some(termination_success_tx);

    ctrlc::set_handler(move || {
        event_tx.send(DevnetEvent::Terminate)
            .expect("Unable to terminate devnet");
        let _res = termination_success_rx.recv();
        std::process::exit(0);
    }).expect("Error setting Ctrl-C handler");

    devnet.boot().await;
    
    devnet.run().await;

    Ok(true)
}

pub enum DevnetEvent {
    Log(String),
    Restart,
    Terminate,
}

#[derive(Default, Debug)]
pub struct DevnetOrchestrator {
    name: String,
    network_name: String,
    manifest_path: PathBuf,
    network_config_path: PathBuf,
    event_rx: Option<Receiver<DevnetEvent>>,
    pub event_tx: Option<Sender<DevnetEvent>>,
    termination_success_tx: Option<Sender<bool>>,
    stacks_blockchain_container_id: Option<String>,
    stacks_blockchain_api_container_id: Option<String>,
    stacks_explorer_container_id: Option<String>,
    bitcoin_blockchain_container_id: Option<String>,
    bitcoin_explorer_container_id: Option<String>,
    postgres_container_id: Option<String>,
    docker_client: Option<Docker>,
}

impl DevnetOrchestrator {

    pub fn new(manifest_path: PathBuf) -> DevnetOrchestrator {
        let docker_client = Docker::connect_with_socket_defaults().unwrap();

        let mut project_path = manifest_path.clone();
        project_path.pop();
    
        let mut network_config_path = project_path.clone();
        network_config_path.push("settings");
        network_config_path.push("Devnet.toml");
    
        let project_config = MainConfig::from_path(&manifest_path);
        let name = project_config.project.name.clone();
        let network_name = format!("{}.devnet", name);

        let (event_tx, event_rx) = channel();

        DevnetOrchestrator {
            name,
            network_name,
            manifest_path,
            network_config_path,
            event_rx: Some(event_rx),
            event_tx: Some(event_tx),
            docker_client: Some(docker_client),
            ..Default::default()
        }
    }

    pub async fn run(&mut self) {
        println!("Runloop");
        let event_rx = self.event_rx
            .take()
            .expect("Unable to get event receiver");
        
        while let Ok(event) = event_rx.recv() {
            match event {
                DevnetEvent::Terminate => {
                    self.terminate().await;
                }
                _ => {}
            }
        }
    }

    pub async fn boot(&mut self) {
        let docker = match self.docker_client {
            Some(ref docker) => docker.clone(),
            None => return
        };

        let mut network = docker.create_network(CreateNetworkOptions {
            name: self.network_name.clone(),
            ..Default::default()
        }).await.expect("Unable to create network");
        

        // Start bitcoind
        let bitcoin_container_id = match self.boot_bitcoin_container().await {
            Ok(id) => id,
            Err(message) => {
                println!("{}", message);
                self.terminate().await;
                std::process::exit(1);
            }
        };

        // // Start bitcoind puppeteer
        // let bitcoin_puppeteer_container_id = match self.boot_bitcoin_puppeteer_container().await {
        //     Ok(id) => id,
        //     Err(message) => {
        //         println!("{}", message);
        //         self.terminate().await;
        //         std::process::exit(1);
        //     }
        // };

        // // Start postgres
        // let postgres_container_id = match self.boot_postgres_container().await {
        //     Ok(id) => id,
        //     Err(message) => {
        //         println!("{}", message);
        //         self.terminate().await;
        //         std::process::exit(1);
        //     }
        // };

        // // Start stacks-blockchain-api
        // let stacks_blockchain_api_container_id = match self.boot_stacks_blockchain_api_container().await {
        //     Ok(id) => id,
        //     Err(message) => {
        //         println!("{}", message);
        //         self.terminate().await;
        //         std::process::exit(1);
        //     }
        // };

        // // Start stacks-blockchain-api
        // let stacks_blockchain_container_id = match self.boot_stacks_blockchain_container().await {
        //     Ok(id) => id,
        //     Err(message) => {
        //         println!("{}", message);
        //         self.terminate().await;
        //         std::process::exit(1);
        //     }
        // };

        // // Start stacks-explorer
        // let stacks_explorer_container_id = match self.boot_stacks_explorer_container().await {
        //     Ok(id) => id,
        //     Err(message) => {
        //         println!("{}", message);
        //         self.terminate().await;
        //         std::process::exit(1);
        //     }
        // };
        
        // // Start bitcoin-explorer
        // let bitcoin_explorer_container_id = match self.boot_bitcoin_explorer_container().await {
        //     Ok(id) => id,
        //     Err(message) => {
        //         println!("{}", message);
        //         self.terminate().await;
        //         std::process::exit(1);
        //     }
        // };

        // Start local observer
        // TODO
    }

    pub async fn boot_bitcoin_container(&mut self) -> Result<(), String> {
        let docker = match self.docker_client {
            Some(ref docker) => docker,
            None => return Err("Unable to get Docker client".into())
        };
    
        let info = docker
            .create_image(
                Some(CreateImageOptions {
                    from_image: BITCOIN_BLOCKCHAIN_IMAGE,
                    ..Default::default()
                }),
                None,
                None,
            )
            .try_collect::<Vec<_>>()
            .await
            .map_err(|_| "Unable to create image".to_string())?;
    
        let mut port_bindings = HashMap::new();
        port_bindings.insert(
            String::from("18444/tcp"),
            Some(vec![PortBinding {
                host_ip: Some(String::from("0.0.0.0")),
                host_port: Some(String::from("18444")),
            }]),
        );
        port_bindings.insert(
            String::from("18443/tcp"),
            Some(vec![PortBinding {
                host_ip: Some(String::from("0.0.0.0")),
                host_port: Some(String::from("18443")),
            }]),
        );

        let config = Config {
            image: Some(BITCOIN_BLOCKCHAIN_IMAGE.to_string()),
            domainname: Some(self.network_name.to_string()),
            tty: Some(true),
            host_config: Some(HostConfig {
                port_bindings: Some(port_bindings),
                // publish_all_ports: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        };
        
        let options = CreateContainerOptions {
            name: format!("bitcoin.{}", self.network_name)
        };

        let container = docker
            .create_container::<String, String>(Some(options), config)
            .await
            .map_err(|e| format!("Unable to create container: {}", e))?
            .id;
        
        self.bitcoin_blockchain_container_id = Some(container.clone());

        docker.start_container::<String>(&container, None)
            .await
            .map_err(|_| "Unable to start container".to_string())?;
        
        // let res = docker.connect_network(&self.network_name, ConnectNetworkOptions {
        //     container,
        //     ..Default::default()
        // }).await;

        // if let Err(e) = res {
        //     let err = format!("Error connecting container: {}", e);
        //     println!("{}", err);
        //     return Err(err)
        // }

        Ok(())
    }

    pub async fn boot_bitcoin_puppeteer_container(&mut self) -> Result<String, String> {
        let docker = match self.docker_client {
            Some(ref docker) => docker,
            None => return Err("Unable to get Docker client".into())
        };
    
        let info = docker
            .create_image(
                Some(CreateImageOptions {
                    from_image: BITCOIN_BLOCKCHAIN_IMAGE,
                    ..Default::default()
                }),
                None,
                None,
            )
            .try_collect::<Vec<_>>()
            .await
            .map_err(|_| "Unable to create image".to_string())?;
    
        let bitcoin_config = Config {
            image: Some(BITCOIN_BLOCKCHAIN_IMAGE),
            tty: Some(true),
            ..Default::default()
        };
    
        let id = docker
            .create_container::<&str, &str>(None, bitcoin_config)
            .await
            .map_err(|_| "Unable to create container".to_string())?
            .id;
        
        docker.start_container::<String>(&id, None)
            .await
            .map_err(|_| "Unable to start container".to_string())?;
        
        self.bitcoin_blockchain_container_id = Some(id.clone());

        Ok(id)
    }

    pub async fn boot_stacks_blockchain_container(&mut self) -> Result<String, String> {
        let docker = match self.docker_client {
            Some(ref docker) => docker,
            None => return Err("Unable to get Docker client".into())
        };
    
        let info = docker
            .create_image(
                Some(CreateImageOptions {
                    from_image: BITCOIN_BLOCKCHAIN_IMAGE,
                    ..Default::default()
                }),
                None,
                None,
            )
            .try_collect::<Vec<_>>()
            .await
            .map_err(|_| "Unable to create image".to_string())?;
    
        let bitcoin_config = Config {
            image: Some(BITCOIN_BLOCKCHAIN_IMAGE),
            tty: Some(true),
            ..Default::default()
        };
    
        let id = docker
            .create_container::<&str, &str>(None, bitcoin_config)
            .await
            .map_err(|_| "Unable to create container".to_string())?
            .id;
        
        docker.start_container::<String>(&id, None)
            .await
            .map_err(|_| "Unable to start container".to_string())?;
        
        self.bitcoin_blockchain_container_id = Some(id.clone());

        Ok(id)
    }

    pub async fn boot_postgres_container(&mut self) -> Result<String, String> {
        let docker = match self.docker_client {
            Some(ref docker) => docker,
            None => return Err("Unable to get Docker client".into())
        };
    
        let info = docker
            .create_image(
                Some(CreateImageOptions {
                    from_image: BITCOIN_BLOCKCHAIN_IMAGE,
                    ..Default::default()
                }),
                None,
                None,
            )
            .try_collect::<Vec<_>>()
            .await
            .map_err(|_| "Unable to create image".to_string())?;
    
        let bitcoin_config = Config {
            image: Some(BITCOIN_BLOCKCHAIN_IMAGE),
            tty: Some(true),
            ..Default::default()
        };
    
        let id = docker
            .create_container::<&str, &str>(None, bitcoin_config)
            .await
            .map_err(|_| "Unable to create container".to_string())?
            .id;
        
        docker.start_container::<String>(&id, None)
            .await
            .map_err(|_| "Unable to start container".to_string())?;
        
        self.bitcoin_blockchain_container_id = Some(id.clone());

        Ok(id)
    }

    pub async fn boot_stacks_blockchain_api_container(&mut self) -> Result<String, String> {
        let docker = match self.docker_client {
            Some(ref docker) => docker,
            None => return Err("Unable to get Docker client".into())
        };
    
        let info = docker
            .create_image(
                Some(CreateImageOptions {
                    from_image: BITCOIN_BLOCKCHAIN_IMAGE,
                    ..Default::default()
                }),
                None,
                None,
            )
            .try_collect::<Vec<_>>()
            .await
            .map_err(|_| "Unable to create image".to_string())?;
    
        let bitcoin_config = Config {
            image: Some(BITCOIN_BLOCKCHAIN_IMAGE),
            tty: Some(true),
            ..Default::default()
        };
    
        let id = docker
            .create_container::<&str, &str>(None, bitcoin_config)
            .await
            .map_err(|_| "Unable to create container".to_string())?
            .id;
        
        docker.start_container::<String>(&id, None)
            .await
            .map_err(|_| "Unable to start container".to_string())?;
        
        self.bitcoin_blockchain_container_id = Some(id.clone());

        Ok(id)
    }

    pub async fn boot_stacks_explorer_container(&mut self) -> Result<String, String> {
        let docker = match self.docker_client {
            Some(ref docker) => docker,
            None => return Err("Unable to get Docker client".into())
        };
    
        let info = docker
            .create_image(
                Some(CreateImageOptions {
                    from_image: BITCOIN_BLOCKCHAIN_IMAGE,
                    ..Default::default()
                }),
                None,
                None,
            )
            .try_collect::<Vec<_>>()
            .await
            .map_err(|_| "Unable to create image".to_string())?;
    
        let bitcoin_config = Config {
            image: Some(BITCOIN_BLOCKCHAIN_IMAGE),
            tty: Some(true),
            ..Default::default()
        };
    
        let id = docker
            .create_container::<&str, &str>(None, bitcoin_config)
            .await
            .map_err(|_| "Unable to create container".to_string())?
            .id;
        
        docker.start_container::<String>(&id, None)
            .await
            .map_err(|_| "Unable to start container".to_string())?;
        
        self.bitcoin_blockchain_container_id = Some(id.clone());

        Ok(id)
    }

    pub async fn boot_bitcoin_explorer_container(&mut self) -> Result<String, String> {
        let docker = match self.docker_client {
            Some(ref docker) => docker,
            None => return Err("Unable to get Docker client".into())
        };
    
        let info = docker
            .create_image(
                Some(CreateImageOptions {
                    from_image: BITCOIN_BLOCKCHAIN_IMAGE,
                    ..Default::default()
                }),
                None,
                None,
            )
            .try_collect::<Vec<_>>()
            .await
            .map_err(|_| "Unable to create image".to_string())?;
    
        let bitcoin_config = Config {
            image: Some(BITCOIN_BLOCKCHAIN_IMAGE),
            tty: Some(true),
            ..Default::default()
        };
    
        let id = docker
            .create_container::<&str, &str>(None, bitcoin_config)
            .await
            .map_err(|_| "Unable to create container".to_string())?
            .id;
        
        docker.start_container::<String>(&id, None)
            .await
            .map_err(|_| "Unable to start container".to_string())?;
        
        self.bitcoin_blockchain_container_id = Some(id.clone());

        Ok(id)
    }

    pub async fn restart(&mut self) {

    }

    pub async fn terminate(&mut self) {
        let docker = match self.docker_client {
            Some(ref docker) => docker,
            None => std::process::exit(1)
        };

        println!("Initiating termination sequence");

        let options = Some(KillContainerOptions{
            signal: "SIGKILL",
        });        

        if let Some(ref bitcoin_explorer_container_id) = self.bitcoin_explorer_container_id {
            println!("Terminating bitcoin_explorer");
            let _ = docker.kill_container(bitcoin_explorer_container_id, options.clone()).await;
            let _ = docker.remove_container(bitcoin_explorer_container_id, None);
        }

        if let Some(ref stacks_explorer_container_id) = self.stacks_explorer_container_id {
            println!("Terminating stacks_explorer");
            let _ = docker.kill_container(stacks_explorer_container_id, options.clone()).await;
        }

        if let Some(ref bitcoin_blockchain_container_id) = self.bitcoin_blockchain_container_id {
            println!("Terminating bitcoin_blockchain {}", bitcoin_blockchain_container_id);
            let _ = docker.kill_container(bitcoin_blockchain_container_id, options.clone()).await;
            let _ = docker.remove_container(bitcoin_blockchain_container_id, None);
        }

        if let Some(ref stacks_blockchain_api_container_id) = self.stacks_blockchain_api_container_id {
            println!("Terminating stacks_blockchain_api");
            let _ = docker.kill_container(stacks_blockchain_api_container_id, options.clone()).await;
        }

        if let Some(ref postgres_container_id) = self.postgres_container_id {
            println!("Terminating postgres");
            let _ = docker.kill_container(postgres_container_id, options.clone()).await;
        }

        if let Some(ref stacks_blockchain_container_id) = self.stacks_blockchain_container_id {
            println!("Terminating stacks_blockchain");
            let _ = docker.kill_container(stacks_blockchain_container_id, options).await;
        }

        docker.remove_network(&self.network_name).await;

        println!("Ended termination sequence");
        if let Some(ref tx) = self.termination_success_tx {
            tx.send(true).expect("Unable to confirm termination");
        }
    }
}
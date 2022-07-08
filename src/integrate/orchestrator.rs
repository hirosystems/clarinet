use super::DevnetEvent;
use crate::integrate::{ServiceStatusData, Status};
use bollard::container::{
    Config, CreateContainerOptions, KillContainerOptions, ListContainersOptions,
    PruneContainersOptions, WaitContainerOptions,
};
use bollard::errors::Error as DockerError;
use bollard::exec::CreateExecOptions;
use bollard::image::CreateImageOptions;
use bollard::models::{HostConfig, PortBinding};
use bollard::network::{ConnectNetworkOptions, CreateNetworkOptions, PruneNetworksOptions};
use bollard::Docker;
use clarinet_files::{DevnetConfigFile, NetworkManifest, ProjectManifest, DEFAULT_DEVNET_BALANCE};
use crossterm::terminal::disable_raw_mode;
use futures::stream::TryStreamExt;
use orchestra_types::StacksNetwork;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use tracing::info;

#[derive(Debug)]
pub struct DevnetOrchestrator {
    pub name: String,
    network_name: String,
    pub manifest: ProjectManifest,
    pub network_config: Option<NetworkManifest>,
    pub termination_success_tx: Option<Sender<bool>>,
    pub can_exit: bool,
    stacks_node_container_id: Option<String>,
    stacks_api_container_id: Option<String>,
    stacks_explorer_container_id: Option<String>,
    bitcoin_node_container_id: Option<String>,
    bitcoin_explorer_container_id: Option<String>,
    postgres_container_id: Option<String>,
    hyperchain_node_container_id: Option<String>,
    hyperchain_api_container_id: Option<String>,
    docker_client: Option<Docker>,
}

impl DevnetOrchestrator {
    pub fn new(
        manifest: ProjectManifest,
        devnet_override: Option<DevnetConfigFile>,
    ) -> Result<DevnetOrchestrator, String> {
        let docker_client = Docker::connect_with_socket_defaults()
            .map_err(|e| format!("unable to connect to docker: {:?}", e))?;

        let mut network_config = NetworkManifest::from_project_manifest_location(
            &manifest.location,
            &StacksNetwork::Devnet.get_networks(),
        )?;

        let name = manifest.project.name.clone();
        let network_name = format!("{}.devnet", name);

        match (&mut network_config.devnet, devnet_override) {
            (Some(ref mut devnet_config), Some(ref devnet_override)) => {
                if let Some(val) = devnet_override.orchestrator_port {
                    devnet_config.orchestrator_ingestion_port = val;
                }

                if let Some(val) = devnet_override.bitcoin_node_p2p_port {
                    devnet_config.bitcoin_node_p2p_port = val;
                }

                if let Some(val) = devnet_override.bitcoin_node_rpc_port {
                    devnet_config.bitcoin_node_rpc_port = val;
                }

                if let Some(val) = devnet_override.stacks_node_p2p_port {
                    devnet_config.stacks_node_p2p_port = val;
                }

                if let Some(val) = devnet_override.stacks_node_rpc_port {
                    devnet_config.stacks_node_rpc_port = val;
                }

                if let Some(ref val) = devnet_override.stacks_node_events_observers {
                    devnet_config.stacks_node_events_observers = val.clone();
                }

                if let Some(val) = devnet_override.stacks_api_port {
                    devnet_config.stacks_api_port = val;
                }

                if let Some(val) = devnet_override.stacks_api_events_port {
                    devnet_config.stacks_api_events_port = val;
                }

                if let Some(val) = devnet_override.bitcoin_explorer_port {
                    devnet_config.bitcoin_explorer_port = val;
                }

                if let Some(val) = devnet_override.stacks_explorer_port {
                    devnet_config.stacks_explorer_port = val;
                }

                if let Some(ref val) = devnet_override.bitcoin_node_username {
                    devnet_config.bitcoin_node_username = val.clone();
                }

                if let Some(ref val) = devnet_override.bitcoin_node_password {
                    devnet_config.bitcoin_node_password = val.clone();
                }

                if let Some(ref val) = devnet_override.miner_mnemonic {
                    devnet_config.miner_mnemonic = val.clone();
                }

                if let Some(ref val) = devnet_override.miner_derivation_path {
                    devnet_config.miner_derivation_path = val.clone();
                }

                if let Some(val) = devnet_override.bitcoin_controller_block_time {
                    devnet_config.bitcoin_controller_block_time = val;
                }

                if let Some(ref val) = devnet_override.working_dir {
                    devnet_config.working_dir = val.clone();
                }

                if let Some(val) = devnet_override.postgres_port {
                    devnet_config.postgres_port = val;
                }

                if let Some(ref val) = devnet_override.postgres_username {
                    devnet_config.postgres_username = val.clone();
                }

                if let Some(ref val) = devnet_override.postgres_password {
                    devnet_config.postgres_password = val.clone();
                }

                if let Some(ref val) = devnet_override.stacks_api_postgres_database {
                    devnet_config.stacks_api_postgres_database = val.clone();
                }

                if let Some(ref val) = devnet_override.hyperchain_api_postgres_database {
                    devnet_config.hyperchain_api_postgres_database = val.clone();
                }

                if let Some(ref val) = devnet_override.pox_stacking_orders {
                    devnet_config.pox_stacking_orders = val.clone();
                }

                if let Some(ref val) = devnet_override.execute_script {
                    devnet_config.execute_script = val.clone();
                }

                if let Some(ref val) = devnet_override.bitcoin_node_image_url {
                    devnet_config.bitcoin_node_image_url = val.clone();
                }

                if let Some(ref val) = devnet_override.bitcoin_explorer_image_url {
                    devnet_config.bitcoin_explorer_image_url = val.clone();
                }

                if let Some(ref val) = devnet_override.stacks_node_image_url {
                    devnet_config.stacks_node_image_url = val.clone();
                }

                if let Some(ref val) = devnet_override.stacks_api_image_url {
                    devnet_config.stacks_api_image_url = val.clone();
                }

                if let Some(ref val) = devnet_override.stacks_explorer_image_url {
                    devnet_config.stacks_explorer_image_url = val.clone();
                }

                if let Some(ref val) = devnet_override.postgres_image_url {
                    devnet_config.postgres_image_url = val.clone();
                }

                if let Some(val) = devnet_override.disable_bitcoin_explorer {
                    devnet_config.disable_bitcoin_explorer = val;
                }

                if let Some(val) = devnet_override.disable_stacks_explorer {
                    devnet_config.disable_stacks_explorer = val;
                }

                if let Some(val) = devnet_override.disable_stacks_api {
                    devnet_config.disable_stacks_api = val;
                }

                if let Some(val) = devnet_override.bitcoin_controller_automining_disabled {
                    devnet_config.bitcoin_controller_automining_disabled = val;
                }

                if let Some(val) = devnet_override.enable_hyperchain_node {
                    devnet_config.enable_hyperchain_node = val;
                }

                if let Some(val) = devnet_override.hyperchain_node_p2p_port {
                    devnet_config.hyperchain_node_p2p_port = val;
                }

                if let Some(val) = devnet_override.hyperchain_node_rpc_port {
                    devnet_config.hyperchain_node_rpc_port = val;
                }

                if let Some(val) = devnet_override.hyperchain_events_ingestion_port {
                    devnet_config.hyperchain_events_ingestion_port = val;
                }

                if let Some(ref val) = devnet_override.hyperchain_node_events_observers {
                    devnet_config.hyperchain_node_events_observers = val.clone();
                }

                if let Some(ref val) = devnet_override.hyperchain_node_image_url {
                    devnet_config.hyperchain_node_image_url = val.clone();
                }

                if let Some(ref val) = devnet_override.hyperchain_leader_derivation_path {
                    devnet_config.hyperchain_leader_derivation_path = val.clone();
                }

                if let Some(ref val) = devnet_override.hyperchain_leader_mnemonic {
                    devnet_config.hyperchain_leader_mnemonic = val.clone();
                }

                if let Some(ref val) = devnet_override.hyperchain_leader_mnemonic {
                    devnet_config.hyperchain_leader_mnemonic = val.clone();
                }
            }
            _ => {}
        };

        Ok(DevnetOrchestrator {
            name,
            network_name,
            manifest,
            network_config: Some(network_config),
            docker_client: Some(docker_client),
            can_exit: true,
            termination_success_tx: None,
            stacks_node_container_id: None,
            stacks_api_container_id: None,
            stacks_explorer_container_id: None,
            bitcoin_node_container_id: None,
            bitcoin_explorer_container_id: None,
            postgres_container_id: None,
            hyperchain_node_container_id: None,
            hyperchain_api_container_id: None,
        })
    }

    #[allow(dead_code)]
    pub fn get_stacks_node_url(&self) -> String {
        match self.network_config {
            Some(ref config) => match config.devnet {
                Some(ref devnet) => format!("http://localhost:{}", devnet.stacks_node_rpc_port),
                _ => unreachable!(),
            },
            _ => unreachable!(),
        }
    }

    pub async fn start(
        &mut self,
        event_tx: Sender<DevnetEvent>,
        terminator_rx: Receiver<bool>,
    ) -> Result<(), String> {
        let (docker, devnet_config) = match (&self.docker_client, &self.network_config) {
            (Some(ref docker), Some(ref network_config)) => match network_config.devnet {
                Some(ref devnet_config) => (docker, devnet_config),
                _ => return Err(format!("unable to get devnet config")),
            },
            _ => return Err(format!("unable to get devnet config")),
        };

        // First, let's make sure that we pruned staled resources correctly
        self.clean_previous_session().await;

        let mut boot_index = 1;

        let _ = event_tx.send(DevnetEvent::info(format!(
            "Initiating Devnet boot sequence (working_dir: {})",
            devnet_config.working_dir
        )));

        let mut devnet_path = PathBuf::from(&devnet_config.working_dir);
        devnet_path.push("data");

        let disable_stacks_api = devnet_config.disable_stacks_api;
        let disable_stacks_explorer = devnet_config.disable_stacks_explorer;
        let disable_bitcoin_explorer = devnet_config.disable_bitcoin_explorer;
        let enable_hyperchain_node = devnet_config.enable_hyperchain_node;
        let disable_hyperchain_api = devnet_config.disable_hyperchain_api;

        let _ = fs::create_dir(format!("{}", devnet_config.working_dir));
        let _ = fs::create_dir(format!("{}/conf", devnet_config.working_dir));
        let _ = fs::create_dir(format!("{}/data", devnet_config.working_dir));

        let bitcoin_explorer_port = devnet_config.bitcoin_explorer_port;
        let stacks_explorer_port = devnet_config.stacks_explorer_port;
        let stacks_api_port = devnet_config.stacks_api_port;
        let hyperchain_api_port = devnet_config.hyperchain_api_port;

        let _ = event_tx.send(DevnetEvent::ServiceStatus(ServiceStatusData {
            order: 0,
            status: Status::Red,
            name: "bitcoin-node".into(),
            comment: "initializing".into(),
        }));

        let _ = event_tx.send(DevnetEvent::ServiceStatus(ServiceStatusData {
            order: 1,
            status: Status::Red,
            name: "stacks-node".into(),
            comment: "initializing".into(),
        }));

        let _ = event_tx.send(DevnetEvent::ServiceStatus(ServiceStatusData {
            order: 2,
            status: Status::Red,
            name: "stacks-api".into(),
            comment: if disable_stacks_api {
                "disabled".into()
            } else {
                "initializing".into()
            },
        }));

        let _ = event_tx.send(DevnetEvent::ServiceStatus(ServiceStatusData {
            order: 3,
            status: Status::Red,
            name: "stacks-explorer".into(),
            comment: if disable_stacks_explorer {
                "disabled".into()
            } else {
                "initializing".into()
            },
        }));

        let _ = event_tx.send(DevnetEvent::ServiceStatus(ServiceStatusData {
            order: 4,
            status: Status::Red,
            name: "bitcoin-explorer".into(),
            comment: if disable_bitcoin_explorer {
                "disabled".into()
            } else {
                "initializing".into()
            },
        }));

        if enable_hyperchain_node {
            let _ = event_tx.send(DevnetEvent::ServiceStatus(ServiceStatusData {
                order: 5,
                status: Status::Red,
                name: "hyperchain-node".into(),
                comment: "initializing".into(),
            }));

            let _ = event_tx.send(DevnetEvent::ServiceStatus(ServiceStatusData {
                order: 6,
                status: Status::Red,
                name: "hyperchain-api".into(),
                comment: if disable_hyperchain_api {
                    "disabled".into()
                } else {
                    "initializing".into()
                },
            }));
        }

        let _ = event_tx.send(DevnetEvent::info(format!(
            "Creating network {}",
            self.network_name
        )));
        let mut labels = HashMap::new();
        labels.insert("project".to_string(), self.network_name.to_string());

        let _network = docker
            .create_network(CreateNetworkOptions {
                name: self.network_name.clone(),
                driver: "bridge".to_string(),
                labels,
                ..Default::default()
            })
            .await
            .map_err(|e| format!("unable to create network: {:?}", e))?;

        // Start bitcoind
        let _ = event_tx.send(DevnetEvent::info(format!("Starting bitcoin-node")));
        let _ = event_tx.send(DevnetEvent::ServiceStatus(ServiceStatusData {
            order: 0,
            status: Status::Yellow,
            name: "bitcoin-node".into(),
            comment: "preparing container".into(),
        }));
        match self.prepare_bitcoin_node_container().await {
            Ok(_) => {}
            Err(message) => {
                let _ = event_tx.send(DevnetEvent::FatalError(message.clone()));
                self.kill().await;
                return Err(message);
            }
        };
        let _ = event_tx.send(DevnetEvent::ServiceStatus(ServiceStatusData {
            order: 0,
            status: Status::Yellow,
            name: "bitcoin-node".into(),
            comment: "booting".into(),
        }));
        match self.boot_bitcoin_node_container().await {
            Ok(_) => {
                self.initialize_bitcoin_node(&event_tx)?;
            }
            Err(message) => {
                let _ = event_tx.send(DevnetEvent::FatalError(message.clone()));
                self.kill().await;
                return Err(message);
            }
        };

        // Start stacks-api
        if !disable_stacks_api {
            // Start postgres
            let _ = event_tx.send(DevnetEvent::ServiceStatus(ServiceStatusData {
                order: 2,
                status: Status::Yellow,
                name: "stacks-api".into(),
                comment: "preparing postgres container".into(),
            }));
            let _ = event_tx.send(DevnetEvent::info(format!("Starting postgres")));
            match self.prepare_postgres_container().await {
                Ok(_) => {}
                Err(message) => {
                    let _ = event_tx.send(DevnetEvent::FatalError(message.clone()));
                    self.kill().await;
                    return Err(message);
                }
            };
            match self.boot_postgres_container().await {
                Ok(_) => {}
                Err(message) => {
                    let _ = event_tx.send(DevnetEvent::FatalError(message.clone()));
                    self.kill().await;
                    return Err(message);
                }
            };
            let _ = event_tx.send(DevnetEvent::ServiceStatus(ServiceStatusData {
                order: 2,
                status: Status::Yellow,
                name: "stacks-api".into(),
                comment: "preparing container".into(),
            }));

            let _ = event_tx.send(DevnetEvent::info(format!("Starting stacks-api")));
            match self.prepare_stacks_api_container().await {
                Ok(_) => {}
                Err(message) => {
                    let _ = event_tx.send(DevnetEvent::FatalError(message.clone()));
                    self.kill().await;
                    return Err(message);
                }
            };
            let _ = event_tx.send(DevnetEvent::ServiceStatus(ServiceStatusData {
                order: 2,
                status: Status::Green,
                name: "stacks-api".into(),
                comment: format!("http://localhost:{}/doc", stacks_api_port),
            }));
            match self.boot_stacks_api_container().await {
                Ok(_) => {}
                Err(message) => {
                    let _ = event_tx.send(DevnetEvent::FatalError(message.clone()));
                    self.kill().await;
                    return Err(message);
                }
            };
        }

        // Start Hyperchain node
        if enable_hyperchain_node {
            let _ = event_tx.send(DevnetEvent::info(format!("Starting hyperchain-node")));
            match self.prepare_hyperchain_node_container(boot_index).await {
                Ok(_) => {}
                Err(message) => {
                    let _ = event_tx.send(DevnetEvent::FatalError(message.clone()));
                    self.kill().await;
                    return Err(message);
                }
            };
            let _ = event_tx.send(DevnetEvent::ServiceStatus(ServiceStatusData {
                order: 5,
                status: Status::Yellow,
                name: "hyperchain-node".into(),
                comment: "booting".into(),
            }));
            match self.boot_hyperchain_node_container().await {
                Ok(_) => {}
                Err(message) => {
                    let _ = event_tx.send(DevnetEvent::FatalError(message.clone()));
                    self.kill().await;
                    return Err(message);
                }
            };

            if !disable_hyperchain_api {
                let _ = event_tx.send(DevnetEvent::info(format!("Starting hyperchain-api")));
                match self.prepare_hyperchain_api_container().await {
                    Ok(_) => {}
                    Err(message) => {
                        let _ = event_tx.send(DevnetEvent::FatalError(message.clone()));
                        self.kill().await;
                        return Err(message);
                    }
                };
                let _ = event_tx.send(DevnetEvent::ServiceStatus(ServiceStatusData {
                    order: 6,
                    status: Status::Green,
                    name: "hyperchain-api".into(),
                    comment: format!("http://localhost:{}/doc", hyperchain_api_port),
                }));
                match self.boot_hyperchain_api_container().await {
                    Ok(_) => {}
                    Err(message) => {
                        let _ = event_tx.send(DevnetEvent::FatalError(message.clone()));
                        self.kill().await;
                        return Err(message);
                    }
                };
            }
        }

        // Start stacks-blockchain
        let _ = event_tx.send(DevnetEvent::info(format!("Starting stacks-node")));
        let _ = event_tx.send(DevnetEvent::ServiceStatus(ServiceStatusData {
            order: 1,
            status: Status::Yellow,
            name: "stacks-node".into(),
            comment: "updating image".into(),
        }));
        match self.prepare_stacks_node_container(boot_index).await {
            Ok(_) => {}
            Err(message) => {
                let _ = event_tx.send(DevnetEvent::FatalError(message.clone()));
                self.kill().await;
                return Err(message);
            }
        };
        let _ = event_tx.send(DevnetEvent::ServiceStatus(ServiceStatusData {
            order: 1,
            status: Status::Yellow,
            name: "stacks-node".into(),
            comment: "booting".into(),
        }));
        match self.boot_stacks_node_container().await {
            Ok(_) => {}
            Err(message) => {
                let _ = event_tx.send(DevnetEvent::FatalError(message.clone()));
                self.kill().await;
                return Err(message);
            }
        };

        // Start stacks-explorer
        if !disable_stacks_explorer {
            let _ = event_tx.send(DevnetEvent::ServiceStatus(ServiceStatusData {
                order: 3,
                status: Status::Yellow,
                name: "stacks-explorer".into(),
                comment: "preparing container".into(),
            }));
            match self.prepare_stacks_explorer_container().await {
                Ok(_) => {}
                Err(message) => {
                    let _ = event_tx.send(DevnetEvent::FatalError(message.clone()));
                    self.kill().await;
                    return Err(message);
                }
            };
            let _ = event_tx.send(DevnetEvent::info(format!("Starting stacks-explorer")));
            match self.boot_stacks_explorer_container().await {
                Ok(_) => {}
                Err(message) => {
                    let _ = event_tx.send(DevnetEvent::FatalError(message.clone()));
                    self.kill().await;
                    return Err(message);
                }
            };
            let _ = event_tx.send(DevnetEvent::ServiceStatus(ServiceStatusData {
                order: 3,
                status: Status::Green,
                name: "stacks-explorer".into(),
                comment: format!("http://localhost:{}", stacks_explorer_port),
            }));
        }

        // Start bitcoin-explorer
        if !disable_bitcoin_explorer {
            let _ = event_tx.send(DevnetEvent::ServiceStatus(ServiceStatusData {
                order: 4,
                status: Status::Yellow,
                name: "bitcoin-explorer".into(),
                comment: "preparing container".into(),
            }));
            match self.prepare_bitcoin_explorer_container().await {
                Ok(_) => {}
                Err(message) => {
                    let _ = event_tx.send(DevnetEvent::FatalError(message.clone()));
                    self.kill().await;
                    return Err(message);
                }
            };
            let _ = event_tx.send(DevnetEvent::info(format!("Starting bitcoin-explorer")));
            match self.boot_bitcoin_explorer_container().await {
                Ok(_) => {}
                Err(message) => {
                    let _ = event_tx.send(DevnetEvent::FatalError(message.clone()));
                    self.kill().await;
                    return Err(message);
                }
            };
            let _ = event_tx.send(DevnetEvent::ServiceStatus(ServiceStatusData {
                order: 4,
                status: Status::Green,
                name: "bitcoin-explorer".into(),
                comment: format!("http://localhost:{}", bitcoin_explorer_port),
            }));
        }

        loop {
            boot_index += 1;
            match terminator_rx.recv() {
                Ok(true) => {
                    self.kill().await;
                    break;
                }
                Ok(false) => {
                    let _ = event_tx.send(DevnetEvent::ServiceStatus(ServiceStatusData {
                        order: 0,
                        status: Status::Yellow,
                        name: "bitcoin-node".into(),
                        comment: "restarting".into(),
                    }));
                    let _ = event_tx.send(DevnetEvent::ServiceStatus(ServiceStatusData {
                        order: 1,
                        status: Status::Yellow,
                        name: "stacks-node".into(),
                        comment: "restarting".into(),
                    }));

                    let _ = event_tx.send(DevnetEvent::debug("Killing containers...".into()));
                    let _ = self.stop_containers().await;

                    let _ = event_tx.send(DevnetEvent::debug("Restarting containers...".into()));
                    let (bitcoin_node_c_id, stacks_node_c_id) = self
                        .start_containers(boot_index)
                        .await
                        .map_err(|e| format!("unable to reboot: {:?}", e))?;
                    self.bitcoin_node_container_id = Some(bitcoin_node_c_id);
                    self.stacks_node_container_id = Some(stacks_node_c_id);
                }
                Err(_) => {
                    break;
                }
            }
        }
        Ok(())
    }

    pub fn prepare_bitcoin_node_config(&self, boot_index: u32) -> Result<Config<String>, String> {
        let devnet_config = match &self.network_config {
            Some(ref network_config) => match network_config.devnet {
                Some(ref devnet_config) => devnet_config,
                _ => return Err("unable to get devnet configuration".into()),
            },
            _ => return Err("unable to get Docker client".into()),
        };

        let mut port_bindings = HashMap::new();
        port_bindings.insert(
            format!("{}/tcp", devnet_config.bitcoin_node_p2p_port),
            Some(vec![PortBinding {
                host_ip: Some(String::from("0.0.0.0")),
                host_port: Some(format!("{}/tcp", devnet_config.bitcoin_node_p2p_port)),
            }]),
        );
        port_bindings.insert(
            format!("{}/tcp", devnet_config.bitcoin_node_rpc_port),
            Some(vec![PortBinding {
                host_ip: Some(String::from("0.0.0.0")),
                host_port: Some(format!("{}/tcp", devnet_config.bitcoin_node_rpc_port)),
            }]),
        );

        let bitcoind_conf = format!(
            r#"
server=1
regtest=1
rpcallowip=0.0.0.0/0
rpcallowip=::/0
rpcuser={bitcoin_node_username}
rpcpassword={bitcoin_node_password}
txindex=1
listen=1
discover=0
dns=0
dnsseed=0
listenonion=0
rpcserialversion=0
rpcworkqueue=100
disablewallet=0
fallbackfee=0.00001

[regtest]
bind=0.0.0.0:{bitcoin_node_p2p_port}
rpcbind=0.0.0.0:{bitcoin_node_rpc_port}
rpcport={bitcoin_node_rpc_port}
"#,
            bitcoin_node_username = devnet_config.bitcoin_node_username,
            bitcoin_node_password = devnet_config.bitcoin_node_password,
            bitcoin_node_p2p_port = devnet_config.bitcoin_node_p2p_port,
            bitcoin_node_rpc_port = devnet_config.bitcoin_node_rpc_port,
        );
        let mut bitcoind_conf_path = PathBuf::from(&devnet_config.working_dir);
        bitcoind_conf_path.push("conf/bitcoin.conf");
        let mut file = File::create(bitcoind_conf_path)
            .map_err(|e| format!("unable to create bitcoin.conf: {:?}", e))?;

        file.write_all(bitcoind_conf.as_bytes())
            .map_err(|e| format!("unable to write bitcoin.conf: {:?}", e))?;

        let mut bitcoind_data_path = PathBuf::from(&devnet_config.working_dir);
        bitcoind_data_path.push("data");
        bitcoind_data_path.push(format!("{}", boot_index));
        bitcoind_data_path.push("bitcoin");
        fs::create_dir_all(bitcoind_data_path)
            .map_err(|e| format!("unable to create bitcoin directory: {:?}", e))?;

        let mut exposed_ports = HashMap::new();
        exposed_ports.insert(
            format!("{}/tcp", devnet_config.bitcoin_node_rpc_port),
            HashMap::new(),
        );
        exposed_ports.insert(
            format!("{}/tcp", devnet_config.bitcoin_node_p2p_port),
            HashMap::new(),
        );

        let mut labels = HashMap::new();
        labels.insert("project".to_string(), self.network_name.to_string());
        labels.insert("reset".to_string(), "true".to_string());

        let mut env = vec![];
        if devnet_config.bitcoin_controller_automining_disabled {
            env.push("STACKS_BITCOIN_AUTOMINING_DISABLED=1".to_string());
        }

        let mut binds = vec![format!("{}/conf:/etc/bitcoin", devnet_config.working_dir)];

        if devnet_config.bind_containers_volumes {
            binds.push(format!(
                "{}/data/{}/bitcoin:/root/.bitcoin",
                devnet_config.working_dir, boot_index
            ));
        }

        let config = Config {
            labels: Some(labels),
            image: Some(devnet_config.bitcoin_node_image_url.clone()),
            domainname: Some(self.network_name.to_string()),
            tty: None,
            exposed_ports: Some(exposed_ports),
            entrypoint: Some(vec![]),
            env: Some(env),
            host_config: Some(HostConfig {
                port_bindings: Some(port_bindings),
                binds: Some(binds),
                extra_hosts: Some(vec!["host.docker.internal:host-gateway".into()]),
                ..Default::default()
            }),
            ..Default::default()
        };

        Ok(config)
    }

    pub async fn prepare_bitcoin_node_container(&mut self) -> Result<(), String> {
        let (docker, devnet_config) = match (&self.docker_client, &self.network_config) {
            (Some(ref docker), Some(ref network_config)) => match network_config.devnet {
                Some(ref devnet_config) => (docker, devnet_config),
                _ => return Err("unable to get devnet configuration".into()),
            },
            _ => return Err("unable to get Docker client".into()),
        };

        let _info = docker
            .create_image(
                Some(CreateImageOptions {
                    from_image: devnet_config.bitcoin_node_image_url.clone(),
                    ..Default::default()
                }),
                None,
                None,
            )
            .try_collect::<Vec<_>>()
            .await
            .map_err(|e| formatted_docker_error("unable to create bitcoind image", e))?;

        let config = self.prepare_bitcoin_node_config(1)?;
        let options = CreateContainerOptions {
            name: format!("bitcoin-node.{}", self.network_name),
        };

        let container = docker
            .create_container::<String, String>(Some(options), config)
            .await
            .map_err(|e| formatted_docker_error("unable to create bitcoind container", e))?
            .id;
        info!("Created container bitcoin-node: {}", container);
        self.bitcoin_node_container_id = Some(container);

        Ok(())
    }

    pub async fn clean_previous_session(&self) {
        let mut filters = HashMap::new();
        filters.insert(
            "label".to_string(),
            vec![format!("project={}", self.network_name)],
        );
        let options = Some(ListContainersOptions {
            all: true,
            filters,
            ..Default::default()
        });

        let docker = match &self.docker_client {
            Some(ref docker) => docker,
            _ => panic!("unable to get Docker client"),
        };
        let res = docker.list_containers(options).await;
        let containers = match res {
            Ok(containers) => containers,
            Err(_) => {
                println!("unable to start Devnet: make sure that Docker is installed on this machine and running.");
                return process_exit();
            }
        };
        let options = KillContainerOptions { signal: "SIGKILL" };

        for container in containers.iter() {
            let container_id = match &container.id {
                Some(id) => id,
                None => continue,
            };
            let _ = docker
                .kill_container(&container_id, Some(options.clone()))
                .await;

            let _ = docker
                .wait_container(&container_id, None::<WaitContainerOptions<String>>)
                .try_collect::<Vec<_>>()
                .await;
        }
        self.prune().await;
    }

    pub async fn boot_bitcoin_node_container(&self) -> Result<(), String> {
        let container = match &self.bitcoin_node_container_id {
            Some(container) => container.clone(),
            _ => return Err(format!("unable to boot container")),
        };

        let docker = match &self.docker_client {
            Some(ref docker) => docker,
            _ => return Err("unable to get Docker client".into()),
        };

        docker
            .start_container::<String>(&container, None)
            .await
            .map_err(|e| formatted_docker_error("unable to start bitcoind container", e))?;

        let res = docker
            .connect_network(
                &self.network_name,
                ConnectNetworkOptions {
                    container,
                    ..Default::default()
                },
            )
            .await;

        if let Err(e) = res {
            let err = format!("Error connecting container: {}", e);
            println!("{}", err);
            return Err(err);
        }

        Ok(())
    }

    pub fn prepare_stacks_node_config(&self, boot_index: u32) -> Result<Config<String>, String> {
        let (network_config, devnet_config) = match &self.network_config {
            Some(ref network_config) => match network_config.devnet {
                Some(ref devnet_config) => (network_config, devnet_config),
                _ => return Err("unable to get devnet configuration".into()),
            },
            _ => return Err("unable to get Docker client".into()),
        };

        let mut port_bindings = HashMap::new();
        port_bindings.insert(
            format!("{}/tcp", devnet_config.stacks_node_p2p_port),
            Some(vec![PortBinding {
                host_ip: Some(String::from("0.0.0.0")),
                host_port: Some(format!("{}/tcp", devnet_config.stacks_node_p2p_port)),
            }]),
        );
        port_bindings.insert(
            format!("{}/tcp", devnet_config.stacks_node_rpc_port),
            Some(vec![PortBinding {
                host_ip: Some(String::from("0.0.0.0")),
                host_port: Some(format!("{}/tcp", devnet_config.stacks_node_rpc_port)),
            }]),
        );

        let mut stacks_conf = format!(
            r#"
[node]
working_dir = "/devnet"
rpc_bind = "0.0.0.0:{stacks_node_rpc_port}"
p2p_bind = "0.0.0.0:{stacks_node_p2p_port}"
miner = true
seed = "{miner_secret_key_hex}"
local_peer_seed = "{miner_secret_key_hex}"
wait_time_for_microblocks = 5000
pox_sync_sample_secs = 10
microblock_frequency = 15000

[burnchain]
chain = "bitcoin"
mode = "krypton"
poll_time_secs = 1
peer_host = "host.docker.internal"
username = "{bitcoin_node_username}"
password = "{bitcoin_node_password}"
rpc_port = {orchestrator_ingestion_port}
peer_port = {bitcoin_node_p2p_port}

[miner]
first_attempt_time_ms = 10000
subsequent_attempt_time_ms = 10000
# microblock_attempt_time_ms = 15000
"#,
            stacks_node_rpc_port = devnet_config.stacks_node_rpc_port,
            stacks_node_p2p_port = devnet_config.stacks_node_p2p_port,
            miner_secret_key_hex = devnet_config.miner_secret_key_hex,
            bitcoin_node_username = devnet_config.bitcoin_node_username,
            bitcoin_node_password = devnet_config.bitcoin_node_password,
            bitcoin_node_p2p_port = devnet_config.bitcoin_node_p2p_port,
            orchestrator_ingestion_port = devnet_config.orchestrator_ingestion_port,
        );

        for (_, account) in network_config.accounts.iter() {
            stacks_conf.push_str(&format!(
                r#"
[[ustx_balance]]
address = "{}"
amount = {}
"#,
                account.stx_address, account.balance
            ));
        }

        stacks_conf.push_str(&format!(
            r#"
# Add orchestrator (docker-host) as an event observer
[[events_observer]]
endpoint = "host.docker.internal:{orchestrator_ingestion_port}"
retry_count = 255
include_data_events = true
events_keys = ["*"]
"#,
            orchestrator_ingestion_port = devnet_config.orchestrator_ingestion_port,
        ));

        if !devnet_config.disable_stacks_api {
            stacks_conf.push_str(&format!(
                r#"
# Add stacks-api as an event observer
[[events_observer]]
endpoint = "{}"
retry_count = 255
include_data_events = false
events_keys = ["*"]
"#,
                format!(
                    "stacks-api.{}:{}",
                    self.network_name, devnet_config.stacks_api_events_port
                ),
            ));
        }

        for chains_coordinator in devnet_config.stacks_node_events_observers.iter() {
            stacks_conf.push_str(&format!(
                r#"
[[events_observer]]
endpoint = "{}"
retry_count = 255
events_keys = ["*"]
"#,
                chains_coordinator,
            ));
        }

        let mut stacks_conf_path = PathBuf::from(&devnet_config.working_dir);
        stacks_conf_path.push("conf/Stacks.toml");
        let mut file = File::create(stacks_conf_path)
            .map_err(|e| format!("unable to create Stacks.toml: {:?}", e))?;
        file.write_all(stacks_conf.as_bytes())
            .map_err(|e| format!("unable to write Stacks.toml: {:?}", e))?;

        let mut stacks_node_data_path = PathBuf::from(&devnet_config.working_dir);
        stacks_node_data_path.push("data");
        stacks_node_data_path.push(format!("{}", boot_index));
        stacks_node_data_path.push("stacks");
        fs::create_dir_all(stacks_node_data_path)
            .map_err(|e| format!("unable to create stacks directory: {:?}", e))?;

        let mut exposed_ports = HashMap::new();
        exposed_ports.insert(
            format!("{}/tcp", devnet_config.stacks_node_rpc_port),
            HashMap::new(),
        );
        exposed_ports.insert(
            format!("{}/tcp", devnet_config.stacks_node_p2p_port),
            HashMap::new(),
        );

        let mut labels = HashMap::new();
        labels.insert("project".to_string(), self.network_name.to_string());
        labels.insert("reset".to_string(), "true".to_string());

        let mut binds = vec![format!(
            "{}/conf:/src/stacks-node/",
            devnet_config.working_dir
        )];

        if devnet_config.bind_containers_volumes {
            binds.push(format!(
                "{}/data/{}/stacks:/devnet/",
                devnet_config.working_dir, boot_index
            ))
        }

        let config = Config {
            labels: Some(labels),
            image: Some(devnet_config.stacks_node_image_url.clone()),
            domainname: Some(self.network_name.to_string()),
            tty: None,
            exposed_ports: Some(exposed_ports),
            entrypoint: Some(vec![
                "stacks-node".into(),
                "start".into(),
                "--config=/src/stacks-node/Stacks.toml".into(),
            ]),
            env: Some(vec![
                "STACKS_LOG_PP=1".to_string(),
                // "STACKS_LOG_DEBUG=1".to_string(),
                "BLOCKSTACK_USE_TEST_GENESIS_CHAINSTATE=1".to_string(),
            ]),
            host_config: Some(HostConfig {
                port_bindings: Some(port_bindings),
                binds: Some(binds),
                extra_hosts: Some(vec!["host.docker.internal:host-gateway".into()]),
                ..Default::default()
            }),
            ..Default::default()
        };

        Ok(config)
    }

    pub async fn prepare_stacks_node_container(&mut self, boot_index: u32) -> Result<(), String> {
        let (docker, devnet_config) = match (&self.docker_client, &self.network_config) {
            (Some(ref docker), Some(ref network_config)) => match network_config.devnet {
                Some(ref devnet_config) => (docker, devnet_config),
                _ => return Err("unable to get devnet configuration".into()),
            },
            _ => return Err("unable to get Docker client".into()),
        };

        let _info = docker
            .create_image(
                Some(CreateImageOptions {
                    from_image: devnet_config.stacks_node_image_url.clone(),
                    ..Default::default()
                }),
                None,
                None,
            )
            .try_collect::<Vec<_>>()
            .await
            .map_err(|e| format!("unable to create image: {}", e))?;

        let config = self.prepare_stacks_node_config(boot_index)?;

        let options = CreateContainerOptions {
            name: format!("stacks-node.{}", self.network_name),
        };

        let container = docker
            .create_container::<String, String>(Some(options), config)
            .await
            .map_err(|e| format!("unable to create container: {}", e))?
            .id;

        info!("Created container stacks-node: {}", container);
        self.stacks_node_container_id = Some(container.clone());

        Ok(())
    }

    pub async fn boot_stacks_node_container(&self) -> Result<(), String> {
        let container = match &self.stacks_node_container_id {
            Some(container) => container.clone(),
            _ => return Err(format!("unable to boot container")),
        };

        let docker = match &self.docker_client {
            Some(ref docker) => docker,
            _ => return Err("unable to get Docker client".into()),
        };

        docker
            .start_container::<String>(&container, None)
            .await
            .map_err(|e| formatted_docker_error("unable to start stacks-node container", e))?;

        let res = docker
            .connect_network(
                &self.network_name,
                ConnectNetworkOptions {
                    container,
                    ..Default::default()
                },
            )
            .await;

        if let Err(e) = res {
            let err = format!("Error connecting container: {}", e);
            println!("{}", err);
            return Err(err);
        }

        Ok(())
    }

    pub fn prepare_hyperchain_node_config(
        &self,
        boot_index: u32,
    ) -> Result<Config<String>, String> {
        let (network_config, devnet_config) = match &self.network_config {
            Some(ref network_config) => match network_config.devnet {
                Some(ref devnet_config) => (network_config, devnet_config),
                _ => return Err("unable to get devnet configuration".into()),
            },
            _ => return Err("unable to get Docker client".into()),
        };

        let mut port_bindings = HashMap::new();
        port_bindings.insert(
            format!("{}/tcp", devnet_config.hyperchain_node_p2p_port),
            Some(vec![PortBinding {
                host_ip: Some(String::from("0.0.0.0")),
                host_port: Some(format!("{}/tcp", devnet_config.hyperchain_node_p2p_port)),
            }]),
        );
        port_bindings.insert(
            format!("{}/tcp", devnet_config.hyperchain_node_rpc_port),
            Some(vec![PortBinding {
                host_ip: Some(String::from("0.0.0.0")),
                host_port: Some(format!("{}/tcp", devnet_config.hyperchain_node_rpc_port)),
            }]),
        );
        port_bindings.insert(
            format!("{}/tcp", devnet_config.hyperchain_events_ingestion_port),
            Some(vec![PortBinding {
                host_ip: Some(String::from("0.0.0.0")),
                host_port: Some(format!(
                    "{}/tcp",
                    devnet_config.hyperchain_events_ingestion_port
                )),
            }]),
        );

        let mut hyperchain_conf = format!(
            r#"
[node]
working_dir = "/devnet"
rpc_bind = "0.0.0.0:{hyperchain_node_rpc_port}"
p2p_bind = "0.0.0.0:{hyperchain_node_p2p_port}"
miner = true
seed = "{hyperchain_leader_secret_key_hex}"
mining_key = "{hyperchain_leader_secret_key_hex}"
local_peer_seed = "{hyperchain_leader_secret_key_hex}"
wait_time_for_microblocks = 3_000
wait_before_first_anchored_block = 0

[miner]
first_attempt_time_ms = 5_000
subsequent_attempt_time_ms = 5_000
# microblock_attempt_time_ms = 15_000

[burnchain]
chain = "stacks_layer_1"
mode = "hyperchain"
first_burn_header_height = {first_burn_header_height}
peer_host = "host.docker.internal"
rpc_port = {stacks_node_rpc_port}
peer_port = {stacks_node_p2p_port}
contract_identifier = "{hyperchain_contract_id}"
observer_port = {hyperchain_events_ingestion_port}

# Add orchestrator (docker-host) as an event observer
# [[events_observer]]
# endpoint = "host.docker.internal:{orchestrator_port}"
# retry_count = 255
# include_data_events = true
# events_keys = ["*"]
"#,
            hyperchain_node_rpc_port = devnet_config.hyperchain_node_rpc_port,
            hyperchain_node_p2p_port = devnet_config.hyperchain_node_p2p_port,
            hyperchain_leader_secret_key_hex = devnet_config.hyperchain_leader_secret_key_hex,
            stacks_node_rpc_port = devnet_config.stacks_node_rpc_port,
            stacks_node_p2p_port = devnet_config.stacks_node_p2p_port,
            orchestrator_port = devnet_config.orchestrator_ingestion_port,
            hyperchain_events_ingestion_port = devnet_config.hyperchain_events_ingestion_port,
            first_burn_header_height = 0,
            hyperchain_contract_id = "ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.hc" //devnet_config.hyperchain_contract_id,
        );

        hyperchain_conf.push_str(&format!(
            r#"
[[ustx_balance]]
address = "{hyperchain_leader_stx_address}"
amount = {default_balance}
"#,
            hyperchain_leader_stx_address = devnet_config.hyperchain_leader_stx_address,
            default_balance = DEFAULT_DEVNET_BALANCE
        ));

        for (_, account) in network_config.accounts.iter() {
            hyperchain_conf.push_str(&format!(
                r#"
[[ustx_balance]]
address = "{}"
amount = {}
"#,
                account.stx_address, account.balance
            ));
        }

        for events_observer in devnet_config.hyperchain_node_events_observers.iter() {
            hyperchain_conf.push_str(&format!(
                r#"
[[events_observer]]
endpoint = "{}"
retry_count = 255
events_keys = ["*"]
"#,
                events_observer,
            ));
        }

        if !devnet_config.disable_hyperchain_api {
            hyperchain_conf.push_str(&format!(
                r#"
# Add hyperchain-api as an event observer
[[events_observer]]
endpoint = "{}"
retry_count = 255
include_data_events = false
events_keys = ["*"]
"#,
                format!(
                    "hyperchain-api.{}:{}",
                    self.network_name, devnet_config.hyperchain_api_events_port
                ),
            ));
        }

        let mut hyperchain_conf_path = PathBuf::from(&devnet_config.working_dir);
        hyperchain_conf_path.push("conf/Hyperchain.toml");
        let mut file = File::create(hyperchain_conf_path)
            .map_err(|e| format!("unable to create Hyperchain.toml: {:?}", e))?;
        file.write_all(hyperchain_conf.as_bytes())
            .map_err(|e| format!("unable to write Hyperchain.toml: {:?}", e))?;

        let mut stacks_node_data_path = PathBuf::from(&devnet_config.working_dir);
        stacks_node_data_path.push("data");
        stacks_node_data_path.push(format!("{}", boot_index));
        let _ = fs::create_dir(stacks_node_data_path.clone());
        stacks_node_data_path.push("hyperchain");
        fs::create_dir(stacks_node_data_path)
            .map_err(|e| format!("to create working dir: {:?}", e))?;

        let mut exposed_ports = HashMap::new();
        exposed_ports.insert(
            format!("{}/tcp", devnet_config.hyperchain_node_rpc_port),
            HashMap::new(),
        );
        exposed_ports.insert(
            format!("{}/tcp", devnet_config.hyperchain_node_p2p_port),
            HashMap::new(),
        );
        exposed_ports.insert(
            format!("{}/tcp", devnet_config.hyperchain_events_ingestion_port),
            HashMap::new(),
        );

        let mut labels = HashMap::new();
        labels.insert("project".to_string(), self.network_name.to_string());
        labels.insert("reset".to_string(), "true".to_string());

        let mut binds = vec![format!(
            "{}/conf:/src/hyperchain-node/",
            devnet_config.working_dir
        )];

        if devnet_config.bind_containers_volumes {
            binds.push(format!(
                "{}/data/{}/hyperchain:/devnet/",
                devnet_config.working_dir, boot_index
            ))
        }

        let config = Config {
            labels: Some(labels),
            image: Some(devnet_config.hyperchain_node_image_url.clone()),
            domainname: Some(self.network_name.to_string()),
            tty: None,
            exposed_ports: Some(exposed_ports),
            entrypoint: Some(vec![
                "hyperchain-node".into(),
                "start".into(),
                "--config=/src/hyperchain-node/Hyperchain.toml".into(),
            ]),
            env: Some(vec![
                "STACKS_LOG_PP=1".to_string(),
                "STACKS_LOG_DEBUG=1".to_string(),
                // "BLOCKSTACK_USE_TEST_GENESIS_CHAINSTATE=1".to_string(),
            ]),
            host_config: Some(HostConfig {
                port_bindings: Some(port_bindings),
                binds: Some(binds),
                extra_hosts: Some(vec!["host.docker.internal:host-gateway".into()]),
                ..Default::default()
            }),
            ..Default::default()
        };

        Ok(config)
    }

    pub async fn prepare_hyperchain_node_container(
        &mut self,
        boot_index: u32,
    ) -> Result<(), String> {
        let (docker, devnet_config) = match (&self.docker_client, &self.network_config) {
            (Some(ref docker), Some(ref network_config)) => match network_config.devnet {
                Some(ref devnet_config) => (docker, devnet_config),
                _ => return Err("unable to get devnet configuration".into()),
            },
            _ => return Err("unable to get Docker client".into()),
        };

        let _info = docker
            .create_image(
                Some(CreateImageOptions {
                    from_image: devnet_config.hyperchain_node_image_url.clone(),
                    ..Default::default()
                }),
                None,
                None,
            )
            .try_collect::<Vec<_>>()
            .await
            .map_err(|e| format!("unable to create image: {}", e))?;

        let config = self.prepare_hyperchain_node_config(boot_index)?;

        let options = CreateContainerOptions {
            name: format!("hyperchain-node.{}", self.network_name),
        };

        let container = docker
            .create_container::<String, String>(Some(options), config)
            .await
            .map_err(|e| format!("unable to create container: {}", e))?
            .id;

        info!("Created container hyperchain-node: {}", container);
        self.hyperchain_node_container_id = Some(container.clone());

        Ok(())
    }

    pub async fn boot_hyperchain_node_container(&self) -> Result<(), String> {
        let container = match &self.hyperchain_node_container_id {
            Some(container) => container.clone(),
            _ => return Err(format!("unable to boot container")),
        };

        let docker = match &self.docker_client {
            Some(ref docker) => docker,
            _ => return Err("unable to get Docker client".into()),
        };

        docker
            .start_container::<String>(&container, None)
            .await
            .map_err(|e| format!("unable to start container - {}", e))?;

        let res = docker
            .connect_network(
                &self.network_name,
                ConnectNetworkOptions {
                    container,
                    ..Default::default()
                },
            )
            .await;

        if let Err(e) = res {
            let err = format!("Error connecting container: {}", e);
            println!("{}", err);
            return Err(err);
        }

        Ok(())
    }

    pub async fn prepare_stacks_api_container(&mut self) -> Result<(), String> {
        let (docker, _, devnet_config) = match (&self.docker_client, &self.network_config) {
            (Some(ref docker), Some(ref network_config)) => match network_config.devnet {
                Some(ref devnet_config) => (docker, network_config, devnet_config),
                _ => return Err("unable to get devnet configuration".into()),
            },
            _ => return Err("unable to get Docker client".into()),
        };

        let _info = docker
            .create_image(
                Some(CreateImageOptions {
                    from_image: devnet_config.stacks_api_image_url.clone(),
                    ..Default::default()
                }),
                None,
                None,
            )
            .try_collect::<Vec<_>>()
            .await
            .map_err(|_| "unable to create image".to_string())?;

        let mut port_bindings = HashMap::new();
        port_bindings.insert(
            format!("{}/tcp", devnet_config.stacks_api_port),
            Some(vec![PortBinding {
                host_ip: Some(String::from("0.0.0.0")),
                host_port: Some(format!("{}/tcp", devnet_config.stacks_api_port)),
            }]),
        );

        let mut exposed_ports = HashMap::new();
        exposed_ports.insert(
            format!("{}/tcp", devnet_config.stacks_api_port),
            HashMap::new(),
        );

        let mut labels = HashMap::new();
        labels.insert("project".to_string(), self.network_name.to_string());

        let config = Config {
            labels: Some(labels),
            image: Some(devnet_config.stacks_api_image_url.clone()),
            domainname: Some(self.network_name.to_string()),
            tty: None,
            exposed_ports: Some(exposed_ports),
            env: Some(vec![
                format!("STACKS_CORE_RPC_HOST=stacks-node.{}", self.network_name),
                format!("STACKS_BLOCKCHAIN_API_DB=pg"),
                format!("STACKS_CORE_RPC_PORT={}", devnet_config.stacks_node_rpc_port),
                format!(
                    "STACKS_BLOCKCHAIN_API_PORT={}",
                    devnet_config.stacks_api_port
                ),
                format!("STACKS_BLOCKCHAIN_API_HOST=0.0.0.0"),
                format!(
                    "STACKS_CORE_EVENT_PORT={}",
                    devnet_config.stacks_api_events_port
                ),
                format!("STACKS_CORE_EVENT_HOST=0.0.0.0"),
                format!("STACKS_API_ENABLE_FT_METADATA=1"),
                format!("PG_HOST=postgres.{}", self.network_name),
                format!("PG_PORT=5432"),
                format!("PG_USER={}", devnet_config.postgres_username),
                format!("PG_PASSWORD={}", devnet_config.postgres_password),
                format!("PG_DATABASE={}", devnet_config.stacks_api_postgres_database),
                format!("STACKS_CHAIN_ID=2147483648"),
                format!("V2_POX_MIN_AMOUNT_USTX=90000000260"),
                "NODE_ENV=development".to_string(),
            ]),
            host_config: Some(HostConfig {
                port_bindings: Some(port_bindings),
                extra_hosts: Some(vec!["host.docker.internal:host-gateway".into()]),
                ..Default::default()
            }),
            ..Default::default()
        };

        let options = CreateContainerOptions {
            name: format!("stacks-api.{}", self.network_name),
        };

        let container = docker
            .create_container::<String, String>(Some(options), config)
            .await
            .map_err(|e| format!("unable to create container: {}", e))?
            .id;

        info!("Created container stacks-api: {}", container);
        self.stacks_api_container_id = Some(container);

        Ok(())
    }

    pub async fn boot_stacks_api_container(&self) -> Result<(), String> {
        let container = match &self.stacks_api_container_id {
            Some(container) => container.clone(),
            _ => return Err(format!("unable to boot container")),
        };

        let docker = match &self.docker_client {
            Some(ref docker) => docker,
            _ => return Err("unable to get Docker client".into()),
        };

        docker
            .start_container::<String>(&container, None)
            .await
            .map_err(|e| formatted_docker_error("unable to start stacks-api container", e))?;

        let res = docker
            .connect_network(
                &self.network_name,
                ConnectNetworkOptions {
                    container,
                    ..Default::default()
                },
            )
            .await;

        if let Err(e) = res {
            let err = format!("Error connecting container: {}", e);
            println!("{}", err);
            return Err(err);
        }

        Ok(())
    }

    pub async fn prepare_hyperchain_api_container(&mut self) -> Result<(), String> {
        let (docker, _, devnet_config) = match (&self.docker_client, &self.network_config) {
            (Some(ref docker), Some(ref network_config)) => match network_config.devnet {
                Some(ref devnet_config) => (docker, network_config, devnet_config),
                _ => return Err("unable to get devnet configuration".into()),
            },
            _ => return Err("unable to get Docker client".into()),
        };

        let _info = docker
            .create_image(
                Some(CreateImageOptions {
                    from_image: devnet_config.hyperchain_api_image_url.clone(),
                    ..Default::default()
                }),
                None,
                None,
            )
            .try_collect::<Vec<_>>()
            .await
            .map_err(|_| "unable to create image".to_string())?;

        let mut port_bindings = HashMap::new();
        port_bindings.insert(
            format!("{}/tcp", devnet_config.hyperchain_api_port),
            Some(vec![PortBinding {
                host_ip: Some(String::from("0.0.0.0")),
                host_port: Some(format!("{}/tcp", devnet_config.hyperchain_api_port)),
            }]),
        );

        let mut exposed_ports = HashMap::new();
        exposed_ports.insert(
            format!("{}/tcp", devnet_config.hyperchain_api_port),
            HashMap::new(),
        );

        let mut labels = HashMap::new();
        labels.insert("project".to_string(), self.network_name.to_string());

        let config = Config {
            labels: Some(labels),
            image: Some(devnet_config.hyperchain_api_image_url.clone()),
            domainname: Some(self.network_name.to_string()),
            tty: None,
            exposed_ports: Some(exposed_ports),
            env: Some(vec![
                format!("STACKS_CORE_RPC_HOST=hyperchain-node.{}", self.network_name),
                format!("STACKS_BLOCKCHAIN_API_DB=pg"),
                format!("STACKS_CORE_RPC_PORT={}", devnet_config.hyperchain_node_rpc_port),
                format!(
                    "STACKS_BLOCKCHAIN_API_PORT={}",
                    devnet_config.hyperchain_api_port
                ),
                format!("STACKS_BLOCKCHAIN_API_HOST=0.0.0.0"),
                format!(
                    "STACKS_CORE_EVENT_PORT={}",
                    devnet_config.hyperchain_api_events_port
                ),
                format!("STACKS_CORE_EVENT_HOST=0.0.0.0"),
                format!("STACKS_API_ENABLE_FT_METADATA=1"),
                format!("PG_HOST=postgres.{}", self.network_name),
                format!("PG_PORT=5432"),
                format!("PG_USER={}", devnet_config.postgres_username),
                format!("PG_PASSWORD={}", devnet_config.postgres_password),
                format!(
                    "PG_DATABASE={}",
                    devnet_config.hyperchain_api_postgres_database
                ),
                format!("STACKS_CHAIN_ID=2147483648"),
                format!("V2_POX_MIN_AMOUNT_USTX=90000000260"),
                "NODE_ENV=development".to_string(),
            ]),
            host_config: Some(HostConfig {
                port_bindings: Some(port_bindings),
                extra_hosts: Some(vec!["host.docker.internal:host-gateway".into()]),
                ..Default::default()
            }),
            ..Default::default()
        };

        let options = CreateContainerOptions {
            name: format!("hyperchain-api.{}", self.network_name),
        };

        let container = docker
            .create_container::<String, String>(Some(options), config)
            .await
            .map_err(|e| format!("unable to create container: {}", e))?
            .id;

        info!("Created container hyperchain-api: {}", container);
        self.hyperchain_api_container_id = Some(container);

        Ok(())
    }

    pub async fn boot_hyperchain_api_container(&self) -> Result<(), String> {
        // Before booting the hyperchain-api, we need to create an additional DB in the postgres container.
        let (docker, _, devnet_config) = match (&self.docker_client, &self.network_config) {
            (Some(ref docker), Some(ref network_config)) => match network_config.devnet {
                Some(ref devnet_config) => (docker, network_config, devnet_config),
                _ => return Err("unable to get devnet configuration".into()),
            },
            _ => return Err("unable to get Docker client".into()),
        };

        let postgres_container = match &self.postgres_container_id {
            Some(container) => container.clone(),
            _ => return Err(format!("unable to boot container")),
        };

        let psql_command = format!(
            "CREATE DATABASE {};",
            devnet_config.hyperchain_api_postgres_database
        );

        let config = CreateExecOptions {
            cmd: Some(vec!["psql", "-U", "postgres", "-c", psql_command.as_str()]),
            attach_stdout: Some(false),
            attach_stderr: Some(false),
            ..Default::default()
        };

        let exec = docker
            .create_exec::<&str>(&postgres_container, config)
            .await
            .map_err(|e| formatted_docker_error("unable to create exec command", e))?;

        let _ = docker
            .start_exec(&exec.id, None)
            .await
            .map_err(|e| formatted_docker_error("unable to start exec command", e))?;

        let container = match &self.hyperchain_api_container_id {
            Some(container) => container.clone(),
            _ => return Err(format!("unable to boot container")),
        };

        let docker = match &self.docker_client {
            Some(ref docker) => docker,
            _ => return Err("unable to get Docker client".into()),
        };

        docker
            .start_container::<String>(&container, None)
            .await
            .map_err(|e| formatted_docker_error("unable to start stacks-api container", e))?;

        let res = docker
            .connect_network(
                &self.network_name,
                ConnectNetworkOptions {
                    container,
                    ..Default::default()
                },
            )
            .await;

        if let Err(e) = res {
            let err = format!("Error connecting container: {}", e);
            println!("{}", err);
            return Err(err);
        }

        Ok(())
    }

    pub async fn prepare_postgres_container(&mut self) -> Result<(), String> {
        let (docker, _, devnet_config) = match (&self.docker_client, &self.network_config) {
            (Some(ref docker), Some(ref network_config)) => match network_config.devnet {
                Some(ref devnet_config) => (docker, network_config, devnet_config),
                _ => return Err("unable to get devnet configuration".into()),
            },
            _ => return Err("unable to get Docker client".into()),
        };

        let _info = docker
            .create_image(
                Some(CreateImageOptions {
                    from_image: devnet_config.postgres_image_url.clone(),
                    ..Default::default()
                }),
                None,
                None,
            )
            .try_collect::<Vec<_>>()
            .await
            .map_err(|_| "unable to create image".to_string())?;

        let mut port_bindings = HashMap::new();
        port_bindings.insert(
            format!("5432/tcp"),
            Some(vec![PortBinding {
                host_ip: Some(String::from("0.0.0.0")),
                host_port: Some(format!("{}/tcp", devnet_config.postgres_port)),
            }]),
        );

        let exposed_ports = HashMap::new();

        let mut labels = HashMap::new();
        labels.insert("project".to_string(), self.network_name.to_string());

        let config = Config {
            labels: Some(labels),
            image: Some(devnet_config.postgres_image_url.clone()),
            domainname: Some(self.network_name.to_string()),
            tty: None,
            exposed_ports: Some(exposed_ports),
            env: Some(vec![
                format!("POSTGRES_PASSWORD={}", devnet_config.postgres_password),
                format!("POSTGRES_DB={}", devnet_config.stacks_api_postgres_database),
            ]),
            host_config: Some(HostConfig {
                port_bindings: Some(port_bindings),
                extra_hosts: Some(vec!["host.docker.internal:host-gateway".into()]),
                ..Default::default()
            }),
            ..Default::default()
        };

        let options = CreateContainerOptions {
            name: format!("postgres.{}", self.network_name),
        };

        let container = docker
            .create_container::<String, String>(Some(options), config)
            .await
            .map_err(|e| format!("unable to create container: {}", e))?
            .id;

        info!("Created container postgres: {}", container);
        self.postgres_container_id = Some(container);

        Ok(())
    }

    pub async fn boot_postgres_container(&self) -> Result<(), String> {
        let container = match &self.postgres_container_id {
            Some(container) => container.clone(),
            _ => return Err(format!("unable to boot container")),
        };

        let docker = match &self.docker_client {
            Some(ref docker) => docker,
            _ => return Err("unable to get Docker client".into()),
        };

        docker
            .start_container::<String>(&container, None)
            .await
            .map_err(|e| formatted_docker_error("unable to start postgres container", e))?;

        let res = docker
            .connect_network(
                &self.network_name,
                ConnectNetworkOptions {
                    container,
                    ..Default::default()
                },
            )
            .await;

        if let Err(e) = res {
            let err = format!("Error connecting container: {}", e);
            println!("{}", err);
            return Err(err);
        }

        Ok(())
    }

    pub async fn prepare_stacks_explorer_container(&mut self) -> Result<(), String> {
        let (docker, _, devnet_config) = match (&self.docker_client, &self.network_config) {
            (Some(ref docker), Some(ref network_config)) => match network_config.devnet {
                Some(ref devnet_config) => (docker, network_config, devnet_config),
                _ => return Err("unable to get devnet configuration".into()),
            },
            _ => return Err("unable to get Docker client".into()),
        };

        let _info = docker
            .create_image(
                Some(CreateImageOptions {
                    from_image: devnet_config.stacks_explorer_image_url.clone(),
                    ..Default::default()
                }),
                None,
                None,
            )
            .try_collect::<Vec<_>>()
            .await
            .map_err(|e| format!("unable to create image: {}", e))?;
        let explorer_guest_port = 3000;
        let mut port_bindings = HashMap::new();
        port_bindings.insert(
            format!("{}/tcp", explorer_guest_port),
            Some(vec![PortBinding {
                host_ip: Some(String::from("0.0.0.0")),
                host_port: Some(format!("{}/tcp", devnet_config.stacks_explorer_port)),
            }]),
        );

        let mut exposed_ports = HashMap::new();
        exposed_ports.insert(format!("{}/tcp", explorer_guest_port), HashMap::new());

        let mut labels = HashMap::new();
        labels.insert("project".to_string(), self.network_name.to_string());

        let config = Config {
            labels: Some(labels),
            image: Some(devnet_config.stacks_explorer_image_url.clone()),
            domainname: Some(self.network_name.to_string()),
            tty: None,
            exposed_ports: Some(exposed_ports),
            env: Some(vec![
                format!(
                    "NEXT_PUBLIC_REGTEST_API_SERVER=http://localhost:{}",
                    devnet_config.stacks_api_port
                ),
                format!(
                    "NEXT_PUBLIC_TESTNET_API_SERVER=http://localhost:{}",
                    devnet_config.stacks_api_port
                ),
                format!(
                    "NEXT_PUBLIC_MAINNET_API_SERVER=http://localhost:{}",
                    devnet_config.stacks_api_port
                ),
                format!("NEXT_PUBLIC_DEFAULT_POLLING_INTERVAL={}", 5000),
                "NODE_ENV=development".to_string(),
            ]),
            host_config: Some(HostConfig {
                port_bindings: Some(port_bindings),
                extra_hosts: Some(vec!["host.docker.internal:host-gateway".into()]),
                ..Default::default()
            }),
            ..Default::default()
        };

        let options = CreateContainerOptions {
            name: format!("stacks-explorer.{}", self.network_name),
        };

        let container = docker
            .create_container::<String, String>(Some(options), config)
            .await
            .map_err(|e| format!("unable to create container: {}", e))?
            .id;

        info!("Created container stacks-explorer: {}", container);
        self.stacks_explorer_container_id = Some(container);

        Ok(())
    }

    pub async fn boot_stacks_explorer_container(&self) -> Result<(), String> {
        let container = match &self.stacks_explorer_container_id {
            Some(container) => container.clone(),
            _ => return Err(format!("unable to boot container")),
        };

        let docker = match &self.docker_client {
            Some(ref docker) => docker,
            _ => return Err("unable to get Docker client".into()),
        };

        docker
            .start_container::<String>(&container, None)
            .await
            .map_err(|e| format!("unable to create container: {}", e))?;

        let res = docker
            .connect_network(
                &self.network_name,
                ConnectNetworkOptions {
                    container,
                    ..Default::default()
                },
            )
            .await;

        if let Err(e) = res {
            let err = format!("Error connecting container: {}", e);
            println!("{}", err);
            return Err(err);
        }

        Ok(())
    }

    pub async fn prepare_bitcoin_explorer_container(&mut self) -> Result<(), String> {
        let (docker, _, devnet_config) = match (&self.docker_client, &self.network_config) {
            (Some(ref docker), Some(ref network_config)) => match network_config.devnet {
                Some(ref devnet_config) => (docker, network_config, devnet_config),
                _ => return Err("unable to get devnet configuration".into()),
            },
            _ => return Err("unable to get Docker client".into()),
        };

        let _info = docker
            .create_image(
                Some(CreateImageOptions {
                    from_image: devnet_config.bitcoin_explorer_image_url.clone(),
                    ..Default::default()
                }),
                None,
                None,
            )
            .try_collect::<Vec<_>>()
            .await
            .map_err(|e| format!("unable to create image: {}", e))?;

        let mut port_bindings = HashMap::new();
        port_bindings.insert(
            format!("{}/tcp", devnet_config.bitcoin_explorer_port),
            Some(vec![PortBinding {
                host_ip: Some(String::from("0.0.0.0")),
                host_port: Some(format!("{}/tcp", devnet_config.bitcoin_explorer_port)),
            }]),
        );

        let mut exposed_ports = HashMap::new();
        exposed_ports.insert(
            format!("{}/tcp", devnet_config.bitcoin_explorer_port),
            HashMap::new(),
        );

        let mut labels = HashMap::new();
        labels.insert("project".to_string(), self.network_name.to_string());

        let config = Config {
            labels: Some(labels),
            image: Some(devnet_config.bitcoin_explorer_image_url.clone()),
            domainname: Some(self.network_name.to_string()),
            tty: None,
            exposed_ports: Some(exposed_ports),
            env: Some(vec![
                format!("BTCEXP_HOST=0.0.0.0",),
                format!("BTCEXP_PORT={}", devnet_config.bitcoin_explorer_port),
                format!("BTCEXP_BITCOIND_HOST=host.docker.internal",),
                format!(
                    "BTCEXP_BITCOIND_PORT={}",
                    devnet_config.bitcoin_node_rpc_port
                ),
                format!(
                    "BTCEXP_BITCOIND_USER={}",
                    devnet_config.bitcoin_node_username
                ),
                format!(
                    "BTCEXP_BITCOIND_PASS={}",
                    devnet_config.bitcoin_node_password
                ),
                format!(
                    "BTCEXP_BASIC_AUTH_PASSWORD={}",
                    devnet_config.bitcoin_node_password
                ),
                format!("BTCEXP_RPC_ALLOWALL=true",),
            ]),
            host_config: Some(HostConfig {
                port_bindings: Some(port_bindings),
                extra_hosts: Some(vec!["host.docker.internal:host-gateway".into()]),
                ..Default::default()
            }),
            ..Default::default()
        };

        let options = CreateContainerOptions {
            name: format!("bitcoin-explorer.{}", self.network_name),
        };

        let container = docker
            .create_container::<String, String>(Some(options), config)
            .await
            .map_err(|e| format!("unable to create container: {}", e))?
            .id;

        info!("Created container bitcoin-explorer: {}", container);
        self.bitcoin_explorer_container_id = Some(container);

        Ok(())
    }

    pub async fn boot_bitcoin_explorer_container(&self) -> Result<(), String> {
        let container = match &self.bitcoin_explorer_container_id {
            Some(container) => container.clone(),
            _ => return Err(format!("unable to boot container")),
        };

        let docker = match &self.docker_client {
            Some(ref docker) => docker,
            _ => return Err("unable to get Docker client".into()),
        };

        docker
            .start_container::<String>(&container, None)
            .await
            .map_err(|e| format!("unable to create container: {}", e))?;

        let res = docker
            .connect_network(
                &self.network_name,
                ConnectNetworkOptions {
                    container,
                    ..Default::default()
                },
            )
            .await;

        if let Err(e) = res {
            let err = format!("Error connecting container: {}", e);
            println!("{}", err);
            return Err(err);
        }

        Ok(())
    }

    pub async fn stop_containers(&self) -> Result<(), String> {
        let containers_ids = match (
            &self.stacks_node_container_id,
            &self.stacks_api_container_id,
            &self.stacks_explorer_container_id,
            &self.bitcoin_node_container_id,
            &self.bitcoin_explorer_container_id,
            &self.postgres_container_id,
        ) {
            (Some(c1), Some(c2), Some(c3), Some(c4), Some(c5), Some(c6)) => {
                (c1, c2, c3, c4, c5, c6)
            }
            _ => return Err(format!("unable to boot container")),
        };
        let (
            stacks_node_c_id,
            stacks_api_c_id,
            stacks_explorer_c_id,
            bitcoin_node_c_id,
            bitcoin_explorer_c_id,
            postgres_c_id,
        ) = containers_ids;

        let docker = match &self.docker_client {
            Some(ref docker) => docker,
            _ => return Err("unable to get Docker client".into()),
        };

        let options = KillContainerOptions { signal: "SIGKILL" };

        let _ = docker
            .kill_container(stacks_node_c_id, Some(options.clone()))
            .await;

        let _ = docker
            .kill_container(stacks_api_c_id, Some(options.clone()))
            .await;

        let _ = docker
            .kill_container(stacks_explorer_c_id, Some(options.clone()))
            .await;

        let _ = docker
            .kill_container(bitcoin_node_c_id, Some(options.clone()))
            .await;

        let _ = docker
            .kill_container(bitcoin_explorer_c_id, Some(options.clone()))
            .await;

        let _ = docker
            .kill_container(postgres_c_id, Some(options.clone()))
            .await;

        let _ = docker
            .wait_container(stacks_node_c_id, None::<WaitContainerOptions<String>>)
            .try_collect::<Vec<_>>()
            .await;

        Ok(())
    }

    pub async fn start_containers(&self, boot_index: u32) -> Result<(String, String), String> {
        let containers_ids = match (
            &self.stacks_api_container_id,
            &self.stacks_explorer_container_id,
            &self.bitcoin_explorer_container_id,
            &self.postgres_container_id,
        ) {
            (Some(c1), Some(c2), Some(c3), Some(c4)) => (c1, c2, c3, c4),
            _ => return Err(format!("unable to boot container")),
        };
        let (stacks_api_c_id, stacks_explorer_c_id, bitcoin_explorer_c_id, postgres_c_id) =
            containers_ids;

        let docker = match &self.docker_client {
            Some(ref docker) => docker,
            _ => return Err("unable to get Docker client".into()),
        };

        // TODO(lgalabru): should we spawn
        // docker run -d -p 5000:5000 --name registry registry:2.7
        // ?

        // Prune
        let mut filters = HashMap::new();
        filters.insert(
            "label".to_string(),
            vec![
                format!("project={}", self.network_name),
                "reset=true".to_string(),
            ],
        );
        let _ = docker
            .prune_containers(Some(PruneContainersOptions { filters }))
            .await;

        let bitcoin_node_config = self.prepare_bitcoin_node_config(boot_index)?;
        let options = CreateContainerOptions {
            name: format!("bitcoin-node.{}", self.network_name),
        };
        let bitcoin_node_c_id = docker
            .create_container::<String, String>(Some(options), bitcoin_node_config)
            .await
            .map_err(|e| format!("unable to create container: {}", e))?
            .id;

        let stacks_node_config = self.prepare_stacks_node_config(boot_index)?;
        let options = CreateContainerOptions {
            name: format!("stacks-node.{}", self.network_name),
        };
        let stacks_node_c_id = docker
            .create_container::<String, String>(Some(options), stacks_node_config)
            .await
            .map_err(|e| format!("unable to create container: {}", e))?
            .id;

        // Start all the containers
        let _ = docker
            .start_container::<String>(&bitcoin_node_c_id, None)
            .await;
        let _ = docker
            .connect_network(
                &self.network_name,
                ConnectNetworkOptions {
                    container: bitcoin_node_c_id.clone(),
                    ..Default::default()
                },
            )
            .await;

        let _ = docker
            .start_container::<String>(bitcoin_explorer_c_id, None)
            .await;

        let _ = docker.start_container::<String>(postgres_c_id, None).await;

        let _ = docker
            .start_container::<String>(stacks_api_c_id, None)
            .await;

        let _ = docker
            .start_container::<String>(stacks_explorer_c_id, None)
            .await;

        let _ = docker
            .start_container::<String>(&stacks_node_c_id, None)
            .await;
        let _ = docker
            .connect_network(
                &self.network_name,
                ConnectNetworkOptions {
                    container: stacks_node_c_id.clone(),
                    ..Default::default()
                },
            )
            .await;

        Ok((bitcoin_node_c_id, stacks_node_c_id))
    }

    pub async fn kill(&self) {
        let (docker, devnet_config) = match (&self.docker_client, &self.network_config) {
            (Some(ref docker), Some(ref network_config)) => match network_config.devnet {
                Some(ref devnet_config) => (docker, devnet_config),
                _ => return,
            },
            _ => return,
        };
        let options = Some(KillContainerOptions { signal: "SIGKILL" });

        // Terminate containers
        if let Some(ref bitcoin_explorer_container_id) = self.bitcoin_explorer_container_id {
            let _ = docker
                .kill_container(bitcoin_explorer_container_id, options.clone())
                .await;
            println!("Terminating bitcoin-explorer...");
            let _ = docker.remove_container(bitcoin_explorer_container_id, None);
        }

        if let Some(ref stacks_explorer_container_id) = self.stacks_explorer_container_id {
            let _ = docker
                .kill_container(stacks_explorer_container_id, options.clone())
                .await;
            println!("Terminating stacks-explorer...");
            let _ = docker.remove_container(stacks_explorer_container_id, None);
        }

        if let Some(ref bitcoin_node_container_id) = self.bitcoin_node_container_id {
            let _ = docker
                .kill_container(bitcoin_node_container_id, options.clone())
                .await;
            println!("Terminating bitcoin-node...");
            let _ = docker.remove_container(bitcoin_node_container_id, None);
        }

        if let Some(ref stacks_api_container_id) = self.stacks_api_container_id {
            let _ = docker
                .kill_container(stacks_api_container_id, options.clone())
                .await;
            println!("Terminating stacks-api...");
            let _ = docker.remove_container(stacks_api_container_id, None);
        }

        if let Some(ref postgres_container_id) = self.postgres_container_id {
            let _ = docker
                .kill_container(postgres_container_id, options.clone())
                .await;
            println!("Terminating postgres...");
            let _ = docker.remove_container(postgres_container_id, None);
        }

        if let Some(ref stacks_node_container_id) = self.stacks_node_container_id {
            let _ = docker
                .kill_container(stacks_node_container_id, options.clone())
                .await;
            println!("Terminating stacks-node...");
            let _ = docker.remove_container(stacks_node_container_id, None);
        }

        if let Some(ref hyperchain_node_container_id) = self.hyperchain_node_container_id {
            let _ = docker
                .kill_container(hyperchain_node_container_id, options.clone())
                .await;
            println!("Terminating hyperchain-node...");
            let _ = docker.remove_container(hyperchain_node_container_id, None);
        }

        if let Some(ref hyperchain_api_container_id) = self.hyperchain_api_container_id {
            let _ = docker
                .kill_container(hyperchain_api_container_id, options)
                .await;
            println!("Terminating hyperchain-api...");
            let _ = docker.remove_container(hyperchain_api_container_id, None);
        }

        // Prune network
        println!("Pruning network and containers...");
        self.prune().await;
        if let Some(ref tx) = self.termination_success_tx {
            let _ = tx.send(true);
        }

        println!(
            "Artifacts (logs, conf, chainstates) available here: {}",
            devnet_config.working_dir
        );
        println!("✌️");
        if self.can_exit {
            std::process::exit(0);
        }
    }

    pub async fn prune(&self) {
        let docker = match &self.docker_client {
            Some(ref docker) => docker,
            _ => return,
        };

        let mut filters = HashMap::new();
        filters.insert(
            "label".to_string(),
            vec![format!("project={}", self.network_name)],
        );
        let _ = docker
            .prune_containers(Some(PruneContainersOptions {
                filters: filters.clone(),
            }))
            .await;

        let _ = docker
            .prune_networks(Some(PruneNetworksOptions { filters }))
            .await;
    }

    pub fn initialize_bitcoin_node(
        &self,
        devnet_event_tx: &Sender<DevnetEvent>,
    ) -> Result<(), String> {
        use bitcoincore_rpc::bitcoin::Address;
        use bitcoincore_rpc::{Auth, Client, RpcApi};
        use std::str::FromStr;

        let (devnet_config, accounts) = match &self.network_config {
            Some(ref network_config) => match network_config.devnet {
                Some(ref devnet_config) => (devnet_config, &network_config.accounts),
                _ => return Err(format!("unable to initialize bitcoin node")),
            },
            _ => return Err(format!("unable to initialize bitcoin node")),
        };

        let rpc = Client::new(
            &format!("http://localhost:{}", devnet_config.bitcoin_node_rpc_port),
            Auth::UserPass(
                devnet_config.bitcoin_node_username.to_string(),
                devnet_config.bitcoin_node_password.to_string(),
            ),
        )
        .map_err(|e| format!("unable to create RPC client: {:?}", e))?;

        let _ = devnet_event_tx.send(DevnetEvent::info(format!("Configuring bitcoin-node",)));

        loop {
            match rpc.get_network_info() {
                Ok(_r) => break,
                Err(_e) => {}
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
            let _ = devnet_event_tx.send(DevnetEvent::info(format!("Waiting for bitcoin-node",)));
        }

        let miner_address = Address::from_str(&devnet_config.miner_btc_address)
            .map_err(|e| format!("unable to create miner address: {:?}", e))?;

        let faucet_address = Address::from_str(&devnet_config.faucet_btc_address)
            .map_err(|e| format!("unable to create faucet address: {:?}", e))?;

        let _ = rpc.generate_to_address(3, &miner_address);
        let _ = rpc.generate_to_address(97, &faucet_address);
        let _ = rpc.generate_to_address(1, &miner_address);
        let _ = rpc.create_wallet("", None, None, None, None);
        let _ = rpc.import_address(&miner_address, None, None);
        let _ = rpc.import_address(&faucet_address, None, None);
        // Index devnet's wallets by default
        for (_, account) in accounts.iter() {
            let address = Address::from_str(&account.btc_address)
                .map_err(|e| format!("unable to create address: {:?}", e))?;
            let _ = rpc.import_address(&address, None, None);
        }
        Ok(())
    }
}

fn formatted_docker_error(message: &str, error: DockerError) -> String {
    let error = match &error {
        DockerError::DockerResponseServerError {
            status_code: _c,
            message: m,
        } => m.to_string(),
        _ => format!("{:?}", error),
    };
    format!("{}: {}", message, error)
}

fn process_exit() {
    let _ = disable_raw_mode();
    std::process::exit(1);
}

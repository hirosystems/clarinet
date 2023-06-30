use bollard::container::{
    Config, CreateContainerOptions, KillContainerOptions, ListContainersOptions,
    PruneContainersOptions, WaitContainerOptions,
};
use bollard::errors::Error as DockerError;
use bollard::exec::CreateExecOptions;
use bollard::image::CreateImageOptions;
use bollard::models::{HostConfig, PortBinding};
use bollard::network::{CreateNetworkOptions, PruneNetworksOptions};
use bollard::service::Ipam;
use bollard::Docker;
use chainhook_sdk::utils::Context;
use clarinet_files::chainhook_types::StacksNetwork;
use clarinet_files::{DevnetConfigFile, NetworkManifest, ProjectManifest};
use futures::stream::TryStreamExt;
use hiro_system_kit::slog;
use reqwest::RequestBuilder;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;

use crate::event::{DevnetEvent, ServiceStatusData, Status};

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
    subnet_node_container_id: Option<String>,
    subnet_api_container_id: Option<String>,
    docker_client: Option<Docker>,
    services_map_hosts: Option<ServicesMapHosts>,
}

// pub enum DevnetServices {
//     BitcoinNode,
//     StacksNode,
//     StacksApi,
//     StacksExplorer,
//     BitcoinExplorer,
//     SubnetNode,
//     SubnetApi,
// }

#[derive(Clone, Debug)]
pub struct ServicesMapHosts {
    pub bitcoin_node_host: String,
    pub stacks_node_host: String,
    pub stacks_api_host: String,
    pub postgres_host: String,
    pub stacks_explorer_host: String,
    pub bitcoin_explorer_host: String,
    pub subnet_node_host: String,
    pub subnet_api_host: String,
}

impl DevnetOrchestrator {
    pub fn new(
        manifest: ProjectManifest,
        devnet_override: Option<DevnetConfigFile>,
        should_use_docker: bool,
    ) -> Result<DevnetOrchestrator, String> {
        let mut network_config = NetworkManifest::from_project_manifest_location(
            &manifest.location,
            &StacksNetwork::Devnet.get_networks(),
            Some(&manifest.project.cache_location),
            devnet_override,
        )?;

        if let Some(ref mut devnet) = network_config.devnet {
            let working_dir = PathBuf::from(&devnet.working_dir);
            let devnet_path = if working_dir.is_absolute() {
                working_dir
            } else {
                let mut cwd = std::env::current_dir()
                    .map_err(|e| format!("unable to retrieve current dir ({})", e.to_string()))?;
                cwd.push(&working_dir);
                let _ = fs::create_dir(&cwd);
                cwd.canonicalize().map_err(|e| {
                    format!(
                        "unable to canonicalize working_dir {} ({})",
                        working_dir.display(),
                        e.to_string()
                    )
                })?
            };
            devnet.working_dir = format!("{}", devnet_path.display());
        }

        let name = manifest.project.name.to_string();
        let mut network_name = name.clone();
        if let Some(ref devnet) = network_config.devnet {
            if let Some(ref network_id) = devnet.network_id {
                network_name.push_str(&format!(".{}", network_id));
            }
            network_name.push_str(&format!(".{}", devnet.name));
        } else {
            network_name.push_str(".net");
        }

        let docker_client = match should_use_docker {
            true => match network_config.devnet {
                Some(ref devnet) => {
                    let client = Docker::connect_with_socket(
                        &devnet.docker_host,
                        120,
                        bollard::API_DEFAULT_VERSION,
                    )
                    .or_else(|_| Docker::connect_with_socket_defaults())
                    .or_else(|_| {
                        let mut user_space_docker_socket =
                            dirs::home_dir().expect("unable to retrieve homedir");
                        user_space_docker_socket.push(".docker");
                        user_space_docker_socket.push("run");
                        user_space_docker_socket.push("docker.sock");
                        Docker::connect_with_socket(
                            &user_space_docker_socket.to_str().unwrap(),
                            120,
                            bollard::API_DEFAULT_VERSION,
                        )
                    })
                    .map_err(|e| format!("unable to connect to docker: {:?}", e))?;
                    Some(client)
                }
                None => unreachable!(),
            },
            false => None,
        };

        Ok(DevnetOrchestrator {
            name,
            network_name,
            manifest,
            network_config: Some(network_config),
            docker_client: docker_client,
            can_exit: true,
            termination_success_tx: None,
            stacks_node_container_id: None,
            stacks_api_container_id: None,
            stacks_explorer_container_id: None,
            bitcoin_node_container_id: None,
            bitcoin_explorer_container_id: None,
            postgres_container_id: None,
            subnet_node_container_id: None,
            subnet_api_container_id: None,
            services_map_hosts: None,
        })
    }

    pub fn prepare_service_network(&mut self, namespace: &str) -> Result<ServicesMapHosts, String> {
        let services_map_hosts = ServicesMapHosts {
            bitcoin_node_host: format!(
                "bitcoind-chain-coordinator-service.{namespace}.svc.cluster.local:18443"
            ),
            stacks_node_host: format!("stacks-node-service.{namespace}.svc.cluster.local:20443"),
            postgres_host: format!("stacks-api-service.{namespace}.svc.cluster.local:5432"),
            stacks_api_host: format!("stacks-api-service.{namespace}.svc.cluster.local:3999"),
            stacks_explorer_host: "localhost".into(), // todo (micaiah)
            bitcoin_explorer_host: "localhost".into(), // todo (micaiah)
            subnet_node_host: "localhost".into(),     // todo (micaiah)
            subnet_api_host: "localhost".into(),      // todo (micaiah)
        };

        self.services_map_hosts = Some(services_map_hosts.clone());

        Ok(services_map_hosts)
    }
    pub async fn prepare_local_network(&mut self) -> Result<ServicesMapHosts, String> {
        let (docker, devnet_config) = match (&self.docker_client, &self.network_config) {
            (Some(ref docker), Some(ref network_config)) => match network_config.devnet {
                Some(ref devnet_config) => (docker, devnet_config),
                _ => return Err(format!("unable to get devnet config")),
            },
            _ => return Err(format!("unable to get devnet config")),
        };

        // First, let's make sure that we pruned staled resources correctly
        // self.clean_previous_session().await?;

        let mut labels = HashMap::new();
        labels.insert("project", self.network_name.as_str());

        let mut options = HashMap::new();
        options.insert("enable_ip_masquerade".into(), "true".into());
        options.insert("enable_icc".into(), "true".into());
        options.insert("host_binding_ipv4".into(), "0.0.0.0".into());
        options.insert("com.docker.network.bridge.enable_icc".into(), "true".into());
        options.insert(
            "com.docker.network.bridge.enable_ip_masquerade".into(),
            "true".into(),
        );
        options.insert(
            "com.docker.network.bridge.host_binding_ipv4".into(),
            "0.0.0.0".into(),
        );

        let network_id = docker
            .create_network::<&str>(CreateNetworkOptions {
                name: &self.network_name,
                driver: "bridge",
                ipam: Ipam {
                    ..Default::default()
                },
                labels,
                options: options,
                ..Default::default()
            })
            .await
            .map_err(|e| {
                format!(
                    "clarinet was unable to create network. Is docker running locally? (error: {})",
                    e.to_string()
                )
            })?
            .id
            .ok_or("unable to retrieve network_id")?;

        let res = docker
            .inspect_network::<&str>(&network_id, None)
            .await
            .map_err(|e| format!("unable to retrieve network: {}", e.to_string()))?;

        let gateway = res
            .ipam
            .as_ref()
            .and_then(|ipam| ipam.config.as_ref())
            .and_then(|config| config.first())
            .and_then(|map| map.get("Gateway"))
            .ok_or("unable to retrieve gateway")?;

        let services_map_hosts = if devnet_config.use_docker_gateway_routing {
            ServicesMapHosts {
                bitcoin_node_host: format!("{}:{}", gateway, devnet_config.bitcoin_node_rpc_port),
                stacks_node_host: format!("{}:{}", gateway, devnet_config.stacks_node_rpc_port),
                postgres_host: format!("{}:{}", gateway, devnet_config.postgres_port),
                stacks_api_host: format!("{}:{}", gateway, devnet_config.stacks_api_port),
                stacks_explorer_host: format!("{}:{}", gateway, devnet_config.stacks_explorer_port),
                bitcoin_explorer_host: format!(
                    "{}:{}",
                    gateway, devnet_config.bitcoin_explorer_port
                ),
                subnet_node_host: format!("{}:{}", gateway, devnet_config.subnet_node_rpc_port),
                subnet_api_host: format!("{}:{}", gateway, devnet_config.subnet_api_port),
            }
        } else {
            ServicesMapHosts {
                bitcoin_node_host: format!("localhost:{}", devnet_config.bitcoin_node_rpc_port),
                stacks_node_host: format!("localhost:{}", devnet_config.stacks_node_rpc_port),
                postgres_host: format!("localhost:{}", devnet_config.postgres_port),
                stacks_api_host: format!("localhost:{}", devnet_config.stacks_api_port),
                stacks_explorer_host: format!("localhost:{}", devnet_config.stacks_explorer_port),
                bitcoin_explorer_host: format!("localhost:{}", devnet_config.bitcoin_explorer_port),
                subnet_node_host: format!("localhost:{}", devnet_config.subnet_node_rpc_port),
                subnet_api_host: format!("localhost:{}", devnet_config.subnet_api_port),
            }
        };

        self.services_map_hosts = Some(services_map_hosts.clone());

        Ok(services_map_hosts)
    }

    pub async fn start(
        &mut self,
        event_tx: Sender<DevnetEvent>,
        terminator_rx: Receiver<bool>,
        ctx: &Context,
    ) -> Result<(), String> {
        let (_docker, devnet_config) = match (&self.docker_client, &self.network_config) {
            (Some(ref docker), Some(ref network_config)) => match network_config.devnet {
                Some(ref devnet_config) => (docker, devnet_config),
                _ => return Err(format!("unable to get devnet config")),
            },
            _ => return Err(format!("unable to get devnet config")),
        };

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
        let enable_subnet_node = devnet_config.enable_subnet_node;
        let disable_subnet_api = devnet_config.disable_subnet_api;

        let _ = fs::create_dir(format!("{}", devnet_config.working_dir));
        let _ = fs::create_dir(format!("{}/conf", devnet_config.working_dir));
        let _ = fs::create_dir(format!("{}/data", devnet_config.working_dir));

        let bitcoin_explorer_port = devnet_config.bitcoin_explorer_port;
        let stacks_explorer_port = devnet_config.stacks_explorer_port;
        let stacks_api_port = devnet_config.stacks_api_port;
        let subnet_api_port = devnet_config.subnet_api_port;

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

        if enable_subnet_node {
            let _ = event_tx.send(DevnetEvent::ServiceStatus(ServiceStatusData {
                order: 5,
                status: Status::Red,
                name: "subnet-node".into(),
                comment: "initializing".into(),
            }));

            let _ = event_tx.send(DevnetEvent::ServiceStatus(ServiceStatusData {
                order: 6,
                status: Status::Red,
                name: "subnet-api".into(),
                comment: if disable_subnet_api {
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

        // Start bitcoind
        let _ = event_tx.send(DevnetEvent::info(format!("Starting bitcoin-node")));
        let _ = event_tx.send(DevnetEvent::ServiceStatus(ServiceStatusData {
            order: 0,
            status: Status::Yellow,
            name: "bitcoin-node".into(),
            comment: "preparing container".into(),
        }));
        match self.prepare_bitcoin_node_container(&ctx).await {
            Ok(_) => {}
            Err(message) => {
                let _ = event_tx.send(DevnetEvent::FatalError(message.clone()));
                self.kill(ctx, Some(&message)).await;
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
                self.initialize_bitcoin_node(&event_tx).await?;
            }
            Err(message) => {
                let _ = event_tx.send(DevnetEvent::FatalError(message.clone()));
                self.kill(ctx, Some(&message)).await;
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
            match self.prepare_postgres_container(ctx).await {
                Ok(_) => {}
                Err(message) => {
                    let _ = event_tx.send(DevnetEvent::FatalError(message.clone()));
                    self.kill(ctx, Some(&message)).await;
                    return Err(message);
                }
            };
            match self.boot_postgres_container(ctx).await {
                Ok(_) => {}
                Err(message) => {
                    let _ = event_tx.send(DevnetEvent::FatalError(message.clone()));
                    self.kill(ctx, Some(&message)).await;
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
            match self.prepare_stacks_api_container(ctx).await {
                Ok(_) => {}
                Err(message) => {
                    let _ = event_tx.send(DevnetEvent::FatalError(message.clone()));
                    self.kill(ctx, Some(&message)).await;
                    return Err(message);
                }
            };
            let _ = event_tx.send(DevnetEvent::ServiceStatus(ServiceStatusData {
                order: 2,
                status: Status::Green,
                name: "stacks-api".into(),
                comment: format!("http://localhost:{}/doc", stacks_api_port),
            }));
            match self.boot_stacks_api_container(ctx).await {
                Ok(_) => {}
                Err(message) => {
                    let _ = event_tx.send(DevnetEvent::FatalError(message.clone()));
                    self.kill(ctx, Some(&message)).await;
                    return Err(message);
                }
            };
        }

        // Start subnet node
        if enable_subnet_node {
            let _ = event_tx.send(DevnetEvent::info(format!("Starting subnet-node")));
            match self.prepare_subnet_node_container(boot_index, ctx).await {
                Ok(_) => {}
                Err(message) => {
                    let _ = event_tx.send(DevnetEvent::FatalError(message.clone()));
                    self.kill(ctx, Some(&message)).await;
                    return Err(message);
                }
            };
            let _ = event_tx.send(DevnetEvent::ServiceStatus(ServiceStatusData {
                order: 5,
                status: Status::Yellow,
                name: "subnet-node".into(),
                comment: "booting".into(),
            }));
            match self.boot_subnet_node_container().await {
                Ok(_) => {}
                Err(message) => {
                    let _ = event_tx.send(DevnetEvent::FatalError(message.clone()));
                    self.kill(ctx, Some(&message)).await;
                    return Err(message);
                }
            };

            if !disable_subnet_api {
                let _ = event_tx.send(DevnetEvent::info(format!("Starting subnet-api")));
                match self.prepare_subnet_api_container(ctx).await {
                    Ok(_) => {}
                    Err(message) => {
                        let _ = event_tx.send(DevnetEvent::FatalError(message.clone()));
                        self.kill(ctx, Some(&message)).await;
                        return Err(message);
                    }
                };
                let _ = event_tx.send(DevnetEvent::ServiceStatus(ServiceStatusData {
                    order: 6,
                    status: Status::Green,
                    name: "subnet-api".into(),
                    comment: format!("http://localhost:{}/doc", subnet_api_port),
                }));
                match self.boot_subnet_api_container().await {
                    Ok(_) => {}
                    Err(message) => {
                        let _ = event_tx.send(DevnetEvent::FatalError(message.clone()));
                        self.kill(ctx, Some(&message)).await;
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
        match self.prepare_stacks_node_container(boot_index, ctx).await {
            Ok(_) => {}
            Err(message) => {
                let _ = event_tx.send(DevnetEvent::FatalError(message.clone()));
                self.kill(ctx, Some(&message)).await;
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
                self.kill(ctx, Some(&message)).await;
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
            match self.prepare_stacks_explorer_container(ctx).await {
                Ok(_) => {}
                Err(message) => {
                    let _ = event_tx.send(DevnetEvent::FatalError(message.clone()));
                    self.kill(ctx, Some(&message)).await;
                    return Err(message);
                }
            };
            let _ = event_tx.send(DevnetEvent::info(format!("Starting stacks-explorer")));
            match self.boot_stacks_explorer_container(ctx).await {
                Ok(_) => {}
                Err(message) => {
                    let _ = event_tx.send(DevnetEvent::FatalError(message.clone()));
                    self.kill(ctx, Some(&message)).await;
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
            match self.prepare_bitcoin_explorer_container(ctx).await {
                Ok(_) => {}
                Err(message) => {
                    let _ = event_tx.send(DevnetEvent::FatalError(message.clone()));
                    self.kill(ctx, Some(&message)).await;
                    return Err(message);
                }
            };
            let _ = event_tx.send(DevnetEvent::info(format!("Starting bitcoin-explorer")));
            match self.boot_bitcoin_explorer_container(ctx).await {
                Ok(_) => {}
                Err(message) => {
                    let _ = event_tx.send(DevnetEvent::FatalError(message.clone()));
                    self.kill(ctx, Some(&message)).await;
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
                    self.kill(ctx, None).await;
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

                    let _ = event_tx.send(DevnetEvent::debug("Killing containers".into()));
                    let _ = self.stop_containers().await;

                    let _ = event_tx.send(DevnetEvent::debug("Restarting containers".into()));
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
rpcworkqueue=100
rpcserialversion=1
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
        bitcoind_conf_path.push("conf");
        fs::create_dir_all(&bitcoind_conf_path)
            .map_err(|e| format!("unable to create bitcoin conf directory: {}", e.to_string()))?;
        bitcoind_conf_path.push("bitcoin.conf");
        let mut file = File::create(bitcoind_conf_path)
            .map_err(|e| format!("unable to create bitcoin.conf: {}", e.to_string()))?;

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
            // domainname: Some(self.network_name.to_string()),
            tty: None,
            exposed_ports: Some(exposed_ports),
            entrypoint: Some(vec![]),
            env: Some(env),
            host_config: Some(HostConfig {
                auto_remove: Some(true),
                binds: Some(binds),
                network_mode: Some(self.network_name.clone()),
                port_bindings: Some(port_bindings),
                extra_hosts: Some(vec!["host.docker.internal:host-gateway".into()]),
                ..Default::default()
            }),
            cmd: Some(vec![
                "/usr/local/bin/bitcoind".into(),
                "-conf=/etc/bitcoin/bitcoin.conf".into(),
                "-nodebuglogfile".into(),
                "-pid=/run/bitcoind.pid".into(),
                // "-datadir=/root/.bitcoin".into(),
            ]),
            ..Default::default()
        };

        Ok(config)
    }

    pub async fn prepare_bitcoin_node_container(&mut self, ctx: &Context) -> Result<(), String> {
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
                    platform: devnet_config.docker_platform.clone(),
                    ..Default::default()
                }),
                None,
                None,
            )
            .try_collect::<Vec<_>>()
            .await
            .map_err(|e| formatted_docker_error("unable to create bitcoind image", e))?;

        let config = self.prepare_bitcoin_node_config(1)?;
        let container_name = format!("bitcoin-node.{}", self.network_name);
        let options = CreateContainerOptions {
            name: container_name.as_str(),
        };

        let container = match docker
            .create_container::<&str, String>(Some(options.clone()), config.clone())
            .await
            .map_err(|e| formatted_docker_error("unable to create bitcoind container", e))
        {
            Ok(container) => container.id,
            Err(_e) => {
                // Attempt to clean eventual subsequent artifacts
                let _ = docker.kill_container::<String>(&container_name, None).await;
                docker
                    .create_container::<&str, String>(Some(options), config)
                    .await
                    .map_err(|e| formatted_docker_error("unable to create bitcoind container", e))?
                    .id
            }
        };
        ctx.try_log(|logger| slog::info!(logger, "Created container bitcoin-node: {}", container));
        self.bitcoin_node_container_id = Some(container);

        Ok(())
    }

    pub async fn clean_previous_session(&self) -> Result<(), String> {
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
            Err(e) => {
                let err = format!("unable to communicate with Docker: {}\nvisit https://docs.hiro.so/clarinet/troubleshooting#i-am-unable-to-start-devnet-though-my-docker-is-running to resolve this issue.", e.to_string());
                return Err(err);
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
        Ok(())
    }

    pub async fn boot_bitcoin_node_container(&mut self) -> Result<(), String> {
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
pox_sync_sample_secs = 0
wait_time_for_blocks = 0
wait_time_for_microblocks = {wait_time_for_microblocks}
microblock_frequency = 1000

[connection_options]
# inv_sync_interval = 10
# download_interval = 10
# walk_interval = 10
disable_block_download = true
disable_inbound_handshakes = true
disable_inbound_walks = true
public_ip_address = "1.1.1.1:1234"

[miner]
first_attempt_time_ms = {first_attempt_time_ms}
subsequent_attempt_time_ms = {subsequent_attempt_time_ms}
block_reward_recipient = "{miner_coinbase_recipient}"
# microblock_attempt_time_ms = 15000
"#,
            stacks_node_rpc_port = devnet_config.stacks_node_rpc_port,
            stacks_node_p2p_port = devnet_config.stacks_node_p2p_port,
            miner_secret_key_hex = devnet_config.miner_secret_key_hex,
            wait_time_for_microblocks = devnet_config.stacks_node_wait_time_for_microblocks,
            first_attempt_time_ms = devnet_config.stacks_node_first_attempt_time_ms,
            subsequent_attempt_time_ms = devnet_config.stacks_node_subsequent_attempt_time_ms,
            miner_coinbase_recipient = devnet_config.miner_coinbase_recipient,
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

        if devnet_config.enable_subnet_node {
            stacks_conf.push_str(&format!(
                r#"
# Add subnet-node as an event observer
[[events_observer]]
endpoint = "{}"
retry_count = 255
events_keys = ["*"]
"#,
                format!(
                    "subnet-node.{}:{}",
                    self.network_name, devnet_config.subnet_events_ingestion_port
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

        stacks_conf.push_str(&format!(
            r#"
[burnchain]
chain = "bitcoin"
mode = "krypton"
poll_time_secs = 1
timeout = 30
peer_host = "host.docker.internal"
rpc_ssl = false
wallet_name = "devnet"
username = "{bitcoin_node_username}"
password = "{bitcoin_node_password}"
rpc_port = {orchestrator_ingestion_port}
peer_port = {bitcoin_node_p2p_port}
"#,
            bitcoin_node_username = devnet_config.bitcoin_node_username,
            bitcoin_node_password = devnet_config.bitcoin_node_password,
            bitcoin_node_p2p_port = devnet_config.bitcoin_node_p2p_port,
            orchestrator_ingestion_port = devnet_config.orchestrator_ingestion_port,
        ));

        stacks_conf.push_str(&format!(
            r#"pox_2_activation = {pox_2_activation}

[[burnchain.epochs]]
epoch_name = "1.0"
start_height = 0

[[burnchain.epochs]]
epoch_name = "2.0"
start_height = {epoch_2_0}

[[burnchain.epochs]]
epoch_name = "2.05"
start_height = {epoch_2_05}

[[burnchain.epochs]]
epoch_name = "2.1"
start_height = {epoch_2_1}

[[burnchain.epochs]]
epoch_name = "2.2"
start_height = {epoch_2_2}

[[burnchain.epochs]]
epoch_name = "2.3"
start_height = {epoch_2_3}

[[burnchain.epochs]]
epoch_name = "2.4"
start_height = {epoch_2_4}
"#,
            epoch_2_0 = devnet_config.epoch_2_0,
            epoch_2_05 = devnet_config.epoch_2_05,
            epoch_2_1 = devnet_config.epoch_2_1,
            epoch_2_2 = devnet_config.epoch_2_2,
            epoch_2_3 = devnet_config.epoch_2_3,
            epoch_2_4 = devnet_config.epoch_2_4,
            pox_2_activation = devnet_config.pox_2_activation,
        ));

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

        let mut env = vec![
            "STACKS_LOG_PP=1".to_string(),
            "BLOCKSTACK_USE_TEST_GENESIS_CHAINSTATE=1".to_string(),
        ];
        env.append(&mut devnet_config.stacks_node_env_vars.clone());

        let config = Config {
            labels: Some(labels),
            image: Some(devnet_config.stacks_node_image_url.clone()),
            // domainname: Some(self.network_name.to_string()),
            tty: None,
            exposed_ports: Some(exposed_ports),
            entrypoint: Some(vec![
                "stacks-node".into(),
                "start".into(),
                "--config=/src/stacks-node/Stacks.toml".into(),
            ]),
            env: Some(env),
            host_config: Some(HostConfig {
                auto_remove: Some(true),
                binds: Some(binds),
                network_mode: Some(self.network_name.clone()),
                port_bindings: Some(port_bindings),
                extra_hosts: Some(vec!["host.docker.internal:host-gateway".into()]),
                ..Default::default()
            }),
            ..Default::default()
        };

        Ok(config)
    }

    pub async fn prepare_stacks_node_container(
        &mut self,
        boot_index: u32,
        ctx: &Context,
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
                    from_image: devnet_config.stacks_node_image_url.clone(),
                    platform: devnet_config.docker_platform.clone(),
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

        ctx.try_log(|logger| slog::info!(logger, "Created container stacks-node: {}", container));
        self.stacks_node_container_id = Some(container.clone());

        Ok(())
    }

    pub async fn boot_stacks_node_container(&mut self) -> Result<(), String> {
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

        Ok(())
    }

    pub fn prepare_subnet_node_config(&self, boot_index: u32) -> Result<Config<String>, String> {
        let devnet_config = match &self.network_config {
            Some(network_config) => match &network_config.devnet {
                Some(devnet_config) => devnet_config,
                _ => return Err("unable to get devnet configuration".into()),
            },
            _ => return Err("unable to get Docker client".into()),
        };

        let mut port_bindings = HashMap::new();
        port_bindings.insert(
            format!("{}/tcp", devnet_config.subnet_node_p2p_port),
            Some(vec![PortBinding {
                host_ip: Some(String::from("0.0.0.0")),
                host_port: Some(format!("{}/tcp", devnet_config.subnet_node_p2p_port)),
            }]),
        );
        port_bindings.insert(
            format!("{}/tcp", devnet_config.subnet_node_rpc_port),
            Some(vec![PortBinding {
                host_ip: Some(String::from("0.0.0.0")),
                host_port: Some(format!("{}/tcp", devnet_config.subnet_node_rpc_port)),
            }]),
        );
        port_bindings.insert(
            format!("{}/tcp", devnet_config.subnet_events_ingestion_port),
            Some(vec![PortBinding {
                host_ip: Some(String::from("0.0.0.0")),
                host_port: Some(format!(
                    "{}/tcp",
                    devnet_config.subnet_events_ingestion_port
                )),
            }]),
        );

        let mut subnet_conf = format!(
            r#"
[node]
working_dir = "/devnet"
rpc_bind = "0.0.0.0:{subnet_node_rpc_port}"
p2p_bind = "0.0.0.0:{subnet_node_p2p_port}"
miner = true
seed = "{subnet_leader_secret_key_hex}"
mining_key = "{subnet_leader_secret_key_hex}"
local_peer_seed = "{subnet_leader_secret_key_hex}"
wait_time_for_microblocks = {wait_time_for_microblocks}
wait_before_first_anchored_block = 0

[miner]
first_attempt_time_ms = {first_attempt_time_ms}
subsequent_attempt_time_ms = {subsequent_attempt_time_ms}
# microblock_attempt_time_ms = 15_000

[burnchain]
chain = "stacks_layer_1"
mode = "subnet"
first_burn_header_height = {first_burn_header_height}
peer_host = "host.docker.internal"
rpc_port = {stacks_node_rpc_port}
peer_port = {stacks_node_p2p_port}
contract_identifier = "{subnet_contract_id}"
observer_port = {subnet_events_ingestion_port}

# Add orchestrator (docker-host) as an event observer
# [[events_observer]]
# endpoint = "host.docker.internal:{orchestrator_port}"
# retry_count = 255
# include_data_events = true
# events_keys = ["*"]
"#,
            subnet_node_rpc_port = devnet_config.subnet_node_rpc_port,
            subnet_node_p2p_port = devnet_config.subnet_node_p2p_port,
            subnet_leader_secret_key_hex = devnet_config.subnet_leader_secret_key_hex,
            stacks_node_rpc_port = devnet_config.stacks_node_rpc_port,
            stacks_node_p2p_port = devnet_config.stacks_node_p2p_port,
            orchestrator_port = devnet_config.orchestrator_ingestion_port,
            subnet_events_ingestion_port = devnet_config.subnet_events_ingestion_port,
            first_burn_header_height = 0,
            subnet_contract_id = devnet_config.remapped_subnet_contract_id,
            wait_time_for_microblocks = devnet_config.stacks_node_wait_time_for_microblocks,
            first_attempt_time_ms = devnet_config.stacks_node_first_attempt_time_ms,
            subsequent_attempt_time_ms = devnet_config.stacks_node_subsequent_attempt_time_ms,
        );

        for events_observer in devnet_config.subnet_node_events_observers.iter() {
            subnet_conf.push_str(&format!(
                r#"
[[events_observer]]
endpoint = "{}"
retry_count = 255
events_keys = ["*"]
"#,
                events_observer,
            ));
        }

        if !devnet_config.disable_subnet_api {
            subnet_conf.push_str(&format!(
                r#"
# Add subnet-api as an event observer
[[events_observer]]
endpoint = "{}"
retry_count = 255
include_data_events = false
events_keys = ["*"]
"#,
                format!(
                    "subnet-api.{}:{}",
                    self.network_name, devnet_config.subnet_api_events_port
                ),
            ));
        }

        let mut subnet_conf_path = PathBuf::from(&devnet_config.working_dir);
        subnet_conf_path.push("conf/Subnet.toml");
        let mut file = File::create(subnet_conf_path.clone()).map_err(|e| {
            format!(
                "unable to create Subnet.toml ({}): {:?}",
                subnet_conf_path.to_str().unwrap(),
                e
            )
        })?;
        file.write_all(subnet_conf.as_bytes())
            .map_err(|e| format!("unable to write Subnet.toml: {:?}", e))?;

        let mut stacks_node_data_path = PathBuf::from(&devnet_config.working_dir);
        stacks_node_data_path.push("data");
        stacks_node_data_path.push(format!("{}", boot_index));
        let _ = fs::create_dir(stacks_node_data_path.clone());
        stacks_node_data_path.push("subnet");

        let mut exposed_ports = HashMap::new();
        exposed_ports.insert(
            format!("{}/tcp", devnet_config.subnet_node_rpc_port),
            HashMap::new(),
        );
        exposed_ports.insert(
            format!("{}/tcp", devnet_config.subnet_node_p2p_port),
            HashMap::new(),
        );
        exposed_ports.insert(
            format!("{}/tcp", devnet_config.subnet_events_ingestion_port),
            HashMap::new(),
        );

        let mut labels = HashMap::new();
        labels.insert("project".to_string(), self.network_name.to_string());
        labels.insert("reset".to_string(), "true".to_string());

        let mut binds = vec![format!(
            "{}/conf:/src/subnet-node/",
            devnet_config.working_dir
        )];

        if devnet_config.bind_containers_volumes {
            binds.push(format!(
                "{}/data/{}/subnet:/devnet/",
                devnet_config.working_dir, boot_index
            ))
        }

        let mut env = vec![
            "STACKS_LOG_PP=1".to_string(),
            "STACKS_LOG_DEBUG=1".to_string(),
            // "BLOCKSTACK_USE_TEST_GENESIS_CHAINSTATE=1".to_string(),
        ];
        env.append(&mut devnet_config.subnet_node_env_vars.clone());

        let config = Config {
            labels: Some(labels),
            image: Some(devnet_config.subnet_node_image_url.clone()),
            // domainname: Some(self.network_name.to_string()),
            tty: None,
            exposed_ports: Some(exposed_ports),
            entrypoint: Some(vec![
                "subnet-node".into(),
                "start".into(),
                "--config=/src/subnet-node/Subnet.toml".into(),
            ]),
            env: Some(env),
            host_config: Some(HostConfig {
                auto_remove: Some(true),
                binds: Some(binds),
                network_mode: Some(self.network_name.clone()),
                port_bindings: Some(port_bindings),
                extra_hosts: Some(vec!["host.docker.internal:host-gateway".into()]),
                ..Default::default()
            }),
            ..Default::default()
        };

        Ok(config)
    }

    pub async fn prepare_subnet_node_container(
        &mut self,
        boot_index: u32,
        ctx: &Context,
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
                    from_image: devnet_config.subnet_node_image_url.clone(),
                    platform: devnet_config.docker_platform.clone(),
                    ..Default::default()
                }),
                None,
                None,
            )
            .try_collect::<Vec<_>>()
            .await
            .map_err(|e| format!("unable to create image: {}", e))?;

        let config = self.prepare_subnet_node_config(boot_index)?;

        let options = CreateContainerOptions {
            name: format!("subnet-node.{}", self.network_name),
        };

        let container = docker
            .create_container::<String, String>(Some(options), config)
            .await
            .map_err(|e| format!("unable to create container: {}", e))?
            .id;

        ctx.try_log(|logger| slog::info!(logger, "Created container subnet-node: {}", container));
        self.subnet_node_container_id = Some(container.clone());

        Ok(())
    }

    pub async fn boot_subnet_node_container(&self) -> Result<(), String> {
        let container = match &self.subnet_node_container_id {
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

        Ok(())
    }

    pub async fn prepare_stacks_api_container(&mut self, ctx: &Context) -> Result<(), String> {
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
                    platform: devnet_config.docker_platform.clone(),
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

        let mut env = vec![
            format!("STACKS_CORE_RPC_HOST=stacks-node.{}", self.network_name),
            format!("STACKS_BLOCKCHAIN_API_DB=pg"),
            format!(
                "STACKS_CORE_RPC_PORT={}",
                devnet_config.stacks_node_rpc_port
            ),
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
            format!("FAUCET_PRIVATE_KEY={}", devnet_config.faucet_secret_key_hex),
            "NODE_ENV=development".to_string(),
        ];
        env.append(&mut devnet_config.stacks_api_env_vars.clone());

        let config = Config {
            labels: Some(labels),
            image: Some(devnet_config.stacks_api_image_url.clone()),
            // domainname: Some(self.network_name.to_string()),
            tty: None,
            exposed_ports: Some(exposed_ports),
            env: Some(env),
            host_config: Some(HostConfig {
                auto_remove: Some(true),
                network_mode: Some(self.network_name.clone()),
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

        ctx.try_log(|logger| slog::info!(logger, "Created container stacks-api: {}", container));
        self.stacks_api_container_id = Some(container);

        Ok(())
    }

    pub async fn boot_stacks_api_container(&self, _ctx: &Context) -> Result<(), String> {
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

        Ok(())
    }

    pub async fn prepare_subnet_api_container(&mut self, ctx: &Context) -> Result<(), String> {
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
                    from_image: devnet_config.subnet_api_image_url.clone(),
                    platform: devnet_config.docker_platform.clone(),
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
            format!("{}/tcp", devnet_config.subnet_api_port),
            Some(vec![PortBinding {
                host_ip: Some(String::from("0.0.0.0")),
                host_port: Some(format!("{}/tcp", devnet_config.subnet_api_port)),
            }]),
        );

        let mut exposed_ports = HashMap::new();
        exposed_ports.insert(
            format!("{}/tcp", devnet_config.subnet_api_port),
            HashMap::new(),
        );

        let mut labels = HashMap::new();
        labels.insert("project".to_string(), self.network_name.to_string());

        let mut env = vec![
            format!("STACKS_CORE_RPC_HOST=subnet-node.{}", self.network_name),
            format!("STACKS_BLOCKCHAIN_API_DB=pg"),
            format!(
                "STACKS_CORE_RPC_PORT={}",
                devnet_config.subnet_node_rpc_port
            ),
            format!(
                "STACKS_BLOCKCHAIN_API_PORT={}",
                devnet_config.subnet_api_port
            ),
            format!("STACKS_BLOCKCHAIN_API_HOST=0.0.0.0"),
            format!(
                "STACKS_CORE_EVENT_PORT={}",
                devnet_config.subnet_api_events_port
            ),
            format!("STACKS_CORE_EVENT_HOST=0.0.0.0"),
            format!("STACKS_API_ENABLE_FT_METADATA=1"),
            format!("PG_HOST=postgres.{}", self.network_name),
            format!("PG_PORT={}", devnet_config.postgres_port),
            format!("PG_USER={}", devnet_config.postgres_username),
            format!("PG_PASSWORD={}", devnet_config.postgres_password),
            format!("PG_DATABASE={}", devnet_config.subnet_api_postgres_database),
            format!("STACKS_CHAIN_ID=0x55005500"),
            format!("CUSTOM_CHAIN_IDS=testnet=0x55005500"),
            format!("V2_POX_MIN_AMOUNT_USTX=90000000260"),
            "NODE_ENV=development".to_string(),
        ];
        env.append(&mut devnet_config.subnet_api_env_vars.clone());

        let config = Config {
            labels: Some(labels),
            image: Some(devnet_config.subnet_api_image_url.clone()),
            // domainname: Some(self.network_name.to_string()),
            tty: None,
            exposed_ports: Some(exposed_ports),
            env: Some(env),
            host_config: Some(HostConfig {
                auto_remove: Some(true),
                network_mode: Some(self.network_name.clone()),
                port_bindings: Some(port_bindings),
                extra_hosts: Some(vec!["host.docker.internal:host-gateway".into()]),
                ..Default::default()
            }),
            ..Default::default()
        };

        let options = CreateContainerOptions {
            name: format!("subnet-api.{}", self.network_name),
        };

        let container = docker
            .create_container::<String, String>(Some(options), config)
            .await
            .map_err(|e| format!("unable to create container: {}", e))?
            .id;

        ctx.try_log(|logger| slog::info!(logger, "Created container subnet-api: {}", container));
        self.subnet_api_container_id = Some(container);

        Ok(())
    }

    pub async fn boot_subnet_api_container(&self) -> Result<(), String> {
        // Before booting the subnet-api, we need to create an additional DB in the postgres container.
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
            devnet_config.subnet_api_postgres_database
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

        // Pause to ensure the postgres container is ready.
        // TODO
        std::thread::sleep(std::time::Duration::from_secs(10));

        let _res = docker
            .start_exec(&exec.id, None)
            .await
            .map_err(|e| formatted_docker_error("unable to start exec command", e))?;

        let container = match &self.subnet_api_container_id {
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

        Ok(())
    }

    pub async fn prepare_postgres_container(&mut self, ctx: &Context) -> Result<(), String> {
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
                    platform: devnet_config.docker_platform.clone(),
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
            // domainname: Some(self.network_name.to_string()),
            tty: None,
            exposed_ports: Some(exposed_ports),
            env: Some(vec![
                format!("POSTGRES_PASSWORD={}", devnet_config.postgres_password),
                format!("POSTGRES_DB={}", devnet_config.stacks_api_postgres_database),
            ]),
            host_config: Some(HostConfig {
                auto_remove: Some(true),
                network_mode: Some(self.network_name.clone()),
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

        ctx.try_log(|logger| slog::info!(logger, "Created container postgres: {}", container));
        self.postgres_container_id = Some(container);

        Ok(())
    }

    pub async fn boot_postgres_container(&self, _ctx: &Context) -> Result<(), String> {
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

        Ok(())
    }

    pub async fn prepare_stacks_explorer_container(&mut self, ctx: &Context) -> Result<(), String> {
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
                    platform: devnet_config.docker_platform.clone(),
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

        let mut env = vec![
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
        ];
        env.append(&mut devnet_config.stacks_explorer_env_vars.clone());

        let config = Config {
            labels: Some(labels),
            image: Some(devnet_config.stacks_explorer_image_url.clone()),
            // domainname: Some(self.network_name.to_string()),
            tty: None,
            exposed_ports: Some(exposed_ports),
            env: Some(env),
            host_config: Some(HostConfig {
                auto_remove: Some(true),
                network_mode: Some(self.network_name.clone()),
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

        ctx.try_log(|logger| {
            slog::info!(logger, "Created container stacks-explorer: {}", container)
        });
        self.stacks_explorer_container_id = Some(container);

        Ok(())
    }

    pub async fn boot_stacks_explorer_container(&self, _ctx: &Context) -> Result<(), String> {
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

        Ok(())
    }

    pub async fn prepare_bitcoin_explorer_container(
        &mut self,
        ctx: &Context,
    ) -> Result<(), String> {
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
                    platform: devnet_config.docker_platform.clone(),
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
            // domainname: Some(self.network_name.to_string()),
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
                auto_remove: Some(true),
                network_mode: Some(self.network_name.clone()),
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

        ctx.try_log(|logger| {
            slog::info!(logger, "Created container bitcoin-explorer: {}", container)
        });
        self.bitcoin_explorer_container_id = Some(container);

        Ok(())
    }

    pub async fn boot_bitcoin_explorer_container(&self, _ctx: &Context) -> Result<(), String> {
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

        Ok((bitcoin_node_c_id, stacks_node_c_id))
    }

    pub async fn kill(&self, ctx: &Context, fatal_message: Option<&str>) {
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
            ctx.try_log(|logger| slog::info!(logger, "Terminating bitcoin-explorer"));
            let _ = docker.remove_container(bitcoin_explorer_container_id, None);
        }

        if let Some(ref stacks_explorer_container_id) = self.stacks_explorer_container_id {
            let _ = docker
                .kill_container(stacks_explorer_container_id, options.clone())
                .await;
            ctx.try_log(|logger| slog::info!(logger, "Terminating stacks-explorer"));
            let _ = docker.remove_container(stacks_explorer_container_id, None);
        }

        if let Some(ref bitcoin_node_container_id) = self.bitcoin_node_container_id {
            let _ = docker
                .kill_container(bitcoin_node_container_id, options.clone())
                .await;
            ctx.try_log(|logger| slog::info!(logger, "Terminating bitcoin-node"));
            let _ = docker.remove_container(bitcoin_node_container_id, None);
        }

        if let Some(ref stacks_api_container_id) = self.stacks_api_container_id {
            let _ = docker
                .kill_container(stacks_api_container_id, options.clone())
                .await;
            ctx.try_log(|logger| slog::info!(logger, "Terminating stacks-api"));
            let _ = docker.remove_container(stacks_api_container_id, None);
        }

        if let Some(ref postgres_container_id) = self.postgres_container_id {
            let _ = docker
                .kill_container(postgres_container_id, options.clone())
                .await;
            ctx.try_log(|logger| slog::info!(logger, "Terminating postgres"));
            let _ = docker.remove_container(postgres_container_id, None);
        }

        if let Some(ref stacks_node_container_id) = self.stacks_node_container_id {
            let _ = docker
                .kill_container(stacks_node_container_id, options.clone())
                .await;
            ctx.try_log(|logger| slog::info!(logger, "Terminating stacks-node"));
            let _ = docker.remove_container(stacks_node_container_id, None);
        }

        if let Some(ref subnet_node_container_id) = self.subnet_node_container_id {
            let _ = docker
                .kill_container(subnet_node_container_id, options.clone())
                .await;
            ctx.try_log(|logger| slog::info!(logger, "Terminating subnet-node"));
            let _ = docker.remove_container(subnet_node_container_id, None);
        }

        if let Some(ref subnet_api_container_id) = self.subnet_api_container_id {
            let _ = docker
                .kill_container(subnet_api_container_id, options)
                .await;
            ctx.try_log(|logger| slog::info!(logger, "Terminating subnet-api"));
            let _ = docker.remove_container(subnet_api_container_id, None);
        }

        // Delete network
        let _ = docker.remove_network(&self.network_name).await;

        ctx.try_log(|logger| slog::info!(logger, "Pruning network and containers"));

        self.prune().await;
        if let Some(ref tx) = self.termination_success_tx {
            let _ = tx.send(true);
        }

        ctx.try_log(|logger| {
            slog::info!(
                logger,
                "Artifacts (logs, conf, chainstates) available here: {}",
                devnet_config.working_dir
            )
        });

        if let Some(message) = fatal_message {
            ctx.try_log(|logger| slog::info!(logger, "⚠️  fatal error - {}", message));
        } else {
            ctx.try_log(|logger| slog::info!(logger, "✌️"));
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

    pub async fn initialize_bitcoin_node(
        &self,
        devnet_event_tx: &Sender<DevnetEvent>,
    ) -> Result<(), String> {
        use bitcoincore_rpc::bitcoin::Address;
        use reqwest::Client as HttpClient;
        use serde_json::json;
        use std::str::FromStr;

        let (devnet_config, accounts) = match &self.network_config {
            Some(ref network_config) => match network_config.devnet {
                Some(ref devnet_config) => (devnet_config, &network_config.accounts),
                _ => return Err(format!("unable to initialize bitcoin node")),
            },
            _ => return Err(format!("unable to initialize bitcoin node")),
        };

        let miner_address = Address::from_str(&devnet_config.miner_btc_address)
            .map_err(|e| format!("unable to create miner address: {:?}", e))?;

        let faucet_address = Address::from_str(&devnet_config.faucet_btc_address)
            .map_err(|e| format!("unable to create faucet address: {:?}", e))?;

        let bitcoin_node_url = format!(
            "http://{}/",
            self.services_map_hosts.as_ref().unwrap().bitcoin_node_host
        );

        fn base_builder(node_url: &str, username: &str, password: &str) -> RequestBuilder {
            let http_client = HttpClient::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("Unable to build http client");
            http_client
                .post(node_url.clone())
                .timeout(Duration::from_secs(3))
                .basic_auth(&username, Some(&password))
                .header("Content-Type", "application/json")
                .header("Host", &node_url[7..])
        }

        let _ = devnet_event_tx.send(DevnetEvent::info(format!("Configuring bitcoin-node",)));

        let max_errors = 30;

        let mut error_count = 0;
        loop {
            let network_info = base_builder(
                &bitcoin_node_url,
                &devnet_config.bitcoin_node_username,
                &devnet_config.bitcoin_node_password,
            )
            .json(&json!({
                "jsonrpc": "1.0",
                "id": "stacks-network",
                "method": "getnetworkinfo",
                "params": []
            }))
            .send()
            .await
            .map_err(|e| format!("unable to send 'getnetworkinfo' request ({})", e));

            match network_info {
                Ok(_r) => break,
                Err(e) => {
                    error_count += 1;
                    if error_count > max_errors {
                        return Err(e);
                    } else if error_count > 1 {
                        let _ = devnet_event_tx.send(DevnetEvent::error(e));
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
            let _ = devnet_event_tx.send(DevnetEvent::info(format!("Waiting for bitcoin-node",)));
        }

        let mut error_count = 0;
        loop {
            let rpc_call = base_builder(
                &bitcoin_node_url,
                &devnet_config.bitcoin_node_username,
                &devnet_config.bitcoin_node_password,
            )
            .json(&json!({
                "jsonrpc": "1.0",
                "id": "stacks-network",
                "method": "generatetoaddress",
                "params": [json!(3), json!(miner_address)]
            }))
            .send()
            .await
            .map_err(|e| format!("unable to send 'generatetoaddress' request ({})", e));

            match rpc_call {
                Ok(_r) => break,
                Err(e) => {
                    error_count += 1;
                    if error_count > max_errors {
                        return Err(e);
                    } else if error_count > 1 {
                        let _ = devnet_event_tx.send(DevnetEvent::error(e));
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
            let _ = devnet_event_tx.send(DevnetEvent::info(format!("Waiting for bitcoin-node",)));
        }

        let mut error_count = 0;
        loop {
            let rpc_call = base_builder(
                &bitcoin_node_url,
                &devnet_config.bitcoin_node_username,
                &devnet_config.bitcoin_node_password,
            )
            .json(&json!({
                "jsonrpc": "1.0",
                "id": "stacks-network",
                "method": "generatetoaddress",
                "params": [json!(97), json!(faucet_address)]
            }))
            .send()
            .await
            .map_err(|e| format!("unable to send 'generatetoaddress' request ({})", e));

            match rpc_call {
                Ok(_r) => break,
                Err(e) => {
                    error_count += 1;
                    if error_count > max_errors {
                        return Err(e);
                    } else if error_count > 1 {
                        let _ = devnet_event_tx.send(DevnetEvent::error(e));
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
            let _ = devnet_event_tx.send(DevnetEvent::info(format!("Waiting for bitcoin-node",)));
        }

        let mut error_count = 0;
        loop {
            let rpc_call = base_builder(
                &bitcoin_node_url,
                &devnet_config.bitcoin_node_username,
                &devnet_config.bitcoin_node_password,
            )
            .json(&json!({
            "jsonrpc": "1.0",
            "id": "stacks-network",
            "method": "generatetoaddress",
            "params": [json!(1), json!(miner_address)]
            }))
            .send()
            .await
            .map_err(|e| format!("unable to send 'generatetoaddress' request ({})", e));

            match rpc_call {
                Ok(_r) => break,
                Err(e) => {
                    error_count += 1;
                    if error_count > max_errors {
                        return Err(e);
                    } else if error_count > 1 {
                        let _ = devnet_event_tx.send(DevnetEvent::error(e));
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
            let _ = devnet_event_tx.send(DevnetEvent::info(format!("Waiting for bitcoin-node",)));
        }

        let mut error_count = 0;
        loop {
            let rpc_call = base_builder(
                &bitcoin_node_url,
                &devnet_config.bitcoin_node_username,
                &devnet_config.bitcoin_node_password,
            )
            .json(&json!({
                "jsonrpc": "1.0",
                "id": "stacks-network",
                "method": "createwallet",
                "params": json!({ "wallet_name": "", "disable_private_keys": true })
            }))
            .send()
            .await
            .map_err(|e| format!("unable to send 'createwallet' request ({})", e));

            match rpc_call {
                Ok(r) => {
                    if r.status().is_success() {
                        break;
                    } else {
                        let err = r.text().await;
                        let msg = format!("{:?}", err);
                        let _ = devnet_event_tx.send(DevnetEvent::error(msg));
                    }
                }
                Err(e) => {
                    error_count += 1;
                    if error_count > max_errors {
                        return Err(e);
                    } else if error_count > 1 {
                        let _ = devnet_event_tx.send(DevnetEvent::error(e));
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
            let _ = devnet_event_tx.send(DevnetEvent::info(format!("Waiting for bitcoin-node",)));
        }

        let mut error_count = 0;
        loop {
            let descriptor = format!("addr({})", miner_address);
            let rpc_result: JsonValue = base_builder(
                &bitcoin_node_url,
                &devnet_config.bitcoin_node_username,
                &devnet_config.bitcoin_node_password,
            )
            .json(&json!({
                "jsonrpc": "1.0",
                "id": "stacks-network",
                "method": "getdescriptorinfo",
                "params": [json!(descriptor)]

            }))
            .send()
            .await
            .map_err(|e| format!("unable to send 'getdescriptorinfo' request ({})", e))
            .map_err(|e| format!("unable to receive 'getdescriptorinfo' response: {}", e))?
            .json()
            .await
            .map_err(|e| format!("unable to parse 'getdescriptorinfo' result: {}", e))?;

            let checksum = rpc_result
                .as_object()
                .ok_or(format!("unable to parse 'getdescriptorinfo'"))?
                .get("result")
                .ok_or(format!("unable to parse 'getdescriptorinfo'"))?
                .as_object()
                .ok_or(format!("unable to parse 'getdescriptorinfo'"))?
                .get("checksum")
                .ok_or(format!("unable to parse 'getdescriptorinfo'"))?
                .as_str()
                .ok_or(format!("unable to parse 'getdescriptorinfo'"))?
                .to_string();

            let _ = devnet_event_tx.send(DevnetEvent::info(format!(
                "Registering {descriptor}#{checksum}"
            )));
            let payload = json!({
                "jsonrpc": "1.0",
                "id": "stacks-network",
                "method": "importdescriptors",
                "params": {
                    "requests": [{
                        "desc": format!("{}#{}", descriptor, checksum),
                        "timestamp": 0,
                    }]
                }
            });
            let rpc_call = base_builder(
                &bitcoin_node_url,
                &devnet_config.bitcoin_node_username,
                &devnet_config.bitcoin_node_password,
            )
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("unable to send 'importdescriptors' request ({})", e));

            match rpc_call {
                Ok(_r) => {
                    break;
                }
                Err(e) => {
                    error_count += 1;
                    if error_count > max_errors {
                        return Err(e);
                    } else if error_count > 1 {
                        let _ = devnet_event_tx.send(DevnetEvent::error(e));
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
            let _ = devnet_event_tx.send(DevnetEvent::info(format!("Waiting for bitcoin-node",)));
        }

        let mut error_count = 0;
        loop {
            let descriptor = format!("addr({})", faucet_address);
            let rpc_result: JsonValue = base_builder(
                &bitcoin_node_url,
                &devnet_config.bitcoin_node_username,
                &devnet_config.bitcoin_node_password,
            )
            .json(&json!({
                "jsonrpc": "1.0",
                "id": "stacks-network",
                "method": "getdescriptorinfo",
                "params": [json!(descriptor)]

            }))
            .send()
            .await
            .map_err(|e| format!("unable to send 'getdescriptorinfo' request ({})", e))
            .map_err(|e| format!("unable to receive 'getdescriptorinfo' response: {}", e))?
            .json()
            .await
            .map_err(|e| format!("unable to parse 'getdescriptorinfo' result: {}", e))?;

            let checksum = rpc_result
                .as_object()
                .ok_or(format!("unable to parse 'getdescriptorinfo'"))?
                .get("result")
                .ok_or(format!("unable to parse 'getdescriptorinfo'"))?
                .as_object()
                .ok_or(format!("unable to parse 'getdescriptorinfo'"))?
                .get("checksum")
                .ok_or(format!("unable to parse 'getdescriptorinfo'"))?
                .as_str()
                .ok_or(format!("unable to parse 'getdescriptorinfo'"))?
                .to_string();

            let _ = devnet_event_tx.send(DevnetEvent::info(format!(
                "Registering {descriptor}#{checksum}"
            )));
            let payload = json!({
                "jsonrpc": "1.0",
                "id": "stacks-network",
                "method": "importdescriptors",
                "params": {
                    "requests": [{
                        "desc": format!("{}#{}", descriptor, checksum),
                        "timestamp": 0,
                    }]
                }
            });
            let rpc_call = base_builder(
                &bitcoin_node_url,
                &devnet_config.bitcoin_node_username,
                &devnet_config.bitcoin_node_password,
            )
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("unable to send 'importdescriptors' request ({})", e));

            match rpc_call {
                Ok(_r) => {
                    break;
                }
                Err(e) => {
                    error_count += 1;
                    if error_count > max_errors {
                        return Err(e);
                    } else if error_count > 1 {
                        let _ = devnet_event_tx.send(DevnetEvent::error(e));
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
            let _ = devnet_event_tx.send(DevnetEvent::info(format!("Waiting for bitcoin-node",)));
        }
        // Index devnet's wallets by default
        for (_, account) in accounts.iter() {
            let address = Address::from_str(&account.btc_address)
                .map_err(|e| format!("unable to create address: {:?}", e))?;

            let mut error_count = 0;
            loop {
                let descriptor = format!("addr({})", address);
                let rpc_result: JsonValue = base_builder(
                    &bitcoin_node_url,
                    &devnet_config.bitcoin_node_username,
                    &devnet_config.bitcoin_node_password,
                )
                .json(&json!({
                    "jsonrpc": "1.0",
                    "id": "stacks-network",
                    "method": "getdescriptorinfo",
                    "params": [json!(descriptor)]

                }))
                .send()
                .await
                .map_err(|e| format!("unable to send 'getdescriptorinfo' request ({})", e))
                .map_err(|e| format!("unable to receive 'getdescriptorinfo' response: {}", e))?
                .json()
                .await
                .map_err(|e| format!("unable to parse 'getdescriptorinfo' result: {}", e))?;

                let checksum = rpc_result
                    .as_object()
                    .ok_or(format!("unable to parse 'getdescriptorinfo'"))?
                    .get("result")
                    .ok_or(format!("unable to parse 'getdescriptorinfo'"))?
                    .as_object()
                    .ok_or(format!("unable to parse 'getdescriptorinfo'"))?
                    .get("checksum")
                    .ok_or(format!("unable to parse 'getdescriptorinfo'"))?
                    .as_str()
                    .ok_or(format!("unable to parse 'getdescriptorinfo'"))?
                    .to_string();

                let _ = devnet_event_tx.send(DevnetEvent::info(format!(
                    "Registering {descriptor}#{checksum}"
                )));
                let payload = json!({
                    "jsonrpc": "1.0",
                    "id": "stacks-network",
                    "method": "importdescriptors",
                    "params": {
                        "requests": [{
                            "desc": format!("{}#{}", descriptor, checksum),
                            "timestamp": 0,
                        }]
                    }
                });
                let rpc_call = base_builder(
                    &bitcoin_node_url,
                    &devnet_config.bitcoin_node_username,
                    &devnet_config.bitcoin_node_password,
                )
                .json(&payload)
                .send()
                .await
                .map_err(|e| format!("unable to send 'importdescriptors' request ({})", e));

                match rpc_call {
                    Ok(_r) => {
                        break;
                    }
                    Err(e) => {
                        error_count += 1;
                        if error_count > max_errors {
                            return Err(e);
                        } else if error_count > 1 {
                            let _ = devnet_event_tx.send(DevnetEvent::error(e));
                        }
                    }
                }
                std::thread::sleep(std::time::Duration::from_secs(1));
                let _ =
                    devnet_event_tx.send(DevnetEvent::info(format!("Waiting for bitcoin-node",)));
            }
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

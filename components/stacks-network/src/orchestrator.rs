use super::DevnetEvent;
use clarinet_files::chainhook_types::StacksNetwork;
use clarinet_files::{DevnetConfigFile, NetworkManifest, ProjectManifest};
use reqwest::RequestBuilder;
use serde_json::Value as JsonValue;
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::time::Duration;

#[derive(Debug)]
pub struct DevnetOrchestrator {
    pub name: String,
    pub manifest: ProjectManifest,
    pub network_config: Option<NetworkManifest>,
    pub termination_success_tx: Option<Sender<bool>>,
    pub can_exit: bool,
    services_map_hosts: Option<ServicesMapHosts>,
}

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

        Ok(DevnetOrchestrator {
            name,
            manifest,
            network_config: Some(network_config),
            can_exit: true,
            termination_success_tx: None,
            services_map_hosts: None,
        })
    }

    pub fn set_services_map_hosts(&mut self, hosts: ServicesMapHosts) {
        self.services_map_hosts = Some(hosts);
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

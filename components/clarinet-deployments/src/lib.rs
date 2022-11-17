use clarity_repl::clarity::stacks_common::types::StacksEpochId;
use clarity_repl::clarity::ClarityVersion;
use clarity_repl::repl::DEFAULT_EPOCH;
use clarity_repl::repl::{ClarityCodeSource, ClarityContract, ContractDeployer};

extern crate serde;

#[macro_use]
extern crate serde_derive;

#[cfg(feature = "onchain")]
pub mod onchain;
pub mod requirements;
pub mod types;

use self::types::{
    DeploymentSpecification, EmulatedContractPublishSpecification, GenesisSpecification,
    TransactionPlanSpecification, TransactionsBatchSpecification, WalletSpecification,
};
use chainhook_types::StacksNetwork;
use clarinet_files::{FileAccessor, FileLocation};
use clarinet_files::{NetworkManifest, ProjectManifest};
use clarity_repl::analysis::ast_dependency_detector::{ASTDependencyDetector, DependencySet};
use clarity_repl::clarity::vm::ast::ContractAST;
use clarity_repl::clarity::vm::diagnostic::Diagnostic;
use clarity_repl::clarity::vm::types::PrincipalData;
use clarity_repl::clarity::vm::types::QualifiedContractIdentifier;
use clarity_repl::clarity::vm::ContractName;
use clarity_repl::clarity::vm::EvaluationResult;
use clarity_repl::clarity::vm::ExecutionResult;
use clarity_repl::repl::session::BOOT_CONTRACTS_DATA;
use clarity_repl::repl::Session;
use clarity_repl::repl::SessionSettings;
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use types::ContractPublishSpecification;
use types::DeploymentGenerationArtifacts;
use types::RequirementPublishSpecification;
use types::TransactionSpecification;

pub fn setup_session_with_deployment(
    manifest: &ProjectManifest,
    deployment: &DeploymentSpecification,
    contracts_asts: Option<&HashMap<QualifiedContractIdentifier, ContractAST>>,
) -> DeploymentGenerationArtifacts {
    let mut session = initiate_session_from_deployment(&manifest);
    update_session_with_genesis_accounts(&mut session, deployment);
    let results = update_session_with_contracts_executions(
        &mut session,
        deployment,
        contracts_asts,
        false,
        None,
    );

    let deps = HashMap::new();
    let mut diags = HashMap::new();
    let mut asts = HashMap::new();
    let mut contracts_analysis = HashMap::new();
    let mut success = true;
    for (contract_id, res) in results.into_iter() {
        match res {
            Ok(execution_result) => {
                diags.insert(contract_id.clone(), execution_result.diagnostics);
                match execution_result.result {
                    EvaluationResult::Contract(contract_result) => {
                        asts.insert(contract_id.clone(), contract_result.contract.ast);
                        contracts_analysis.insert(contract_id, contract_result.contract.analysis);
                    }
                    _ => (),
                }
            }
            Err(errors) => {
                success = false;
                diags.insert(contract_id.clone(), errors);
            }
        }
    }

    let artifacts = DeploymentGenerationArtifacts {
        asts,
        deps,
        diags,
        success,
        session,
        analysis: contracts_analysis,
    };
    artifacts
}

pub fn initiate_session_from_deployment(manifest: &ProjectManifest) -> Session {
    let mut settings = SessionSettings::default();
    settings.repl_settings = manifest.repl_settings.clone();
    settings.disk_cache_enabled = true;
    let session = Session::new(settings);
    session
}

pub fn update_session_with_genesis_accounts(
    session: &mut Session,
    deployment: &DeploymentSpecification,
) {
    if let Some(ref spec) = deployment.genesis {
        for wallet in spec.wallets.iter() {
            let _ = session.interpreter.mint_stx_balance(
                wallet.address.clone().into(),
                wallet.balance.try_into().unwrap(),
            );
            if wallet.name == "deployer" {
                session.set_tx_sender(wallet.address.to_address());
            }
        }
        session.load_boot_contracts();
    }
}

pub fn update_session_with_contracts_executions(
    session: &mut Session,
    deployment: &DeploymentSpecification,
    contracts_asts: Option<&HashMap<QualifiedContractIdentifier, ContractAST>>,
    code_coverage_enabled: bool,
    forced_epoch: Option<StacksEpochId>,
) -> BTreeMap<QualifiedContractIdentifier, Result<ExecutionResult, Vec<Diagnostic>>> {
    let boot_contracts_data = BOOT_CONTRACTS_DATA.clone();
    for (_, (boot_contract, mut ast)) in boot_contracts_data {
        session
            .interpreter
            .run_ast(&boot_contract, &mut ast, false, None)
            .expect("failed to interprete boot contract");
    }

    let mut results = BTreeMap::new();
    for batch in deployment.plan.batches.iter() {
        for transaction in batch.transactions.iter() {
            match transaction {
                TransactionSpecification::RequirementPublish(_)
                | TransactionSpecification::BtcTransfer(_)
                | TransactionSpecification::ContractCall(_)
                | TransactionSpecification::ContractPublish(_) => {
                    panic!("emulated-contract-call and emulated-contract-publish are the only operations admitted in simnet deployments")
                }
                TransactionSpecification::StxTransfer(tx) => {
                    let default_tx_sender = session.get_tx_sender();
                    session.set_tx_sender(tx.expected_sender.to_string());
                    let _ = session.stx_transfer(tx.mstx_amount, &tx.recipient.to_string());
                    session.set_tx_sender(default_tx_sender);
                }
                TransactionSpecification::EmulatedContractPublish(tx) => {
                    let default_tx_sender = session.get_tx_sender();
                    session.set_tx_sender(tx.emulated_sender.to_string());

                    let contract_id = QualifiedContractIdentifier::new(
                        tx.emulated_sender.clone(),
                        tx.contract_name.clone(),
                    );
                    let mut contract_ast = contracts_asts
                        .as_ref()
                        .and_then(|m| m.get(&contract_id))
                        .and_then(|c| Some(c.clone()));
                    let contract = ClarityContract {
                        code_source: ClarityCodeSource::ContractInMemory(tx.source.clone()),
                        deployer: ContractDeployer::Address(tx.emulated_sender.to_string()),
                        name: tx.contract_name.to_string(),
                        clarity_version: tx.clarity_version,
                        epoch: forced_epoch.unwrap_or(DEFAULT_EPOCH),
                    };

                    let result = session.deploy_contract(
                        &contract,
                        None,
                        false,
                        match code_coverage_enabled {
                            true => Some("__analysis__".to_string()),
                            false => None,
                        },
                        &mut contract_ast,
                    );
                    results.insert(contract_id, result);
                    session.set_tx_sender(default_tx_sender);
                }
                TransactionSpecification::EmulatedContractCall(tx) => {
                    let _ = session.invoke_contract_call(
                        &tx.contract_id.to_string(),
                        &tx.method.to_string(),
                        &tx.parameters,
                        &tx.emulated_sender.to_string(),
                        "deployment".to_string(),
                    );
                }
            }
        }
        session.advance_chain_tip(1);
    }
    results
}

pub async fn generate_default_deployment(
    manifest: &ProjectManifest,
    network: &StacksNetwork,
    no_batch: bool,
    file_accessor: Option<&Box<dyn FileAccessor>>,
    forced_epoch: Option<StacksEpochId>,
) -> Result<(DeploymentSpecification, DeploymentGenerationArtifacts), String> {
    let network_manifest = match file_accessor {
        None => NetworkManifest::from_project_manifest_location(
            &manifest.location,
            &network.get_networks(),
            Some(&manifest.project.cache_location),
        )?,
        Some(file_accessor) => {
            NetworkManifest::from_project_manifest_location_using_file_accessor(
                &manifest.location,
                &network.get_networks(),
                file_accessor,
            )
            .await?
        }
    };

    let (stacks_node, bitcoin_node) = match network {
        StacksNetwork::Simnet => (None, None),
        StacksNetwork::Devnet => {
            let (stacks_node, bitcoin_node) = match network_manifest.devnet {
                Some(ref devnet) => {
                    let stacks_node = format!("http://localhost:{}", devnet.stacks_node_rpc_port);
                    let bitcoin_node = format!(
                        "http://{}:{}@localhost:{}",
                        devnet.bitcoin_node_username,
                        devnet.bitcoin_node_password,
                        devnet.bitcoin_node_rpc_port
                    );
                    (stacks_node, bitcoin_node)
                }
                None => {
                    let stacks_node = format!("http://localhost:20443");
                    let bitcoin_node = format!("http://devnet:devnet@localhost:18443");
                    (stacks_node, bitcoin_node)
                }
            };
            (Some(stacks_node), Some(bitcoin_node))
        }
        StacksNetwork::Testnet => {
            let stacks_node = network_manifest
                .network
                .stacks_node_rpc_address
                .unwrap_or("http://stacks-node-api.testnet.stacks.co".to_string());
            let bitcoin_node =
                network_manifest
                    .network
                    .bitcoin_node_rpc_address
                    .unwrap_or(format!(
                        "http://blockstack:blockstacksystem@bitcoind.testnet.stacks.co:18332"
                    ));
            (Some(stacks_node), Some(bitcoin_node))
        }
        StacksNetwork::Mainnet => {
            let stacks_node = network_manifest
                .network
                .stacks_node_rpc_address
                .unwrap_or("http://stacks-node-api.mainnet.stacks.co".to_string());
            let bitcoin_node = network_manifest.network.bitcoin_node_rpc_address.unwrap_or(
                "http://blockstack:blockstacksystem@bitcoin.blockstack.com:8332".to_string(),
            );
            (Some(stacks_node), Some(bitcoin_node))
        }
    };

    let deployment_fee_rate = network_manifest.network.deployment_fee_rate;

    let default_deployer = match network_manifest.accounts.get("deployer") {
        Some(deployer) => deployer,
        None => {
            return Err(format!("unable to retrieve default deployer account"));
        }
    };
    let default_deployer_address =
        match PrincipalData::parse_standard_principal(&default_deployer.stx_address) {
            Ok(res) => res,
            Err(_) => {
                return Err(format!(
                    "unable to turn address {} as a valid Stacks address",
                    default_deployer.stx_address
                ))
            }
        };

    let mut transactions = vec![];
    let mut contracts_map = BTreeMap::new();
    let mut requirements_asts = BTreeMap::new();
    let mut requirements_deps = HashMap::new();

    let mut settings = SessionSettings::default();
    settings.repl_settings = manifest.repl_settings.clone();

    let session = Session::new(settings.clone());

    let boot_contracts_data = BOOT_CONTRACTS_DATA.clone();
    let mut boot_contracts_ids = BTreeSet::new();
    let mut boot_contracts_asts = BTreeMap::new();
    for (id, (_, ast)) in boot_contracts_data {
        boot_contracts_ids.insert(id.clone());
        boot_contracts_asts.insert(id, ast);
    }
    requirements_asts.append(&mut boot_contracts_asts);

    let mut queue = VecDeque::new();

    if let Some(ref devnet) = network_manifest.devnet {
        if devnet.enable_subnet_node {
            let contract_id = match QualifiedContractIdentifier::parse(&devnet.subnet_contract_id) {
                Ok(contract_id) => contract_id,
                Err(_e) => {
                    return Err(format!(
                        "malformatted subnet_contract_id: {}",
                        devnet.subnet_contract_id
                    ))
                }
            };
            queue.push_front(contract_id)
        }
    }

    // Build the ASTs / DependencySet for requirements - step required for Simnet/Devnet/Testnet/Mainnet
    if let Some(ref requirements) = manifest.project.requirements {
        let cache_location = &manifest.project.cache_location;
        let mut emulated_contracts_publish = HashMap::new();
        let mut requirements_publish = HashMap::new();

        // Load all the requirements
        // Some requirements are explicitly listed, some are discovered as we compute the ASTs.

        for requirement in requirements.iter() {
            let contract_id = match QualifiedContractIdentifier::parse(&requirement.contract_id) {
                Ok(contract_id) => contract_id,
                Err(_e) => {
                    return Err(format!(
                        "malformatted contract_id: {}",
                        requirement.contract_id
                    ))
                }
            };
            queue.push_front(contract_id);
        }

        while let Some(contract_id) = queue.pop_front() {
            // Extract principal from contract_id
            if requirements_deps.contains_key(&contract_id) {
                continue;
            }

            // Did we already get the source in a prior cycle?
            let ast = match requirements_asts.remove(&contract_id) {
                Some(ast) => ast,
                None => {
                    // Download the code
                    let (source, clarity_version, contract_location) =
                        requirements::retrieve_contract(
                            &contract_id,
                            &cache_location,
                            &file_accessor,
                        )
                        .await?;

                    // Build the struct representing the requirement in the deployment
                    if network.is_simnet() {
                        let data = EmulatedContractPublishSpecification {
                            contract_name: contract_id.name.clone(),
                            emulated_sender: contract_id.issuer.clone(),
                            source: source.clone(),
                            location: contract_location,
                            clarity_version: ClarityVersion::Clarity1,
                        };
                        emulated_contracts_publish.insert(contract_id.clone(), data);
                    } else if network.either_devnet_or_testnet() {
                        let mut remap_principals = BTreeMap::new();
                        remap_principals
                            .insert(contract_id.issuer.clone(), default_deployer_address.clone());
                        match network_manifest.devnet {
                            Some(ref devnet)
                                if devnet.subnet_contract_id == contract_id.to_string() =>
                            {
                                remap_principals.insert(
                                    contract_id.issuer.clone(),
                                    PrincipalData::parse_standard_principal(
                                        &devnet.subnet_leader_stx_address,
                                    )
                                    .unwrap(),
                                );
                            }
                            _ => {}
                        }
                        let data = RequirementPublishSpecification {
                            contract_id: contract_id.clone(),
                            remap_sender: default_deployer_address.clone(),
                            source: source.clone(),
                            location: contract_location,
                            cost: deployment_fee_rate * source.len() as u64,
                            remap_principals,
                            clarity_version,
                        };
                        requirements_publish.insert(contract_id.clone(), data);
                    }

                    // Compute the AST
                    let contract = ClarityContract {
                        code_source: ClarityCodeSource::ContractInMemory(source),
                        name: contract_id.name.to_string(),
                        deployer: ContractDeployer::ContractIdentifier(contract_id.clone()),
                        clarity_version: ClarityVersion::Clarity1,
                        epoch: forced_epoch.unwrap_or(DEFAULT_EPOCH),
                    };
                    let (ast, _, _) = session.interpreter.build_ast(&contract);
                    ast
                }
            };

            // Detect the eventual dependencies for this AST
            let mut contract_ast = HashMap::new();
            contract_ast.insert(contract_id.clone(), ast);
            let dependencies =
                ASTDependencyDetector::detect_dependencies(&contract_ast, &requirements_asts);
            let ast = contract_ast
                .remove(&contract_id)
                .expect("unable to retrieve ast");

            // Extract the known / unknown dependencies
            match dependencies {
                Ok(inferable_dependencies) => {
                    // Looping could be confusing - in this case, we submitted a HashMap with one contract, so we have at most one
                    // result in the `inferable_dependencies` map. We will just extract and keep the associated data (source, ast, deps).
                    for (contract_id, dependencies) in inferable_dependencies.into_iter() {
                        for dependency in dependencies.iter() {
                            queue.push_back(dependency.contract_id.clone());
                        }
                        requirements_deps.insert(contract_id.clone(), dependencies);
                        requirements_asts.insert(contract_id.clone(), ast);
                        break;
                    }
                }
                Err((inferable_dependencies, non_inferable_dependencies)) => {
                    // In the case of unknown dependencies, we were unable to construct an exhaustive list of dependencies.
                    // As such, we will re-enqueue the present (front) and push all the unknown contract_ids in front of it,
                    // and we will keep the source in memory to avoid useless disk access.
                    for (_, dependencies) in inferable_dependencies.iter() {
                        for dependency in dependencies.iter() {
                            queue.push_back(dependency.contract_id.clone());
                        }
                    }
                    requirements_asts.insert(contract_id.clone(), ast);
                    queue.push_front(contract_id);

                    for non_inferable_contract_id in non_inferable_dependencies.into_iter() {
                        queue.push_front(non_inferable_contract_id);
                    }
                }
            };
        }

        // Avoid listing requirements as deployment transactions to the deployment specification on Mainnet
        if !network.is_mainnet() {
            let mut ordered_contracts_ids =
                match ASTDependencyDetector::order_contracts(&requirements_deps) {
                    Ok(ordered_contracts) => ordered_contracts,
                    Err(e) => return Err(format!("unable to order requirements {}", e)),
                };

            // Filter out boot contracts from requirement dependencies
            ordered_contracts_ids.retain(|contract_id| !boot_contracts_ids.contains(contract_id));

            if network.is_simnet() {
                for contract_id in ordered_contracts_ids.iter() {
                    let data = emulated_contracts_publish
                        .remove(contract_id)
                        .expect("unable to retrieve contract");
                    let tx = TransactionSpecification::EmulatedContractPublish(data);
                    transactions.push(tx);
                }
            } else if network.either_devnet_or_testnet() {
                for contract_id in ordered_contracts_ids.iter() {
                    let data = requirements_publish
                        .remove(contract_id)
                        .expect("unable to retrieve contract");
                    let tx = TransactionSpecification::RequirementPublish(data);
                    transactions.push(tx);
                }
            }
        }
    }

    let mut contracts = HashMap::new();
    let mut contracts_sources = HashMap::new();

    let sources: HashMap<String, String> = match file_accessor {
        None => {
            let mut sources = HashMap::new();
            for (contract_location, _) in manifest.contracts_settings.iter() {
                let source = contract_location.read_content_as_utf8().map_err(|_| {
                    format!(
                        "unable to find contract at {}",
                        contract_location.to_string()
                    )
                })?;
                sources.insert(contract_location.to_string(), source);
            }
            sources
        }
        Some(file_accessor) => {
            let contracts_location = manifest
                .contracts_settings
                .iter()
                .map(|(contract_location, _)| contract_location.to_string())
                .collect();
            file_accessor
                .read_contracts_content(contracts_location)
                .await?
        }
    };

    for (contract_location, contract_config) in manifest.contracts_settings.iter() {
        let contract_name = match ContractName::try_from(contract_config.name.to_string()) {
            Ok(res) => res,
            Err(_) => {
                return Err(format!(
                    "unable to use {} as a valid contract name",
                    contract_config.name
                ))
            }
        };

        let deployer = match &contract_config.deployer {
            ContractDeployer::DefaultDeployer => default_deployer,
            ContractDeployer::LabeledDeployer(deployer) => {
                let deployer = match network_manifest.accounts.get(deployer) {
                    Some(deployer) => deployer,
                    None => {
                        return Err(format!("unable to retrieve account '{}'", deployer));
                    }
                };
                deployer
            }
            _ => unreachable!(),
        };

        let sender = match PrincipalData::parse_standard_principal(&deployer.stx_address) {
            Ok(res) => res,
            Err(_) => {
                return Err(format!(
                    "unable to turn emulated_sender {} as a valid Stacks address",
                    deployer.stx_address
                ))
            }
        };

        let source = sources
            .get(&contract_location.to_string())
            .ok_or(format!(
                "Invalid Clarinet.toml, source file not found for: {}",
                &contract_config.name
            ))?
            .clone();

        let contract_id = QualifiedContractIdentifier::new(sender.clone(), contract_name.clone());

        contracts_sources.insert(
            contract_id.clone(),
            ClarityContract {
                code_source: ClarityCodeSource::ContractInMemory(source.clone()),
                deployer: ContractDeployer::Address(sender.to_address()),
                name: contract_name.to_string(),
                clarity_version: contract_config.clarity_version,
                epoch: forced_epoch.unwrap_or(contract_config.epoch),
            },
        );

        let contract_spec = if network.is_simnet() {
            TransactionSpecification::EmulatedContractPublish(
                EmulatedContractPublishSpecification {
                    contract_name,
                    emulated_sender: sender,
                    source,
                    location: contract_location.clone(),
                    clarity_version: contract_config.clarity_version,
                },
            )
        } else {
            TransactionSpecification::ContractPublish(ContractPublishSpecification {
                contract_name,
                expected_sender: sender,
                location: contract_location.clone(),
                cost: deployment_fee_rate
                    .saturating_mul(source.as_bytes().len().try_into().unwrap()),
                source,
                anchor_block_only: true,
                clarity_version: contract_config.clarity_version,
            })
        };

        contracts.insert(contract_id, contract_spec);
    }

    let session = Session::new(settings);

    let mut contract_asts = HashMap::new();
    let mut contract_diags = HashMap::new();

    let mut asts_success = true;

    for (contract_id, contract) in contracts_sources.into_iter() {
        let (ast, diags, ast_success) = session.interpreter.build_ast(&contract);
        contract_asts.insert(contract_id.clone(), ast);
        contract_diags.insert(contract_id, diags);
        asts_success = asts_success && ast_success;
    }

    let dependencies =
        ASTDependencyDetector::detect_dependencies(&contract_asts, &requirements_asts);

    let mut dependencies = match dependencies {
        Ok(dependencies) => dependencies,
        Err((dependencies, _)) => {
            // No need to report an error here, it will be caught and reported
            // with proper location information by the later analyses.
            dependencies
        }
    };

    for contract_id in boot_contracts_ids.into_iter() {
        dependencies.insert(contract_id.clone(), DependencySet::new());
    }

    dependencies.extend(requirements_deps);

    let ordered_contracts_ids = match ASTDependencyDetector::order_contracts(&dependencies) {
        Ok(ordered_contracts_ids) => ordered_contracts_ids,
        Err(e) => return Err(e.err.to_string()),
    };

    for contract_id in ordered_contracts_ids.into_iter() {
        if requirements_asts.contains_key(&contract_id) {
            continue;
        }
        let tx = contracts
            .remove(&contract_id)
            .expect("unable to retrieve contract");

        match tx {
            TransactionSpecification::EmulatedContractPublish(ref data) => {
                contracts_map.insert(
                    contract_id.clone(),
                    (data.source.clone(), data.location.clone()),
                );
            }
            TransactionSpecification::ContractPublish(ref data) => {
                contracts_map.insert(
                    contract_id.clone(),
                    (data.source.clone(), data.location.clone()),
                );
            }
            _ => unreachable!(),
        }
        transactions.push(tx);
    }

    let tx_chain_limit = match no_batch {
        true => 100_000,
        false => 25,
    };

    let mut batches = vec![];
    for (id, transactions) in transactions.chunks(tx_chain_limit).enumerate() {
        batches.push(TransactionsBatchSpecification {
            id: id,
            transactions: transactions.to_vec(),
        })
    }

    let mut wallets = vec![];
    if network.is_simnet() {
        for (name, account) in network_manifest.accounts.into_iter() {
            let address = match PrincipalData::parse_standard_principal(&account.stx_address) {
                Ok(res) => res,
                Err(_) => {
                    return Err(format!(
                        "unable to parse wallet {} in a valid Stacks address",
                        account.stx_address
                    ))
                }
            };

            wallets.push(WalletSpecification {
                name,
                address,
                balance: account.balance.into(),
            });
        }
    }

    let name = match network {
        StacksNetwork::Simnet => format!("Simulated deployment, used as a default for `clarinet console`, `clarinet test` and `clarinet check`"),
        _ => format!("{:?} deployment", network)
    };

    let deployment = DeploymentSpecification {
        id: 0,
        name,
        stacks_node,
        bitcoin_node,
        network: network.clone(),
        genesis: if network.is_simnet() {
            Some(GenesisSpecification {
                wallets,
                contracts: manifest.project.boot_contracts.clone(),
            })
        } else {
            None
        },
        plan: TransactionPlanSpecification { batches },
        contracts: contracts_map,
    };

    let artifacts = DeploymentGenerationArtifacts {
        asts: contract_asts,
        deps: dependencies,
        diags: contract_diags,
        success: asts_success,
        analysis: HashMap::new(),
        session,
    };

    Ok((deployment, artifacts))
}

pub fn get_default_deployment_path(
    manifest: &ProjectManifest,
    network: &StacksNetwork,
) -> Result<FileLocation, String> {
    let mut deployment_path = manifest.location.get_project_root_location()?;
    deployment_path.append_path("deployments")?;
    deployment_path.append_path(match network {
        StacksNetwork::Simnet => "default.simnet-plan.yaml",
        StacksNetwork::Devnet => "default.devnet-plan.yaml",
        StacksNetwork::Testnet => "default.testnet-plan.yaml",
        StacksNetwork::Mainnet => "default.mainnet-plan.yaml",
    })?;
    Ok(deployment_path)
}

pub fn load_deployment(
    manifest: &ProjectManifest,
    deployment_plan_location: &FileLocation,
) -> Result<DeploymentSpecification, String> {
    let project_root_location = manifest.location.get_project_root_location()?;
    let spec = match DeploymentSpecification::from_config_file(
        &deployment_plan_location,
        &project_root_location,
    ) {
        Ok(spec) => spec,
        Err(msg) => {
            return Err(format!(
                "error: {} syntax incorrect\n{}",
                deployment_plan_location.to_string(),
                msg
            ));
        }
    };
    Ok(spec)
}

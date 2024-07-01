use clarity_repl::clarity::StacksEpochId;
use clarity_repl::repl::{ClarityCodeSource, ClarityContract, ContractDeployer};
use clarity_repl::repl::{DEFAULT_CLARITY_VERSION, DEFAULT_EPOCH};

extern crate serde;

#[macro_use]
extern crate serde_derive;

pub mod diagnostic_digest;
#[cfg(feature = "onchain")]
pub mod onchain;
pub mod requirements;
pub mod types;

#[cfg(test)]
mod deployment_plan_test;

use self::types::{
    DeploymentSpecification, EmulatedContractPublishSpecification, GenesisSpecification,
    TransactionPlanSpecification, TransactionsBatchSpecification, WalletSpecification,
};
use clarinet_files::chainhook_types::StacksNetwork;
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
use types::DeploymentGenerationArtifacts;
use types::RequirementPublishSpecification;
use types::TransactionSpecification;
use types::{ContractPublishSpecification, EpochSpec};

pub type ExecutionResultMap =
    BTreeMap<QualifiedContractIdentifier, Result<ExecutionResult, Vec<Diagnostic>>>;

pub struct UpdateSessionExecutionResult {
    pub boot_contracts: ExecutionResultMap,
    pub contracts: ExecutionResultMap,
}

pub fn setup_session_with_deployment(
    manifest: &ProjectManifest,
    deployment: &DeploymentSpecification,
    contracts_asts: Option<&BTreeMap<QualifiedContractIdentifier, ContractAST>>,
) -> DeploymentGenerationArtifacts {
    let mut session = initiate_session_from_deployment(manifest);
    update_session_with_genesis_accounts(&mut session, deployment);
    let UpdateSessionExecutionResult { contracts, .. } = update_session_with_contracts_executions(
        &mut session,
        deployment,
        contracts_asts,
        false,
        None,
    );

    let deps = BTreeMap::new();
    let mut diags = HashMap::new();
    let mut results_values = HashMap::new();
    let mut asts = BTreeMap::new();
    let mut contracts_analysis = HashMap::new();
    let mut success = true;
    for (contract_id, res) in contracts.into_iter() {
        match res {
            Ok(execution_result) => {
                diags.insert(contract_id.clone(), execution_result.diagnostics);
                if let EvaluationResult::Contract(contract_result) = execution_result.result {
                    results_values.insert(contract_id.clone(), contract_result.result);
                    asts.insert(contract_id.clone(), contract_result.contract.ast);
                    contracts_analysis.insert(contract_id, contract_result.contract.analysis);
                }
            }
            Err(errors) => {
                success = false;
                diags.insert(contract_id.clone(), errors);
            }
        }
    }

    DeploymentGenerationArtifacts {
        asts,
        deps,
        diags,
        results_values,
        success,
        session,
        analysis: contracts_analysis,
    }
}

pub fn initiate_session_from_deployment(manifest: &ProjectManifest) -> Session {
    let settings = SessionSettings {
        repl_settings: manifest.repl_settings.clone(),
        disk_cache_enabled: true,
        ..Default::default()
    };
    Session::new(settings)
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
                session.set_tx_sender(&wallet.address.to_string());
            }
        }
    }
}

pub fn update_session_with_contracts_executions(
    session: &mut Session,
    deployment: &DeploymentSpecification,
    contracts_asts: Option<&BTreeMap<QualifiedContractIdentifier, ContractAST>>,
    code_coverage_enabled: bool,
    forced_min_epoch: Option<StacksEpochId>,
) -> UpdateSessionExecutionResult {
    let boot_contracts_data = BOOT_CONTRACTS_DATA.clone();

    let mut boot_contracts = BTreeMap::new();
    for (contract_id, (boot_contract, ast)) in boot_contracts_data {
        let result = session
            .interpreter
            .run(&boot_contract, &mut Some(ast), false, None);
        boot_contracts.insert(contract_id, result);
    }

    let mut contracts = BTreeMap::new();
    for batch in deployment.plan.batches.iter() {
        let epoch: StacksEpochId = match (batch.epoch, forced_min_epoch) {
            (Some(epoch), _) => epoch.into(),
            (None, Some(min_epoch)) => std::cmp::max(min_epoch, DEFAULT_EPOCH),
            _ => DEFAULT_EPOCH,
        };
        session.advance_chain_tip(1);
        session.update_epoch(epoch);

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
                    session.set_tx_sender(&tx.expected_sender.to_string());
                    let _ = session.stx_transfer(tx.mstx_amount, &tx.recipient.to_string());
                    session.set_tx_sender(&default_tx_sender);
                }
                TransactionSpecification::EmulatedContractPublish(tx) => {
                    let default_tx_sender = session.get_tx_sender();
                    session.set_tx_sender(&tx.emulated_sender.to_string());

                    let contract_id = QualifiedContractIdentifier::new(
                        tx.emulated_sender.clone(),
                        tx.contract_name.clone(),
                    );
                    let mut contract_ast = contracts_asts
                        .as_ref()
                        .and_then(|m| m.get(&contract_id))
                        .cloned();
                    let contract = ClarityContract {
                        code_source: ClarityCodeSource::ContractInMemory(tx.source.clone()),
                        deployer: ContractDeployer::Address(tx.emulated_sender.to_string()),
                        name: tx.contract_name.to_string(),
                        clarity_version: tx.clarity_version,
                        epoch,
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
                    contracts.insert(contract_id, result);
                    session.set_tx_sender(&default_tx_sender);
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
    }
    UpdateSessionExecutionResult {
        boot_contracts,
        contracts,
    }
}

pub async fn generate_default_deployment(
    manifest: &ProjectManifest,
    network: &StacksNetwork,
    no_batch: bool,
    file_accessor: Option<&dyn FileAccessor>,
    forced_min_epoch: Option<StacksEpochId>,
) -> Result<(DeploymentSpecification, DeploymentGenerationArtifacts), String> {
    let network_manifest = match file_accessor {
        None => NetworkManifest::from_project_manifest_location(
            &manifest.location,
            &network.get_networks(),
            Some(&manifest.project.cache_location),
            None,
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
                    let stacks_node = "http://localhost:20443".to_string();
                    let bitcoin_node = "http://devnet:devnet@localhost:18443".to_string();
                    (stacks_node, bitcoin_node)
                }
            };
            (Some(stacks_node), Some(bitcoin_node))
        }
        StacksNetwork::Testnet => {
            let stacks_node = network_manifest
                .network
                .stacks_node_rpc_address
                .unwrap_or("https://api.testnet.hiro.so".to_string());
            let bitcoin_node = network_manifest.network.bitcoin_node_rpc_address.unwrap_or(
                "http://blockstack:blockstacksystem@bitcoind.testnet.stacks.co:18332".to_string(),
            );
            (Some(stacks_node), Some(bitcoin_node))
        }
        StacksNetwork::Mainnet => {
            let stacks_node = network_manifest
                .network
                .stacks_node_rpc_address
                .unwrap_or("https://api.hiro.so".to_string());
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
            return Err("unable to retrieve default deployer account".to_string());
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

    let mut transactions = BTreeMap::new();
    let mut contracts_map = BTreeMap::new();
    let mut requirements_data = BTreeMap::new();
    let mut requirements_deps = BTreeMap::new();

    let settings = SessionSettings {
        repl_settings: manifest.repl_settings.clone(),
        ..Default::default()
    };

    let session = Session::new(settings.clone());

    let boot_contracts_data = BOOT_CONTRACTS_DATA.clone();
    let mut boot_contracts_ids = BTreeSet::new();
    let mut boot_contracts_asts = BTreeMap::new();
    for (id, (contract, ast)) in boot_contracts_data {
        boot_contracts_ids.insert(id.clone());
        boot_contracts_asts.insert(id, (contract.clarity_version, ast));
    }
    requirements_data.append(&mut boot_contracts_asts);

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
            queue.push_front((contract_id, Some(DEFAULT_CLARITY_VERSION)));
        }
    }

    let mut contract_epochs = HashMap::new();

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
            queue.push_front((contract_id, None));
        }

        while let Some((contract_id, forced_clarity_version)) = queue.pop_front() {
            if requirements_deps.contains_key(&contract_id) {
                continue;
            }

            // Did we already get the source in a prior cycle?
            let requirement_data = match requirements_data.remove(&contract_id) {
                Some(requirement_data) => requirement_data,
                None => {
                    // Download the code
                    let (source, epoch, clarity_version, contract_location) =
                        requirements::retrieve_contract(
                            &contract_id,
                            cache_location,
                            &file_accessor,
                        )
                        .await?;

                    let epoch = match forced_min_epoch {
                        Some(min_epoch) => std::cmp::max(min_epoch, epoch),
                        None => epoch,
                    };

                    contract_epochs.insert(contract_id.clone(), epoch);

                    // Build the struct representing the requirement in the deployment
                    if network.is_simnet() {
                        let data = EmulatedContractPublishSpecification {
                            contract_name: contract_id.name.clone(),
                            emulated_sender: contract_id.issuer.clone(),
                            source: source.clone(),
                            location: contract_location,
                            clarity_version,
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
                        clarity_version,
                        epoch,
                    };
                    let (ast, _, _) = session.interpreter.build_ast(&contract);
                    (clarity_version, ast)
                }
            };

            // Detect the eventual dependencies for this AST
            let mut contract_data = BTreeMap::new();
            let (_, ast) = requirement_data;
            let clarity_version = match forced_clarity_version {
                Some(clarity_version) => clarity_version,
                None => {
                    let (_, _, clarity_version, _) = requirements::retrieve_contract(
                        &contract_id,
                        cache_location,
                        &file_accessor,
                    )
                    .await?;
                    clarity_version
                }
            };
            contract_data.insert(contract_id.clone(), (clarity_version, ast));
            let dependencies =
                ASTDependencyDetector::detect_dependencies(&contract_data, &requirements_data);
            let (_, ast) = contract_data
                .remove(&contract_id)
                .expect("unable to retrieve ast");

            // Extract the known / unknown dependencies
            match dependencies {
                Ok(inferable_dependencies) => {
                    if inferable_dependencies.len() > 1 {
                        println!("warning: inferable_dependencies contains more than one entry");
                    }
                    // We submitted a HashMap with one contract, so we have at most one result in the `inferable_dependencies` map.
                    // We will extract and keep the associated data (source, ast, deps).
                    if let Some((contract_id, dependencies)) =
                        inferable_dependencies.into_iter().next()
                    {
                        for dependency in dependencies.iter() {
                            queue.push_back((dependency.contract_id.clone(), None));
                        }
                        requirements_deps.insert(contract_id.clone(), dependencies);
                        requirements_data.insert(contract_id.clone(), (clarity_version, ast));
                    }
                }
                Err((inferable_dependencies, non_inferable_dependencies)) => {
                    // In the case of unknown dependencies, we were unable to construct an exhaustive list of dependencies.
                    // As such, we will re-enqueue the present (front) and push all the unknown contract_ids in front of it,
                    // and we will keep the source in memory to avoid useless disk access.
                    for (_, dependencies) in inferable_dependencies.iter() {
                        for dependency in dependencies.iter() {
                            queue.push_back((dependency.contract_id.clone(), None));
                        }
                    }
                    requirements_data.insert(contract_id.clone(), (clarity_version, ast));
                    queue.push_front((contract_id, None));

                    for non_inferable_contract_id in non_inferable_dependencies.into_iter() {
                        queue.push_front((non_inferable_contract_id, None));
                    }
                }
            };
        }

        // Avoid listing requirements as deployment transactions to the deployment specification on Mainnet
        if !network.is_mainnet() {
            let mut ordered_contracts_ids = match ASTDependencyDetector::order_contracts(
                &requirements_deps,
                &contract_epochs,
            ) {
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
                    add_transaction_to_epoch(
                        &mut transactions,
                        tx,
                        &contract_epochs[contract_id].into(),
                    );
                }
            } else if network.either_devnet_or_testnet() {
                for contract_id in ordered_contracts_ids.iter() {
                    let data = requirements_publish
                        .remove(contract_id)
                        .expect("unable to retrieve contract");
                    let tx = TransactionSpecification::RequirementPublish(data);
                    add_transaction_to_epoch(
                        &mut transactions,
                        tx,
                        &contract_epochs[contract_id].into(),
                    );
                }
            }
        }
    }

    let mut contracts = HashMap::new();
    let mut contracts_sources = HashMap::new();

    let base_location = manifest.location.clone().get_parent_location()?;

    let sources: HashMap<String, String> = match file_accessor {
        None => {
            let mut sources = HashMap::new();
            for (_, contract_config) in manifest.contracts.iter() {
                let mut contract_location = base_location.clone();
                contract_location
                    .append_path(contract_config.expect_contract_path_as_str())
                    .map_err(|_| {
                        format!(
                            "unable to build path for contract {}",
                            contract_config.expect_contract_path_as_str()
                        )
                    })?;

                let source = contract_location
                    .read_content_as_utf8()
                    .map_err(|_| format!("unable to find contract at {}", contract_location))?;
                sources.insert(contract_location.to_string(), source);
            }
            sources
        }
        Some(file_accessor) => {
            let contracts_location = manifest
                .contracts
                .values()
                .map(|contract_config| {
                    let mut contract_location = base_location.clone();
                    contract_location
                        .append_path(contract_config.expect_contract_path_as_str())
                        .unwrap();
                    contract_location.to_string()
                })
                .collect();
            file_accessor.read_files(contracts_location).await?
        }
    };

    for (name, contract_config) in manifest.contracts.iter() {
        let contract_name = match ContractName::try_from(name.to_string()) {
            Ok(res) => res,
            Err(_) => return Err(format!("unable to use {} as a valid contract name", name)),
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

        let mut contract_location = base_location.clone();
        contract_location.append_path(contract_config.expect_contract_path_as_str())?;
        let source = sources
            .get(&contract_location.to_string())
            .ok_or(format!(
                "Invalid Clarinet.toml, source file not found for: {}",
                &name
            ))?
            .clone();

        let contract_id = QualifiedContractIdentifier::new(sender.clone(), contract_name.clone());

        let epoch = match forced_min_epoch {
            Some(min_epoch) => std::cmp::max(min_epoch, contract_config.epoch),
            None => contract_config.epoch,
        };

        contracts_sources.insert(
            contract_id.clone(),
            ClarityContract {
                code_source: ClarityCodeSource::ContractInMemory(source.clone()),
                deployer: ContractDeployer::Address(sender.to_address()),
                name: contract_name.to_string(),
                clarity_version: contract_config.clarity_version,
                epoch,
            },
        );

        let contract_spec = if network.is_simnet() {
            TransactionSpecification::EmulatedContractPublish(
                EmulatedContractPublishSpecification {
                    contract_name,
                    emulated_sender: sender,
                    source,
                    location: contract_location,
                    clarity_version: contract_config.clarity_version,
                },
            )
        } else {
            TransactionSpecification::ContractPublish(ContractPublishSpecification {
                contract_name,
                expected_sender: sender,
                location: contract_location,
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

    let mut contract_asts = BTreeMap::new();
    let mut contract_data = BTreeMap::new();
    let mut contract_diags = HashMap::new();

    let mut asts_success = true;

    for (contract_id, contract) in contracts_sources.into_iter() {
        let (ast, diags, ast_success) = session.interpreter.build_ast(&contract);
        contract_asts.insert(contract_id.clone(), ast.clone());
        contract_data.insert(contract_id.clone(), (contract.clarity_version, ast));
        contract_diags.insert(contract_id.clone(), diags);
        contract_epochs.insert(contract_id, contract.epoch);
        asts_success = asts_success && ast_success;
    }

    let dependencies =
        ASTDependencyDetector::detect_dependencies(&contract_data, &requirements_data);

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

    let ordered_contracts_ids =
        match ASTDependencyDetector::order_contracts(&dependencies, &contract_epochs) {
            Ok(ordered_contracts_ids) => ordered_contracts_ids,
            Err(e) => return Err(e.err.to_string()),
        };

    // Track the latest epoch that a contract is deployed in, so that we can
    // ensure that all contracts are deployed after their dependencies.
    for contract_id in ordered_contracts_ids.into_iter() {
        if requirements_data.contains_key(contract_id) {
            continue;
        }
        let tx = contracts
            .remove(contract_id)
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
        add_transaction_to_epoch(&mut transactions, tx, &contract_epochs[contract_id].into());
    }

    let tx_chain_limit = match no_batch {
        true => 100_000,
        false => 25,
    };

    let mut batches = vec![];
    let mut batch_count = 0;
    for (epoch, epoch_transactions) in transactions {
        for txs in epoch_transactions.chunks(tx_chain_limit) {
            batches.push(TransactionsBatchSpecification {
                id: batch_count,
                transactions: txs.to_vec(),
                epoch: Some(epoch),
            });
            batch_count += 1;
        }
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
        StacksNetwork::Simnet => "Simulated deployment, used as a default for `clarinet console`, `clarinet test` and `clarinet check`".to_string(),
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
        results_values: HashMap::new(),
        analysis: HashMap::new(),
        session,
    };

    Ok((deployment, artifacts))
}

fn add_transaction_to_epoch(
    transactions: &mut BTreeMap<EpochSpec, Vec<TransactionSpecification>>,
    transaction: TransactionSpecification,
    epoch: &EpochSpec,
) {
    let epoch_transactions = match transactions.get_mut(epoch) {
        Some(v) => v,
        None => {
            transactions.insert(*epoch, vec![]);
            transactions.get_mut(epoch).unwrap()
        }
    };
    epoch_transactions.push(transaction);
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
        deployment_plan_location,
        &project_root_location,
    ) {
        Ok(spec) => spec,
        Err(msg) => {
            return Err(format!(
                "error: {} syntax incorrect\n{}",
                deployment_plan_location, msg
            ));
        }
    };
    Ok(spec)
}

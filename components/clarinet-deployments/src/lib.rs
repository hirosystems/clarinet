extern crate serde;

#[macro_use]
extern crate serde_derive;

pub mod requirements;
pub mod types;

use self::types::{
    DeploymentSpecification, EmulatedContractPublishSpecification, GenesisSpecification,
    TransactionPlanSpecification, TransactionsBatchSpecification, WalletSpecification,
};
use clarity_repl::clarity::diagnostic::DiagnosableError;
use types::ContractPublishSpecification;
use types::DeploymentGenerationArtifacts;
use types::RequirementPublishSpecification;
use types::TransactionSpecification;

use clarinet_files::{NetworkManifest, ProjectManifest};

use clarity_repl::analysis::ast_dependency_detector::{ASTDependencyDetector, DependencySet};
use clarity_repl::clarity::ast::ContractAST;
use clarity_repl::clarity::diagnostic::Diagnostic;
use clarity_repl::clarity::types::{PrincipalData, QualifiedContractIdentifier};
use clarity_repl::clarity::ContractName;
use clarity_repl::repl::SessionSettings;
use clarity_repl::repl::{ExecutionResult, Session};
use orchestra_types::StacksNetwork;
use std::collections::{BTreeMap, HashMap, VecDeque};

pub fn setup_session_with_deployment(
    manifest: &ProjectManifest,
    deployment: &DeploymentSpecification,
    contracts_asts: Option<&HashMap<QualifiedContractIdentifier, ContractAST>>,
) -> DeploymentGenerationArtifacts {
    let mut session = initiate_session_from_deployment(&manifest);
    update_session_with_genesis_accounts(&mut session, deployment);
    let results =
        update_session_with_contracts_executions(&mut session, deployment, contracts_asts, false);

    let deps = HashMap::new();
    let mut diags = HashMap::new();
    let mut asts = HashMap::new();
    let mut contracts_analysis = HashMap::new();
    let mut success = true;
    for (contract_id, res) in results.into_iter() {
        match res {
            Ok(execution_result) => {
                diags.insert(contract_id.clone(), execution_result.diagnostics);
                if let Some((_, _, _, ast, analysis)) = execution_result.contract {
                    asts.insert(contract_id.clone(), ast);
                    contracts_analysis.insert(contract_id, analysis);
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
    settings
        .include_boot_contracts
        .append(&mut manifest.project.boot_contracts.clone());
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
) -> BTreeMap<QualifiedContractIdentifier, Result<ExecutionResult, Vec<Diagnostic>>> {
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
                TransactionSpecification::EmulatedContractPublish(tx) => {
                    let default_tx_sender = session.get_tx_sender();
                    session.set_tx_sender(tx.emulated_sender.to_string());

                    let contract_id = QualifiedContractIdentifier::new(
                        tx.emulated_sender.clone(),
                        tx.contract_name.clone(),
                    );
                    let contract_ast = contracts_asts.as_ref().and_then(|m| m.get(&contract_id));
                    let result = session.interpret(
                        tx.source.clone(),
                        Some(tx.contract_name.to_string()),
                        None,
                        false,
                        match code_coverage_enabled {
                            true => Some("__analysis__".to_string()),
                            false => None,
                        },
                        contract_ast,
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
) -> Result<(DeploymentSpecification, DeploymentGenerationArtifacts), String> {
    let network_manifest = NetworkManifest::from_project_manifest_location(
        &manifest.location,
        &network.get_networks(),
    )?;

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

    let parser_version = manifest.repl_settings.parser_version;

    let mut settings = SessionSettings::default();
    settings.include_boot_contracts = manifest.project.boot_contracts.clone();
    settings.repl_settings = manifest.repl_settings.clone();

    let session = Session::new(settings.clone());
    let mut boot_contracts_asts = session.get_boot_contracts_asts();
    let boot_contracts_ids = boot_contracts_asts
        .iter()
        .map(|(k, _)| k.clone())
        .collect::<Vec<QualifiedContractIdentifier>>();
    requirements_asts.append(&mut boot_contracts_asts);

    let mut queue = VecDeque::new();

    if let Some(ref devnet) = network_manifest.devnet {
        if devnet.enable_hyperchain_node {
            let contract_id =
                match QualifiedContractIdentifier::parse(&devnet.hyperchain_contract_id) {
                    Ok(contract_id) => contract_id,
                    Err(_e) => {
                        return Err(format!(
                            "malformatted hyperchain_contract_id: {}",
                            devnet.hyperchain_contract_id
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
                    let (source, contract_location) =
                        requirements::retrieve_contract(&contract_id, &cache_location).await?;

                    // Build the struct representing the requirement in the deployment
                    if network.is_simnet() {
                        let data = EmulatedContractPublishSpecification {
                            contract_name: contract_id.name.clone(),
                            emulated_sender: contract_id.issuer.clone(),
                            source: source.clone(),
                            location: contract_location,
                        };
                        emulated_contracts_publish.insert(contract_id.clone(), data);
                    } else if network.either_devnet_or_testnet() {
                        let mut remap_principals = BTreeMap::new();
                        remap_principals
                            .insert(contract_id.issuer.clone(), default_deployer_address.clone());
                        match network_manifest.devnet {
                            Some(ref devnet)
                                if devnet.hyperchain_contract_id == contract_id.to_string() =>
                            {
                                remap_principals.insert(
                                    contract_id.issuer.clone(),
                                    PrincipalData::parse_standard_principal(
                                        &devnet.hyperchain_leader_stx_address,
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
                        };
                        requirements_publish.insert(contract_id.clone(), data);
                    }

                    // Compute the AST
                    let (ast, _, _) = session.interpreter.build_ast(
                        contract_id.clone(),
                        source.to_string(),
                        parser_version,
                    );
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
            let ordered_contracts_ids =
                match ASTDependencyDetector::order_contracts(&requirements_deps) {
                    Ok(ordered_contracts) => ordered_contracts,
                    Err(e) => return Err(format!("unable to order requirements {}", e)),
                };

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
    for (name, contract_config) in manifest.contracts.iter() {
        let contract_name = match ContractName::try_from(name.to_string()) {
            Ok(res) => res,
            Err(_) => return Err(format!("unable to use {} as a valid contract name", name)),
        };

        let deployer = match contract_config.deployer {
            Some(ref deployer) => {
                let deployer = match network_manifest.accounts.get(deployer) {
                    Some(deployer) => deployer,
                    None => {
                        return Err(format!("unable to retrieve account '{}'", deployer));
                    }
                };
                deployer
            }
            None => default_deployer,
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

        let mut contract_location = manifest.location.get_project_root_location()?;
        contract_location.append_path(&contract_config.path)?;
        let source = contract_location.read_content_as_utf8()?;

        let contract_id = QualifiedContractIdentifier::new(sender.clone(), contract_name.clone());

        contracts_sources.insert(contract_id.clone(), source.clone());

        let contract_spec = if network.is_simnet() {
            TransactionSpecification::EmulatedContractPublish(
                EmulatedContractPublishSpecification {
                    contract_name,
                    emulated_sender: sender,
                    source,
                    location: contract_location,
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
            })
        };

        contracts.insert(contract_id, contract_spec);
    }

    let session = Session::new(settings);

    let mut contract_asts = HashMap::new();
    let mut contract_diags = HashMap::new();

    let mut asts_success = true;

    for (contract_id, source) in contracts_sources.into_iter() {
        let (ast, diags, ast_success) =
            session
                .interpreter
                .build_ast(contract_id.clone(), source, parser_version);
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
        Err(e) => return Err(e.err.message()),
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

use super::utils;
use super::DeploymentCache;
use clarinet_deployments::types::DeploymentSpecification;
use clarinet_deployments::update_session_with_contracts_executions;
use clarinet_files::ProjectManifest;
use clarity::vm::analysis::contract_interface_builder::build_contract_interface;
use clarity_repl::analysis::coverage::TestCoverageReport;
use clarity_repl::repl::session::CostsReport;
use clarity_repl::repl::Session;
use deno::tools::test_runner::TestEvent;
use deno::tsc::exec;
use deno::{create_main_worker, ProgramState};
use deno_core::error::AnyError;
use deno_core::serde_json::{self, json, Value};
use deno_core::{ModuleSpecifier, OpFn, OpState};
use deno_runtime::permissions::Permissions;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::mpsc::{self, Sender};
use std::sync::Arc;

pub enum ClarinetTestEvent {
    SessionTerminated(SessionArtifacts),
}

pub struct SessionArtifacts {
    pub coverage_reports: Vec<TestCoverageReport>,
    pub costs_reports: Vec<CostsReport>,
}

pub async fn run_bridge(
    program_state: Arc<ProgramState>,
    main_module: ModuleSpecifier,
    test_module: ModuleSpecifier,
    permissions: Permissions,
    channel: Sender<TestEvent>,
    allow_wallets: bool,
    mut cache: Option<DeploymentCache>,
) -> Result<Vec<SessionArtifacts>, AnyError> {
    let mut worker = create_main_worker(&program_state, main_module.clone(), permissions, true);
    let (event_tx, event_rx) = mpsc::channel();
    {
        let js_runtime = &mut worker.js_runtime;
        js_runtime.register_op("api/v1/new_session", deno_core::op_sync(new_session));
        js_runtime.register_op(
            "api/v1/load_deployment",
            deno_core::op_sync(load_deployment),
        );
        js_runtime.register_op(
            "api/v1/terminate_session",
            deno_core::op_sync(terminate_session),
        );
        js_runtime.register_op("api/v1/mine_block", deno_core::op_sync(mine_block));
        js_runtime.register_op(
            "api/v1/mine_empty_blocks",
            deno_core::op_sync(mine_empty_blocks),
        );
        js_runtime.register_op(
            "api/v1/call_read_only_fn",
            deno_core::op_sync(call_read_only_fn),
        );
        js_runtime.register_op(
            "api/v1/get_assets_maps",
            deno_core::op_sync(get_assets_maps),
        );

        // Additionally, we're catching this legacy ops to display a human readable error
        js_runtime.register_op("setup_chain", deno_core::op_sync(deprecation_notice));
        js_runtime.register_op("start_setup_chain", deno_core::op_sync(deprecation_notice));

        js_runtime.sync_ops_cache();

        let sessions: HashMap<u32, (String, Session)> = HashMap::new();
        let mut deployments: HashMap<Option<String>, DeploymentCache> = HashMap::new();
        if let Some(cache) = cache.take() {
            // Using None as key - it will be used as our default deployment
            deployments.insert(None, cache);
        }

        js_runtime.op_state().borrow_mut().put(allow_wallets);
        js_runtime.op_state().borrow_mut().put(deployments);
        js_runtime.op_state().borrow_mut().put(sessions);
        js_runtime.op_state().borrow_mut().put(0u32);
        js_runtime
            .op_state()
            .borrow_mut()
            .put::<Sender<ClarinetTestEvent>>(event_tx.clone());
        js_runtime
            .op_state()
            .borrow_mut()
            .put::<Sender<TestEvent>>(channel);
    }

    let execute_result = worker.execute_module(&main_module).await;
    if let Err(e) = execute_result {
        println!("{}", e);
        return Err(e);
    }

    let execute_result = worker.execute("window.dispatchEvent(new Event('load'))");
    if let Err(e) = execute_result {
        println!("{}", e);
        return Err(e);
    }

    let execute_result = worker.execute_module(&test_module).await;
    if let Err(e) = execute_result {
        println!("{}", e);
        return Err(e);
    }

    let execute_result = worker.execute("window.dispatchEvent(new Event('unload'))");
    if let Err(e) = execute_result {
        println!("{}", e);
        return Err(e);
    }

    let mut artifacts = vec![];
    while let Ok(ClarinetTestEvent::SessionTerminated(artifact)) = event_rx.try_recv() {
        artifacts.push(artifact);
    }
    Ok(artifacts)
}

pub fn deprecation_notice(state: &mut OpState, args: Value, _: ()) -> Result<(), AnyError> {
    println!("{}: clarinet v{} is incompatible with the version of the library being imported in the test files.", red!("error"), option_env!("CARGO_PKG_VERSION").expect("Unable to detect version"));
    println!("The test files should import the latest version.");
    std::process::exit(1);
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NewSessionArgs {
    name: String,
    load_deployment: bool,
    deployment_path: Option<String>,
}

pub fn new_session(state: &mut OpState, args: Value, _: ()) -> Result<String, AnyError> {
    let args: NewSessionArgs =
        serde_json::from_value(args).expect("Invalid request from JavaScript for \"op_load\".");

    let session_id = {
        let session_id = match state.try_borrow_mut::<u32>() {
            Some(session_id) => session_id,
            None => panic!(),
        };
        *session_id += 1;
        session_id.clone()
    };

    let cache = {
        let caches = state.borrow::<HashMap<Option<String>, DeploymentCache>>();
        let cache = match args.deployment_path {
            Some(deploynent_path) => {
                let mut entry = caches.get(&Some(deploynent_path.clone()));
                if entry.is_none() {
                    let mut default_entry = caches.get(&None);
                    if let Some(default_entry) = default_entry.take() {
                        if default_entry.deployment_path == Some(deploynent_path.clone()) {
                            entry = Some(default_entry);
                        }
                    }
                    if entry.is_none() {
                        // TODO(lgalabru): Ability to specify a deployment plan in tests
                        // https://github.com/hirosystems/clarinet/issues/357
                        println!("{}: feature identified, but is not supported yet. Please comment in https://github.com/hirosystems/clarinet/issues/357", red!("Error"));
                        std::process::exit(1);
                    }
                }
                entry
            }
            None => {
                let mut default_entry = caches.get(&None);
                if let Some(default_entry) = default_entry.take() {
                    Some(default_entry)
                } else {
                    unreachable!();
                }
            }
        };
        cache.unwrap()
    };

    let allow_wallets = state.borrow::<bool>();
    let accounts = if *allow_wallets {
        cache.deployment.genesis.as_ref().unwrap().wallets.clone()
    } else {
        vec![]
    };

    let mut serialized_contracts = vec![];
    let session = if args.load_deployment {
        for (contract_id, artifacts) in cache.contracts_artifacts.iter() {
            serialized_contracts.push(json!({
                "contract_id": contract_id.to_string(),
                "contract_interface": artifacts.interface,
                "dependencies": artifacts.dependencies,
                "source": artifacts.source,
            }));
        }
        cache.session.clone()
    } else {
        cache.session_accounts_only.clone()
    };

    {
        let sessions = match state.try_borrow_mut::<HashMap<u32, (String, Session)>>() {
            Some(sessions) => sessions,
            None => panic!(),
        };
        let session_id = sessions.insert(session_id, (args.name, session));
    }

    Ok(json!({
        "session_id": session_id,
        "accounts": accounts.iter().map(|a| json!({
            "address": a.address.to_string(),
            "balance": u64::try_from(a.balance)
                .expect("u128 unsupported at the moment, please open an issue."),
            "name": a.name.to_string(),
          })).collect::<Vec<_>>(),
        "contracts": serialized_contracts,
    })
    .to_string())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoadDeploymentArgs {
    session_id: u32,
    deployment_path: Option<String>,
}

pub fn load_deployment(state: &mut OpState, args: Value, _: ()) -> Result<String, AnyError> {
    let args: LoadDeploymentArgs =
        serde_json::from_value(args).expect("Invalid request from JavaScript for \"op_load\".");

    // Retrieve deployment
    let deployment = {
        let caches = state.borrow::<HashMap<Option<String>, DeploymentCache>>();
        let cache = caches
            .get(&args.deployment_path)
            .expect("unable to retrieve deployment");
        cache.deployment.clone()
    };

    // Retrieve session
    let sessions = state
        .try_borrow_mut::<HashMap<u32, (String, Session)>>()
        .expect("unable to retrieve sessions");
    let (label, session) = sessions
        .get_mut(&args.session_id)
        .expect("unable to retrieve session");

    // Execute deployment on session
    let results = update_session_with_contracts_executions(session, &deployment, None, true);
    let mut serialized_contracts = vec![];
    for (contract_id, result) in results.into_iter() {
        match result {
            Ok(execution) => {
                if let Some((_, source, functions, ast, analysis)) = execution.contract {
                    serialized_contracts.push(json!({
                        "contract_id": contract_id.to_string(),
                        "contract_interface": build_contract_interface(&analysis),
                        "source": source,
                    }))
                }
            }
            Err(e) => {
                println!(
                    "{}: unable to load deployment {:?} in test {}",
                    red!("Error"),
                    args.deployment_path,
                    label
                );
                std::process::exit(1);
            }
        }
    }

    let allow_wallets = state.borrow::<bool>();
    let accounts = if *allow_wallets {
        deployment.genesis.as_ref().unwrap().wallets.clone()
    } else {
        vec![]
    };

    Ok(json!({
        "session_id": args.session_id,
        "accounts": accounts.iter().map(|a| json!({
            "address": a.address.to_string(),
            "balance": u64::try_from(a.balance)
                .expect("u128 unsupported at the moment, please open an issue."),
            "name": a.name.to_string(),
            })).collect::<Vec<_>>(),
        "contracts": serialized_contracts,
    })
    .to_string())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TerminateSessionArgs {
    session_id: u32,
}

pub fn terminate_session(state: &mut OpState, args: Value, _: ()) -> Result<(), AnyError> {
    let args: TerminateSessionArgs =
        serde_json::from_value(args).expect("Invalid request from JavaScript for \"op_load\".");

    // Retrieve session
    let session_artifacts = {
        let sessions = state
            .try_borrow_mut::<HashMap<u32, (String, Session)>>()
            .expect("unable to retrieve sessions");
        let (label, mut session) = sessions
            .remove(&args.session_id)
            .expect("unable to retrieve session");

        let mut coverage_reports = vec![];
        coverage_reports.append(&mut session.coverage_reports);

        let mut costs_reports = vec![];
        costs_reports.append(&mut session.costs_reports);

        SessionArtifacts {
            coverage_reports,
            costs_reports,
        }
    };

    let tx = state.borrow::<Sender<ClarinetTestEvent>>();
    let _ = tx.send(ClarinetTestEvent::SessionTerminated(session_artifacts));

    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MineEmptyBlocksArgs {
    session_id: u32,
    count: u32,
}

pub fn mine_empty_blocks(state: &mut OpState, args: Value, _: ()) -> Result<String, AnyError> {
    let args: MineEmptyBlocksArgs =
        serde_json::from_value(args).expect("Invalid request from JavaScript.");
    let block_height = perform_block(state, args.session_id, |name, session| {
        let block_height = session.advance_chain_tip(args.count);
        Ok(block_height)
    })?;

    Ok(json!({
      "session_id": args.session_id,
      "block_height": block_height,
    })
    .to_string())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CallReadOnlyFnArgs {
    session_id: u32,
    sender: String,
    contract: String,
    method: String,
    args: Vec<String>,
}

pub fn call_read_only_fn(state: &mut OpState, args: Value, _: ()) -> Result<String, AnyError> {
    let args: CallReadOnlyFnArgs =
        serde_json::from_value(args).expect("Invalid request from JavaScript.");
    let (result, events) = perform_block(state, args.session_id, |name, session| {
        let execution = session
            .invoke_contract_call(
                &args.contract,
                &args.method,
                &args.args,
                &args.sender,
                "readonly-calls".into(),
            )
            .unwrap(); // TODO(lgalabru)
        let result = match execution.result {
            Some(output) => format!("{}", output),
            _ => unreachable!("Value empty"),
        };
        Ok((result, execution.events))
    })?;
    Ok(json!({
      "session_id": args.session_id,
      "result": result,
      "events": events,
    })
    .to_string())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetAssetsMapsArgs {
    session_id: u32,
}

pub fn get_assets_maps(state: &mut OpState, args: Value, _: ()) -> Result<String, AnyError> {
    let args: GetAssetsMapsArgs =
        serde_json::from_value(args).expect("Invalid request from JavaScript.");
    let assets_maps = perform_block(state, args.session_id, |name, session| {
        let assets_maps = session.get_assets_maps();
        let mut lev1 = BTreeMap::new();
        for (key1, map1) in assets_maps.into_iter() {
            let mut lev2 = BTreeMap::new();
            for (key2, val2) in map1.into_iter() {
                lev2.insert(
                    key2,
                    u64::try_from(val2)
                        .expect("u128 unsupported at the moment, please open an issue."),
                );
            }
            lev1.insert(key1, lev2);
        }
        Ok(lev1)
    })?;
    Ok(json!({
      "session_id": args.session_id,
      "assets": assets_maps,
    })
    .to_string())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MineBlockArgs {
    session_id: u32,
    transactions: Vec<TransactionArgs>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionArgs {
    sender: String,
    contract_call: Option<ContractCallArgs>,
    deploy_contract: Option<DeployContractArgs>,
    transfer_stx: Option<TransferSTXArgs>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContractCallArgs {
    contract: String,
    method: String,
    args: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeployContractArgs {
    name: String,
    code: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TransferSTXArgs {
    amount: u64,
    recipient: String,
}

pub fn mine_block(state: &mut OpState, args: Value, _: ()) -> Result<String, AnyError> {
    let args: MineBlockArgs =
        serde_json::from_value(args).expect("Invalid request from JavaScript.");
    let (block_height, receipts) = perform_block(state, args.session_id, |name, session| {
        let initial_tx_sender = session.get_tx_sender();
        let mut receipts = vec![];
        for tx in args.transactions.iter() {
            if let Some(ref args) = tx.contract_call {
                let execution = match session.invoke_contract_call(
                    &args.contract,
                    &args.method,
                    &args.args,
                    &tx.sender,
                    name.into(),
                ) {
                    Ok(res) => res,
                    Err(diagnostics) => {
                        if diagnostics.len() > 0 {
                            // TODO(lgalabru): if CLARINET_BACKTRACE=1
                            // Retrieve the AST (penultimate entry), and the expression id (last entry)
                            println!(
                                "Runtime error: {}::{}({}) -> {}",
                                args.contract,
                                args.method,
                                args.args.join(", "),
                                diagnostics.last().unwrap().message
                            );
                        }
                        continue;
                    }
                };
                let result = match execution.result {
                    Some(output) => utils::value_to_string(&output),
                    _ => unreachable!("Value empty"),
                };
                receipts.push((result, execution.events));
            } else {
                session.set_tx_sender(tx.sender.clone());
                if let Some(ref args) = tx.deploy_contract {
                    let execution = session
                        .interpret(
                            args.code.clone(),
                            Some(args.name.clone()),
                            None,
                            false,
                            Some(name.into()),
                            None,
                        )
                        .unwrap(); // TODO(lgalabru)
                    let result = match execution.result {
                        Some(output) => format!("{}", output),
                        _ => unreachable!("Value empty"),
                    };
                    receipts.push((result, execution.events));
                } else if let Some(ref args) = tx.transfer_stx {
                    let snippet = format!(
                        "(stx-transfer? u{} tx-sender '{})",
                        args.amount, args.recipient
                    );
                    let execution = session
                        .interpret(snippet, None, None, false, Some(name.into()), None)
                        .unwrap(); // TODO(lgalabru)
                    let result = match execution.result {
                        Some(output) => format!("{}", output),
                        _ => unreachable!("Value empty"),
                    };
                    receipts.push((result, execution.events));
                }
                session.set_tx_sender(initial_tx_sender.clone());
            }
        }
        let block_height = session.advance_chain_tip(1);
        Ok((block_height, receipts))
    })?;

    let payload = json!({
      "session_id": args.session_id,
      "block_height": block_height,
      "receipts":  receipts.iter().map(|r| {
        json!({
          "result": r.0,
          "events": r.1,
        })
      }).collect::<Vec<_>>()
    });

    Ok(payload.to_string())
}

pub fn perform_block<F, R>(state: &mut OpState, session_id: u32, handler: F) -> Result<R, AnyError>
where
    F: FnOnce(&str, &mut Session) -> Result<R, AnyError>,
{
    let sessions = match state.try_borrow_mut::<HashMap<u32, (String, Session)>>() {
        Some(sessions) => sessions,
        None => panic!(),
    };

    match sessions.get_mut(&session_id) {
        None => {
            println!("Error: unable to retrieve session");
            panic!()
        }
        Some((name, ref mut session)) => handler(name.as_str(), session),
    }
}

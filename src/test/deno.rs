use deno::create_main_worker;
use deno::File;
use deno::MediaType;
use deno::Flags;
use deno::ProgramState;
use deno::tools;
use deno::fs_util;
use deno::tsc::{op, State};
use deno_core::{OpFn};
use deno_core::serde_json::{self, json, Value};
use deno_core::op_sync;
use deno_core::error::AnyError;
use deno_core::url::Url;
use deno_runtime::web_worker::WebWorkerOptions;
use deno_runtime::ops::worker_host::CreateWebWorkerCb;
use deno_runtime::web_worker::WebWorker;
use deno_runtime::permissions::Permissions;
use deno_runtime::worker::{MainWorker, WorkerOptions};
use std::rc::Rc;
use std::sync::Arc;
use serde::Serialize;
use serde::de::DeserializeOwned;
use clarity_repl::clarity::coverage::CoverageReporter;

mod sessions {
    use std::sync::Mutex;
    use std::fs;
    use std::env;
    use std::collections::HashMap;
    use deno_core::error::AnyError;
    use clarity_repl::repl::{self, Session};
    use clarity_repl::repl::settings::Account;
    use crate::types::{ChainConfig, MainConfig};
    use super::TransactionArgs;

    lazy_static! {
        pub static ref SESSIONS: Mutex<HashMap<u32, (String, Session)>> = Mutex::new(HashMap::new());
    }

    pub fn handle_setup_chain(name: String, transactions: Vec<TransactionArgs>) -> Result<(u32, Vec<Account>), AnyError> {
        let mut sessions = SESSIONS.lock().unwrap();
        let session_id = sessions.len() as u32;

        let mut settings = repl::SessionSettings::default();
        let root_path = env::current_dir().unwrap();
        let mut project_config_path = root_path.clone();
        project_config_path.push("Clarinet.toml");
    
        let mut chain_config_path = root_path.clone();
        chain_config_path.push("settings");
        chain_config_path.push("Development.toml");
    
        let project_config = MainConfig::from_path(&project_config_path);
        let chain_config = ChainConfig::from_path(&chain_config_path);
    
        let mut deployer_address = None;
        let mut initial_deployer = None;

        for (name, account) in chain_config.accounts.iter() {
            let account = repl::settings::Account {
                name: name.clone(),
                balance: account.balance,
                address: account.address.clone(),
                mnemonic: account.mnemonic.clone(),
                derivation: account.derivation.clone(),
            };
            if name == "deployer" {
                initial_deployer = Some(account.clone());
                deployer_address = Some(account.address.clone());
            }
            settings
                .initial_accounts
                .push(account);
        }

        for tx in transactions.iter() {
          let deployer = Some(tx.sender.clone());
          if let Some(ref deploy_contract) = tx.deploy_contract {
            settings
              .initial_contracts
              .push(repl::settings::InitialContract {
                  code: deploy_contract.code.clone(),
                  path: "".into(),
                  name: Some(deploy_contract.name.clone()),
                  deployer,
              });
          }
          // if let Some(ref contract_call) tx.contract_call {
          // TODO: initial_tx_sender
          //   let code = format!("(contract-call? '{}.{} {} {})", initial_tx_sender, contract_call.contract, contract_call.method, contract_call.args.join(" "));
          //   settings
          //     .initial_contracts
          //     .push(repl::settings::InitialContract {
          //         code: code,
          //         name: Some(name.clone()),
          //         deployer: tx.sender.clone(),
          //     });
          // }
        }


        for (name, config) in project_config.ordered_contracts().iter() {
            let mut contract_path = root_path.clone();
            contract_path.push(&config.path);
    
            let code = fs::read_to_string(&contract_path).unwrap();
    
            settings
                .initial_contracts
                .push(repl::settings::InitialContract {
                    code: code,
                    path: contract_path.to_str().unwrap().into(),
                    name: Some(name.clone()),
                    deployer: deployer_address.clone(),
                });
        }
        settings.initial_deployer = initial_deployer;
        settings.include_boot_contracts = vec!["pox".to_string(), "costs".to_string(), "bns".to_string()];
        let mut session = Session::new(settings.clone());
        session.start();
        session.advance_chain_tip(1);
        sessions.insert(session_id, (name, session));
        Ok((session_id, settings.initial_accounts))
    }

    pub fn perform_block<F, R>(session_id: u32, handler: F) -> Result<R, AnyError> where F: FnOnce(&str, &mut Session) -> Result<R, AnyError> {
        let mut sessions = SESSIONS.lock().unwrap();
        match sessions.get_mut(&session_id) {
            None => {
                println!("Error: unable to retrieve session");
                unreachable!()
            }
            Some((name , ref mut session)) => handler(name.as_str(), session),
        }
    }
}

pub async fn run_tests(files: Vec<String>, include_coverage: bool) -> Result<(), AnyError> {

    let fail_fast = true;
    let quiet = false;
    let filter = None;

    let mut flags = Flags::default();
    flags.unstable = true;
    let program_state = ProgramState::build(flags.clone()).await?;
    let permissions = Permissions::from_options(&flags.clone().into());
    let cwd = std::env::current_dir().expect("No current directory");

    let include = if files.len() > 0 {
      files.clone()
    } else {
      vec!["./tests/".to_string()]
    };
    let test_modules =
      tools::test_runner::collect_test_module_specifiers(include, &cwd, fs_util::is_supported_ext)?;
  
    if test_modules.is_empty() {
      println!("No matching test modules found");
      return Ok(());
    }
    let main_module = deno_core::resolve_path("$deno$test.ts")?;
    // Create a dummy source file.

    let source = render_test_file(
      test_modules.clone(),
      fail_fast,
      quiet,
      filter,
    );

    let source_file = File {
      local: main_module.to_file_path().unwrap(),
      maybe_types: None,
      media_type: MediaType::TypeScript,
      source,
      specifier: main_module.clone(),
    };

    // Save our fake file into file fetcher cache
    // to allow module access by TS compiler
    program_state.file_fetcher.insert_cached(source_file);
  
    let mut worker =
      create_main_worker(&program_state, main_module.clone(), permissions, false);

    worker.js_runtime.register_op("setup_chain", op(setup_chain));
    worker.js_runtime.register_op("mine_block", op(mine_block));
    worker.js_runtime.register_op("mine_empty_blocks", op(mine_empty_blocks));
    worker.js_runtime.register_op("call_read_only_fn", op(call_read_only_fn));
    worker.js_runtime.register_op("get_assets_maps", op(get_assets_maps));
    
    let res = worker.execute_module(&main_module).await;
    if let Err(e) = res {
      println!("{}", e);
      return Err(e);
    }
  
    worker.execute("window.dispatchEvent(new Event('load'))");
    let res = worker.run_event_loop(false).await;
    if let Err(e) = res {
      println!("{}", e);
      return Err(e);
    }

    worker.execute("window.dispatchEvent(new Event('unload'))")?;
    let res = worker.run_event_loop(false).await;
    if let Err(e) = res {
      println!("{}", e);
      return Err(e);
    }

    if include_coverage {
      let mut coverage_reporter = CoverageReporter::new();
      let sessions = sessions::SESSIONS.lock().unwrap();
      for (session_id, (name, session)) in sessions.iter() {
        
        for contract in session.settings.initial_contracts.iter() {
          if let Some(ref name) = contract.name {
            if contract.path != "" {
              coverage_reporter.register_contract(name.clone(), contract.path.clone());
            }
          }
        }
        coverage_reporter.add_reports(&session.coverage_reports);
        coverage_reporter.add_asts(&session.asts);
      }
  
      coverage_reporter.write_lcov_file("coverage.lcov");  
    }
    Ok(())
}

pub fn render_test_file(
  modules: Vec<Url>,
  fail_fast: bool,
  quiet: bool,
  filter: Option<String>,
) -> String {
  let mut test_file = "".to_string();

  for module in modules {
    test_file.push_str(&format!("import \"{}\";\n", module.to_string()));
  }

  let options = if let Some(filter) = filter {
    json!({ "failFast": fail_fast, "reportToConsole": !quiet, "disableLog": quiet, "filter": filter })
  } else {
    json!({ "failFast": fail_fast, "reportToConsole": !quiet, "disableLog": quiet })
  };

  test_file.push_str("// @ts-ignore\n");

  test_file.push_str(&format!(
    "await Deno[Deno.internal].runTests({});\n",
    options
  ));

  test_file
}

// fn op<F, V, R>(op_fn: F) -> Box<OpFn>
// where
//   F: Fn(V) -> Result<R, AnyError> + 'static,
//   V: DeserializeOwned,
//   R: Serialize,
// {
//     op_sync(move |s, args, _: ()| {
//         op_fn(args)
//     })    
// }

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetupChainArgs {
  name: String,
  transactions: Vec<TransactionArgs>
}

fn setup_chain(state: &mut State, args: Value) -> Result<Value, AnyError> {
    let args: SetupChainArgs = serde_json::from_value(args)
      .expect("Invalid request from JavaScript for \"op_load\".");
    let (session_id, accounts) = sessions::handle_setup_chain(args.name, args.transactions)?;

    Ok(json!({
        "session_id": session_id,
        "accounts": accounts,
    }))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MineBlockArgs {
  session_id: u32,
  transactions: Vec<TransactionArgs>
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

fn mine_block(state: &mut State, args: Value) -> Result<Value, AnyError> {
  let args: MineBlockArgs = serde_json::from_value(args)
    .expect("Invalid request from JavaScript.");
  let (block_height, receipts) = sessions::perform_block(args.session_id, |name, session| {
      let initial_tx_sender = session.get_tx_sender();
      let mut receipts = vec![];
      for tx in args.transactions.iter() {
        session.set_tx_sender(tx.sender.clone());
        if let Some(ref args) = tx.contract_call {

          // Kludge for handling fully qualified contract_id vs sugared syntax
          let first_char = args.contract.chars().next().unwrap();
          let snippet = if first_char.to_string() == "S" {
            format!("(contract-call? '{} {} {})", args.contract, args.method, args.args.join(" "))
          } else {
            format!("(contract-call? '{}.{} {} {})", initial_tx_sender, args.contract, args.method, args.args.join(" "))
          };
          let execution = session.interpret(snippet, None, true, Some(name.into())).unwrap(); // todo(ludo)
          receipts.push((execution.result, execution.events));
        }

        if let Some(ref args) = tx.deploy_contract {
          let execution = session.interpret(args.code.clone(), Some(args.name.clone()), true, Some(name.into())).unwrap(); // todo(ludo)
          receipts.push((execution.result, execution.events));
        }
      }
      session.set_tx_sender(initial_tx_sender);
      let block_height = session.advance_chain_tip(1);
      Ok((block_height, receipts))
  })?;
  Ok(json!({
    "session_id": args.session_id,
    "block_height": block_height,
    "receipts":  receipts.iter().map(|r| {
      json!({
        "result": r.0,
        "events": r.1,
      })
    }).collect::<Vec<_>>()
  }))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MineEmptyBlocksArgs {
  session_id: u32,
  count: u32,
}

fn mine_empty_blocks(state: &mut State, args: Value) -> Result<Value, AnyError> {
  let args: MineEmptyBlocksArgs = serde_json::from_value(args)
    .expect("Invalid request from JavaScript.");
  let block_height = sessions::perform_block(args.session_id, |name, session| {
    let block_height = session.advance_chain_tip(args.count);
    Ok(block_height)
  })?;

  Ok(json!({
    "session_id": args.session_id,
    "block_height": block_height,
  }))
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

fn call_read_only_fn(state: &mut State, args: Value) -> Result<Value, AnyError> {
  let args: CallReadOnlyFnArgs = serde_json::from_value(args)
    .expect("Invalid request from JavaScript.");
  let (result, events) = sessions::perform_block(args.session_id, |name, session| {
    let initial_tx_sender = session.get_tx_sender();
    session.set_tx_sender(args.sender.clone());

    // Kludge for handling fully qualified contract_id vs sugared syntax
    let first_char = args.contract.chars().next().unwrap();
    let snippet = if first_char.to_string() == "S" {
      format!("(contract-call? '{} {} {})", args.contract, args.method, args.args.join(" "))
    } else {
      format!("(contract-call? '{}.{} {} {})", initial_tx_sender, args.contract, args.method, args.args.join(" "))
    };

    let execution = session.interpret(snippet, None, true, Some(name.into())).unwrap(); // todo(ludo)
    session.set_tx_sender(initial_tx_sender);
    Ok((execution.result, execution.events))
  })?;
  Ok(json!({
    "session_id": args.session_id,
    "result": result,
    "events": events,
  }))
}


#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetAssetsMapsArgs {
  session_id: u32,
}

fn get_assets_maps(state: &mut State, args: Value) -> Result<Value, AnyError> {
  let args: GetAssetsMapsArgs = serde_json::from_value(args)
    .expect("Invalid request from JavaScript.");
  let assets_maps = sessions::perform_block(args.session_id, |name, session| {
    let assets_maps = session.get_assets_maps();    
    Ok(assets_maps)
  })?;
  Ok(json!({
    "session_id": args.session_id,
    "assets": assets_maps,
  }))
}

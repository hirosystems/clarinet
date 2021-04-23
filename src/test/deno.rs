use deno_core::serde_json::{json, Value};
use deno_core::json_op_sync;
use deno_core::error::AnyError;
use deno_runtime::permissions::Permissions;
use deno_runtime::worker::MainWorker;
use deno_runtime::worker::WorkerOptions;
use std::rc::Rc;
use std::sync::Arc;
use serde::Serialize;
use serde::de::DeserializeOwned;
use deno_core::{OpFn};
use super::source_maps::apply_source_map;
use super::file_fetcher::File;
use super::media_type::MediaType;
use super::flags::Flags;
use super::program_state::ProgramState;
use super::tools;
use super::ops;
use super::fmt_errors::PrettyJsError;
use super::module_loader::CliModuleLoader;
use deno_core::ModuleSpecifier;
use deno_runtime::web_worker::WebWorkerOptions;
use deno_runtime::ops::worker_host::CreateWebWorkerCb;
use deno_runtime::web_worker::WebWorker;

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
        static ref SESSIONS: Mutex<HashMap<u32, Session>> = Mutex::new(HashMap::new());
    }

    pub fn handle_setup_chain(transactions: Vec<TransactionArgs>) -> Result<(u32, Vec<Account>), AnyError> {
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
                    name: Some(name.clone()),
                    deployer: deployer_address.clone(),
                });
        }
        settings.initial_deployer = initial_deployer;
        settings.include_boot_contracts = true;
  
        let mut session = Session::new(settings.clone());
        session.start();
        session.advance_chain_tip(1);
        sessions.insert(session_id, session);
        Ok((session_id, settings.initial_accounts))
    }

    pub fn perform_block<F, R>(session_id: u32, handler: F) -> Result<R, AnyError> where F: FnOnce(&mut Session) -> Result<R, AnyError> {
        let mut sessions = SESSIONS.lock().unwrap();
        match sessions.get_mut(&session_id) {
            None => {
                println!("Error: unable to retrieve session");
                unreachable!()
            }
            Some(ref mut session) => handler(session),
        }
    }
}

pub async fn run_tests() -> Result<(), AnyError> {

    let fail_fast = true;
    let quiet = false;
    let filter = None;

    let mut flags = Flags::default();
    flags.unstable = true;
    let program_state = ProgramState::build(flags.clone()).await?;
    let permissions = Permissions::from_options(&flags.clone().into());
    let cwd = std::env::current_dir().expect("No current directory");
    let include = vec![".".to_string()];
    let test_modules =
      tools::test_runner::prepare_test_modules_urls(include, &cwd)?;
  
    if test_modules.is_empty() {
      println!("No matching test modules found");
      return Ok(());
    }
    let main_module = deno_core::resolve_path("$deno$test.ts")?;
    // Create a dummy source file.

    let source = tools::test_runner::render_test_file(
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
      create_main_worker(&program_state, main_module.clone(), permissions);

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
    let res = worker.run_event_loop().await;
    if let Err(e) = res {
      println!("{}", e);
      return Err(e);
    }

    worker.execute("window.dispatchEvent(new Event('unload'))")?;
    let res = worker.run_event_loop().await;
    if let Err(e) = res {
      println!("{}", e);
      return Err(e);
    }

    Ok(())
}

fn create_web_worker_callback(
    program_state: Arc<ProgramState>,
  ) -> Arc<CreateWebWorkerCb> {
    Arc::new(move |args| {
      let global_state_ = program_state.clone();
      let js_error_create_fn = Rc::new(move |core_js_error| {
        let source_mapped_error =
          apply_source_map(&core_js_error, global_state_.clone());
        PrettyJsError::create(source_mapped_error)
      });
  
      let attach_inspector = program_state.maybe_inspector_server.is_some()
        || program_state.coverage_dir.is_some();
      let maybe_inspector_server = program_state.maybe_inspector_server.clone();
  
      let module_loader = CliModuleLoader::new_for_worker(
        program_state.clone(),
        args.parent_permissions.clone(),
      );
      let create_web_worker_cb =
        create_web_worker_callback(program_state.clone());
  
      let options = WebWorkerOptions {
        args: program_state.flags.argv.clone(),
        apply_source_maps: true,
        debug_flag: false,
        unstable: program_state.flags.unstable,
        ca_data: program_state.ca_data.clone(),
        user_agent: super::version::get_user_agent(),
        seed: program_state.flags.seed,
        module_loader,
        create_web_worker_cb,
        js_error_create_fn: Some(js_error_create_fn),
        use_deno_namespace: args.use_deno_namespace,
        attach_inspector,
        maybe_inspector_server,
        runtime_version: super::version::deno(),
        ts_version: super::version::TYPESCRIPT.to_string(),
        no_color: !super::colors::use_color(),
        get_error_class_fn: None,
      };
  
      let mut worker = WebWorker::from_options(
        args.name,
        args.permissions,
        args.main_module,
        args.worker_id,
        &options,
      );
  
      // This block registers additional ops and state that
      // are only available in the CLI
      {
        let js_runtime = &mut worker.js_runtime;
        js_runtime
          .op_state()
          .borrow_mut()
          .put::<Arc<ProgramState>>(program_state.clone());
        // Applies source maps - works in conjuction with `js_error_create_fn`
        // above
        ops::errors::init(js_runtime);
        if args.use_deno_namespace {
          ops::runtime_compiler::init(js_runtime);
        }
      }
      worker.bootstrap(&options);
  
      worker
    })
  }

pub fn create_main_worker(
    program_state: &Arc<ProgramState>,
    main_module: ModuleSpecifier,
    permissions: Permissions,
  ) -> MainWorker {
    let module_loader = CliModuleLoader::new(program_state.clone());
  
    let global_state_ = program_state.clone();
  
    let js_error_create_fn = Rc::new(move |core_js_error| {
      let source_mapped_error =
        apply_source_map(&core_js_error, global_state_.clone());
      PrettyJsError::create(source_mapped_error)
    });
  
    let attach_inspector = program_state.maybe_inspector_server.is_some()
      || program_state.flags.repl
      || program_state.coverage_dir.is_some();
    let maybe_inspector_server = program_state.maybe_inspector_server.clone();
    let should_break_on_first_statement =
      program_state.flags.inspect_brk.is_some();
  
    let create_web_worker_cb = create_web_worker_callback(program_state.clone());
  
    let options = WorkerOptions {
      apply_source_maps: true,
      args: program_state.flags.argv.clone(),
      debug_flag: false,
      unstable: program_state.flags.unstable,
      ca_data: program_state.ca_data.clone(),
      user_agent: super::version::get_user_agent(),
      seed: program_state.flags.seed,
      js_error_create_fn: Some(js_error_create_fn),
      create_web_worker_cb,
      attach_inspector,
      maybe_inspector_server,
      should_break_on_first_statement,
      module_loader,
      runtime_version: super::version::deno(),
      ts_version: super::version::TYPESCRIPT.to_string(),
      no_color: !super::colors::use_color(),
      get_error_class_fn: None,
      location: program_state.flags.location.clone(),
    };
  
    let mut worker = MainWorker::from_options(main_module, permissions, &options);
  
    // This block registers additional ops and state that
    // are only available in the CLI
    {
      let js_runtime = &mut worker.js_runtime;
      js_runtime
        .op_state()
        .borrow_mut()
        .put::<Arc<ProgramState>>(program_state.clone());
      // Applies source maps - works in conjuction with `js_error_create_fn`
      // above
      ops::errors::init(js_runtime);
      ops::runtime_compiler::init(js_runtime);
    }
    worker.bootstrap(&options);
  
    worker
  }

  
fn get_error_class_name(e: &AnyError) -> &'static str {
    deno_runtime::errors::get_error_class_name(e).unwrap_or("Error")
}

fn op<F, V, R>(op_fn: F) -> Box<OpFn>
where
  F: Fn(V) -> Result<R, AnyError> + 'static,
  V: DeserializeOwned,
  R: Serialize,
{
    json_op_sync(move |s, args, _bufs| {
        op_fn(args)
    })    
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetupChainArgs {
  transactions: Vec<TransactionArgs>
}

fn setup_chain(args: SetupChainArgs) -> Result<Value, AnyError> {
    let (session_id, accounts) = sessions::handle_setup_chain(args.transactions)?;

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

fn mine_block(args: MineBlockArgs) -> Result<Value, AnyError> {
  let (block_height, receipts) = sessions::perform_block(args.session_id, |session| {
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
          let execution = session.interpret(snippet, None).unwrap(); // todo(ludo)
          receipts.push((execution.result, execution.events));
        }

        if let Some(ref args) = tx.deploy_contract {
          let execution = session.interpret(args.code.clone(), Some(args.name.clone())).unwrap(); // todo(ludo)
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

fn mine_empty_blocks(args: MineEmptyBlocksArgs) -> Result<Value, AnyError> {
  let block_height = sessions::perform_block(args.session_id, |session| {
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

fn call_read_only_fn(args: CallReadOnlyFnArgs) -> Result<Value, AnyError> {
  let (result, events) = sessions::perform_block(args.session_id, |session| {
    let initial_tx_sender = session.get_tx_sender();
    session.set_tx_sender(args.sender.clone());

    // Kludge for handling fully qualified contract_id vs sugared syntax
    let first_char = args.contract.chars().next().unwrap();
    let snippet = if first_char.to_string() == "S" {
      format!("(contract-call? '{} {} {})", args.contract, args.method, args.args.join(" "))
    } else {
      format!("(contract-call? '{}.{} {} {})", initial_tx_sender, args.contract, args.method, args.args.join(" "))
    };

    let execution = session.interpret(snippet, None).unwrap(); // todo(ludo)
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

fn get_assets_maps(args: GetAssetsMapsArgs) -> Result<Value, AnyError> {
  let assets_maps = sessions::perform_block(args.session_id, |session| {
    let assets_maps = session.get_assets_maps();    
    Ok(assets_maps)
  })?;
  Ok(json!({
    "session_id": args.session_id,
    "assets": assets_maps,
  }))
}

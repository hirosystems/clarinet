use deno_core::serde_json::{json, Value};
use deno_core::json_op_sync;
use deno_core::error::AnyError;
use deno_core::FsModuleLoader;
use deno_core::url::Url;
use deno_runtime::permissions::Permissions;
use deno_runtime::worker::MainWorker;
use deno_runtime::worker::WorkerOptions;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;
use serde::Serialize;
use serde::de::DeserializeOwned;
use deno_core::{OpFn};

mod sessions {
    use std::sync::Mutex;
    use std::fs;
    use std::env;
    use std::collections::HashMap;
    use deno_core::error::AnyError;
    use clarity_repl::repl::{self, Session};
    use clarity_repl::repl::settings::Account;
    use crate::types::{ChainConfig, MainConfig};

    lazy_static! {
        static ref SESSIONS: Mutex<HashMap<u32, Session>> = Mutex::new(HashMap::new());
    }

    pub fn handle_setup_chain() -> Result<(u32, Vec<Account>), AnyError> {
        let mut sessions = SESSIONS.lock().unwrap();
        let session_id = sessions.len() as u32;

        let mut settings = repl::SessionSettings::default();
        let root_path = env::current_dir().unwrap();
        let mut project_config_path = root_path.clone();
        project_config_path.push("Clarinet.toml");
    
        let mut chain_config_path = root_path.clone();
        chain_config_path.push("settings");
        chain_config_path.push("Local.toml");
    
        let project_config = MainConfig::from_path(&project_config_path);
        let chain_config = ChainConfig::from_path(&chain_config_path);
    
        for (name, config) in project_config.contracts.iter() {
            let mut contract_path = root_path.clone();
            contract_path.push(&config.path);
    
            let code = fs::read_to_string(&contract_path).unwrap();
    
            settings
                .initial_contracts
                .push(repl::settings::InitialContract {
                    code: code,
                    name: Some(name.clone()),
                    deployer: Some("ST1D0XTBR7WVNSYBJ7M26XSJAXMDJGJQKNEXAM6JH".to_string()),
                });
        }
    
        for (name, account) in chain_config.accounts.iter() {
            settings
                .initial_accounts
                .push(repl::settings::Account {
                    name: name.clone(),
                    balance: account.balance,
                    address: account.address.clone(),
                    mnemonic: account.mnemonic.clone(),
                    derivation: account.derivation.clone(),
                });
        }

        let session = Session::new(settings.clone());
        sessions.insert(session_id, session);
        Ok((session_id, settings.initial_accounts))
    }

    pub fn get_session() -> Result<(), AnyError> {
        Ok(())
    }
}


pub async fn run_tests() -> Result<(), AnyError> {
    let module_loader = Rc::new(FsModuleLoader);
    let create_web_worker_cb = Arc::new(|_| {
      todo!("Web workers are not supported in the example");
    });
  
    let options = WorkerOptions {
      apply_source_maps: false,
      args: vec![],
      debug_flag: false,
      unstable: false,
      ca_data: None,
      user_agent: "Clarinet".to_string(),
      seed: None,   
      js_error_create_fn: None,
      create_web_worker_cb,
      attach_inspector: false,
      maybe_inspector_server: None,
      should_break_on_first_statement: false,
      module_loader,
      runtime_version: "1.8.0".to_string(),
      ts_version: "4.1.3".to_string(),
      no_color: false,
      get_error_class_fn: Some(&get_error_class_name),
      location: None,
    };

    let js_path =
      Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/bbtc/tests/bbtc_test.ts");
    let main_module = deno_core::resolve_path(&js_path.to_string_lossy())?;
    let permissions = Permissions::allow_all();
  
    let mut worker =
      MainWorker::from_options(main_module.clone(), permissions, &options);
    worker.bootstrap(&options);
    let root_module = deno_core::resolve_path("$deno$test.ts")?;
    let root_body = render_clarinet_root_file(
        vec![main_module],
        true,
        false,
        None,
    );
    worker.js_runtime.register_op("setup_chain", op(setup_chain));
    let mod_identifier = worker.js_runtime.load_module(&root_module, Some(root_body)).await;
    println!("-> {:?}", mod_identifier);
    worker.js_runtime.mod_evaluate(mod_identifier.unwrap());
    worker.execute("window.dispatchEvent(new Event('load'))")?;
    worker.run_event_loop().await?;
    worker.execute("window.dispatchEvent(new Event('unload'))")?;
    worker.run_event_loop().await?;

    Ok(())
}

pub fn render_clarinet_root_file(
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
//   specifier: String,
//   version: String,
//   start: usize,
//   end: usize,
}

fn setup_chain(args: SetupChainArgs) -> Result<Value, AnyError> {
    let (chain_id, accounts) = sessions::handle_setup_chain()?;

    Ok(json!({
        "chainId": chain_id,
        "accounts": accounts,
    }))
}

fn mine_block() {

}

fn set_tx_sender() {

}

fn get_accounts() {
    
}
use std::env;
use std::fs::{self, File};
use std::io::{prelude::*, BufReader, Read};

use crate::generators::{self, changes::Changes};
use crate::types::{MainConfig, MainConfigFile, ChainConfig};
use clap::Clap;
use clarity_repl::{repl, Terminal};
use toml;

// use deno_core::{JsRuntime, RuntimeOptions};

#[derive(Clap)]
#[clap(version = "1.0")]
struct Opts {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Clap)]
enum Command {
    /// New subcommand
    #[clap(name = "new")]
    New(GenerateProject),
    /// Generate subcommand
    #[clap(name = "generate")]
    Generate(Generate),
    /// Console subcommand
    #[clap(name = "console")]
    Console(Console),
    // /// Test subcommand
    // #[clap(name = "test")]
    // Test(Test),
}

#[derive(Clap)]
enum Generate {
    /// Generate contract subcommand
    #[clap(name = "contract")]
    Contract(GenerateContract),
    /// Generate notebook subcommand
    #[clap(name = "notebook")]
    Notebook(GenerateNotebook),
}

#[derive(Clap)]
struct GenerateProject {
    /// Project's name
    pub name: String,
    /// Print debug info
    #[clap(short = 'd')]
    pub debug: bool,
}

#[derive(Clap)]
struct GenerateContract {
    /// Contract's name
    pub name: String,
    /// Print debug info
    #[clap(short = 'd')]
    pub debug: bool,
}

#[derive(Clap)]
struct GenerateNotebook {
    /// Notebook's name
    pub name: String,
    /// Print debug info
    #[clap(short = 'd')]
    pub debug: bool,
}

#[derive(Clap)]
struct Console {
    /// Print debug info
    #[clap(short = 'd')]
    pub debug: bool,
}

#[derive(Clap)]
struct Test {
    /// Print debug info
    #[clap(short = 'd')]
    pub debug: bool,
}

pub fn main() {
    let opts: Opts = Opts::parse();

    let current_path = {
        let current_dir = env::current_dir().expect("Unable to read current directory");
        current_dir.to_str().unwrap().to_owned()
    };

    match opts.command {
        Command::New(project_opts) => {
            let changes = generators::get_changes_for_new_project(current_path, project_opts.name);
            execute_changes(changes);
        }
        Command::Generate(subcommand) => match subcommand {
            Generate::Contract(contract_opts) => {
                let changes =
                    generators::get_changes_for_new_contract(current_path, contract_opts.name);
                execute_changes(changes);
            }
            Generate::Notebook(notebook_opts) => {
                let changes =
                    generators::get_changes_for_new_notebook(current_path, notebook_opts.name);
                execute_changes(changes);
            }
        },
        Command::Console(t) => {
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
                        derivation_path: account.derivation_path.clone(),
                    });
            }

            let mut session = Terminal::new(settings);
            let res = session.start();
        } // Command::Test(t) => {
          //             let js_filename = "./tests/bns/registration.ts";
          //             let js_source: String = fs::read_to_string(js_filename).unwrap();

          //             let runtime_options = RuntimeOptions::default();
          //             let mut runtime = JsRuntime::new(runtime_options);
          //             let pre = r#"
          // // @deno-types="https://unpkg.com/@types/mocha@7.0.2/index.d.ts"
          // import "https://unpkg.com/mocha@7.2.0/mocha.js";
          // import { expect } from "https://deno.land/x/expect@v0.2.1/mod.ts";

          // function onCompleted(failures: number): void {
          //   if (failures > 0) {
          //       Deno.exit(1);
          //   } else {
          //       Deno.exit(0);
          //   }
          // }

          // (window as any).location = new URL("http://localhost:0");

          // mocha.setup({ ui: "bdd", reporter: "spec" });

          // mocha.checkLeaks();
          //             "#;

          //             let post = r#"
          // mocha.run(onCompleted).globals(["onerror"])
          //             "#;
          //             let js_source = format!("{}\n{}\n{}", pre, js_source, post);
          //             println!("-> \n {}", js_source);
          //             let res = runtime.execute(js_filename, &js_source);
          //             println!("{:?}", res);
          // }
    }
}

fn execute_changes(changes: Vec<Changes>) {
    for change in changes.iter() {
        match change {
            Changes::AddFile(options) => {
                println!("{}", options.comment);
                let mut file = File::create(options.path.clone()).expect("Unable to create file");
                file.write_all(options.content.as_bytes())
                    .expect("Unable to write file");
            }
            Changes::AddDirectory(options) => {
                println!("{}", options.comment);
                fs::create_dir_all(options.path.clone()).expect("Unable to create directory");
            }
            Changes::EditTOML(options) => {
                let path = File::open(options.path.clone()).unwrap();
                let mut config_file_reader = BufReader::new(path);
                let mut config_file = vec![];
                config_file_reader.read_to_end(&mut config_file).unwrap();
                let config_file: MainConfigFile = toml::from_slice(&config_file[..]).unwrap();
                let config: MainConfig = MainConfig::from_config_file(config_file);
                println!("{:?}", config);
            }
        }
    }
}

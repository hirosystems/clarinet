use std::env;
use std::fs::{self, File};
use std::io::{prelude::*, BufReader, Read};

use clap::Clap;
use toml;
use clarity_repl::repl;
use crate::generators::{self, changes::Changes};
use crate::types::{PaperConfig};

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
    pub debug: bool
}

#[derive(Clap)]
struct GenerateContract {
    /// Contract's name
    pub name: String,
    /// Print debug info
    #[clap(short = 'd')]
    pub debug: bool
}

#[derive(Clap)]
struct GenerateNotebook {
    /// Notebook's name
    pub name: String,
    /// Print debug info
    #[clap(short = 'd')]
    pub debug: bool
}

#[derive(Clap)]
struct Console {
    /// Print debug info
    #[clap(short = 'd')]
    pub debug: bool
}

#[derive(Clap)]
struct Test {
    /// Print debug info
    #[clap(short = 'd')]
    pub debug: bool
}

pub fn main() {
    let opts: Opts = Opts::parse();

    let current_path = {
        let current_dir = env::current_dir()
            .expect("Unable to read current directory");
        current_dir.to_str().unwrap().to_owned()
    };

    match opts.command {
        Command::New(project_opts) => {
            let changes = generators::get_changes_for_new_project(current_path, project_opts.name);
            execute_changes(changes);
        },
        Command::Generate(subcommand) => {
            match subcommand {
                Generate::Contract(contract_opts) => {
                    let changes = generators::get_changes_for_new_contract(current_path, contract_opts.name);
                    execute_changes(changes);        
                },
                Generate::Notebook(notebook_opts) => {
                    let changes = generators::get_changes_for_new_notebook(current_path, notebook_opts.name);
                    execute_changes(changes);        
                },
            }
        }
        Command::Console(t) => {

            let mut settings = repl::SessionSettings::default();
            
            let root_path = env::current_dir().unwrap();
            let mut config_path = root_path.clone();
            config_path.push("Paper.toml");

            let config = PaperConfig::from_path(&config_path);

            settings.initial_balances.push(
                repl::settings::InitialBalance { amount: 10000, address: "ST1D0XTBR7WVNSYBJ7M26XSJAXMDJGJQKNEXAM6JH".to_string() }
            );
        
            for contract in config.contracts.iter() {
                let mut contract_path = root_path.clone();
                contract_path.push(&contract.path);

                let code = fs::read_to_string(&contract_path).unwrap();

                settings.initial_contracts.push(
                    repl::settings::InitialContract { 
                        code: code, 
                        name: Some(contract.name.clone()),
                        deployer: Some("ST1D0XTBR7WVNSYBJ7M26XSJAXMDJGJQKNEXAM6JH".to_string())
                    }
                );    
            }
        
            let mut session = repl::Session::new(settings);
            let res = session.start();
            println!("{}", res);
        }
        // Command::Test(t) => {
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
                let mut file = File::create(options.path.clone())
                    .expect("Unable to create file");
                file.write_all(options.content.as_bytes())
                    .expect("Unable to write file");
            },
            Changes::AddDirectory(options) => {
                println!("{}", options.comment);
                fs::create_dir_all(options.path.clone())
                    .expect("Unable to create directory");
            },
            Changes::EditTOML(options) => {
                let path = File::open(options.path.clone()).unwrap();
                let mut config_file_reader = BufReader::new(path);
                let mut config_file = vec![];
                config_file_reader.read_to_end(&mut config_file).unwrap();    
                let config: PaperConfig = toml::from_slice(&config_file[..]).unwrap();
        
                println!("{:?}", config_file)
            },
        }
    }
}
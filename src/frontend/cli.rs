use std::env;
use std::fs::{self, File};
use std::io::{prelude::*, BufReader, Read};

use crate::generators::{self, changes::Changes};
use crate::types::{MainConfig, MainConfigFile};
use crate::console::load_session;
use crate::test::run_tests;

use clap::Clap;
use toml;

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
    /// Test subcommand
    #[clap(name = "test")]
    Test(Test),
    /// Check subcommand
    #[clap(name = "check")]
    Check(Check),
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
    pub files: Vec<String>,
}

#[derive(Clap)]
struct Check {
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
        Command::Console(_) => {
            let start_repl = true;
            load_session(start_repl).expect("Unable to start REPL");
        },
        Command::Check(_) => {
            let start_repl = false;
            let res = load_session(start_repl);
            if let Err(e) = res {
                println!("{}", e);
                return;
            }
        },
        Command::Test(test) => {
            let start_repl = false;
            let res = load_session(start_repl);
            if let Err(e) = res {
                println!("{}", e);
                return;
            }
            run_tests(test.files);
        }
    };
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
                let file = File::open(options.path.clone()).unwrap();
                let mut config_file_reader = BufReader::new(file);
                let mut config_file = vec![];
                config_file_reader.read_to_end(&mut config_file).unwrap();
                let config_file: MainConfigFile = toml::from_slice(&config_file[..]).unwrap();
                let mut config: MainConfig = MainConfig::from_config_file(config_file);
                for (contract_name, contract_config) in options.contracts_to_add.iter() {
                    config.contracts.insert(contract_name.clone(), contract_config.clone());
                }
                let toml = toml::to_string(&config).unwrap();
                let mut file = File::create(options.path.clone()).unwrap();
                file.write_all(&toml.as_bytes()).unwrap();
                println!("{}", options.comment);
            }
        }
    }
}

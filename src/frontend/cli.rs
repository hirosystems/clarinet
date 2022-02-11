use std::collections::{BTreeSet, HashMap};
use std::fs::{self, File};
use std::io::{prelude::*, BufReader, Read};
use std::path::PathBuf;
use std::{env, process};

use crate::generate::{
    self,
    changes::{Changes, TOMLEdition},
};
use crate::integrate::{self, DevnetOrchestrator};
use crate::lsp::run_lsp;
use crate::poke::load_session;
use crate::publish::{publish_all_contracts, Network};
use crate::runnner::run_scripts;
use crate::types::{ProjectManifest, ProjectManifestFile, RequirementConfig};
use clarity_repl::clarity::analysis::{AnalysisDatabase, ContractAnalysis};
use clarity_repl::clarity::costs::LimitedCostTracker;
use clarity_repl::clarity::types::QualifiedContractIdentifier;
use clarity_repl::{analysis, repl};

use clap::Clap;
use toml;

#[cfg(feature = "telemetry")]
use super::telemetry::{telemetry_report_event, DeveloperUsageDigest, DeveloperUsageEvent};

#[derive(Clap, PartialEq, Clone, Debug)]
#[clap(version = option_env!("CARGO_PKG_VERSION").expect("Unable to detect version"))]
struct Opts {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Clap, PartialEq, Clone, Debug)]
enum Command {
    /// Create and scaffold a new project
    #[clap(name = "new")]
    New(GenerateProject),
    /// Contract subcommand
    #[clap(subcommand, name = "contract")]
    Contract(Contract),
    /// Load contracts in a REPL for interactions
    #[clap(name = "poke")]
    Poke(Poke),
    #[clap(name = "console")]
    Console(Poke),
    /// Execute test suite
    #[clap(name = "test")]
    Test(Test),
    /// Check contracts syntax
    #[clap(name = "check")]
    Check(Check),
    /// Publish contracts on chain
    #[clap(name = "publish")]
    Publish(Publish),
    /// Execute Clarinet Extension
    #[clap(name = "run")]
    Run(Run),
    /// Work on contracts integration
    #[clap(name = "integrate")]
    Integrate(Integrate),
    /// Start a LSP session
    #[clap(name = "lsp")]
    LSP,
}

#[derive(Clap, PartialEq, Clone, Debug)]
enum Contract {
    /// New contract subcommand
    #[clap(name = "new")]
    NewContract(NewContract),
    /// Import contract subcommand
    #[clap(name = "requirement")]
    LinkContract(LinkContract),
    /// Fork contract subcommand
    #[clap(name = "fork")]
    ForkContract(ForkContract),
}

#[derive(Clap, PartialEq, Clone, Debug)]
struct GenerateProject {
    /// Project's name
    pub name: String,
    /// Enable developer usage telemetry
    #[clap(long = "disable-telemetry")]
    pub disable_telemetry: Option<bool>,
}

#[derive(Clap, PartialEq, Clone, Debug)]
struct NewContract {
    /// Contract's name
    pub name: String,
    /// Path to Clarinet.toml
    #[clap(long = "manifest-path")]
    pub manifest_path: Option<String>,
}

#[derive(Clap, PartialEq, Clone, Debug)]
struct LinkContract {
    /// Contract id
    pub contract_id: String,
    /// Path to Clarinet.toml
    #[clap(long = "manifest-path")]
    pub manifest_path: Option<String>,
}

#[derive(Clap, PartialEq, Clone, Debug)]
struct ForkContract {
    /// Contract id
    pub contract_id: String,
    /// Path to Clarinet.toml
    #[clap(long = "manifest-path")]
    pub manifest_path: Option<String>,
    // /// Fork contract and all its dependencies
    // #[clap(short = 'r')]
    // pub recursive: bool,
}

#[derive(Clap, PartialEq, Clone, Debug)]
struct Poke {
    /// Path to Clarinet.toml
    #[clap(long = "manifest-path")]
    pub manifest_path: Option<String>,
}

#[derive(Clap, PartialEq, Clone, Debug)]
struct Integrate {
    /// Path to Clarinet.toml
    #[clap(long = "manifest-path")]
    pub manifest_path: Option<String>,
    /// Display streams of logs instead of terminal UI dashboard
    #[clap(long = "no-dashboard")]
    pub no_dashboard: bool,
}

#[derive(Clap, PartialEq, Clone, Debug)]
struct Test {
    /// Generate coverage
    #[clap(long = "coverage")]
    pub coverage: bool,
    /// Generate costs report
    #[clap(long = "costs")]
    pub costs_report: bool,
    /// Path to Clarinet.toml
    #[clap(long = "manifest-path")]
    pub manifest_path: Option<String>,
    /// Relaunch tests on updates
    #[clap(long = "watch")]
    pub watch: bool,
    /// Files to includes
    pub files: Vec<String>,
}

#[derive(Clap, PartialEq, Clone, Debug)]
struct Run {
    /// Script to run
    pub script: String,
    /// Path to Clarinet.toml
    #[clap(long = "manifest-path")]
    pub manifest_path: Option<String>,
    /// Allow access to wallets
    #[clap(long = "allow-wallets")]
    pub allow_wallets: bool,
    /// Allow write access to disk
    #[clap(long = "allow-write")]
    pub allow_disk_write: bool,
    /// Allow read access to disk
    #[clap(long = "allow-read")]
    #[allow(dead_code)]
    pub allow_disk_read: bool,
}

#[derive(Clap, PartialEq, Clone, Debug)]
struct Publish {
    /// Deploy contracts on devnet, using settings/Devnet.toml
    #[clap(
        long = "devnet",
        conflicts_with = "testnet",
        conflicts_with = "mainnet"
    )]
    pub devnet: bool,
    /// Deploy contracts on testnet, using settings/Testnet.toml
    #[clap(
        long = "testnet",
        conflicts_with = "devnet",
        conflicts_with = "mainnet"
    )]
    pub testnet: bool,
    /// Deploy contracts on mainnet, using settings/Mainnet.toml
    #[clap(
        long = "mainnet",
        conflicts_with = "testnet",
        conflicts_with = "devnet"
    )]
    pub mainnet: bool,
    /// Path to Clarinet.toml
    #[clap(long = "manifest-path")]
    pub manifest_path: Option<String>,
}

#[derive(Clap, PartialEq, Clone, Debug)]
struct Check {
    /// Path to Clarinet.toml
    #[clap(long = "manifest-path")]
    pub manifest_path: Option<String>,
    /// If specified, check this file
    pub file: Option<String>,
}

pub fn main() {
    let opts: Opts = match Opts::try_parse() {
        Ok(opts) => opts,
        Err(_e) => {
            if _e.kind == clap::ErrorKind::UnknownArgument {
                match get_manifest_path(None) {
                    Some(manifest_path) => {
                        let manifest = ProjectManifest::from_path(&manifest_path);
                        if manifest.project.telemetry {
                            #[cfg(feature = "telemetry")]
                            telemetry_report_event(DeveloperUsageEvent::UnknownCommand(
                                DeveloperUsageDigest::new(
                                    &manifest.project.name,
                                    &manifest.project.authors,
                                ),
                                format!("{}", _e),
                            ));
                        }
                    }
                    None => {}
                };
            }
            println!("{}", _e);
            process::exit(1);
        }
    };

    let hints_enabled = if env::var("CLARINET_DISABLE_HINTS") == Ok("1".into()) {
        false
    } else {
        true
    };

    match opts.command {
        Command::New(project_opts) => {
            let current_path = {
                let current_dir = env::current_dir().expect("Unable to read current directory");
                current_dir.to_str().unwrap().to_owned()
            };

            let telemetry_enabled = if cfg!(feature = "telemetry") {
                if let Some(disable_telemetry) = project_opts.disable_telemetry {
                    !disable_telemetry
                } else {
                    println!("{}", yellow!("Send usage data to Hiro."));
                    println!("{}", yellow!("Help Hiro improve its products and services by automatically sending diagnostics and usage data."));
                    println!("{}", yellow!("Only high level usage information, and no information identifying you or your project are collected."));
                    // todo(ludo): once we have a privacy policy available, add a link
                    // println!("{}", yellow!("Visit http://hiro.so/clarinet-privacy for details."));
                    println!("{}", yellow!("Enable [Y/n]?"));
                    let mut buffer = String::new();
                    std::io::stdin().read_line(&mut buffer).unwrap();
                    buffer != "n\n"
                }
            } else {
                false
            };
            if telemetry_enabled {
                println!(
                    "{}",
                    yellow!("Telemetry enabled. Thanks for helping to improve clarinet!")
                );
            } else {
                println!(
                    "{}",
                    yellow!(
                        "Telemetry disabled. Clarinet will not collect any data on this project."
                    )
                );
            }
            let project_id = project_opts.name.clone();
            let changes =
                generate::get_changes_for_new_project(current_path, project_id, telemetry_enabled);
            execute_changes(changes);
            if hints_enabled {
                display_post_check_hint();
            }
            if telemetry_enabled {
                #[cfg(feature = "telemetry")]
                telemetry_report_event(DeveloperUsageEvent::NewProject(DeveloperUsageDigest::new(
                    &project_opts.name,
                    &vec![],
                )));
            }
        }
        Command::Contract(subcommand) => match subcommand {
            Contract::NewContract(new_contract) => {
                let manifest_path = get_manifest_path_or_exit(new_contract.manifest_path);

                let changes = generate::get_changes_for_new_contract(
                    manifest_path,
                    new_contract.name,
                    None,
                    true,
                    vec![],
                );
                execute_changes(changes);
                if hints_enabled {
                    display_post_check_hint();
                }
            }
            Contract::LinkContract(required_contract) => {
                let manifest_path = get_manifest_path_or_exit(required_contract.manifest_path);

                let change = TOMLEdition {
                    comment: format!(
                        "Adding {} as a requirement to Clarinet.toml",
                        required_contract.contract_id
                    ),
                    manifest_path,
                    contracts_to_add: HashMap::new(),
                    requirements_to_add: vec![RequirementConfig {
                        contract_id: required_contract.contract_id.clone(),
                    }],
                };
                execute_changes(vec![Changes::EditTOML(change)]);
                if hints_enabled {
                    display_post_check_hint();
                }
            }
            Contract::ForkContract(fork_contract) => {
                let manifest_path = get_manifest_path_or_exit(fork_contract.manifest_path);

                println!(
                    "Resolving {} and its dependencies...",
                    fork_contract.contract_id
                );

                let settings = repl::SessionSettings::default();
                let mut session = repl::Session::new(settings);

                let mut resolved = BTreeSet::new();
                let res = session.resolve_link(
                    &repl::settings::InitialLink {
                        contract_id: fork_contract.contract_id.clone(),
                        stacks_node_addr: None,
                        cache: None,
                    },
                    &mut resolved,
                );
                let contracts = res.unwrap();
                let mut changes = vec![];
                for (contract_id, code, deps) in contracts.into_iter() {
                    let components: Vec<&str> = contract_id.split('.').collect();
                    let contract_name = components.last().unwrap();

                    if &contract_id == &fork_contract.contract_id {
                        let mut change_set = generate::get_changes_for_new_contract(
                            manifest_path.clone(),
                            contract_name.to_string(),
                            Some(code),
                            false,
                            vec![],
                        );
                        changes.append(&mut change_set);

                        for dep in deps.iter() {
                            let mut change_set = generate::get_changes_for_new_link(
                                manifest_path.clone(),
                                dep.clone(),
                                None,
                            );
                            changes.append(&mut change_set);
                        }
                    }
                }
                execute_changes(changes);
                if hints_enabled {
                    display_post_check_hint();
                }
            }
        },
        Command::Poke(cmd) | Command::Console(cmd) => {
            let manifest_path = get_manifest_path_or_exit(cmd.manifest_path);
            let start_repl = true;
            let (_, _, project_manifest, _) =
                load_session(manifest_path, start_repl, &Network::Devnet)
                    .expect("Unable to start REPL");
            if hints_enabled {
                display_post_poke_hint();
            }
            if project_manifest.project.telemetry {
                #[cfg(feature = "telemetry")]
                telemetry_report_event(DeveloperUsageEvent::PokeExecuted(
                    DeveloperUsageDigest::new(
                        &project_manifest.project.name,
                        &project_manifest.project.authors,
                    ),
                ));
            }
        }
        Command::Check(cmd) if cmd.file.is_some() => {
            let file = cmd.file.unwrap();
            let mut settings = repl::SessionSettings::default();
            settings.repl_settings.analysis.enable_all_passes();

            let mut session = repl::Session::new(settings.clone());
            let code = match fs::read_to_string(&file) {
                Ok(code) => code,
                _ => {
                    println!("{}: unable to read file: '{}'", red!("error"), file);
                    std::process::exit(1);
                }
            };
            let contract_id = QualifiedContractIdentifier::transient();
            let (ast, mut diagnostics, mut success) = session.interpreter.build_ast(
                contract_id.clone(),
                code.clone(),
                settings.repl_settings.parser_version,
            );
            let (annotations, mut annotation_diagnostics) =
                session.interpreter.collect_annotations(&ast, &code);
            diagnostics.append(&mut annotation_diagnostics);

            let mut contract_analysis =
                ContractAnalysis::new(contract_id, ast.expressions, LimitedCostTracker::new_free());
            let mut analysis_db = AnalysisDatabase::new(&mut session.interpreter.datastore);
            let mut analysis_diagnostics = match analysis::run_analysis(
                &mut contract_analysis,
                &mut analysis_db,
                &annotations,
                &settings.repl_settings.analysis,
            ) {
                Ok(diagnostics) => diagnostics,
                Err(diagnostics) => {
                    success = false;
                    diagnostics
                }
            };
            diagnostics.append(&mut analysis_diagnostics);

            let lines = code.lines();
            let formatted_lines: Vec<String> = lines.map(|l| l.to_string()).collect();
            for d in diagnostics {
                for line in d.output(&file, &formatted_lines) {
                    println!("{}", line);
                }
            }

            if success {
                println!("{} Syntax of contract successfully checked", green!("✔"),);
                return;
            } else {
                std::process::exit(1);
            }
        }
        Command::Check(cmd) => {
            let manifest_path = get_manifest_path_or_exit(cmd.manifest_path);
            let start_repl = false;
            let project_manifest = match load_session(manifest_path, start_repl, &Network::Devnet) {
                Err(e) => {
                    println!("{}", e);
                    return;
                }
                Ok((session, _, manifest, output)) => {
                    if let Some(message) = output {
                        println!("{}", message);
                    }
                    println!(
                        "{} Syntax of {} contract(s) successfully checked",
                        green!("✔"),
                        session.settings.initial_contracts.len()
                    );
                    manifest
                }
            };
            if hints_enabled {
                display_post_check_hint();
            }
            if project_manifest.project.telemetry {
                #[cfg(feature = "telemetry")]
                telemetry_report_event(DeveloperUsageEvent::CheckExecuted(
                    DeveloperUsageDigest::new(
                        &project_manifest.project.name,
                        &project_manifest.project.authors,
                    ),
                ));
            }
        }
        Command::Test(cmd) => {
            let manifest_path = get_manifest_path_or_exit(cmd.manifest_path);
            let start_repl = false;
            let res = load_session(manifest_path.clone(), start_repl, &Network::Devnet);
            let (session, project_manifest) = match res {
                Ok((session, _, manifest, output)) => {
                    if let Some(message) = output {
                        println!("{}", message);
                    }
                    (session, manifest)
                }
                Err(e) => {
                    println!("{}", e);
                    return;
                }
            };
            let (success, _count) = match run_scripts(
                cmd.files,
                cmd.coverage,
                cmd.costs_report,
                cmd.watch,
                true,
                false,
                manifest_path,
                Some(session),
            ) {
                Ok(count) => (true, count),
                Err((_, count)) => (false, count),
            };
            if hints_enabled {
                display_tests_pro_tips_hint();
            }
            if project_manifest.project.telemetry {
                #[cfg(feature = "telemetry")]
                telemetry_report_event(DeveloperUsageEvent::TestSuiteExecuted(
                    DeveloperUsageDigest::new(
                        &project_manifest.project.name,
                        &project_manifest.project.authors,
                    ),
                    success,
                    _count,
                ));
            }
            if !success {
                process::exit(1)
            }
        }
        Command::Run(cmd) => {
            let manifest_path = get_manifest_path_or_exit(cmd.manifest_path);
            let start_repl = false;
            let res = load_session(manifest_path.clone(), start_repl, &Network::Devnet);
            let session = match res {
                Ok((session, _, _, output)) => {
                    if let Some(message) = output {
                        println!("{}", message);
                    }
                    session
                }
                Err(e) => {
                    println!("{}", e);
                    return;
                }
            };
            let _ = run_scripts(
                vec![cmd.script],
                false,
                false,
                false,
                cmd.allow_wallets,
                cmd.allow_disk_write,
                manifest_path,
                Some(session),
            );
        }
        Command::Publish(deploy) => {
            let manifest_path = get_manifest_path_or_exit(deploy.manifest_path);

            let network = if deploy.devnet == true {
                Network::Devnet
            } else if deploy.testnet == true {
                Network::Testnet
            } else if deploy.mainnet == true {
                Network::Mainnet
            } else {
                panic!("Target deployment must be specified with --devnet, --testnet or --mainnet")
            };
            let project_manifest = match publish_all_contracts(manifest_path, &network) {
                Ok((results, project_manifest)) => {
                    println!("{}", results.join("\n"));
                    project_manifest
                }
                Err(results) => {
                    println!("{}", results.join("\n"));
                    return;
                }
            };
            if project_manifest.project.telemetry {
                #[cfg(feature = "telemetry")]
                telemetry_report_event(DeveloperUsageEvent::ContractPublished(
                    DeveloperUsageDigest::new(
                        &project_manifest.project.name,
                        &project_manifest.project.authors,
                    ),
                    network,
                ));
            }
        }
        Command::Integrate(cmd) => {
            let manifest_path = get_manifest_path_or_exit(cmd.manifest_path);
            let devnet = DevnetOrchestrator::new(manifest_path, None);
            if devnet.manifest.project.telemetry {
                #[cfg(feature = "telemetry")]
                telemetry_report_event(DeveloperUsageEvent::DevnetExecuted(
                    DeveloperUsageDigest::new(
                        &devnet.manifest.project.name,
                        &devnet.manifest.project.authors,
                    ),
                ));
            }
            let _ = integrate::run_devnet(devnet, None, !cmd.no_dashboard);
            if hints_enabled {
                display_deploy_hint();
            }
        }
        Command::LSP => run_lsp(),
    };
}

fn get_manifest_path(path: Option<String>) -> Option<PathBuf> {
    if let Some(path) = path {
        let manifest_path = PathBuf::from(path);
        if !manifest_path.exists() {
            return None;
        }
        Some(manifest_path)
    } else {
        let mut current_dir = env::current_dir().unwrap();
        loop {
            current_dir.push("Clarinet.toml");

            if current_dir.exists() {
                return Some(current_dir);
            }
            current_dir.pop();

            if !current_dir.pop() {
                return None;
            }
        }
    }
}

fn get_manifest_path_or_exit(path: Option<String>) -> PathBuf {
    println!("");
    match get_manifest_path(path) {
        Some(manifest_path) => manifest_path,
        None => {
            println!("Could not find Clarinet.toml");
            process::exit(1);
        }
    }
}

fn execute_changes(changes: Vec<Changes>) {
    let mut shared_config = None;
    let mut path = PathBuf::new();

    for mut change in changes.into_iter() {
        match change {
            Changes::AddFile(options) => {
                if let Ok(entry) = fs::metadata(&options.path) {
                    if entry.is_file() {
                        println!(
                            "{}, file already exists at path {}",
                            red!("Skip creating file"),
                            options.path
                        );
                        continue;
                    }
                }
                println!("{}", options.comment);
                let mut file = File::create(options.path.clone()).expect("Unable to create file");
                file.write_all(options.content.as_bytes())
                    .expect("Unable to write file");
            }
            Changes::AddDirectory(options) => {
                println!("{}", options.comment);
                fs::create_dir_all(options.path.clone()).expect("Unable to create directory");
            }
            Changes::EditTOML(ref mut options) => {
                let mut config = match shared_config.take() {
                    Some(config) => config,
                    None => {
                        path = options.manifest_path.clone();
                        let file = File::open(path.clone()).unwrap();
                        let mut project_manifest_file_reader = BufReader::new(file);
                        let mut project_manifest_file = vec![];
                        project_manifest_file_reader
                            .read_to_end(&mut project_manifest_file)
                            .unwrap();
                        let project_manifest_file: ProjectManifestFile =
                            toml::from_slice(&project_manifest_file[..]).unwrap();
                        ProjectManifest::from_project_manifest_file(project_manifest_file)
                    }
                };

                let mut requirements = match config.project.requirements.take() {
                    Some(requirements) => requirements,
                    None => vec![],
                };
                for requirement in options.requirements_to_add.drain(..) {
                    if !requirements.contains(&requirement) {
                        requirements.push(requirement);
                    }
                }
                config.project.requirements = Some(requirements);

                for (contract_name, contract_config) in options.contracts_to_add.drain() {
                    config.contracts.insert(contract_name, contract_config);
                }

                shared_config = Some(config);
                println!("{}", options.comment);
            }
        }
    }

    if let Some(config) = shared_config {
        let toml_value = toml::Value::try_from(&config).unwrap();
        let toml = format!("{}", toml_value);
        let mut file = File::create(path).unwrap();
        file.write_all(&toml.as_bytes()).unwrap();
        file.sync_all().unwrap();
    }
}

fn display_separator() {
    println!("{}", yellow!("----------------------------"));
}

fn display_hint_header() {
    display_separator();
    println!("{}", yellow!("Hint: what's next?"));
}

fn display_hint_footer() {
    println!(
        "{}",
        yellow!("Disable these hints with the env var CLARINET_DISABLE_HINTS=1")
    );
    display_separator();
}

fn display_post_check_hint() {
    println!("");
    display_hint_header();
    println!(
        "{}",
        yellow!("Once you are ready to write TypeScript unit tests for your contract, run the following command:\n")
    );
    println!("{}", blue!("  $ clarinet test"));
    println!(
        "{}",
        yellow!("    Run all run tests in the ./tests folder.\n")
    );
    println!("{}", yellow!("Find more information on testing with Clarinet here: https://docs.hiro.so/smart-contracts/clarinet#testing-with-the-test-harness"));
    display_hint_footer();
}

fn display_post_poke_hint() {
    println!("");
    display_hint_header();
    println!(
        "{}",
        yellow!("Once your are ready to write your contracts, run the following commands:\n")
    );
    println!("{}", blue!("  $ clarinet contract new <contract-name>"));
    println!(
        "{}",
        yellow!("    Create new contract scaffolding, including test files.\n")
    );

    println!("{}", blue!("  $ clarinet check"));
    println!(
        "{}",
        yellow!("    Check contract syntax for all files in ./contracts.\n")
    );

    println!("{}", yellow!("Find more information on writing contracts with Clarinet here: https://docs.hiro.so/smart-contracts/clarinet#developing-a-clarity-smart-contract"));
    display_hint_footer();
}

fn display_tests_pro_tips_hint() {
    println!("");
    display_separator();
    println!(
        "{}",
        yellow!("Check out the pro tips to improve your testing process:\n")
    );

    println!("{}", blue!("  $ clarinet test --watch"));
    println!(
        "{}",
        yellow!("    Watch for file changes an re-run all tests.\n")
    );

    println!("{}", blue!("  $ clarinet test --costs"));
    println!(
        "{}",
        yellow!("    Run a cost analysis of the contracts covered by tests.\n")
    );

    println!("{}", blue!("  $ clarinet test --coverage"));
    println!(
        "{}",
        yellow!("    Measure test coverage with the LCOV tooling suite.\n")
    );

    println!("{}", yellow!("Once you are ready to test your contracts on a local developer network, run the following:\n"));

    println!("{}", blue!("  $ clarinet integrate"));
    println!(
        "{}",
        yellow!("    Deploy all contracts to a local dockerized blockchain setup (Devnet).\n")
    );

    println!("{}", yellow!("Find more information on testing with Clarinet here: https://docs.hiro.so/smart-contracts/clarinet#testing-with-clarinet"));
    println!("{}", yellow!("And learn more about local integration testing here: https://docs.hiro.so/smart-contracts/devnet"));
    display_hint_footer();
}

fn display_deploy_hint() {
    println!("");
    display_hint_header();
    println!(
        "{}",
        yellow!("Once your contracts are ready to be deployed, you can run the following:")
    );

    println!("{}", blue!("  $ clarinet deploy --testnet"));
    println!(
        "{}",
        yellow!("    Deploy all contracts to the testnet network.\n")
    );

    println!("{}", blue!("  $ clarinet deploy --mainnet"));
    println!(
        "{}",
        yellow!("    Deploy all contracts to the mainnet network.\n")
    );

    println!(
        "{}",
        yellow!("Keep in mind, you can configure networks by editing the TOML files in the ./settings folder")
    );
    println!(
        "{}",
        yellow!("Find more information on the DevNet here: https://docs.hiro.so/smart-contracts/devnet/")
    );
    display_hint_footer();
}

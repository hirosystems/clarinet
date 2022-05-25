use crate::deployment::{
    self, apply_on_chain_deployment, check_deployments, generate_default_deployment,
    get_absolute_deployment_path, get_default_deployment_path, get_initial_transactions_trackers,
    load_deployment, load_deployment_if_exists, setup_session_with_deployment,
    types::DeploymentSpecification, write_deployment, DeploymentCommand, DeploymentEvent,
    DeploymentGenerationArtifacts,
};
use crate::generate::{
    self,
    changes::{Changes, TOMLEdition},
};
use crate::hook::check_hooks;
use crate::integrate::{self, DevnetOrchestrator};
use crate::lsp::run_lsp;
use crate::runnner::run_scripts;
use crate::runnner::DeploymentCache;
use crate::types::{ProjectManifest, ProjectManifestFile, RequirementConfig, StacksNetwork};
use clarity_repl::clarity::analysis::{AnalysisDatabase, ContractAnalysis};
use clarity_repl::clarity::costs::LimitedCostTracker;
use clarity_repl::clarity::diagnostic::Level;
use clarity_repl::clarity::types::QualifiedContractIdentifier;
use clarity_repl::{analysis, repl, Terminal};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{prelude::*, BufReader, Read};
use std::path::PathBuf;
use std::{env, process};

use clap::{IntoApp, Parser, Subcommand};
use clap_generate::{Generator, Shell};
use toml;

macro_rules! pluralize {
    ($value:expr, $word:expr) => {
        if $value > 1 {
            format!("{} {}s", $value, $word)
        } else {
            format!("{} {}", $value, $word)
        }
    };
}

#[cfg(feature = "telemetry")]
use super::telemetry::{telemetry_report_event, DeveloperUsageDigest, DeveloperUsageEvent};

#[derive(Parser, PartialEq, Clone, Debug)]
#[clap(version = option_env!("CARGO_PKG_VERSION").expect("Unable to detect version"), bin_name = "clarinet")]
struct Opts {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand, PartialEq, Clone, Debug)]
enum Command {
    /// Create and scaffold a new project
    #[clap(name = "new", bin_name = "new")]
    New(GenerateProject),
    /// Subcommands for working with contracts
    #[clap(subcommand, name = "contracts")]
    Contracts(Contracts),
    /// Subcommands for working with requirements
    #[clap(subcommand, name = "requirements")]
    Requirements(Requirements),
    /// Subcommands for working with hooks
    #[clap(subcommand, name = "hooks")]
    Hooks(Hooks),
    /// Subcommands for working with deployments
    #[clap(subcommand, name = "deployments")]
    Deployments(Deployments),
    /// Load contracts in a REPL for an interactive session
    #[clap(name = "console", aliases = &["poke"], bin_name = "console")]
    Console(Console),
    /// Execute test suite
    #[clap(name = "test", bin_name = "test")]
    Test(Test),
    /// Check syntax of your contracts
    #[clap(name = "check", bin_name = "check")]
    Check(Check),
    /// Execute Clarinet extension
    #[clap(name = "run", bin_name = "run")]
    Run(Run),
    /// Start devnet environment for integration testing
    #[clap(name = "integrate", bin_name = "integrate")]
    Integrate(Integrate),
    /// Start an LSP server (for integration with editors)
    #[clap(name = "lsp", bin_name = "lsp")]
    LSP,
    /// Start a DAP server (for debugging from IDE)
    #[clap(name = "dap", bin_name = "dap")]
    DAP,
    /// Generate shell completions scripts
    #[clap(name = "completions", bin_name = "completions")]
    Completions(Completions),
}

#[derive(Subcommand, PartialEq, Clone, Debug)]
#[clap(bin_name = "contract", aliases = &["contract"])]
enum Contracts {
    /// Generate files and settings for a new contract
    #[clap(name = "new", bin_name = "new")]
    NewContract(NewContract),
}

#[derive(Subcommand, PartialEq, Clone, Debug)]
#[clap(bin_name = "req", aliases = &["requirement"])]
enum Requirements {
    /// Add third-party requirements to this project
    #[clap(name = "add", bin_name = "add")]
    AddRequirement(AddRequirement),
}

#[derive(Subcommand, PartialEq, Clone, Debug)]
#[clap(bin_name = "deployment", aliases = &["deployments"])]
enum Deployments {
    /// Check deployments format
    #[clap(name = "check", bin_name = "check")]
    CheckDeployments(CheckDeployments),
    /// Generate new deployment
    #[clap(name = "generate", bin_name = "generate")]
    GenerateDeployment(GenerateDeployment),
    /// Apply deployment
    #[clap(name = "apply", bin_name = "apply")]
    ApplyDeployment(ApplyDeployment),
}

#[derive(Subcommand, PartialEq, Clone, Debug)]
#[clap(bin_name = "hook", aliases = &["hooks"])]
enum Hooks {
    /// Generate files and settings for a new hook
    #[clap(name = "new", bin_name = "new")]
    NewHook(NewHook),
    /// Check hooks format
    #[clap(name = "check", bin_name = "check")]
    CheckHooks(CheckHooks),
    /// Publish contracts on chain
    #[clap(name = "deploy", bin_name = "deploy")]
    DeployHooks(DeployHooks),
}

#[derive(Parser, PartialEq, Clone, Debug)]
struct GenerateProject {
    /// Project's name
    pub name: String,
    /// Do not provide developer usage telemetry for this project
    #[clap(long = "disable-telemetry", takes_value = false)]
    pub disable_telemetry: bool,
}

#[derive(Parser, PartialEq, Clone, Debug)]
struct NewContract {
    /// Contract's name
    pub name: String,
    /// Path to Clarinet.toml
    #[clap(long = "manifest-path", short = 'm')]
    pub manifest_path: Option<String>,
}

#[derive(Parser, PartialEq, Clone, Debug)]
struct AddRequirement {
    /// Contract id (ex. "SP2PABAF9FTAJYNFZH93XENAJ8FVY99RRM50D2JG9.nft-trait")
    pub contract_id: String,
    /// Path to Clarinet.toml
    #[clap(long = "manifest-path", short = 'm')]
    pub manifest_path: Option<String>,
}

#[derive(Parser, PartialEq, Clone, Debug)]
struct CheckDeployments {
    /// Path to Clarinet.toml
    #[clap(long = "manifest-path", short = 'm')]
    pub manifest_path: Option<String>,
}

#[derive(Parser, PartialEq, Clone, Debug)]
struct GenerateDeployment {
    /// Generate a deployment file for simnet environments (console, tests)
    #[clap(
        long = "simnet",
        conflicts_with = "devnet",
        conflicts_with = "testnet",
        conflicts_with = "mainnet"
    )]
    pub simnet: bool,
    /// Generate a deployment file for devnet, using settings/Devnet.toml
    #[clap(
        long = "devnet",
        conflicts_with = "test",
        conflicts_with = "testnet",
        conflicts_with = "mainnet"
    )]
    pub devnet: bool,
    /// Generate a deployment file for devnet, using settings/Testnet.toml
    #[clap(
        long = "testnet",
        conflicts_with = "test",
        conflicts_with = "devnet",
        conflicts_with = "mainnet"
    )]
    pub testnet: bool,
    /// Generate a deployment file for devnet, using settings/Mainnet.toml
    #[clap(
        long = "mainnet",
        conflicts_with = "test",
        conflicts_with = "testnet",
        conflicts_with = "devnet"
    )]
    pub mainnet: bool,
    /// Path to Clarinet.toml
    #[clap(long = "manifest-path", short = 'm')]
    pub manifest_path: Option<String>,
}

#[derive(Parser, PartialEq, Clone, Debug)]
struct NewHook {
    /// Hook's name
    pub name: String,
    /// Path to Clarinet.toml
    #[clap(long = "manifest-path")]
    pub manifest_path: Option<String>,
}

#[derive(Parser, PartialEq, Clone, Debug)]
struct CheckHooks {
    /// Path to Clarinet.toml
    #[clap(long = "manifest-path")]
    pub manifest_path: Option<String>,
    /// Path to Clarinet.toml
    #[clap(long = "output-json")]
    pub output_json: bool,
}

#[derive(Parser, PartialEq, Clone, Debug)]
struct DeployHooks {
    /// Path to Clarinet.toml
    #[clap(long = "manifest-path")]
    pub manifest_path: Option<String>,
}

#[derive(Parser, PartialEq, Clone, Debug)]
struct ApplyDeployment {
    /// Apply default deployment settings/default.devnet-plan.toml
    #[clap(
        long = "devnet",
        conflicts_with = "deployment-plan-path",
        conflicts_with = "testnet",
        conflicts_with = "mainnet"
    )]
    pub devnet: bool,
    /// Apply default deployment settings/default.testnet-plan.toml
    #[clap(
        long = "testnet",
        conflicts_with = "deployment-plan-path",
        conflicts_with = "devnet",
        conflicts_with = "mainnet"
    )]
    pub testnet: bool,
    /// Apply default deployment settings/default.mainnet-plan.toml
    #[clap(
        long = "mainnet",
        conflicts_with = "deployment-plan-path",
        conflicts_with = "testnet",
        conflicts_with = "devnet"
    )]
    pub mainnet: bool,
    /// Path to Clarinet.toml
    #[clap(long = "manifest-path", short = 'm')]
    pub manifest_path: Option<String>,
    /// Apply deployment plan specified
    #[clap(
        long = "deployment-plan-path",
        short = 'p',
        conflicts_with = "devnet",
        conflicts_with = "testnet",
        conflicts_with = "mainnet"
    )]
    pub deployment_plan_path: Option<String>,
    /// Display streams of logs instead of terminal UI dashboard
    #[clap(long = "no-dashboard")]
    pub no_dashboard: bool,
}

#[derive(Parser, PartialEq, Clone, Debug)]
struct Console {
    /// Path to Clarinet.toml
    #[clap(long = "manifest-path", short = 'm')]
    pub manifest_path: Option<String>,
    /// If specified, use this deployment file
    #[clap(long = "deployment-plan-path", short = 'p')]
    pub deployment_plan_path: Option<String>,
}

#[derive(Parser, PartialEq, Clone, Debug)]
struct Integrate {
    /// Path to Clarinet.toml
    #[clap(long = "manifest-path", short = 'm')]
    pub manifest_path: Option<String>,
    /// Display streams of logs instead of terminal UI dashboard
    #[clap(long = "no-dashboard")]
    pub no_dashboard: bool,
    /// If specified, use this deployment file
    #[clap(long = "deployment-plan-path", short = 'p')]
    pub deployment_plan_path: Option<String>,
}

#[derive(Parser, PartialEq, Clone, Debug)]
struct Test {
    /// Generate coverage file (coverage.lcov)
    #[clap(long = "coverage")]
    pub coverage: bool,
    /// Generate costs report
    #[clap(long = "costs")]
    pub costs_report: bool,
    /// Path to Clarinet.toml
    #[clap(long = "manifest-path", short = 'm')]
    pub manifest_path: Option<String>,
    /// Relaunch tests upon updates to contracts
    #[clap(long = "watch")]
    pub watch: bool,
    /// Test files to be included (defaults to all tests found under tests/)
    pub files: Vec<String>,
    /// If specified, use this deployment file
    #[clap(long = "deployment-plan-path", short = 'p')]
    pub deployment_plan_path: Option<String>,
}

#[derive(Parser, PartialEq, Clone, Debug)]
struct Run {
    /// Script to run
    pub script: String,
    /// Path to Clarinet.toml
    #[clap(long = "manifest-path", short = 'm')]
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
    /// If specified, use this deployment file
    #[clap(long = "deployment-plan-path", short = 'p')]
    pub deployment_plan_path: Option<String>,
}

#[derive(Parser, PartialEq, Clone, Debug)]
struct Check {
    /// Path to Clarinet.toml
    #[clap(long = "manifest-path", short = 'm')]
    pub manifest_path: Option<String>,
    /// If specified, check this file
    pub file: Option<String>,
    /// If specified, use this deployment file
    #[clap(long = "deployment-plan-path", short = 'p')]
    pub deployment_plan_path: Option<String>,
}

#[derive(Parser, PartialEq, Clone, Debug)]
struct Completions {
    /// Specify which shell to generation completions script for
    #[clap(arg_enum, ignore_case = true)]
    pub shell: Shell,
}

pub fn main() {
    let opts: Opts = match Opts::try_parse() {
        Ok(opts) => opts,
        Err(e) => {
            if e.kind() == clap::ErrorKind::UnknownArgument {
                let manifest = load_manifest_or_exit(None);
                if manifest.project.telemetry {
                    #[cfg(feature = "telemetry")]
                    telemetry_report_event(DeveloperUsageEvent::UnknownCommand(
                        DeveloperUsageDigest::new(
                            &manifest.project.name,
                            &manifest.project.authors,
                        ),
                        format!("{}", e),
                    ));
                }
            }
            println!("{}", e);
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
                let current_dir = match env::current_dir() {
                    Ok(dir) => dir,
                    Err(e) => {
                        println!("{}: Unable to get current directory: {}", red!("error"), e);
                        std::process::exit(1);
                    }
                };
                current_dir.to_str().unwrap().to_owned()
            };

            let telemetry_enabled = if cfg!(feature = "telemetry") {
                if project_opts.disable_telemetry {
                    false
                } else {
                    println!("{}", yellow!("Send usage data to Hiro."));
                    println!("{}", yellow!("Help Hiro improve its products and services by automatically sending diagnostics and usage data."));
                    println!("{}", yellow!("Only high level usage information, and no information identifying you or your project are collected."));
                    // TODO(lgalabru): once we have a privacy policy available, add a link
                    // println!("{}", yellow!("Visit http://hiro.so/clarinet-privacy for details."));
                    println!("{}", yellow!("Enable [Y/n]?"));
                    let mut buffer = String::new();
                    std::io::stdin().read_line(&mut buffer).unwrap();
                    !buffer.starts_with("n")
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
            if !execute_changes(changes) {
                std::process::exit(1);
            }
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
        Command::Deployments(subcommand) => match subcommand {
            Deployments::CheckDeployments(cmd) => {
                let manifest = load_manifest_or_exit(cmd.manifest_path);
                // Ensure that all the deployments can correctly be deserialized.
                println!("Checking deployments");
                let res = check_deployments(&manifest);
                if let Err(message) = res {
                    println!("{}", message);
                    process::exit(1);
                }
            }
            Deployments::GenerateDeployment(cmd) => {
                let manifest = load_manifest_or_exit(cmd.manifest_path);

                let network = if cmd.devnet == true {
                    StacksNetwork::Devnet
                } else if cmd.testnet == true {
                    StacksNetwork::Testnet
                } else if cmd.mainnet == true {
                    StacksNetwork::Mainnet
                } else {
                    StacksNetwork::Simnet
                };

                let default_deployment_path = get_default_deployment_path(&manifest, &network);
                let (deployment, _) = match generate_default_deployment(&manifest, &network) {
                    Ok(deployment) => deployment,
                    Err(message) => {
                        println!("{}", red!(message));
                        std::process::exit(1);
                    }
                };
                let res = write_deployment(&deployment, &default_deployment_path, true);
                if let Err(message) = res {
                    println!("{}", message);
                    process::exit(1);
                }

                let mut relative_path = PathBuf::from("deployments");
                relative_path.push(default_deployment_path.file_name().unwrap());
                println!("{} {}", green!("Generated file"), relative_path.display());
            }
            Deployments::ApplyDeployment(cmd) => {
                let manifest = load_manifest_or_exit(cmd.manifest_path);

                let network = if cmd.devnet == true {
                    Some(StacksNetwork::Devnet)
                } else if cmd.testnet == true {
                    Some(StacksNetwork::Testnet)
                } else if cmd.mainnet == true {
                    Some(StacksNetwork::Mainnet)
                } else {
                    None
                };

                let result = match (&network, cmd.deployment_plan_path) {
                    (None, None) => {
                        Err(format!("{}: a flag `--devnet`, `--testnet`, `--mainnet` or `--deployment-plan-path=path/to/yaml` should be provided.", yellow!("Command usage")))
                    }
                    (Some(network), None) => {
                        let res = load_deployment_if_exists(&manifest, &network);
                        match res {
                            Some(Ok(deployment)) => {
                                println!(
                                    "{}: using existing deployments/default.{}-plan.yaml",
                                    yellow!("note"),
                                    format!("{:?}", network).to_lowercase(),
                                );
                                Ok(deployment)
                            }
                            Some(Err(e)) => Err(e),
                            None => {
                                let default_deployment_path = get_default_deployment_path(&manifest, &network);
                                let (deployment, _) = match generate_default_deployment(&manifest, &network) {
                                    Ok(deployment) => deployment,
                                    Err(message) => {
                                        println!("{}", red!(message));
                                        std::process::exit(1);
                                    }
                                };
                                let res = write_deployment(&deployment, &default_deployment_path, true);
                                if let Err(message) = res {
                                    Err(message)
                                } else {
                                    let mut relative_path = PathBuf::from("deployments");
                                    relative_path.push(default_deployment_path.file_name().unwrap());
                                    println!("{} {}", green!("Generated file"), relative_path.display());
                                    Ok(deployment)
                                }
                            }
                        }
                    }
                    (None, Some(deployment_plan_path)) => {
                        let deployment_path = get_absolute_deployment_path(&manifest, &deployment_plan_path);
                        load_deployment(&manifest, &deployment_path)
                    }
                    (_, _) => unreachable!()
                };

                let deployment = match result {
                    Ok(deployment) => deployment,
                    Err(e) => {
                        println!("{}", e);
                        std::process::exit(1);
                    }
                };

                let node_url = deployment.node.clone().unwrap();

                println!(
                    "The following deployment plan will be applied:\n{}\n\n{}",
                    deployment.get_synthesis(),
                    yellow!("Continue [Y/n]?")
                );
                let mut buffer = String::new();
                std::io::stdin().read_line(&mut buffer).unwrap();
                if !buffer.starts_with("Y") && !buffer.starts_with("y") && !buffer.starts_with("\n")
                {
                    println!("Deployment aborted");
                    std::process::exit(1);
                }

                let (command_tx, command_rx) = std::sync::mpsc::channel();
                let (event_tx, event_rx) = std::sync::mpsc::channel();
                let manifest_moved = manifest.clone();

                if manifest.project.telemetry {
                    #[cfg(feature = "telemetry")]
                    telemetry_report_event(DeveloperUsageEvent::ProtocolPublished(
                        DeveloperUsageDigest::new(
                            &manifest.project.name,
                            &manifest.project.authors,
                        ),
                        deployment.network.clone(),
                    ));
                }

                let transaction_trackers = if cmd.no_dashboard {
                    vec![]
                } else {
                    get_initial_transactions_trackers(&deployment)
                };

                std::thread::spawn(move || {
                    let manifest = manifest_moved;
                    apply_on_chain_deployment(&manifest, deployment, event_tx, command_rx, true);
                });

                let _ = command_tx.send(DeploymentCommand::Start);

                if cmd.no_dashboard {
                    loop {
                        let cmd = match event_rx.recv() {
                            Ok(cmd) => cmd,
                            Err(_e) => break,
                        };
                        match cmd {
                            DeploymentEvent::Interrupted(message) => {
                                println!("{} Error deploying contracts: {}", red!("x"), message);
                                break;
                            }
                            DeploymentEvent::TransactionUpdate(update) => {
                                println!("{} {:?} {}", blue!("➡"), update.status, update.name);
                            }
                            DeploymentEvent::ProtocolDeployed => {
                                println!(
                                    "{} Contracts successfully deployed on {:?}",
                                    green!("✔"),
                                    network.unwrap()
                                );
                                break;
                            }
                        }
                    }
                } else {
                    let res = deployment::start_ui(&node_url, event_rx, transaction_trackers);
                    match res {
                        Ok(()) => println!(
                            "{} Contracts successfully deployed on {:?}",
                            green!("✔"),
                            network.unwrap()
                        ),
                        Err(message) => {
                            println!("{} Error deploying contracts: {}", red!("x"), message)
                        }
                    }
                }
            }
        },
        Command::Hooks(subcommand) => match subcommand {
            Hooks::NewHook(cmd) => {
                let manifest_path = get_manifest_path_or_exit(cmd.manifest_path);

                // let changes = generate::get_changes_for_new_contract(
                //     manifest_path,
                //     new_contract.name,
                //     None,
                //     true,
                //     vec![],
                // );
                // if !execute_changes(changes) {
                //     std::process::exit(1);
                // }
                // if hints_enabled {
                //     display_post_check_hint();
                // }
            }
            Hooks::CheckHooks(cmd) => {
                let manifest_path = get_manifest_path_or_exit(cmd.manifest_path);
                // Ensure that all the hooks can correctly be deserialized.
                println!("Checking hooks");
                let _ = check_hooks(&manifest_path, cmd.output_json);
            }
            Hooks::DeployHooks(cmd) => {
                let manifest_path = get_manifest_path_or_exit(cmd.manifest_path);
                // Deploy hooks
            }
        },
        Command::Contracts(subcommand) => match subcommand {
            Contracts::NewContract(cmd) => {
                let manifest = load_manifest_or_exit(cmd.manifest_path);

                let changes =
                    generate::get_changes_for_new_contract(&manifest.path, cmd.name, None, true);
                if !execute_changes(changes) {
                    std::process::exit(1);
                }
                if hints_enabled {
                    display_post_check_hint();
                }
            }
        },
        Command::Requirements(subcommand) => match subcommand {
            Requirements::AddRequirement(cmd) => {
                let manifest = load_manifest_or_exit(cmd.manifest_path);

                let change = TOMLEdition {
                    comment: format!(
                        "Adding {} as a requirement to Clarinet.toml",
                        cmd.contract_id
                    ),
                    manifest_path: manifest.path.clone(),
                    contracts_to_add: HashMap::new(),
                    requirements_to_add: vec![RequirementConfig {
                        contract_id: cmd.contract_id.clone(),
                    }],
                };
                if !execute_changes(vec![Changes::EditTOML(change)]) {
                    std::process::exit(1);
                }
                if hints_enabled {
                    display_post_check_hint();
                }
            }
        },
        Command::Console(cmd) => {
            let manifest = load_manifest_or_exit(cmd.manifest_path);

            let cache = build_deployment_cache_or_exit(&manifest, &cmd.deployment_plan_path);

            let mut terminal = Terminal::load(cache.session);
            terminal.start();

            if hints_enabled {
                display_post_console_hint();
            }

            // Report telemetry
            if manifest.project.telemetry {
                #[cfg(feature = "telemetry")]
                telemetry_report_event(DeveloperUsageEvent::PokeExecuted(
                    DeveloperUsageDigest::new(&manifest.project.name, &manifest.project.authors),
                ));

                #[cfg(feature = "telemetry")]
                let mut debug_count = 0;
                for command in terminal.session.executed {
                    if command.starts_with("::debug") {
                        debug_count += 1;
                    }
                }
                if debug_count > 0 {
                    telemetry_report_event(DeveloperUsageEvent::DebugStarted(
                        DeveloperUsageDigest::new(
                            &manifest.project.name,
                            &manifest.project.authors,
                        ),
                        debug_count,
                    ));
                }
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
            let manifest = load_manifest_or_exit(cmd.manifest_path);

            let (deployment, _, artifacts) =
                load_deployments_and_artifacts_or_exit(&manifest, &cmd.deployment_plan_path);

            let contracts_asts = artifacts.and_then(|a| Some(a.asts));

            let (_, results) =
                setup_session_with_deployment(&manifest, &deployment, contracts_asts);

            let mut success = 0;
            let mut warnings = 0;
            let mut errors = 0;
            let mut contracts_checked = 0;
            let mut outputs = vec![];
            for (contract_id, result) in results.into_iter() {
                contracts_checked += 1;
                match result {
                    Ok(result) => {
                        if result.diagnostics.is_empty() {
                            success += 1;
                            continue;
                        }

                        let (source, contract_path) = deployment
                            .contracts
                            .get(&contract_id)
                            .expect("unable to retrieve contract");

                        let lines = source.lines();
                        let formatted_lines: Vec<String> = lines.map(|l| l.to_string()).collect();

                        for diagnostic in result.diagnostics {
                            match diagnostic.level {
                                Level::Error => {
                                    errors += 1;
                                    outputs.push(format!(
                                        "{}: {}",
                                        red!("error"),
                                        diagnostic.message
                                    ));
                                }
                                Level::Warning => {
                                    warnings += 1;
                                    outputs.push(format!(
                                        "{}: {}",
                                        yellow!("warning"),
                                        diagnostic.message
                                    ));
                                }
                                Level::Note => {
                                    outputs.push(format!(
                                        "{}: {}",
                                        green!("note:"),
                                        diagnostic.message
                                    ));
                                    outputs.append(&mut diagnostic.output_code(&formatted_lines));
                                    continue;
                                }
                            }
                            if let Some(span) = diagnostic.spans.first() {
                                outputs.push(format!(
                                    "{} {}:{}:{}",
                                    blue!("-->"),
                                    contract_path,
                                    span.start_line,
                                    span.start_column
                                ));
                            }
                            outputs.append(&mut diagnostic.output_code(&formatted_lines));

                            if let Some(suggestion) = diagnostic.suggestion {
                                outputs.push(format!("{}", suggestion));
                            }
                        }
                    }
                    Err(diagnostics) => {
                        let (source, contract_path) = deployment
                            .contracts
                            .get(&contract_id)
                            .expect("unable to retrieve contract");
                        let lines = source.lines();
                        let formatted_lines: Vec<String> = lines.map(|l| l.to_string()).collect();

                        for diagnostic in diagnostics {
                            match diagnostic.level {
                                Level::Error => {
                                    errors += 1;
                                    outputs.push(format!(
                                        "{}: {}",
                                        red!("error"),
                                        diagnostic.message
                                    ));
                                }
                                Level::Warning => {
                                    warnings += 1;
                                    outputs.push(format!(
                                        "{}: {}",
                                        yellow!("warning"),
                                        diagnostic.message
                                    ));
                                }
                                Level::Note => {
                                    outputs.push(format!(
                                        "{}: {}",
                                        green!("note:"),
                                        diagnostic.message
                                    ));
                                    outputs.append(&mut diagnostic.output_code(&formatted_lines));
                                    continue;
                                }
                            }
                            if let Some(span) = diagnostic.spans.first() {
                                outputs.push(format!(
                                    "{} {}:{}:{}",
                                    blue!("-->"),
                                    contract_path,
                                    span.start_line,
                                    span.start_column
                                ));
                            }
                            outputs.append(&mut diagnostic.output_code(&formatted_lines));

                            if let Some(suggestion) = diagnostic.suggestion {
                                outputs.push(format!("{}", suggestion));
                            }
                        }
                    }
                }
            }
            if !outputs.is_empty() {
                println!("{}\n", outputs.join("\n"));
            }
            if warnings > 0 {
                println!(
                    "{} {} detected",
                    yellow!("!"),
                    pluralize!(warnings, "warning")
                );
            }
            if errors > 0 {
                println!("{} {} detected", red!("x"), pluralize!(errors, "error"));
            } else {
                println!(
                    "{} {} checked, {} successfully validated",
                    green!("✔"),
                    pluralize!(contracts_checked, "contract"),
                    success
                );
            }

            if hints_enabled {
                display_post_check_hint();
            }
            if manifest.project.telemetry {
                #[cfg(feature = "telemetry")]
                telemetry_report_event(DeveloperUsageEvent::CheckExecuted(
                    DeveloperUsageDigest::new(&manifest.project.name, &manifest.project.authors),
                ));
            }
        }
        Command::Test(cmd) => {
            let manifest = load_manifest_or_exit(cmd.manifest_path);
            let deployment_plan_path = cmd.deployment_plan_path.clone();
            let cache = build_deployment_cache_or_exit(&manifest, &deployment_plan_path);

            let (success, _count) = match run_scripts(
                cmd.files,
                cmd.coverage,
                cmd.costs_report,
                cmd.watch,
                true,
                false,
                &manifest,
                cache,
                deployment_plan_path,
            ) {
                Ok(count) => (true, count),
                Err((_, count)) => (false, count),
            };
            if hints_enabled {
                display_tests_pro_tips_hint();
            }
            if manifest.project.telemetry {
                #[cfg(feature = "telemetry")]
                telemetry_report_event(DeveloperUsageEvent::TestSuiteExecuted(
                    DeveloperUsageDigest::new(&manifest.project.name, &manifest.project.authors),
                    success,
                    _count,
                ));
            }
            if !success {
                process::exit(1)
            }
        }
        Command::Run(cmd) => {
            let manifest = load_manifest_or_exit(cmd.manifest_path);

            let cache = build_deployment_cache_or_exit(&manifest, &cmd.deployment_plan_path);

            let _ = run_scripts(
                vec![cmd.script],
                false,
                false,
                false,
                cmd.allow_wallets,
                cmd.allow_disk_write,
                &manifest,
                cache,
                cmd.deployment_plan_path,
            );
        }
        Command::Integrate(cmd) => {
            let manifest = load_manifest_or_exit(cmd.manifest_path);
            println!("Loading deployment plan");
            let result = match cmd.deployment_plan_path {
                None => {
                    let res = load_deployment_if_exists(&manifest, &StacksNetwork::Devnet);
                    match res {
                        Some(Ok(deployment)) => {
                            println!(
                                "{}: using existing deployments/default.devnet-plan.yaml",
                                yellow!("note")
                            );
                            // TODO(lgalabru): Think more about the desired DX.
                            // Compute the latest version, display differences and propose overwrite?
                            Ok(deployment)
                        }
                        Some(Err(e)) => Err(e),
                        None => {
                            let default_deployment_path =
                                get_default_deployment_path(&manifest, &StacksNetwork::Devnet);
                            let (deployment, _) = match generate_default_deployment(
                                &manifest,
                                &StacksNetwork::Devnet,
                            ) {
                                Ok(deployment) => deployment,
                                Err(message) => {
                                    println!("{}", red!(message));
                                    std::process::exit(1);
                                }
                            };
                            let res = write_deployment(&deployment, &default_deployment_path, true);
                            if let Err(message) = res {
                                Err(message)
                            } else {
                                let mut relative_path = PathBuf::from("deployments");
                                relative_path.push(default_deployment_path.file_name().unwrap());
                                println!(
                                    "{} {}",
                                    green!("Generated file"),
                                    relative_path.display()
                                );
                                Ok(deployment)
                            }
                        }
                    }
                }
                Some(deployment_plan_path) => {
                    let deployment_path =
                        get_absolute_deployment_path(&manifest, &deployment_plan_path);
                    load_deployment(&manifest, &deployment_path)
                }
            };

            let deployment = match result {
                Ok(deployment) => deployment,
                Err(e) => {
                    println!("{}", e);
                    std::process::exit(1);
                }
            };

            let devnet = DevnetOrchestrator::new(manifest, None);
            if devnet.manifest.project.telemetry {
                #[cfg(feature = "telemetry")]
                telemetry_report_event(DeveloperUsageEvent::DevnetExecuted(
                    DeveloperUsageDigest::new(
                        &devnet.manifest.project.name,
                        &devnet.manifest.project.authors,
                    ),
                ));
            }
            let _ = integrate::run_devnet(devnet, deployment, None, !cmd.no_dashboard);
            if hints_enabled {
                display_deploy_hint();
            }
        }
        Command::LSP => run_lsp(),
        Command::DAP => match super::dap::run_dap() {
            Ok(_) => (),
            Err(e) => {
                println!("{}: {}", red!("error"), e);
                process::exit(1);
            }
        },
        Command::Completions(cmd) => {
            let mut app = Opts::command();
            let file_name = cmd.shell.file_name("clarinet");
            let mut file = match File::create(file_name.clone()) {
                Ok(file) => file,
                Err(e) => {
                    println!(
                        "{}: Unable to create file {}: {}",
                        red!("error"),
                        file_name,
                        e
                    );
                    std::process::exit(1);
                }
            };
            cmd.shell.generate(&mut app, &mut file);
            println!("{} {}", green!("Created file"), file_name.clone());
            println!("Check your shell's documentation for details about using this file to enable completions for clarinet");
        }
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
    match get_manifest_path(path) {
        Some(manifest_path) => manifest_path,
        None => {
            println!("Could not find Clarinet.toml");
            process::exit(1);
        }
    }
}

fn load_manifest_or_exit(path: Option<String>) -> ProjectManifest {
    let manifest_path = get_manifest_path_or_exit(path);
    let manifest = match ProjectManifest::from_path(&manifest_path) {
        Ok(manifest) => manifest,
        Err(message) => {
            println!(
                "{}: Syntax errors in Clarinet.toml\n{}",
                red!("error"),
                message,
            );
            process::exit(1);
        }
    };
    manifest
}

fn load_deployments_and_artifacts_or_exit(
    manifest: &ProjectManifest,
    deployment_plan_path: &Option<String>,
) -> (
    DeploymentSpecification,
    Option<String>,
    Option<DeploymentGenerationArtifacts>,
) {
    let (res, deployment_path, artifacts) = match deployment_plan_path {
        None => {
            let res = load_deployment_if_exists(&manifest, &StacksNetwork::Simnet);
            match res {
                Some(Ok(deployment)) => {
                    println!(
                        "{}: using deployments/default.simnet-plan.yaml",
                        yellow!("note")
                    );
                    (Ok(deployment), None, None)
                }
                Some(Err(e)) => {
                    println!(
                        "{}: loading deployments/default.simnet-plan.yaml failed with error: {}",
                        red!("error"),
                        e
                    );
                    std::process::exit(1);
                }
                None => match generate_default_deployment(&manifest, &StacksNetwork::Simnet) {
                    Ok((deployment, artifacts)) => (Ok(deployment), None, Some(artifacts)),
                    Err(e) => (Err(e), None, None),
                },
            }
        }
        Some(path) => {
            let deployment_path = get_absolute_deployment_path(&manifest, &path);
            let deployment = load_deployment(&manifest, &deployment_path);
            (
                deployment,
                Some(format!("{}", deployment_path.display())),
                None,
            )
        }
    };

    let deployment = match res {
        Ok(deployment) => deployment,
        Err(e) => {
            println!("{}: {}", red!("error"), e);
            process::exit(1);
        }
    };

    (deployment, deployment_path, artifacts)
}

pub fn build_deployment_cache_or_exit(
    manifest: &ProjectManifest,
    deployment_plan_path: &Option<String>,
) -> DeploymentCache {
    let (deployment, deployment_path, artifacts) =
        load_deployments_and_artifacts_or_exit(manifest, deployment_plan_path);

    let contracts_asts = artifacts.and_then(|a| Some(a.asts));

    let cache = DeploymentCache::new(&manifest, deployment, &deployment_path, contracts_asts);

    cache
}

fn execute_changes(changes: Vec<Changes>) -> bool {
    let mut shared_config = None;
    let mut path = PathBuf::new();

    for mut change in changes.into_iter() {
        match change {
            Changes::AddFile(options) => {
                if let Ok(entry) = fs::metadata(&options.path) {
                    if entry.is_file() {
                        println!(
                            "{}: file already exists at path {}",
                            yellow!("warning"),
                            options.path
                        );
                        continue;
                    }
                }
                let mut file = match File::create(options.path.clone()) {
                    Ok(file) => file,
                    Err(e) => {
                        println!(
                            "{}: Unable to create file {}: {}",
                            red!("error"),
                            options.path,
                            e
                        );
                        return false;
                    }
                };
                match file.write_all(options.content.as_bytes()) {
                    Ok(_) => (),
                    Err(e) => {
                        println!(
                            "{}: Unable to write file {}: {}",
                            red!("error"),
                            options.path,
                            e
                        );
                        return false;
                    }
                };
                println!("{}", options.comment);
            }
            Changes::AddDirectory(options) => {
                match fs::create_dir_all(options.path.clone()) {
                    Ok(_) => (),
                    Err(e) => {
                        println!(
                            "{}: Unable to create directory {}: {}",
                            red!("error"),
                            options.path,
                            e
                        );
                        return false;
                    }
                };
                println!("{}", options.comment);
            }
            Changes::EditTOML(ref mut options) => {
                let mut config = match shared_config.take() {
                    Some(config) => config,
                    None => {
                        path = options.manifest_path.clone();
                        let file = match File::open(path.clone()) {
                            Ok(file) => file,
                            Err(e) => {
                                println!(
                                    "{}: Unable to open file {}: {}",
                                    red!("error"),
                                    path.to_string_lossy(),
                                    e
                                );
                                return false;
                            }
                        };
                        let mut project_manifest_file_reader = BufReader::new(file);
                        let mut project_manifest_file = vec![];
                        match project_manifest_file_reader.read_to_end(&mut project_manifest_file) {
                            Ok(_) => (),
                            Err(e) => {
                                println!("{}: Unable to read manifest file: {}", red!("error"), e);
                                return false;
                            }
                        };
                        let project_manifest_file: ProjectManifestFile =
                            match toml::from_slice(&project_manifest_file[..]) {
                                Ok(manifest) => manifest,
                                Err(e) => {
                                    println!(
                                        "{}: Failed to process manifest file: {}",
                                        red!("error"),
                                        e
                                    );
                                    return false;
                                }
                            };
                        ProjectManifest::from_project_manifest_file(project_manifest_file, &path)
                            .unwrap()
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
        let toml_value = match toml::Value::try_from(&config) {
            Ok(value) => value,
            Err(e) => {
                println!("{}: Failed to encode config file: {}", red!("error"), e);
                return false;
            }
        };
        let toml = format!("{}", toml_value);
        let mut file = match File::create(path.clone()) {
            Ok(file) => file,
            Err(e) => {
                println!(
                    "{}: Unable to create manifest file {}: {}",
                    red!("error"),
                    path.to_string_lossy(),
                    e
                );
                return false;
            }
        };
        match file.write_all(&toml.as_bytes()) {
            Ok(_) => (),
            Err(e) => {
                println!("{}: Unable to write to manifest file: {}", red!("error"), e);
                return false;
            }
        };
        match file.sync_all() {
            Ok(_) => (),
            Err(e) => {
                println!("{}: sync_all: {}", red!("error"), e);
                return false;
            }
        };
    }

    true
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

fn display_post_console_hint() {
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

    println!("{}", blue!("  $ clarinet contract publish --testnet"));
    println!(
        "{}",
        yellow!("    Deploy all contracts to the testnet network.\n")
    );

    println!("{}", blue!("  $ clarinet contract publish --mainnet"));
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

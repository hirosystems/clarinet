use std::collections::{BTreeSet, HashMap};
use std::fs::{self, File};
use std::io::{prelude::*, BufReader, Read};
use std::path::PathBuf;
use std::{env, process};

use crate::dap::run_dap;
use crate::generate::{
    self,
    changes::{Changes, TOMLEdition},
};
use crate::integrate::{self, DevnetOrchestrator};
use crate::lsp::run_lsp;
use crate::poke::load_session;
use crate::publish::publish_all_contracts;
use crate::runnner::run_scripts;
use crate::types::{Network, ProjectManifest, ProjectManifestFile, RequirementConfig};
use clarity_repl::clarity::analysis::{AnalysisDatabase, ContractAnalysis};
use clarity_repl::clarity::costs::LimitedCostTracker;
use clarity_repl::clarity::types::QualifiedContractIdentifier;
use clarity_repl::{analysis, repl};

use clap::{IntoApp, Parser, Subcommand};
use clap_generate::{Generator, Shell};
use toml;

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
    /// Add third-party requirements to this project
    #[clap(name = "requirement", bin_name = "requirement")]
    Requirement(Requirement),
    /// Replicate a third-party contract into this project
    #[clap(name = "fork", bin_name = "fork")]
    ForkContract(ForkContract),
    /// Publish contracts on chain
    #[clap(name = "publish", bin_name = "publish")]
    Publish(Publish),
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
    #[clap(long = "manifest-path")]
    pub manifest_path: Option<String>,
}

#[derive(Parser, PartialEq, Clone, Debug)]
struct Requirement {
    /// Contract id (ex. " SP2PABAF9FTAJYNFZH93XENAJ8FVY99RRM50D2JG9.nft-trait")
    pub contract_id: String,
    /// Path to Clarinet.toml
    #[clap(long = "manifest-path")]
    pub manifest_path: Option<String>,
}

#[derive(Parser, PartialEq, Clone, Debug)]
struct ForkContract {
    /// Contract id (ex. " SP2PABAF9FTAJYNFZH93XENAJ8FVY99RRM50D2JG9.nft-trait")
    pub contract_id: String,
    /// Path to Clarinet.toml
    #[clap(long = "manifest-path")]
    pub manifest_path: Option<String>,
    // /// Fork contract and all its dependencies
    // #[clap(short = 'r')]
    // pub recursive: bool,
}

#[derive(Parser, PartialEq, Clone, Debug)]
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

#[derive(Parser, PartialEq, Clone, Debug)]
struct Console {
    /// Path to Clarinet.toml
    #[clap(long = "manifest-path")]
    pub manifest_path: Option<String>,
}

#[derive(Parser, PartialEq, Clone, Debug)]
struct Integrate {
    /// Path to Clarinet.toml
    #[clap(long = "manifest-path")]
    pub manifest_path: Option<String>,
    /// Display streams of logs instead of terminal UI dashboard
    #[clap(long = "no-dashboard")]
    pub no_dashboard: bool,
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
    #[clap(long = "manifest-path")]
    pub manifest_path: Option<String>,
    /// Relaunch tests upon updates to contracts
    #[clap(long = "watch")]
    pub watch: bool,
    /// Test files to be included (defaults to all tests found under tests/)
    pub files: Vec<String>,
}

#[derive(Parser, PartialEq, Clone, Debug)]
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

#[derive(Parser, PartialEq, Clone, Debug)]
struct Check {
    /// Path to Clarinet.toml
    #[clap(long = "manifest-path")]
    pub manifest_path: Option<String>,
    /// If specified, check this file
    pub file: Option<String>,
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
                                format!("{}", e),
                            ));
                        }
                    }
                    None => {}
                };
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
        Command::Contracts(subcommand) => match subcommand {
            Contracts::NewContract(new_contract) => {
                let manifest_path = get_manifest_path_or_exit(new_contract.manifest_path);

                let changes = generate::get_changes_for_new_contract(
                    manifest_path,
                    new_contract.name,
                    None,
                    true,
                    vec![],
                );
                if !execute_changes(changes) {
                    std::process::exit(1);
                }
                if hints_enabled {
                    display_post_check_hint();
                }
            }
            Contracts::Requirement(required_contract) => {
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
                if !execute_changes(vec![Changes::EditTOML(change)]) {
                    std::process::exit(1);
                }
                if hints_enabled {
                    display_post_check_hint();
                }
            }
            Contracts::ForkContract(fork_contract) => {
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
                if !execute_changes(changes) {
                    std::process::exit(1);
                }
                if hints_enabled {
                    display_post_check_hint();
                }
            }
            Contracts::Publish(deploy) => {
                let manifest_path = get_manifest_path_or_exit(deploy.manifest_path);

                let network = if deploy.devnet == true {
                    Network::Devnet
                } else if deploy.testnet == true {
                    Network::Testnet
                } else if deploy.mainnet == true {
                    Network::Mainnet
                } else {
                    panic!(
                        "Target deployment must be specified with --devnet, --testnet or --mainnet"
                    )
                };
                let project_manifest =
                    match publish_all_contracts(&manifest_path, &network, true, 30, None, None) {
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
        },
        Command::Console(cmd) => {
            let manifest_path = get_manifest_path_or_exit(cmd.manifest_path);
            let start_repl = true;
            let (session, project_manifest) =
                match load_session(&manifest_path, start_repl, &Network::Devnet) {
                    Ok((session, _, project_manifest, _)) => (Some(session), project_manifest),
                    Err((project_manifest, e)) => {
                        println!("{}: Unable to start REPL: {}", red!("error"), e);
                        (None, project_manifest)
                    }
                };
            if hints_enabled {
                display_post_console_hint();
            }
            if project_manifest.project.telemetry {
                #[cfg(feature = "telemetry")]
                telemetry_report_event(DeveloperUsageEvent::PokeExecuted(
                    DeveloperUsageDigest::new(
                        &project_manifest.project.name,
                        &project_manifest.project.authors,
                    ),
                ));

                #[cfg(feature = "telemetry")]
                if let Some(session) = session {
                    let mut debug_count = 0;
                    for command in session.executed {
                        if command.starts_with("::debug") {
                            debug_count += 1;
                        }
                    }
                    if debug_count > 0 {
                        telemetry_report_event(DeveloperUsageEvent::DebugStarted(
                            DeveloperUsageDigest::new(
                                &project_manifest.project.name,
                                &project_manifest.project.authors,
                            ),
                            debug_count,
                        ));
                    }
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
            let manifest_path = get_manifest_path_or_exit(cmd.manifest_path);
            let start_repl = false;
            let project_manifest = match load_session(&manifest_path, start_repl, &Network::Devnet)
            {
                Err((_, e)) => {
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
            let res = load_session(&manifest_path, start_repl, &Network::Devnet);
            let (session, project_manifest) = match res {
                Ok((session, _, manifest, output)) => {
                    if let Some(message) = output {
                        println!("{}", message);
                    }
                    (Some(session), manifest)
                }
                Err((manifest, e)) => {
                    println!("{}", e);
                    (None, manifest)
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
                session,
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
            let res = load_session(&manifest_path, start_repl, &Network::Devnet);
            let session = match res {
                Ok((session, _, _, output)) => {
                    if let Some(message) = output {
                        println!("{}", message);
                    }
                    session
                }
                Err((_, e)) => {
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
        Command::DAP => match run_dap() {
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
    println!("");
    match get_manifest_path(path) {
        Some(manifest_path) => manifest_path,
        None => {
            println!("Could not find Clarinet.toml");
            process::exit(1);
        }
    }
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

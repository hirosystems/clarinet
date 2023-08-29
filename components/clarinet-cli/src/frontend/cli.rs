use crate::deployments::types::DeploymentSynthesis;
use crate::deployments::{
    self, check_deployments, generate_default_deployment, get_absolute_deployment_path,
    write_deployment,
};
use crate::devnet::package as Package;
use crate::generate::{
    self,
    changes::{Changes, TOMLEdition},
};
use crate::integrate;
use crate::lsp::run_lsp;
use crate::runner::{run_scripts, DeploymentCache};

use clarinet_deployments::onchain::{
    apply_on_chain_deployment, get_initial_transactions_trackers, update_deployment_costs,
    DeploymentCommand, DeploymentEvent,
};
use clarinet_deployments::types::{DeploymentGenerationArtifacts, DeploymentSpecification};
use clarinet_deployments::{
    get_default_deployment_path, load_deployment, setup_session_with_deployment,
};
use clarinet_files::chainhook_types::Chain;
use clarinet_files::chainhook_types::StacksNetwork;
use clarinet_files::{
    get_manifest_location, FileLocation, NetworkManifest, ProjectManifest, ProjectManifestFile,
    RequirementConfig,
};
use clarity_repl::analysis::call_checker::ContractAnalysis;
use clarity_repl::analysis::coverage::parse_coverage_str;
use clarity_repl::clarity::vm::analysis::AnalysisDatabase;
use clarity_repl::clarity::vm::costs::LimitedCostTracker;
use clarity_repl::clarity::vm::diagnostic::{Diagnostic, Level};
use clarity_repl::clarity::vm::types::QualifiedContractIdentifier;
use clarity_repl::clarity::ClarityVersion;
use clarity_repl::repl::diagnostic::{output_code, output_diagnostic};
use clarity_repl::repl::{ClarityCodeSource, ClarityContract, ContractDeployer, DEFAULT_EPOCH};
use clarity_repl::{analysis, repl, Terminal};
use stacks_network::chainhook_sdk::chainhooks::types::ChainhookFullSpecification;
use stacks_network::{
    self, check_chainhooks, load_chainhooks, parse_chainhook_full_specification, DevnetOrchestrator,
};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::prelude::*;
use std::path::PathBuf;
use std::{env, process};

use clap::builder::ValueParser;
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
/// Clarinet is a command line tool for Clarity smart contract development.
///
/// For Clarinet documentation, refer to https://docs.hiro.so/clarinet/introduction.
/// Report any issues here https://github.com/hirosystems/clarinet/issues/new.
#[derive(Parser, PartialEq, Clone, Debug)]
#[clap(version = option_env!("CARGO_PKG_VERSION").expect("Unable to detect version"), name = "clarinet", bin_name = "clarinet")]
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
    /// Interact with contracts deployed on Mainnet
    #[clap(subcommand, name = "requirements")]
    Requirements(Requirements),
    /// Subcommands for working with chainhooks
    #[clap(subcommand, name = "chainhooks")]
    Chainhooks(Chainhooks),
    /// Manage contracts deployments on Simnet/Devnet/Testnet/Mainnet
    #[clap(subcommand, name = "deployments")]
    Deployments(Deployments),
    /// Load contracts in a REPL for an interactive session
    #[clap(name = "console", aliases = &["poke"], bin_name = "console")]
    Console(Console),
    /// Execute test suite
    #[clap(name = "test", bin_name = "test")]
    Test(Test),
    /// Check contracts syntax
    #[clap(name = "check", bin_name = "check")]
    Check(Check),
    /// Execute Clarinet extension
    #[clap(name = "run", bin_name = "run")]
    Run(Run),
    /// Start a local Devnet network for interacting with your contracts from your browser
    #[clap(name = "integrate", bin_name = "integrate")]
    Integrate(Integrate),
    /// Get Clarity autocompletion and inline errors from your code editor (VSCode, vim, emacs, etc)
    #[clap(name = "lsp", bin_name = "lsp")]
    LSP,
    /// Step by step debugging and breakpoints from your code editor (VSCode, vim, emacs, etc)
    #[clap(name = "dap", bin_name = "dap")]
    DAP,
    /// Generate shell completions scripts
    #[clap(name = "completions", bin_name = "completions")]
    Completions(Completions),
    /// Subcommands for Devnet usage
    #[clap(subcommand, name = "devnet")]
    Devnet(Devnet),
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
    /// Interact with contracts published on Mainnet
    #[clap(name = "add", bin_name = "add")]
    AddRequirement(AddRequirement),
}

#[derive(Subcommand, PartialEq, Clone, Debug)]
#[clap(bin_name = "deployment", aliases = &["deployment"])]
enum Deployments {
    /// Check deployments format
    #[clap(name = "check", bin_name = "check")]
    CheckDeployments(CheckDeployments),
    /// Generate new deployment
    #[clap(name = "generate", bin_name = "generate", aliases = &["new"])]
    GenerateDeployment(GenerateDeployment),
    /// Apply deployment
    #[clap(name = "apply", bin_name = "apply")]
    ApplyDeployment(ApplyDeployment),
}

#[derive(Subcommand, PartialEq, Clone, Debug)]
#[clap(bin_name = "chainhook", aliases = &["chainhook"])]
enum Chainhooks {
    /// Generate files and settings for a new hook
    #[clap(name = "new", bin_name = "new")]
    NewChainhook(NewChainhook),
    /// Check hooks format
    #[clap(name = "check", bin_name = "check")]
    CheckChainhooks(CheckChainhooks),
    /// Publish contracts on chain
    #[clap(name = "deploy", bin_name = "deploy")]
    DeployChainhook(DeployChainhook),
}

#[derive(Subcommand, PartialEq, Clone, Debug)]
#[clap(bin_name = "devnet")]
enum Devnet {
    /// Generate package of all required devnet artifacts
    #[clap(name = "package", bin_name = "package")]
    Package(DevnetPackage),
}

#[derive(Parser, PartialEq, Clone, Debug)]
struct DevnetPackage {
    /// Output json file name
    #[clap(long = "name", short = 'n')]
    pub package_file_name: Option<String>,
    #[clap(long = "manifest-path", short = 'm')]
    pub manifest_path: Option<String>,
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
        conflicts_with = "simnet",
        conflicts_with = "testnet",
        conflicts_with = "mainnet"
    )]
    pub devnet: bool,
    /// Generate a deployment file for devnet, using settings/Testnet.toml
    #[clap(
        long = "testnet",
        conflicts_with = "simnet",
        conflicts_with = "devnet",
        conflicts_with = "mainnet"
    )]
    pub testnet: bool,
    /// Generate a deployment file for devnet, using settings/Mainnet.toml
    #[clap(
        long = "mainnet",
        conflicts_with = "simnet",
        conflicts_with = "testnet",
        conflicts_with = "devnet"
    )]
    pub mainnet: bool,
    /// Path to Clarinet.toml
    #[clap(long = "manifest-path", short = 'm')]
    pub manifest_path: Option<String>,
    /// Generate a deployment file without trying to batch transactions (simnet only)
    #[clap(
        long = "no-batch",
        conflicts_with = "devnet",
        conflicts_with = "testnet",
        conflicts_with = "mainnet"
    )]
    pub no_batch: bool,
    /// Compute and set cost, using low priority (network connection required)
    #[clap(
        long = "low-cost",
        conflicts_with = "medium-cost",
        conflicts_with = "high-cost",
        conflicts_with = "manual-cost"
    )]
    pub low_cost: bool,
    /// Compute and set cost, using medium priority (network connection required)
    #[clap(
        conflicts_with = "low-cost",
        long = "medium-cost",
        conflicts_with = "high-cost",
        conflicts_with = "manual-cost"
    )]
    pub medium_cost: bool,
    /// Compute and set cost, using high priority (network connection required)
    #[clap(
        conflicts_with = "low-cost",
        conflicts_with = "medium-cost",
        long = "high-cost",
        conflicts_with = "manual-cost"
    )]
    pub high_cost: bool,
    /// Leave cost estimation manual
    #[clap(
        conflicts_with = "low-cost",
        conflicts_with = "medium-cost",
        conflicts_with = "high-cost",
        long = "manual-cost"
    )]
    pub manual_cost: bool,
}

#[derive(Parser, PartialEq, Clone, Debug)]
struct NewChainhook {
    /// Hook's name
    pub name: String,
    /// Path to Clarinet.toml
    #[clap(long = "manifest-path")]
    pub manifest_path: Option<String>,
    /// Generate a Bitcoin chainhook
    #[clap(long = "bitcoin", conflicts_with = "stacks")]
    pub bitcoin: bool,
    /// Generate a Stacks chainhook
    #[clap(long = "stacks", conflicts_with = "bitcoin")]
    pub stacks: bool,
}

#[derive(Parser, PartialEq, Clone, Debug)]
struct CheckChainhooks {
    /// Path to Clarinet.toml
    #[clap(long = "manifest-path")]
    pub manifest_path: Option<String>,
    /// Display chainhooks JSON representation
    #[clap(long = "output-json")]
    pub output_json: bool,
}

#[derive(Parser, PartialEq, Clone, Debug)]
struct DeployChainhook {
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
    /// Use on disk deployment plan (prevent updates computing)
    #[clap(
        long = "use-on-disk-deployment-plan",
        short = 'd',
        conflicts_with = "use-computed-deployment-plan"
    )]
    pub use_on_disk_deployment_plan: bool,
    /// Use computed deployment plan (will overwrite on disk version if any update)
    #[clap(
        long = "use-computed-deployment-plan",
        short = 'c',
        conflicts_with = "use-on-disk-deployment-plan"
    )]
    pub use_computed_deployment_plan: bool,
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
    /// Use on disk deployment plan (prevent updates computing)
    #[clap(
        long = "use-on-disk-deployment-plan",
        short = 'd',
        conflicts_with = "use-computed-deployment-plan"
    )]
    pub use_on_disk_deployment_plan: bool,
    /// Use computed deployment plan (will overwrite on disk version if any update)
    #[clap(
        long = "use-computed-deployment-plan",
        short = 'c',
        conflicts_with = "use-on-disk-deployment-plan"
    )]
    pub use_computed_deployment_plan: bool,
}

#[derive(Parser, PartialEq, Clone, Debug)]
struct Test {
    /// Generate coverage file, and optionally provide name of generated file (defaults to "coverage.lcov")
    #[clap(
        long = "coverage",
        default_missing_value("coverage.lcov"),
        value_parser(ValueParser::new(parse_coverage_str))
    )]
    pub coverage: Option<PathBuf>,
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
    /// Use on disk deployment plan (prevent updates computing)
    #[clap(
        long = "use-on-disk-deployment-plan",
        short = 'd',
        conflicts_with = "use-computed-deployment-plan"
    )]
    pub use_on_disk_deployment_plan: bool,
    /// Use computed deployment plan (will overwrite on disk version if any update)
    #[clap(
        long = "use-computed-deployment-plan",
        short = 'c',
        conflicts_with = "use-on-disk-deployment-plan"
    )]
    pub use_computed_deployment_plan: bool,
    /// Stop after N errors. Defaults to stopping after first failure
    #[clap(long = "fail-fast")]
    pub fail_fast: Option<u16>,
    /// Run tests with this string or pattern in the test name
    #[clap(long = "filter")]
    pub filter: Option<String>,
    /// Load import map file from local file or remote URL
    #[clap(long = "import-map")]
    pub import_map: Option<String>,
    /// Allow network access
    #[clap(long = "allow-net")]
    pub allow_net: bool,
    /// Allow read access to project directory
    #[clap(long = "allow-read")]
    pub allow_disk_read: bool,
    /// Specify optional Typescript config file
    #[clap(long = "ts-config")]
    pub ts_config: Option<String>,
    /// Specify relative path of the chainhooks (yaml format) to evaluate
    #[clap(long = "chainhooks")]
    pub chainhooks: Vec<String>,
    /// Add artificial delay (in seconds) when calling `chain.mineBlock(...)`. Useful when testing chainhooks
    #[clap(long = "mine-block-delay")]
    pub mine_block_delay: Option<u16>,
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
    /// Allow write access to project directory
    #[clap(long = "allow-write")]
    pub allow_disk_write: bool,
    /// Allow read access to project directory
    #[clap(long = "allow-read")]
    pub allow_disk_read: bool,
    /// Allows running a specified list of subprocesses. Use the flag multiple times to allow multiple subprocesses
    #[clap(long = "allow-run")]
    pub allow_run: Option<Vec<String>>,
    /// Allows access to a specified list of environment variables. Use the flag multiple times to allow access to multiple variables
    #[clap(long = "allow-env")]
    pub allow_env: Option<Vec<String>>,
    /// If specified, use this deployment file
    #[clap(long = "deployment-plan-path", short = 'p')]
    pub deployment_plan_path: Option<String>,
    /// Use on disk deployment plan (prevent updates computing)
    #[clap(
        long = "use-on-disk-deployment-plan",
        short = 'd',
        conflicts_with = "use-computed-deployment-plan"
    )]
    pub use_on_disk_deployment_plan: bool,
    /// Use computed deployment plan (will overwrite on disk version if any update)
    #[clap(
        long = "use-computed-deployment-plan",
        short = 'c',
        conflicts_with = "use-on-disk-deployment-plan"
    )]
    pub use_computed_deployment_plan: bool,
}

#[derive(Parser, PartialEq, Clone, Debug)]
struct Check {
    /// Path to Clarinet.toml
    #[clap(long = "manifest-path", short = 'm')]
    pub manifest_path: Option<String>,
    /// If specified, perform a simple syntax-check on just this one file
    pub file: Option<String>,
    /// If specified, use this deployment file
    #[clap(long = "deployment-plan-path", short = 'p')]
    pub deployment_plan_path: Option<String>,
    /// Use on disk deployment plan (prevent updates computing)
    #[clap(
        long = "use-on-disk-deployment-plan",
        short = 'd',
        conflicts_with = "use-computed-deployment-plan"
    )]
    pub use_on_disk_deployment_plan: bool,
    /// Use computed deployment plan (will overwrite on disk version if any update)
    #[clap(
        long = "use-computed-deployment-plan",
        short = 'c',
        conflicts_with = "use-on-disk-deployment-plan"
    )]
    pub use_computed_deployment_plan: bool,
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
                        println!("{}{}", format_err!("unable to get current directory"), e);
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
            let changes = match generate::get_changes_for_new_project(
                current_path,
                project_id,
                telemetry_enabled,
            ) {
                Ok(changes) => changes,
                Err(message) => {
                    println!("{}", format_err!(message));
                    std::process::exit(1);
                }
            };

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
                    println!("{}", format_err!(message));
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

                let default_deployment_path =
                    get_default_deployment_path(&manifest, &network).unwrap();
                let (mut deployment, _) =
                    match generate_default_deployment(&manifest, &network, cmd.no_batch) {
                        Ok(deployment) => deployment,
                        Err(message) => {
                            println!("{}", format_err!(message));
                            std::process::exit(1);
                        }
                    };

                if !cmd.manual_cost && network.either_testnet_or_mainnet() {
                    let priority = match (cmd.low_cost, cmd.medium_cost, cmd.high_cost) {
                        (_, _, true) => 2,
                        (_, true, _) => 1,
                        (true, _, _) => 0,
                        (false, false, false) => {
                            println!("{}", format_err!("cost strategy not specified (--low-cost, --medium-cost, --high-cost, --manual-cost)"));
                            std::process::exit(1);
                        }
                    };
                    match update_deployment_costs(&mut deployment, priority) {
                        Ok(_) => {}
                        Err(message) => {
                            println!(
                                "{} unable to update costs\n{}",
                                yellow!("warning:"),
                                message
                            );
                        }
                    };
                }

                let write_plan = if default_deployment_path.exists() {
                    let existing_deployment =
                        match load_deployment(&manifest, &default_deployment_path) {
                            Ok(deployment) => deployment,
                            Err(message) => {
                                println!(
                                    "{}",
                                    format_err!(format!(
                                        "unable to load {}\n{}",
                                        default_deployment_path.to_string(),
                                        message
                                    ))
                                );
                                process::exit(1);
                            }
                        };
                    should_existing_plan_be_replaced(&existing_deployment, &deployment)
                } else {
                    true
                };

                if write_plan {
                    let res = write_deployment(&deployment, &default_deployment_path, false);
                    if let Err(message) = res {
                        println!("{}", format_err!(message));
                        process::exit(1);
                    }

                    println!(
                        "{} {}",
                        green!("Generated file"),
                        default_deployment_path.get_relative_location().unwrap()
                    );
                }
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
                        let res = load_deployment_if_exists(&manifest, &network, true, false);
                        match res {
                            Some(Ok(deployment)) => {
                                println!(
                                    "{} using existing deployments/default.{}-plan.yaml",
                                    yellow!("note:"),
                                    format!("{:?}", network).to_lowercase(),
                                );
                                Ok(deployment)
                            }
                            Some(Err(e)) => Err(e),
                            None => {
                                let default_deployment_path = get_default_deployment_path(&manifest, &network).unwrap();
                                let (deployment, _) = match generate_default_deployment(&manifest, &network, false) {
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
                                    println!("{} {}", green!("Generated file"), default_deployment_path.get_relative_location().unwrap());
                                    Ok(deployment)
                                }
                            }
                        }
                    }
                    (None, Some(deployment_plan_path)) => {
                        let deployment_path = get_absolute_deployment_path(&manifest, &deployment_plan_path).expect("unable to retrieve deployment");
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
                let network = deployment.network.clone();

                let node_url = deployment.stacks_node.clone().unwrap();

                println!(
                    "The following deployment plan will be applied:\n{}\n\n{}",
                    DeploymentSynthesis::from_deployment(&deployment),
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
                        network.clone(),
                    ));
                }

                let transaction_trackers = if cmd.no_dashboard {
                    vec![]
                } else {
                    get_initial_transactions_trackers(&deployment)
                };
                let network_moved = network.clone();
                std::thread::spawn(move || {
                    let manifest = manifest_moved;
                    let network_manifest = NetworkManifest::from_project_manifest_location(
                        &manifest.location,
                        &network_moved.get_networks(),
                        Some(&manifest.project.cache_location),
                        None,
                    )
                    .expect("unable to load network manifest");
                    apply_on_chain_deployment(
                        network_manifest,
                        deployment,
                        event_tx,
                        command_rx,
                        true,
                        None,
                        None,
                    );
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
                                println!(
                                    "{} Error publishing transactions: {}",
                                    red!("x"),
                                    message
                                );
                                break;
                            }
                            DeploymentEvent::TransactionUpdate(update) => {
                                println!("{} {:?} {}", blue!("➡"), update.status, update.name);
                            }
                            DeploymentEvent::DeploymentCompleted => {
                                println!(
                                    "{} Transactions successfully confirmed on {:?}",
                                    green!("✔"),
                                    network
                                );
                                break;
                            }
                        }
                    }
                } else {
                    let res = deployments::start_ui(&node_url, event_rx, transaction_trackers);
                    match res {
                        Ok(()) => println!(
                            "{} Transactions successfully confirmed on {:?}",
                            green!("✔"),
                            network
                        ),
                        Err(message) => {
                            println!("{} Error publishing transactions: {}", red!("x"), message)
                        }
                    }
                }
            }
        },
        Command::Chainhooks(subcommand) => match subcommand {
            Chainhooks::NewChainhook(cmd) => {
                let manifest = load_manifest_or_exit(cmd.manifest_path);

                let chain = match (cmd.bitcoin, cmd.stacks) {
                    (true, false) => Chain::Bitcoin,
                    (false, true) => Chain::Stacks,
                    (_, _) => {
                        println!(
                            "{}",
                            format_err!("either --bitcoin or --stacks must be passed")
                        );
                        process::exit(1);
                    }
                };

                let changes =
                    match generate::get_changes_for_new_chainhook(&manifest, cmd.name, chain) {
                        Ok(changes) => changes,
                        Err(message) => {
                            println!("{}", format_err!(message));
                            std::process::exit(1);
                        }
                    };

                if !execute_changes(changes) {
                    std::process::exit(1);
                }
                if hints_enabled {
                    display_post_check_hint();
                }
            }
            Chainhooks::CheckChainhooks(cmd) => {
                let manifest_location = get_manifest_location_or_exit(cmd.manifest_path);
                // Ensure that all the hooks can correctly be deserialized.
                println!("Checking chainhooks");
                let _ = check_chainhooks(&manifest_location, cmd.output_json);
            }
            Chainhooks::DeployChainhook(_cmd) => {
                // TODO(lgalabru): follow-up on this implementation
                unimplemented!()
            }
        },
        Command::Contracts(subcommand) => match subcommand {
            Contracts::NewContract(cmd) => {
                let manifest = load_manifest_or_exit(cmd.manifest_path);

                let changes = match generate::get_changes_for_new_contract(
                    &manifest.location,
                    cmd.name,
                    None,
                    true,
                ) {
                    Ok(changes) => changes,
                    Err(message) => {
                        println!("{}", format_err!(message));
                        std::process::exit(1);
                    }
                };

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
                        "{} with requirement {}",
                        yellow!("Updated Clarinet.toml"),
                        green!(format!("{}", cmd.contract_id))
                    ),
                    manifest_location: manifest.location.clone(),
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
            // Loop to handle `::reload` command
            loop {
                let manifest = load_manifest_or_warn(cmd.manifest_path.clone());

                let mut terminal = match manifest {
                    Some(ref manifest) => {
                        let (deployment, _, artifacts) = load_deployment_and_artifacts_or_exit(
                            manifest,
                            &cmd.deployment_plan_path,
                            cmd.use_on_disk_deployment_plan,
                            cmd.use_computed_deployment_plan,
                        );

                        if !artifacts.success {
                            let diags_digest =
                                DiagnosticsDigest::new(&artifacts.diags, &deployment);
                            if diags_digest.has_feedbacks() {
                                println!("{}", diags_digest.message);
                            }
                            if diags_digest.errors > 0 {
                                println!(
                                    "{} {} detected",
                                    red!("x"),
                                    pluralize!(diags_digest.errors, "error")
                                );
                            }
                            std::process::exit(1);
                        }

                        Terminal::load(artifacts.session)
                    }
                    None => Terminal::new(repl::SessionSettings::default()),
                };
                let reload = terminal.start();

                // Report telemetry
                if let Some(manifest) = manifest {
                    if manifest.project.telemetry {
                        #[cfg(feature = "telemetry")]
                        telemetry_report_event(DeveloperUsageEvent::PokeExecuted(
                            DeveloperUsageDigest::new(
                                &manifest.project.name,
                                &manifest.project.authors,
                            ),
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

                if !reload {
                    break;
                }
            }

            if hints_enabled {
                display_post_console_hint();
            }
        }
        Command::Check(cmd) if cmd.file.is_some() => {
            let file = cmd.file.unwrap();
            let mut settings = repl::SessionSettings::default();
            settings.repl_settings.analysis.enable_all_passes();

            let mut session = repl::Session::new(settings.clone());
            let code_source = match fs::read_to_string(&file) {
                Ok(code) => code,
                _ => {
                    println!("{} unable to read file: '{}'", red!("error:"), file);
                    std::process::exit(1);
                }
            };
            let contract_id = QualifiedContractIdentifier::transient();
            let contract = ClarityContract {
                code_source: ClarityCodeSource::ContractInMemory(code_source),
                deployer: ContractDeployer::Transient,
                name: "transient".to_string(),
                clarity_version: ClarityVersion::Clarity1,
                epoch: DEFAULT_EPOCH,
            };
            let (ast, mut diagnostics, mut success) = session.interpreter.build_ast(&contract);
            let (annotations, mut annotation_diagnostics) = session
                .interpreter
                .collect_annotations(&ast, contract.expect_in_memory_code_source());
            diagnostics.append(&mut annotation_diagnostics);

            let mut contract_analysis = ContractAnalysis::new(
                contract_id,
                ast.expressions,
                LimitedCostTracker::new_free(),
                contract.epoch,
                contract.clarity_version,
            );
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

            let lines = contract.expect_in_memory_code_source().lines();
            let formatted_lines: Vec<String> = lines.map(|l| l.to_string()).collect();
            for d in diagnostics {
                for line in output_diagnostic(&d, &file, &formatted_lines) {
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
            let (deployment, _, results) = load_deployment_and_artifacts_or_exit(
                &manifest,
                &cmd.deployment_plan_path,
                cmd.use_on_disk_deployment_plan,
                cmd.use_computed_deployment_plan,
            );

            let diags_digest = DiagnosticsDigest::new(&results.diags, &deployment);
            if diags_digest.has_feedbacks() {
                println!("{}", diags_digest.message);
            }

            if diags_digest.warnings > 0 {
                println!(
                    "{} {} detected",
                    yellow!("!"),
                    pluralize!(diags_digest.warnings, "warning")
                );
            }
            if diags_digest.errors > 0 {
                println!(
                    "{} {} detected",
                    red!("x"),
                    pluralize!(diags_digest.errors, "error")
                );
            } else {
                println!(
                    "{} {} checked",
                    green!("✔"),
                    pluralize!(diags_digest.contracts_checked, "contract"),
                );
            }
            let exit_code = match results.success {
                true => 0,
                false => 1,
            };

            if hints_enabled {
                display_post_check_hint();
            }
            if manifest.project.telemetry {
                #[cfg(feature = "telemetry")]
                telemetry_report_event(DeveloperUsageEvent::CheckExecuted(
                    DeveloperUsageDigest::new(&manifest.project.name, &manifest.project.authors),
                ));
            }
            std::process::exit(exit_code);
        }
        Command::Test(cmd) => {
            let manifest = load_manifest_or_exit(cmd.manifest_path);
            let deployment_plan_path = cmd.deployment_plan_path.clone();
            let cache = build_deployment_cache_or_exit(&manifest, &deployment_plan_path);
            let cache_location = manifest.project.cache_location.clone();
            let mut stacks_chainhooks = vec![];
            let mine_block_delay = cmd.mine_block_delay.unwrap_or(0);

            if cmd.chainhooks.contains(&"*".to_string()) {
                #[allow(unused_imports)]
                use stacks_network::chainhook_sdk::chainhook_types::{
                    BitcoinNetwork, StacksNetwork,
                };
                match load_chainhooks(
                    &manifest.location,
                    &(BitcoinNetwork::Regtest, StacksNetwork::Devnet),
                ) {
                    Ok(ref mut formation) => {
                        stacks_chainhooks.append(&mut formation.stacks_chainhooks);
                    }
                    Err(e) => {
                        println!("{} unable to load chainhooks - {}", red!("error:"), e);
                    }
                };
            } else {
                for chainhook_relative_path in cmd.chainhooks.iter() {
                    let mut chainhook_location = manifest
                        .location
                        .get_project_root_location()
                        .expect("unable to get root location");
                    chainhook_location
                        .append_path(chainhook_relative_path)
                        .expect("unable to build path");
                    match parse_chainhook_full_specification(&chainhook_location.to_string().into())
                    {
                        Ok(hook) => match hook {
                            ChainhookFullSpecification::Bitcoin(_) => {
                                println!(
                                    "{}",
                                    format_err!(
                                        "bitcoin chainhooks not supported in test environments"
                                    )
                                );
                                std::process::exit(1);
                            }
                            ChainhookFullSpecification::Stacks(hook) => {
                                let spec = match hook
                                    .into_selected_network_specification(&stacks_network::chainhook_sdk::chainhook_types::StacksNetwork::Devnet)
                                {
                                    Ok(spec) => spec,
                                    Err(e) => {
                                        println!(
                                            "{} unable to load chainhooks ({})",
                                            red!("error:"),
                                            e
                                        );
                                        std::process::exit(1);
                                    }
                                };
                                stacks_chainhooks.push(spec)
                            }
                        },
                        Err(msg) => {
                            println!("{} unable to load chainhooks ({})", red!("error:"), msg);
                            std::process::exit(1);
                        }
                    };
                }
            }

            let (success, _count) = match run_scripts(
                cmd.files,
                cmd.coverage,
                cmd.costs_report,
                cmd.watch,
                true,
                cmd.allow_disk_read,
                false,
                None,
                None,
                &manifest,
                cache,
                deployment_plan_path,
                cmd.fail_fast,
                cmd.filter,
                cmd.import_map,
                cmd.allow_net,
                cache_location,
                cmd.ts_config,
                stacks_chainhooks,
                mine_block_delay,
            ) {
                Ok(count) => (true, count),
                Err((e, count)) => {
                    println!("{}", format_err!(e.to_string()));
                    (false, count)
                }
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
            let cache_location = manifest.project.cache_location.clone();
            let _ = run_scripts(
                vec![cmd.script],
                None,
                false,
                false,
                cmd.allow_wallets,
                cmd.allow_disk_read,
                cmd.allow_disk_write,
                cmd.allow_run,
                cmd.allow_env,
                &manifest,
                cache,
                cmd.deployment_plan_path,
                None,
                None,
                None,
                false,
                cache_location,
                None,
                vec![],
                0,
            );
        }
        Command::Integrate(cmd) => {
            let manifest = load_manifest_or_exit(cmd.manifest_path);
            println!("Computing deployment plan");
            let result = match cmd.deployment_plan_path {
                None => {
                    let res = load_deployment_if_exists(
                        &manifest,
                        &StacksNetwork::Devnet,
                        cmd.use_on_disk_deployment_plan,
                        cmd.use_computed_deployment_plan,
                    );
                    match res {
                        Some(Ok(deployment)) => {
                            println!(
                                "{} using existing deployments/default.devnet-plan.yaml",
                                yellow!("note:")
                            );
                            // TODO(lgalabru): Think more about the desired DX.
                            // Compute the latest version, display differences and propose overwrite?
                            Ok(deployment)
                        }
                        Some(Err(e)) => Err(e),
                        None => {
                            let default_deployment_path =
                                get_default_deployment_path(&manifest, &StacksNetwork::Devnet)
                                    .unwrap();
                            let (deployment, _) = match generate_default_deployment(
                                &manifest,
                                &StacksNetwork::Devnet,
                                false,
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
                                println!(
                                    "{} {}",
                                    green!("Generated file"),
                                    default_deployment_path.get_relative_location().unwrap()
                                );
                                Ok(deployment)
                            }
                        }
                    }
                }
                Some(deployment_plan_path) => {
                    let deployment_path =
                        get_absolute_deployment_path(&manifest, &deployment_plan_path)
                            .expect("unable to retrieve deployment");
                    load_deployment(&manifest, &deployment_path)
                }
            };

            let deployment = match result {
                Ok(deployment) => deployment,
                Err(e) => {
                    println!("{}", format_err!(e));
                    std::process::exit(1);
                }
            };

            let orchestrator = match DevnetOrchestrator::new(manifest, None, None, true) {
                Ok(orchestrator) => orchestrator,
                Err(e) => {
                    println!("{}", format_err!(e));
                    process::exit(1);
                }
            };

            if orchestrator.manifest.project.telemetry {
                #[cfg(feature = "telemetry")]
                telemetry_report_event(DeveloperUsageEvent::DevnetExecuted(
                    DeveloperUsageDigest::new(
                        &orchestrator.manifest.project.name,
                        &orchestrator.manifest.project.authors,
                    ),
                ));
            }
            if let Err(e) = integrate::run_devnet(orchestrator, deployment, None, !cmd.no_dashboard)
            {
                println!("{}", format_err!(e));
                process::exit(1);
            }
            if hints_enabled {
                display_deploy_hint();
            }
        }
        Command::LSP => run_lsp(),
        Command::DAP => match super::dap::run_dap() {
            Ok(_) => (),
            Err(e) => {
                println!("{}", red!(e));
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
                        "{} Unable to create file {}: {}",
                        red!("error:"),
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

        Command::Devnet(subcommand) => match subcommand {
            Devnet::Package(cmd) => {
                let manifest = load_manifest_or_exit(cmd.manifest_path);
                if let Err(e) = Package::pack(cmd.package_file_name, manifest) {
                    println!("Could not execute the package command. {}", format_err!(e));
                    process::exit(1);
                }
            }
        },
    };
}

fn get_manifest_location_or_exit(path: Option<String>) -> FileLocation {
    match get_manifest_location(path) {
        Some(manifest_location) => manifest_location,
        None => {
            println!("Could not find Clarinet.toml");
            process::exit(1);
        }
    }
}

fn get_manifest_location_or_warn(path: Option<String>) -> Option<FileLocation> {
    match get_manifest_location(path) {
        Some(manifest_location) => Some(manifest_location),
        None => {
            println!(
                "{} no manifest found, starting with default settings.",
                yellow!("note:")
            );
            None
        }
    }
}

fn load_manifest_or_exit(path: Option<String>) -> ProjectManifest {
    let manifest_location = get_manifest_location_or_exit(path);
    let manifest = match ProjectManifest::from_location(&manifest_location) {
        Ok(manifest) => manifest,
        Err(message) => {
            println!(
                "{} syntax errors in Clarinet.toml\n{}",
                red!("error:"),
                message,
            );
            process::exit(1);
        }
    };
    manifest
}

fn load_manifest_or_warn(path: Option<String>) -> Option<ProjectManifest> {
    let manifest_location = get_manifest_location_or_warn(path);
    if manifest_location.is_some() {
        let manifest = match ProjectManifest::from_location(&manifest_location.unwrap()) {
            Ok(manifest) => manifest,
            Err(message) => {
                println!(
                    "{} syntax errors in Clarinet.toml\n{}",
                    red!("error:"),
                    message,
                );
                process::exit(1);
            }
        };
        Some(manifest)
    } else {
        None
    }
}

fn load_deployment_and_artifacts_or_exit(
    manifest: &ProjectManifest,
    deployment_plan_path: &Option<String>,
    force_on_disk: bool,
    force_computed: bool,
) -> (
    DeploymentSpecification,
    Option<String>,
    DeploymentGenerationArtifacts,
) {
    let result = match deployment_plan_path {
        None => {
            let res = load_deployment_if_exists(
                &manifest,
                &StacksNetwork::Simnet,
                force_on_disk,
                force_computed,
            );
            match res {
                Some(Ok(deployment)) => {
                    println!(
                        "{} using deployments/default.simnet-plan.yaml",
                        yellow!("note:")
                    );
                    let artifacts = setup_session_with_deployment(&manifest, &deployment, None);
                    Ok((deployment, None, artifacts))
                }
                Some(Err(e)) => Err(format!(
                    "loading deployments/default.simnet-plan.yaml failed with error: {}",
                    e
                )),
                None => match generate_default_deployment(&manifest, &StacksNetwork::Simnet, false)
                {
                    Ok((deployment, ast_artifacts)) if ast_artifacts.success => {
                        let mut artifacts = setup_session_with_deployment(
                            &manifest,
                            &deployment,
                            Some(&ast_artifacts.asts),
                        );
                        for (contract_id, mut parser_diags) in ast_artifacts.diags.into_iter() {
                            // Merge parser's diags with analysis' diags.
                            if let Some(ref mut diags) = artifacts.diags.remove(&contract_id) {
                                parser_diags.append(diags);
                            }
                            artifacts.diags.insert(contract_id, parser_diags);
                        }
                        Ok((deployment, None, artifacts))
                    }
                    Ok((deployment, ast_artifacts)) => Ok((deployment, None, ast_artifacts)),
                    Err(e) => Err(e),
                },
            }
        }
        Some(path) => {
            let deployment_location = get_absolute_deployment_path(&manifest, &path)
                .expect("unable to retrieve deployment");
            match load_deployment(&manifest, &deployment_location) {
                Ok(deployment) => {
                    let artifacts = setup_session_with_deployment(&manifest, &deployment, None);
                    Ok((deployment, Some(deployment_location.to_string()), artifacts))
                }
                Err(e) => Err(format!("loading {} failed with error: {}", path, e)),
            }
        }
    };

    match result {
        Ok(deployment) => deployment,
        Err(e) => {
            println!("{}", format_err!(e));
            process::exit(1);
        }
    }
}

pub fn should_existing_plan_be_replaced(
    existing_plan: &DeploymentSpecification,
    new_plan: &DeploymentSpecification,
) -> bool {
    use similar::{ChangeTag, TextDiff};

    let existing_file = serde_yaml::to_string(&existing_plan.to_specification_file()).unwrap();

    let new_file = serde_yaml::to_string(&new_plan.to_specification_file()).unwrap();

    if existing_file == new_file {
        return false;
    }

    println!("{}", blue!("A new deployment plan was computed and differs from the default deployment plan currently saved on disk:"));

    let diffs = TextDiff::from_lines(&existing_file, &new_file);

    for change in diffs.iter_all_changes() {
        let formatted_change = match change.tag() {
            ChangeTag::Delete => {
                format!("{} {}", red!("-"), red!(format!("{}", change)))
            }
            ChangeTag::Insert => {
                format!("{} {}", green!("+"), green!(format!("{}", change)))
            }
            ChangeTag::Equal => format!("  {}", change),
        };
        print!("{}", formatted_change);
    }

    println!("{}", yellow!("Overwrite? [Y/n]"));
    let mut buffer = String::new();
    std::io::stdin().read_line(&mut buffer).unwrap();

    if buffer.starts_with("n") {
        return false;
    } else {
        return true;
    }
}

pub fn load_deployment_if_exists(
    manifest: &ProjectManifest,
    network: &StacksNetwork,
    force_on_disk: bool,
    force_computed: bool,
) -> Option<Result<DeploymentSpecification, String>> {
    let default_deployment_location = match get_default_deployment_path(manifest, network) {
        Ok(location) => location,
        Err(e) => return Some(Err(e)),
    };
    if !default_deployment_location.exists() {
        return None;
    }

    if !force_on_disk {
        match generate_default_deployment(manifest, network, true) {
            Ok((deployment, _)) => {
                use similar::{ChangeTag, TextDiff};

                let current_version = match default_deployment_location.read_content_as_utf8() {
                    Ok(content) => content,
                    Err(message) => return Some(Err(message)),
                };

                let file = deployment.to_specification_file();
                let updated_version = match serde_yaml::to_string(&file) {
                    Ok(res) => res,
                    Err(err) => {
                        return Some(Err(format!("failed serializing deployment\n{}", err)))
                    }
                };

                if updated_version == current_version {
                    return Some(load_deployment(manifest, &default_deployment_location));
                }

                if !force_computed {
                    println!("{}", blue!("A new deployment plan was computed and differs from the default deployment plan currently saved on disk:"));

                    let diffs = TextDiff::from_lines(&current_version, &updated_version);

                    for change in diffs.iter_all_changes() {
                        let formatted_change = match change.tag() {
                            ChangeTag::Delete => {
                                format!("{} {}", red!("-"), red!(format!("{}", change)))
                            }
                            ChangeTag::Insert => {
                                format!("{} {}", green!("+"), green!(format!("{}", change)))
                            }
                            ChangeTag::Equal => format!("  {}", change),
                        };
                        print!("{}", formatted_change);
                    }

                    println!("{}", yellow!("Overwrite? [Y/n]"));
                    let mut buffer = String::new();
                    std::io::stdin().read_line(&mut buffer).unwrap();
                    if buffer.starts_with("n") {
                        Some(load_deployment(manifest, &default_deployment_location))
                    } else {
                        default_deployment_location
                            .write_content(updated_version.as_bytes())
                            .ok()?;
                        Some(Ok(deployment))
                    }
                } else {
                    default_deployment_location
                        .write_content(updated_version.as_bytes())
                        .ok()?;
                    Some(Ok(deployment))
                }
            }
            Err(message) => {
                println!(
                    "{} unable to compute an updated plan\n{}",
                    red!("error:"),
                    message
                );
                Some(load_deployment(manifest, &default_deployment_location))
            }
        }
    } else {
        Some(load_deployment(manifest, &default_deployment_location))
    }
}

pub fn build_deployment_cache_or_exit(
    manifest: &ProjectManifest,
    deployment_plan_path: &Option<String>,
) -> DeploymentCache {
    let (deployment, deployment_path, artifacts) =
        load_deployment_and_artifacts_or_exit(manifest, deployment_plan_path, true, false);

    let cache = DeploymentCache::new(&manifest, deployment, &deployment_path, artifacts);

    cache
}

fn execute_changes(changes: Vec<Changes>) -> bool {
    let mut shared_config = None;

    for mut change in changes.into_iter() {
        match change {
            Changes::AddFile(options) => {
                if let Ok(entry) = fs::metadata(&options.path) {
                    if entry.is_file() {
                        println!(
                            "{} file already exists at path {}",
                            yellow!("warning:"),
                            options.path
                        );
                        continue;
                    }
                }
                let mut file = match File::create(options.path.clone()) {
                    Ok(file) => file,
                    Err(e) => {
                        println!(
                            "{} Unable to create file {}: {}",
                            red!("error:"),
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
                            "{} Unable to write file {}: {}",
                            red!("error:"),
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
                            "{} Unable to create directory {}: {}",
                            red!("error:"),
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
                        let manifest_location = options.manifest_location.clone();
                        let project_manifest_content = match manifest_location.read_content() {
                            Ok(content) => content,
                            Err(message) => {
                                println!("{}", format_err!(message));
                                return false;
                            }
                        };

                        let project_manifest_file: ProjectManifestFile =
                            match toml::from_slice(&project_manifest_content[..]) {
                                Ok(manifest) => manifest,
                                Err(message) => {
                                    println!(
                                        "{} Failed to process manifest file: {}",
                                        red!("error:"),
                                        message
                                    );
                                    return false;
                                }
                            };
                        match ProjectManifest::from_project_manifest_file(
                            project_manifest_file,
                            &manifest_location,
                        ) {
                            Ok(content) => content,
                            Err(message) => {
                                println!("{}", format_err!(message));
                                return false;
                            }
                        }
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

    if let Some(project_manifest) = shared_config {
        let toml_value = match toml::Value::try_from(&project_manifest) {
            Ok(value) => value,
            Err(e) => {
                println!("{} failed encoding config file ({})", red!("error:"), e);
                return false;
            }
        };

        let pretty_toml = match toml::ser::to_string_pretty(&toml_value) {
            Ok(value) => value,
            Err(e) => {
                println!("{} failed formatting config file ({})", red!("error:"), e);
                return false;
            }
        };

        if let Err(message) = project_manifest
            .location
            .write_content(pretty_toml.as_bytes())
        {
            println!(
                "{} Unable to update manifest file - {}",
                red!("error:"),
                message
            );
            return false;
        }
    }

    true
}

#[allow(dead_code)]
struct DiagnosticsDigest {
    message: String,
    errors: usize,
    warnings: usize,
    contracts_checked: usize,
    full_success: usize,
    total: usize,
}

impl DiagnosticsDigest {
    fn new(
        contracts_diags: &HashMap<QualifiedContractIdentifier, Vec<Diagnostic>>,
        deployment: &DeploymentSpecification,
    ) -> DiagnosticsDigest {
        let mut full_success = 0;
        let mut warnings = 0;
        let mut errors = 0;
        let mut contracts_checked = 0;
        let mut outputs = vec![];
        let total = deployment.contracts.len();

        for (contract_id, diags) in contracts_diags.into_iter() {
            let (source, contract_location) = match deployment.contracts.get(&contract_id) {
                Some(entry) => {
                    contracts_checked += 1;
                    entry
                }
                None => {
                    // `deployment.contracts` only includes contracts from the project, requirements should be ignored
                    continue;
                }
            };
            if diags.is_empty() {
                full_success += 1;
                continue;
            }

            let lines = source.lines();
            let formatted_lines: Vec<String> = lines.map(|l| l.to_string()).collect();

            for diagnostic in diags {
                match diagnostic.level {
                    Level::Error => {
                        errors += 1;
                        outputs.push(format_err!(diagnostic.message));
                    }
                    Level::Warning => {
                        warnings += 1;
                        outputs.push(format!("{} {}", yellow!("warning:"), diagnostic.message));
                    }
                    Level::Note => {
                        outputs.push(format!("{}: {}", green!("note:"), diagnostic.message));
                        outputs.append(&mut output_code(&diagnostic, &formatted_lines));
                        continue;
                    }
                }
                let contract_path = match contract_location.get_relative_location() {
                    Ok(contract_path) => contract_path,
                    _ => contract_location.to_string(),
                };

                if let Some(span) = diagnostic.spans.first() {
                    outputs.push(format!(
                        "{} {}:{}:{}",
                        blue!("-->"),
                        contract_path,
                        span.start_line,
                        span.start_column
                    ));
                }
                outputs.append(&mut output_code(&diagnostic, &formatted_lines));

                if let Some(ref suggestion) = diagnostic.suggestion {
                    outputs.push(format!("{}", suggestion));
                }
            }
        }

        DiagnosticsDigest {
            full_success,
            errors,
            warnings,
            total,
            contracts_checked,
            message: outputs.join("\n").to_string(),
        }
    }

    pub fn has_feedbacks(&self) -> bool {
        self.errors > 0 || self.warnings > 0
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
    println!("{}", yellow!("Find more information on testing with Clarinet here: https://docs.hiro.so/clarinet/how-to-guides/how-to-set-up-local-development-environment#testing-with-the-test-harness"));
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

    println!("{}", yellow!("Find more information on writing contracts with Clarinet here: https://docs.hiro.so/clarinet/how-to-guides/how-to-set-up-local-development-environment#developing-a-clarity-smart-contract"));
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

    println!("{}", yellow!("Find more information on testing with Clarinet here: https://docs.hiro.so/clarinet/how-to-guides/how-to-set-up-local-development-environment#testing-with-clarinet"));
    println!("{}", yellow!("And learn more about local integration testing here: https://docs.hiro.so/clarinet/how-to-guides/how-to-run-integration-environment"));
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
        yellow!("Find more information on the DevNet here: https://docs.hiro.so/clarinet/how-to-guides/how-to-run-integration-environment")
    );
    display_hint_footer();
}

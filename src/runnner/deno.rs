use clarity_repl::clarity::coverage::CoverageReporter;
use clarity_repl::clarity::types;
use clarity_repl::clarity::util::hash;
use clarity_repl::prettytable::{color, format, Attr, Cell, Row, Table};
use clarity_repl::repl::session::CostsReport;
use clarity_repl::repl::Session;
use deno::ast;
use deno::colors;
use deno::create_main_worker;
use deno::file_watcher::{self, ResolutionResult};
use deno::fs_util;
use deno::module_graph::{self, GraphBuilder, Module};
use deno::specifier_handler::FetchHandler;
use deno::tokio_util;
use deno::tools;
use deno::tools::coverage::CoverageCollector;
use deno::tools::test_runner::{self, create_reporter, TestEvent, TestMessage, TestResult};
use deno::tsc::{op, State};
use deno::File;
use deno::Flags;
use deno::MediaType;
use deno::ProgramState;
use deno_core::error::AnyError;
use deno_core::futures::future;
use deno_core::futures::stream;
use deno_core::futures::FutureExt;
use deno_core::futures::StreamExt;
use deno_core::op_sync;
use deno_core::serde_json::{self, json, Value};
use deno_core::url::Url;
use deno_core::ModuleSpecifier;
use deno_core::{OpFn, OpState};
use deno_runtime::permissions::Permissions;
use regex::Regex;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashSet;
use std::collections::{btree_map::Entry, BTreeMap};
use std::convert::TryFrom;
use std::fmt::Write;
use std::ops::Index;
use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::sync::Mutex;
use swc_common::comments::CommentKind;

mod sessions {
    use super::TransactionArgs;
    use crate::types::{ChainConfig, ProjectManifest};
    use clarity_repl::clarity::analysis::ContractAnalysis;
    use clarity_repl::repl::settings::Account;
    use clarity_repl::repl::{self, Session};
    use deno_core::error::AnyError;
    use std::collections::HashMap;
    use std::env;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Mutex;

    lazy_static! {
        pub static ref SESSIONS: Mutex<HashMap<u32, (String, Session)>> =
            Mutex::new(HashMap::new());
        pub static ref SESSION_TEMPLATE: Mutex<Vec<Session>> = Mutex::new(vec![]);
    }

    pub fn reset() {
        SESSION_TEMPLATE.lock().unwrap().clear();
        SESSIONS.lock().unwrap().clear();
    }

    pub fn handle_setup_chain(
        manifest_path: &PathBuf,
        name: String,
        transactions: Vec<TransactionArgs>,
    ) -> Result<(u32, Vec<Account>, Vec<(ContractAnalysis, String, String)>), AnyError> {
        let mut sessions = SESSIONS.lock().unwrap();
        let session_id = sessions.len() as u32;
        let session_templated = {
            let res = SESSION_TEMPLATE.lock().unwrap();
            !res.is_empty()
        };
        let use_cache = transactions.is_empty() && session_templated;

        let (mut session, contracts) = if !use_cache {
            let mut settings = repl::SessionSettings::default();
            let mut project_path = manifest_path.clone();
            project_path.pop();

            let mut chain_config_path = project_path.clone();
            chain_config_path.push("settings");
            chain_config_path.push("Devnet.toml");

            let project_config = ProjectManifest::from_path(manifest_path);
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
                settings.initial_accounts.push(account);
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
                let mut contract_path = project_path.clone();
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
            settings.costs_version = project_config.project.costs_version;
            settings.include_boot_contracts = vec![
                "pox".to_string(),
                format!("costs-v{}", project_config.project.costs_version),
                "bns".to_string(),
            ];
            let mut session = Session::new(settings.clone());
            let (_, contracts) = match session.start() {
                Ok(res) => res,
                Err(e) => {
                    std::process::exit(1);
                }
            };
            SESSION_TEMPLATE.lock().unwrap().push(session.clone());
            (session, contracts)
        } else {
            let session = SESSION_TEMPLATE.lock().unwrap().last().unwrap().clone();
            let contracts = session.initial_contracts_analysis.clone();
            (session, contracts)
        };

        session.advance_chain_tip(1);
        let accounts = session.settings.initial_accounts.clone();
        sessions.insert(session_id, (name, session));
        Ok((session_id, accounts, contracts))
    }

    pub fn perform_block<F, R>(session_id: u32, handler: F) -> Result<R, AnyError>
    where
        F: FnOnce(&str, &mut Session) -> Result<R, AnyError>,
    {
        let mut sessions = SESSIONS.lock().unwrap();
        match sessions.get_mut(&session_id) {
            None => {
                println!("Error: unable to retrieve session");
                panic!()
            }
            Some((name, ref mut session)) => handler(name.as_str(), session),
        }
    }
}

pub async fn do_run_scripts(
    include: Vec<String>,
    include_coverage: bool,
    include_costs_report: bool,
    watch: bool,
    allow_wallets: bool,
    allow_disk_write: bool,
    manifest_path: PathBuf,
    session: Option<Session>,
) -> Result<u32, AnyError> {
    let mut flags = Flags::default();
    flags.unstable = true;
    flags.reload = true;
    if allow_disk_write {
        let mut write_path = manifest_path.clone();
        write_path.pop();
        write_path.push("artifacts");
        let _ = std::fs::create_dir_all(&write_path);
        flags.allow_write = Some(vec![write_path])
    }
    let program_state = ProgramState::build(flags.clone()).await?;
    let permissions = Permissions::from_options(&flags.clone().into());
    let mut project_path = manifest_path.clone();
    project_path.pop();
    let cwd = Path::new(&project_path);
    let mut include = if include.is_empty() {
        vec!["tests".into()]
    } else {
        include.clone()
    };

    let allow_none = true;
    let no_run = false;
    let concurrent_jobs = 2;
    let quiet = false;
    let filter: Option<String> = None;
    let fail_fast = true;
    let lib = if flags.unstable {
        module_graph::TypeLib::UnstableDenoWindow
    } else {
        module_graph::TypeLib::DenoWindow
    };

    if watch {
        let handler = Arc::new(Mutex::new(FetchHandler::new(
            &program_state,
            Permissions::allow_all(),
            Permissions::allow_all(),
        )?));

        include.push("contracts".into());

        let paths_to_watch: Vec<_> = include.iter().map(PathBuf::from).collect();

        let resolver = |changed: Option<Vec<PathBuf>>| {
            let doc_modules_result = test_runner::collect_test_module_specifiers(
                include.clone(),
                &cwd,
                is_supported_ext,
            );

            let test_modules_result = test_runner::collect_test_module_specifiers(
                include.clone(),
                &cwd,
                test_runner::is_supported,
            );

            let paths_to_watch = paths_to_watch.clone();
            let paths_to_watch_clone = paths_to_watch.clone();

            let handler = handler.clone();
            let program_state = program_state.clone();
            let files_changed = changed.is_some();
            async move {
                let doc_modules = doc_modules_result?;

                let test_modules = test_modules_result?;

                let mut paths_to_watch = paths_to_watch_clone;
                let mut modules_to_reload = if files_changed {
                    Vec::new()
                } else {
                    test_modules
                        .iter()
                        .filter_map(|url| deno_core::resolve_url(url.as_str()).ok())
                        .collect()
                };

                let mut builder = GraphBuilder::new(
                    handler,
                    program_state.maybe_import_map.clone(),
                    program_state.lockfile.clone(),
                );
                for specifier in test_modules.iter() {
                    builder.add(specifier, false).await?;
                }
                let graph = builder.get_graph();

                for specifier in test_modules {
                    fn get_dependencies<'a>(
                        graph: &'a module_graph::Graph,
                        module: &'a Module,
                        // This needs to be accessible to skip getting dependencies if they're already there,
                        // otherwise this will cause a stack overflow with circular dependencies
                        output: &mut HashSet<&'a ModuleSpecifier>,
                    ) -> Result<(), AnyError> {
                        for dep in module.dependencies.values() {
                            if let Some(specifier) = &dep.maybe_code {
                                if !output.contains(specifier) {
                                    output.insert(specifier);

                                    get_dependencies(
                                        &graph,
                                        graph.get_specifier(specifier)?,
                                        output,
                                    )?;
                                }
                            }
                            if let Some(specifier) = &dep.maybe_type {
                                if !output.contains(specifier) {
                                    output.insert(specifier);

                                    get_dependencies(
                                        &graph,
                                        graph.get_specifier(specifier)?,
                                        output,
                                    )?;
                                }
                            }
                        }

                        Ok(())
                    }

                    // This test module and all it's dependencies
                    let mut modules = HashSet::new();
                    modules.insert(&specifier);
                    get_dependencies(&graph, graph.get_specifier(&specifier)?, &mut modules)?;

                    paths_to_watch.extend(
                        modules
                            .iter()
                            .filter_map(|specifier| specifier.to_file_path().ok()),
                    );

                    if let Some(changed) = &changed {
                        for path in changed.iter().filter_map(|path| {
                            deno_core::resolve_url_or_path(&path.to_string_lossy()).ok()
                        }) {
                            if path.path().ends_with(".clar") {
                                modules_to_reload.push(specifier.clone());
                            } else {
                                if modules.contains(&&path) {
                                    modules_to_reload.push(specifier);
                                    break;
                                }
                            }
                        }
                    }
                }

                Ok((paths_to_watch, modules_to_reload))
            }
            .map(move |result| match result {
                Ok((paths_to_watch, modules_to_reload)) => ResolutionResult::Restart {
                    paths_to_watch,
                    result: Ok(modules_to_reload),
                },
                Err(e) => ResolutionResult::Restart {
                    paths_to_watch,
                    result: Err(e),
                },
            })
        };

        file_watcher::watch_func(
            resolver,
            |modules_to_reload| {
                // Clear the screen
                print!("{esc}c", esc = 27 as char);
                // Clear eventual previous sessions
                sessions::reset();
                run_scripts(
                    program_state.clone(),
                    permissions.clone(),
                    lib.clone(),
                    modules_to_reload.clone(),
                    modules_to_reload,
                    no_run,
                    fail_fast,
                    quiet,
                    true,
                    filter.clone(),
                    concurrent_jobs,
                    manifest_path.clone(),
                    allow_wallets,
                    None,
                )
                .map(|res| {
                    if include_costs_report {
                        display_costs_report()
                    }
                    res.map(|_| ())
                })
            },
            "Test",
        )
        .await?;
    } else {
        let doc_modules = vec![];

        let test_modules = test_runner::collect_test_module_specifiers(
            include.clone(),
            &cwd,
            tools::test_runner::is_supported,
        )?;

        let failed = run_scripts(
            program_state.clone(),
            permissions,
            lib,
            doc_modules,
            test_modules,
            no_run,
            fail_fast,
            quiet,
            allow_none,
            filter,
            concurrent_jobs,
            manifest_path,
            allow_wallets,
            session,
        )
        .await?;

        if failed {
            std::process::exit(1);
        }
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

    if include_costs_report {
        display_costs_report()
    }

    let total = sessions::SESSIONS.lock().unwrap().len();

    Ok(total as u32)
}

#[derive(Clone)]
enum Bottleneck {
    Unknown,
    Runtime(u64, u64),
    ReadCount(u64, u64),
    ReadLength(u64, u64),
    WriteCount(u64, u64),
    WriteLength(u64, u64),
}

fn display_costs_report() {
    let mut consolidated: BTreeMap<String, BTreeMap<String, Vec<CostsReport>>> = BTreeMap::new();
    let sessions = sessions::SESSIONS.lock().unwrap();
    let mut mins: BTreeMap<(&String, &String), (f32, CostsReport, Bottleneck)> = BTreeMap::new();
    let mut maxs: BTreeMap<(&String, &String), (f32, CostsReport, Bottleneck)> = BTreeMap::new();

    for (session_id, (name, session)) in sessions.iter() {
        for report in session.costs_reports.iter() {
            let key = report.contract_id.to_string();
            match consolidated.entry(key) {
                Entry::Occupied(ref mut entry) => {
                    match entry.get_mut().entry(report.method.to_string()) {
                        Entry::Occupied(entry) => entry.into_mut().push(report.clone()),
                        Entry::Vacant(entry) => {
                            let mut reports = Vec::new();
                            reports.push(report.clone());
                            entry.insert(reports);
                        }
                    }
                }
                Entry::Vacant(entry) => {
                    let mut reports = Vec::new();
                    reports.push(report.clone());
                    let mut methods = BTreeMap::new();
                    methods.insert(report.method.to_string(), reports);
                    entry.insert(methods);
                }
            };

            // Look for the bounding factor
            let ratios = vec![
                (
                    report.cost_result.total.runtime,
                    report.cost_result.limit.runtime,
                ),
                (
                    report.cost_result.total.read_count,
                    report.cost_result.limit.read_count,
                ),
                (
                    report.cost_result.total.read_length,
                    report.cost_result.limit.read_length,
                ),
                (
                    report.cost_result.total.write_count,
                    report.cost_result.limit.write_count,
                ),
                (
                    report.cost_result.total.write_length,
                    report.cost_result.limit.write_length,
                ),
            ];
            let (bottleneck, mut max) = ratios.iter().enumerate().fold(
                (Bottleneck::Unknown, 0 as f32),
                |(bottleneck, max), (index, (cost, limit))| {
                    let ratio = (*cost as f32) / (*limit as f32);
                    if ratio > max {
                        (
                            match index {
                                0 => Bottleneck::Runtime(*cost, *limit),
                                1 => Bottleneck::ReadCount(*cost, *limit),
                                2 => Bottleneck::ReadLength(*cost, *limit),
                                3 => Bottleneck::WriteCount(*cost, *limit),
                                4 => Bottleneck::WriteLength(*cost, *limit),
                                _ => Bottleneck::Unknown,
                            },
                            ratio,
                        )
                    } else {
                        (bottleneck, max)
                    }
                },
            );

            let key = (&report.contract_id, &report.method);

            mins.entry(key)
                .and_modify(|(cur_min, min_report, cur_bottleneck)| {
                    if &mut max < cur_min {
                        *cur_min = max;
                        *min_report = report.clone();
                        *cur_bottleneck = bottleneck.clone();
                    }
                })
                .or_insert((max, report.clone(), bottleneck.clone()));
            maxs.entry(key)
                .and_modify(|(cur_max, max_report, cur_bottleneck)| {
                    if &mut max > cur_max {
                        *cur_max = max;
                        *max_report = report.clone();
                        *cur_bottleneck = bottleneck.clone();
                    }
                })
                .or_insert((max, report.clone(), bottleneck.clone()));
        }
    }

    println!("\nContract calls cost synthesis");
    let mut table = Table::new();
    let headers = vec![
        "".to_string(),
        "Runtime (units)".to_string(),
        "Read Count".to_string(),
        "Read Length (bytes)".to_string(),
        "Write Count".to_string(),
        "Write Length (bytes)".to_string(),
        "Tx per Block".to_string(),
    ];
    let mut headers_cells = vec![];
    for header in headers.iter() {
        headers_cells.push(Cell::new(&header));
    }
    table.add_row(Row::new(headers_cells.clone()));

    for (contract_id, methods) in consolidated.iter() {
        for (method, reports) in methods.iter() {
            let (min, min_report, min_bottleneck) = mins.get(&(contract_id, method)).unwrap();
            let (max, max_report, max_bottleneck) = mins.get(&(contract_id, method)).unwrap();

            // Not displaying the min row for now - probably not so interesting atm.
            // if min != max {
            //     table.add_row(Row::new(formatted_cost_cells(
            //         "Min",
            //         &min_report,
            //         &min_bottleneck,
            //     )));
            // }

            let contract_name = contract_id.split(".").last().unwrap();
            table.add_row(Row::new(formatted_cost_cells(
                &format!("{}::{}", contract_name, method),
                &max_report,
                &max_bottleneck,
            )));
        }
    }

    if let Some((_, (_, report, _))) = maxs.iter().next() {
        let limit = &report.cost_result.limit;
        table.add_row(Row::new(vec![Cell::new_align(
            &format!(""),
            format::Alignment::LEFT,
        )
        .with_hspan(7)]));

        table.add_row(Row::new(vec![
            Cell::new("Mainnet Block Limits (Stacks 2.0)"),
            Cell::new_align(
                &format!("{}", &limit.runtime.to_string()),
                format::Alignment::RIGHT,
            ),
            Cell::new_align(&limit.read_count.to_string(), format::Alignment::RIGHT),
            Cell::new_align(&format!("{}", limit.read_length), format::Alignment::RIGHT),
            Cell::new_align(&limit.write_count.to_string(), format::Alignment::RIGHT),
            Cell::new_align(&format!("{}", limit.write_length), format::Alignment::RIGHT),
            Cell::new_align("/", format::Alignment::RIGHT),
        ]));
    }

    table.printstd();
    println!("");
}

fn formatted_cost_cells(title: &str, report: &CostsReport, bottleneck: &Bottleneck) -> Vec<Cell> {
    let mut runtime_style = Attr::ForegroundColor(color::BRIGHT_BLACK);
    let mut read_count_style = Attr::ForegroundColor(color::BRIGHT_BLACK);
    let mut read_len_style = Attr::ForegroundColor(color::BRIGHT_BLACK);
    let mut write_count_style = Attr::ForegroundColor(color::BRIGHT_BLACK);
    let mut write_len_style = Attr::ForegroundColor(color::BRIGHT_BLACK);

    let tx_per_block = match bottleneck {
        Bottleneck::Runtime(cost, limit) => {
            runtime_style = Attr::ForegroundColor(color::BRIGHT_WHITE);
            limit / cost
        }
        Bottleneck::ReadCount(cost, limit) => {
            read_count_style = Attr::ForegroundColor(color::BRIGHT_WHITE);
            limit / cost
        }
        Bottleneck::ReadLength(cost, limit) => {
            read_len_style = Attr::ForegroundColor(color::BRIGHT_WHITE);
            limit / cost
        }
        Bottleneck::WriteCount(cost, limit) => {
            write_count_style = Attr::ForegroundColor(color::BRIGHT_WHITE);
            limit / cost
        }
        Bottleneck::WriteLength(cost, limit) => {
            write_len_style = Attr::ForegroundColor(color::BRIGHT_WHITE);
            limit / cost
        }
        _ => 0,
    };

    let block_style = if tx_per_block < 100 {
        Attr::ForegroundColor(color::RED)
    } else if tx_per_block < 500 {
        Attr::ForegroundColor(color::YELLOW)
    } else {
        Attr::ForegroundColor(color::GREEN)
    };

    let ratios = vec![
        (
            report.cost_result.total.runtime,
            report.cost_result.limit.runtime,
        ),
        (
            report.cost_result.total.read_count,
            report.cost_result.limit.read_count,
        ),
        (
            report.cost_result.total.read_length,
            report.cost_result.limit.read_length,
        ),
        (
            report.cost_result.total.write_count,
            report.cost_result.limit.write_count,
        ),
        (
            report.cost_result.total.write_length,
            report.cost_result.limit.write_length,
        ),
    ];

    let annotations = ratios
        .iter()
        .map(|(value, limit)| {
            if *value == 0 {
                "".to_string()
            } else {
                format!(" ({:.2}%)", 100.0 * *value as f32 / *limit as f32)
            }
        })
        .collect::<Vec<String>>();

    vec![
        Cell::new(title),
        Cell::new_align(
            &format!(
                "{}{}",
                report.cost_result.total.runtime.to_string(),
                annotations[0]
            ),
            format::Alignment::RIGHT,
        )
        .with_style(runtime_style),
        Cell::new_align(
            &format!(
                "{}{}",
                report.cost_result.total.read_count.to_string(),
                annotations[1]
            ),
            format::Alignment::RIGHT,
        )
        .with_style(read_count_style),
        Cell::new_align(
            &format!("{}{}", report.cost_result.total.read_length, annotations[2]),
            format::Alignment::RIGHT,
        )
        .with_style(read_len_style),
        Cell::new_align(
            &format!(
                "{}{}",
                report.cost_result.total.write_count.to_string(),
                annotations[3]
            ),
            format::Alignment::RIGHT,
        )
        .with_style(write_count_style),
        Cell::new_align(
            &format!(
                "{}{}",
                report.cost_result.total.write_length, annotations[4]
            ),
            format::Alignment::RIGHT,
        )
        .with_style(write_len_style),
        Cell::new_align(&format!("{}", tx_per_block), format::Alignment::RIGHT)
            .with_style(block_style),
    ]
}

pub fn is_supported_ext(path: &Path) -> bool {
    if let Some(ext) = fs_util::get_extension(path) {
        matches!(ext.as_str(), "ts" | "js" | "clar")
    } else {
        false
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn run_scripts(
    program_state: Arc<ProgramState>,
    permissions: Permissions,
    lib: module_graph::TypeLib,
    doc_modules: Vec<ModuleSpecifier>,
    test_modules: Vec<ModuleSpecifier>,
    no_run: bool,
    fail_fast: bool,
    quiet: bool,
    allow_none: bool,
    filter: Option<String>,
    concurrent_jobs: usize,
    manifest_path: PathBuf,
    allow_wallets: bool,
    session: Option<Session>,
) -> Result<bool, AnyError> {
    if !doc_modules.is_empty() {
        let mut test_programs = Vec::new();

        let blocks_regex = Regex::new(r"```([^\n]*)\n([\S\s]*?)```")?;
        let lines_regex = Regex::new(r"(?:\* ?)(?:\# ?)?(.*)")?;

        for specifier in &doc_modules {
            let mut fetch_permissions = Permissions::allow_all();
            let file = program_state
                .file_fetcher
                .fetch(&specifier, &mut fetch_permissions)
                .await?;

            let parsed_module =
                ast::parse(&file.specifier.as_str(), &file.source, &file.media_type)?;

            let mut comments = parsed_module.get_comments();
            comments.sort_by_key(|comment| {
                let location = parsed_module.get_location(&comment.span);
                location.line
            });

            for comment in comments {
                if comment.kind != CommentKind::Block || !comment.text.starts_with('*') {
                    continue;
                }

                for block in blocks_regex.captures_iter(&comment.text) {
                    let body = block.get(2).unwrap();
                    let text = body.as_str();

                    // TODO(caspervonb) generate an inline source map
                    let mut source = String::new();
                    for line in lines_regex.captures_iter(&text) {
                        let text = line.get(1).unwrap();
                        source.push_str(&format!("{}\n", text.as_str()));
                    }

                    source.push_str("export {};");

                    let element = block.get(0).unwrap();
                    let span = comment
                        .span
                        .from_inner_byte_pos(element.start(), element.end());
                    let location = parsed_module.get_location(&span);

                    let specifier = deno_core::resolve_url_or_path(&format!(
                        "{}${}-{}",
                        location.filename,
                        location.line,
                        location.line + element.as_str().split('\n').count(),
                    ))?;

                    let file = File {
                        local: specifier.to_file_path().unwrap(),
                        maybe_types: None,
                        media_type: MediaType::TypeScript, // media_type.clone(),
                        source: source.clone(),
                        specifier: specifier.clone(),
                    };

                    program_state.file_fetcher.insert_cached(file.clone());
                    test_programs.push(file.specifier.clone());
                }
            }
        }

        program_state
            .prepare_module_graph(
                test_programs.clone(),
                lib.clone(),
                Permissions::allow_all(),
                permissions.clone(),
                program_state.maybe_import_map.clone(),
            )
            .await?;
    } else if test_modules.is_empty() {
        println!("No matching test modules found");
        if !allow_none {
            std::process::exit(1);
        }

        return Ok(false);
    }

    let execution_result = program_state
        .prepare_module_graph(
            test_modules.clone(),
            lib.clone(),
            Permissions::allow_all(),
            permissions.clone(),
            program_state.maybe_import_map.clone(),
        )
        .await;
    if let Err(e) = execution_result {
        println!("{}", e);
        return Err(e);
    }

    if no_run {
        return Ok(false);
    }

    // Because scripts, and therefore worker.execute cannot detect unresolved promises at the moment
    // we generate a module for the actual test execution.
    let test_options = json!({
        "disableLog": quiet,
        "filter": filter,
    });

    let test_module = deno_core::resolve_path("$deno$test.js")?;
    let test_source = format!("await Deno[Deno.internal].runTests({});", test_options);
    let test_file = File {
        local: test_module.to_file_path().unwrap(),
        maybe_types: None,
        media_type: MediaType::JavaScript,
        source: test_source.clone(),
        specifier: test_module.clone(),
    };

    program_state.file_fetcher.insert_cached(test_file);

    let (sender, receiver) = channel::<TestEvent>();

    let join_handles = test_modules.iter().map(move |main_module| {
        let program_state = program_state.clone();
        let main_module = main_module.clone();
        let test_module = test_module.clone();
        let permissions = permissions.clone();
        let sender = sender.clone();
        let session = session.clone();

        let manifest = manifest_path.clone();
        tokio::task::spawn_blocking(move || {
            let join_handle = std::thread::spawn(move || {
                let future = run_script(
                    program_state,
                    main_module,
                    test_module,
                    permissions,
                    sender,
                    manifest,
                    allow_wallets,
                    session,
                );

                tokio_util::run_basic(future)
            });

            join_handle.join().unwrap()
        })
    });

    let join_futures = stream::iter(join_handles)
        .buffer_unordered(concurrent_jobs)
        .collect::<Vec<Result<Result<(), AnyError>, tokio::task::JoinError>>>();

    let mut reporter = create_reporter(concurrent_jobs > 1);
    let handler = {
        tokio::task::spawn_blocking(move || {
            let mut used_only = false;
            let mut has_error = false;
            let mut planned = 0;
            let mut reported = 0;

            for event in receiver.iter() {
                match event.message.clone() {
                    TestMessage::Plan {
                        pending,
                        filtered: _,
                        only,
                    } => {
                        if only {
                            used_only = true;
                        }

                        planned += pending;
                    }
                    TestMessage::Result {
                        name: _,
                        duration: _,
                        result,
                    } => {
                        reported += 1;

                        if let TestResult::Failed(_) = result {
                            has_error = true;
                        }
                    }
                    _ => {}
                }

                reporter.visit_event(event);

                if has_error && fail_fast {
                    break;
                }
            }

            if planned > reported {
                has_error = true;
            }

            reporter.done();

            if planned > reported {
                has_error = true;
            }

            if used_only {
                println!(
                    "{} because the \"only\" option was used\n",
                    colors::red("FAILED")
                );

                has_error = true;
            }

            has_error
        })
    };

    let (result, join_results) = future::join(handler, join_futures).await;

    let mut join_errors = join_results.into_iter().filter_map(|join_result| {
        join_result
            .ok()
            .map(|handle_result| handle_result.err())
            .flatten()
    });

    if let Some(e) = join_errors.next() {
        Err(e)
    } else {
        Ok(result.unwrap_or(false))
    }
}

pub async fn run_script(
    program_state: Arc<ProgramState>,
    main_module: ModuleSpecifier,
    test_module: ModuleSpecifier,
    permissions: Permissions,
    channel: Sender<TestEvent>,
    manifest_path: PathBuf,
    allow_wallets: bool,
    session: Option<Session>,
) -> Result<(), AnyError> {
    let mut worker = create_main_worker(&program_state, main_module.clone(), permissions, true);

    if let Some(template) = session {
        sessions::SESSION_TEMPLATE
            .lock()
            .unwrap()
            .push(template.clone());
    }

    {
        let js_runtime = &mut worker.js_runtime;
        js_runtime.register_op("setup_chain", deno_core::op_sync(setup_chain));
        js_runtime.register_op("mine_block", deno_core::op_sync(mine_block));
        js_runtime.register_op("mine_empty_blocks", deno_core::op_sync(mine_empty_blocks));
        js_runtime.register_op("call_read_only_fn", deno_core::op_sync(call_read_only_fn));
        js_runtime.register_op("get_assets_maps", deno_core::op_sync(get_assets_maps));
        js_runtime.sync_ops_cache();

        js_runtime.op_state().borrow_mut().put(manifest_path);

        js_runtime.op_state().borrow_mut().put(allow_wallets);

        js_runtime
            .op_state()
            .borrow_mut()
            .put::<Sender<TestEvent>>(channel.clone());
    }

    let mut maybe_coverage_collector = if let Some(ref coverage_dir) = program_state.coverage_dir {
        let session = worker.create_inspector_session().await;
        let coverage_dir = PathBuf::from(coverage_dir);
        let mut coverage_collector = CoverageCollector::new(coverage_dir, session);
        worker
            .with_event_loop(coverage_collector.start_collecting().boxed_local())
            .await?;

        Some(coverage_collector)
    } else {
        None
    };

    let execute_result = worker.execute_module(&main_module).await;
    if let Err(e) = execute_result {
        println!("{}", e);
        return Err(e);
    }

    let execute_result = worker.execute("window.dispatchEvent(new Event('load'))");
    if let Err(e) = execute_result {
        println!("{}", e);
        return Err(e);
    }

    let execute_result = worker.execute_module(&test_module).await;
    if let Err(e) = execute_result {
        println!("{}", e);
        return Err(e);
    }

    let execute_result = worker
        .run_event_loop(maybe_coverage_collector.is_none())
        .await;
    if let Err(e) = execute_result {
        println!("{}", e);
        return Err(e);
    }

    let execute_result = worker.execute("window.dispatchEvent(new Event('unload'))");
    if let Err(e) = execute_result {
        println!("{}", e);
        return Err(e);
    }

    if let Some(coverage_collector) = maybe_coverage_collector.as_mut() {
        let execute_result = worker
            .with_event_loop(coverage_collector.stop_collecting().boxed_local())
            .await;
        if let Err(e) = execute_result {
            println!("{}", e);
            return Err(e);
        }
    }

    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetupChainArgs {
    name: String,
    transactions: Vec<TransactionArgs>,
}

fn setup_chain(state: &mut OpState, args: Value, _: ()) -> Result<String, AnyError> {
    let manifest_path = state.borrow::<PathBuf>();
    let args: SetupChainArgs =
        serde_json::from_value(args).expect("Invalid request from JavaScript for \"op_load\".");
    let (session_id, accounts, contracts) =
        sessions::handle_setup_chain(manifest_path, args.name, args.transactions)?;
    let serialized_contracts = contracts.iter().map(|(a, s, _)| json!({
      "contract_id": a.contract_identifier.to_string(),
      "contract_interface": a.contract_interface.clone(),
      "dependencies": a.dependencies.clone().into_iter().map(|c| c.to_string()).collect::<Vec<String>>(),
      "source": s
    })).collect::<Vec<_>>();

    let allow_wallets = state.borrow::<bool>();
    let accounts = if *allow_wallets { accounts } else { vec![] };

    Ok(json!({
        "session_id": session_id,
        "accounts": accounts,
        "contracts": serialized_contracts,
    })
    .to_string())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MineBlockArgs {
    session_id: u32,
    transactions: Vec<TransactionArgs>,
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

fn value_to_string(value: &types::Value) -> String {
    use clarity_repl::clarity::types::{CharType, SequenceData, Value};

    match value {
        Value::Tuple(tup_data) => {
            let mut out = String::new();
            write!(out, "{{");
            for (i, (name, value)) in tup_data.data_map.iter().enumerate() {
                write!(out, "{}: {}", &**name, value_to_string(value));
                if i < tup_data.data_map.len() - 1 {
                    write!(out, ", ");
                }
            }
            write!(out, "}}");
            out
        }
        Value::Optional(opt_data) => match opt_data.data {
            Some(ref x) => format!("(some {})", value_to_string(&**x)),
            None => "none".to_string(),
        },
        Value::Response(res_data) => match res_data.committed {
            true => format!("(ok {})", value_to_string(&*res_data.data)),
            false => format!("(err {})", value_to_string(&*res_data.data)),
        },
        Value::Sequence(SequenceData::String(CharType::ASCII(data))) => {
            format!("\"{}\"", String::from_utf8(data.data.clone()).unwrap())
        }
        Value::Sequence(SequenceData::String(CharType::UTF8(data))) => {
            let mut result = String::new();
            for c in data.data.iter() {
                if c.len() > 1 {
                    // We escape extended charset
                    result.push_str(&format!("\\u{{{}}}", hash::to_hex(&c[..])));
                } else {
                    result.push(c[0] as char)
                }
            }
            format!("u\"{}\"", result)
        }
        Value::Sequence(SequenceData::List(list_data)) => {
            let mut out = String::new();
            write!(out, "[");
            for (ix, v) in list_data.data.iter().enumerate() {
                if ix > 0 {
                    write!(out, ", ");
                }
                write!(out, "{}", value_to_string(v));
            }
            write!(out, "]");
            out
        }
        _ => format!("{}", value),
    }
}

fn mine_block(state: &mut OpState, args: Value, _: ()) -> Result<String, AnyError> {
    let args: MineBlockArgs =
        serde_json::from_value(args).expect("Invalid request from JavaScript.");
    let (block_height, receipts) = sessions::perform_block(args.session_id, |name, session| {
        let initial_tx_sender = session.get_tx_sender();
        let mut receipts = vec![];
        for tx in args.transactions.iter() {
            if let Some(ref args) = tx.contract_call {
                let execution = match session.invoke_contract_call(
                    &args.contract,
                    &args.method,
                    &args.args,
                    &tx.sender,
                    name.into(),
                ) {
                    Ok(res) => res,
                    Err(diagnostics) => {
                        if diagnostics.len() > 0 {
                            // todo(ludo): if CLARINET_BACKTRACE=1
                            // Retrieve the AST (penultimate entry), and the expression id (last entry)
                            println!(
                                "Runtime error: {}::{}({}) -> {}",
                                args.contract,
                                args.method,
                                args.args.join(", "),
                                diagnostics.last().unwrap().message
                            );
                        }
                        continue;
                    }
                };
                let result = match execution.result {
                    Some(output) => value_to_string(&output),
                    _ => unreachable!("Value empty"),
                };
                receipts.push((result, execution.events));
            } else {
                session.set_tx_sender(tx.sender.clone());
                if let Some(ref args) = tx.deploy_contract {
                    let execution = session
                        .interpret(
                            args.code.clone(),
                            Some(args.name.clone()),
                            true,
                            Some(name.into()),
                        )
                        .unwrap(); // todo(ludo)
                    let result = match execution.result {
                        Some(output) => format!("{}", output),
                        _ => unreachable!("Value empty"),
                    };
                    receipts.push((result, execution.events));
                } else if let Some(ref args) = tx.transfer_stx {
                    let snippet = format!(
                        "(stx-transfer? u{} tx-sender '{})",
                        args.amount, args.recipient
                    );
                    let execution = session
                        .interpret(snippet, None, true, Some(name.into()))
                        .unwrap(); // todo(ludo)
                    let result = match execution.result {
                        Some(output) => format!("{}", output),
                        _ => unreachable!("Value empty"),
                    };
                    receipts.push((result, execution.events));
                }
                session.set_tx_sender(initial_tx_sender.clone());
            }
        }
        let block_height = session.advance_chain_tip(1);
        Ok((block_height, receipts))
    })?;

    let payload = json!({
      "session_id": args.session_id,
      "block_height": block_height,
      "receipts":  receipts.iter().map(|r| {
        json!({
          "result": r.0,
          "events": r.1,
        })
      }).collect::<Vec<_>>()
    });

    Ok(payload.to_string())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MineEmptyBlocksArgs {
    session_id: u32,
    count: u32,
}

fn mine_empty_blocks(state: &mut OpState, args: Value, _: ()) -> Result<String, AnyError> {
    let args: MineEmptyBlocksArgs =
        serde_json::from_value(args).expect("Invalid request from JavaScript.");
    let block_height = sessions::perform_block(args.session_id, |name, session| {
        let block_height = session.advance_chain_tip(args.count);
        Ok(block_height)
    })?;

    Ok(json!({
      "session_id": args.session_id,
      "block_height": block_height,
    })
    .to_string())
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

fn call_read_only_fn(state: &mut OpState, args: Value, _: ()) -> Result<String, AnyError> {
    let args: CallReadOnlyFnArgs =
        serde_json::from_value(args).expect("Invalid request from JavaScript.");
    let (result, events) = sessions::perform_block(args.session_id, |name, session| {
        let execution = session
            .invoke_contract_call(
                &args.contract,
                &args.method,
                &args.args,
                &args.sender,
                "readonly-calls".into(),
            )
            .unwrap(); // todo(ludo)
        let result = match execution.result {
            Some(output) => format!("{}", output),
            _ => unreachable!("Value empty"),
        };
        Ok((result, execution.events))
    })?;
    Ok(json!({
      "session_id": args.session_id,
      "result": result,
      "events": events,
    })
    .to_string())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetAssetsMapsArgs {
    session_id: u32,
}

fn get_assets_maps(state: &mut OpState, args: Value, _: ()) -> Result<String, AnyError> {
    let args: GetAssetsMapsArgs =
        serde_json::from_value(args).expect("Invalid request from JavaScript.");
    let assets_maps = sessions::perform_block(args.session_id, |name, session| {
        let assets_maps = session.get_assets_maps();
        let mut lev1 = BTreeMap::new();
        for (key1, map1) in assets_maps.into_iter() {
            let mut lev2 = BTreeMap::new();
            for (key2, val2) in map1.into_iter() {
                lev2.insert(
                    key2,
                    u64::try_from(val2)
                        .expect("u128 unsupported at the moment, please open an issue."),
                );
            }
            lev1.insert(key1, lev2);
        }
        Ok(lev1)
    })?;
    Ok(json!({
      "session_id": args.session_id,
      "assets": assets_maps,
    })
    .to_string())
}

#[cfg(test)]
mod tests {
    use clarity_repl::clarity::representations::ClarityName;
    use clarity_repl::clarity::types::{
        ListTypeData, OptionalData, ResponseData, SequenceData, SequencedValue, TupleData,
    };

    use super::*;

    #[test]
    fn test_value_to_string() {
        let mut s = value_to_string(&types::Value::Int(42));
        assert_eq!(s, "42");

        s = value_to_string(&types::Value::UInt(12345678909876));
        assert_eq!(s, "u12345678909876");

        s = value_to_string(&types::Value::Bool(true));
        assert_eq!(s, "true");

        s = value_to_string(&types::Value::buff_from(vec![1, 2, 3]).unwrap());
        assert_eq!(s, "0x010203");

        s = value_to_string(&types::Value::buff_from(vec![1, 2, 3]).unwrap());
        assert_eq!(s, "0x010203");

        s = value_to_string(&types::Value::Tuple(
            TupleData::from_data(vec![(
                ClarityName::try_from("foo".to_string()).unwrap(),
                types::Value::Bool(true),
            )])
            .unwrap(),
        ));
        assert_eq!(s, "{foo: true}");

        s = value_to_string(&types::Value::Optional(OptionalData {
            data: Some(Box::new(types::Value::UInt(42))),
        }));
        assert_eq!(s, "(some u42)");

        s = value_to_string(&types::NONE);
        assert_eq!(s, "none");

        s = value_to_string(&types::Value::Response(ResponseData {
            committed: true,
            data: Box::new(types::Value::Int(-321)),
        }));
        assert_eq!(s, "(ok -321)");

        s = value_to_string(&types::Value::Response(ResponseData {
            committed: false,
            data: Box::new(types::Value::Sequence(types::SequenceData::String(
                types::CharType::ASCII(types::ASCIIData {
                    data: "'foo'".as_bytes().to_vec(),
                }),
            ))),
        }));
        assert_eq!(s, "(err \"'foo'\")");

        s = value_to_string(&types::Value::Sequence(types::SequenceData::String(
            types::CharType::ASCII(types::ASCIIData {
                data: "Hello, \"world\"\n".as_bytes().to_vec(),
            }),
        )));
        assert_eq!(s, "\"Hello, \"world\"\n\"");

        s = value_to_string(&types::UTF8Data::to_value(
            &"Hello, 'world'\n".as_bytes().to_vec(),
        ));
        assert_eq!(s, "u\"Hello, 'world'\n\"");

        s = value_to_string(&types::Value::Sequence(SequenceData::List(
            types::ListData {
                data: vec![types::Value::Int(-321)],
                type_signature: ListTypeData::new_list(types::TypeSignature::IntType, 2).unwrap(),
            },
        )));
        assert_eq!(s, "[-321]");
    }
}

use super::api_v1::SessionArtifacts;
use super::{api_v1, costs, DeploymentCache};
use clarity_repl::clarity::coverage::CoverageReporter;
use clarity_repl::clarity::types;
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
use std::collections::{btree_map::Entry, BTreeMap};
use std::collections::{HashMap, HashSet};
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

use crate::deployment::types::DeploymentSpecification;

pub async fn do_run_scripts(
    include: Vec<String>,
    include_coverage: bool,
    include_costs_report: bool,
    watch: bool,
    allow_wallets: bool,
    allow_disk_write: bool,
    manifest_path: PathBuf,
    cache: DeploymentCache,
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
                .map(|mut res| {
                    match res.as_mut() {
                        Ok((success, sessions_artifacts)) if *success => {
                            if include_costs_report {
                                costs::display_costs_report(sessions_artifacts)
                            }
                        }
                        _ => {}
                    };
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

        let (success, sessions_artifacts) = run_scripts(
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
            Some(cache.clone()),
        )
        .await?;

        if !success {
            std::process::exit(1);
        }

        if include_coverage {
            let mut coverage_reporter = CoverageReporter::new();
            for (contract_id, analysis_artifacts) in cache.contracts_artifacts.iter() {
                coverage_reporter
                    .asts
                    .insert(contract_id.clone(), analysis_artifacts.ast.clone());
            }
            for (contract_id, (_, contract_path)) in cache.deployment.contracts.iter() {
                coverage_reporter
                    .contract_paths
                    .insert(contract_id.name.to_string(), contract_path.clone());
            }
            for mut artifact in sessions_artifacts.into_iter() {
                coverage_reporter
                    .reports
                    .append(&mut artifact.coverage_reports);
            }
            coverage_reporter.write_lcov_file("coverage.lcov");
        }

        if include_costs_report {
            // costs::display_costs_report()
        }
    }
    Ok(0 as u32)
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
    cache: Option<DeploymentCache>,
) -> Result<(bool, Vec<SessionArtifacts>), AnyError> {
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

        return Ok((false, vec![]));
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
        return Ok((false, vec![]));
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
        let cache = cache.clone();

        let manifest = manifest_path.clone();
        tokio::task::spawn_blocking(move || {
            let join_handle = std::thread::spawn(move || {
                let future = api_v1::run_bridge(
                    program_state,
                    main_module,
                    test_module,
                    permissions,
                    sender,
                    manifest,
                    allow_wallets,
                    cache,
                );

                tokio_util::run_basic(future)
            });

            join_handle.join().unwrap()
        })
    });

    let join_futures = stream::iter(join_handles)
        .buffer_unordered(concurrent_jobs)
        .collect::<Vec<Result<Result<Vec<SessionArtifacts>, AnyError>, tokio::task::JoinError>>>();

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

    let (result, mut join_results) = future::join(handler, join_futures).await;

    let mut reports = vec![];
    let mut error = None;
    for mut res in join_results.drain(..) {
        if let Ok(Ok(artifacts)) = res.as_mut() {
            reports.append(artifacts);
        } else if let Ok(Err(e)) = res {
            error = Some(e);
            break;
        }
    }

    if let Some(e) = error {
        Err(e)
    } else {
        Ok((result.unwrap_or(false), reports))
    }
}

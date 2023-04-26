// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

use hiro_system_kit::nestable_block_on;

use super::vendor::deno_cli::args::{ConfigFlag, TypeCheckMode};
use super::vendor::deno_cli::args::{DenoSubcommand, Flags, TestFlags};
use super::vendor::deno_cli::file_fetcher::File;
use super::vendor::deno_cli::file_watcher;
use super::vendor::deno_cli::file_watcher::ResolutionResult;
use super::vendor::deno_cli::fs_util::collect_specifiers;
use super::vendor::deno_cli::fs_util::is_supported_test_ext;
use super::vendor::deno_cli::fs_util::is_supported_test_path;
use super::vendor::deno_cli::fs_util::specifier_to_file_path;
use super::vendor::deno_cli::graph_util::contains_specifier;
use super::vendor::deno_cli::graph_util::graph_valid;
use super::vendor::deno_cli::proc_state::ProcState;
use super::vendor::deno_cli::tools::test::{
    PrettyTestReporter, TestEvent, TestEventSender, TestFilter, TestMode, TestResult,
    TestSpecifierOptions, TestStepResult, TestSummary,
};

use super::vendor::deno_runtime::permissions::Permissions;
use super::vendor::deno_runtime::tokio_util::run_local;
use super::{api_v1, costs, ChainhookEvent, DeploymentCache, SessionArtifacts};
use clarinet_files::{FileLocation, ProjectManifest};
use clarity_repl::analysis::coverage::CoverageReporter;
use deno_ast::swc::common::comments::CommentKind;
use deno_ast::MediaType;
use deno_ast::SourceRangedForSpanned;
use deno_core::error::generic_error;
use deno_core::error::AnyError;
use deno_core::error::JsError;
use deno_core::futures::future;
use deno_core::futures::stream;
use deno_core::futures::FutureExt;
use deno_core::futures::StreamExt;
use deno_core::ModuleSpecifier;
use deno_graph::ModuleKind;
use indexmap::IndexMap;
use log::Level;
use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use regex::Regex;
use stacks_network::chainhook_event_observer::chainhooks::types::StacksChainhookSpecification;
use std::collections::HashSet;
use std::fmt::Write as _;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::result::Result::Ok;
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc::unbounded_channel;

pub async fn do_run_scripts(
    include: Vec<String>,
    coverage_report: Option<PathBuf>,
    display_costs_report: bool,
    watch: bool,
    allow_wallets: bool,
    allow_disk_read: bool,
    allow_disk_write: bool,
    allow_run: Option<Vec<String>>,
    allow_env: Option<Vec<String>>,
    manifest: &ProjectManifest,
    cache: DeploymentCache,
    _deployment_plan_path: Option<String>,
    fail_fast: Option<u16>,
    filter: Option<String>,
    import_map: Option<String>,
    allow_net: bool,
    cache_location: FileLocation,
    ts_config: Option<String>,
    stacks_chainhooks: Vec<StacksChainhookSpecification>,
    mine_block_delay: u16,
) -> Result<usize, (AnyError, usize)> {
    let project_root = manifest.location.get_project_root_location().unwrap();
    let cwd = PathBuf::from(&project_root.to_string());
    let concurrent_jobs = NonZeroUsize::new(num_cpus::get()).expect("unable to determine num_cp");
    let fail_fast = match fail_fast {
        None | Some(0) => None,
        Some(limit) => Some(NonZeroUsize::new(limit.into()).unwrap()),
    };
    let include = if include.is_empty() {
        let mut tests_default = cwd.clone();
        tests_default.push("tests");
        vec![format!("{}", tests_default.display())]
    } else {
        include.clone()
    };
    let watched = if watch {
        let mut paths_to_watch: Vec<_> = include.iter().map(PathBuf::from).collect();
        let mut contracts_default = cwd.clone();
        contracts_default.push("contracts");
        paths_to_watch.push(contracts_default);
        Some(paths_to_watch)
    } else {
        None
    };

    let allow_read_path = match allow_disk_read {
        true => Some(vec![cwd.clone()]),
        false => None,
    };
    let allow_write_path = match allow_disk_write {
        true => Some(vec![cwd.clone()]),
        false => None,
    };
    let test_flags = TestFlags {
        ignore: vec![],    // todo(lgalabru)
        trace_ops: true,   // todo(lgalabru)
        allow_none: false, // todo(lgalabru)
        fail_fast,
        include,
        filter,
        shuffle: None,
        doc: false,
        concurrent_jobs,
        no_run: false,
    };
    // let watch = if
    let flags = Flags {
        argv: vec![],
        subcommand: DenoSubcommand::Test(test_flags.clone()),
        allow_all: false,
        allow_env,
        allow_hrtime: false,
        allow_net: if allow_net {
            Some(vec!["deno.land".into()])
        } else {
            None
        },
        cache_path: Some(cache_location.to_string().into()),
        watch: watched,
        import_map_path: import_map,
        allow_ffi: None,
        allow_read: allow_read_path,
        allow_write: allow_write_path,
        allow_run,
        cache_blocklist: vec![],              // todo(lgalabru)
        cached_only: false,                   // todo(lgalabru)
        ignore: vec![],                       // todo(lgalabru)
        type_check_mode: TypeCheckMode::None, // todo(lgalabru)
        compat: false,
        config_flag: match ts_config {
            Some(ref ts_config) => ConfigFlag::Path(ts_config.clone()),
            None => ConfigFlag::Discover,
        },
        ..Default::default()
    };

    let chainhook_tx = if !stacks_chainhooks.is_empty() {
        let (chainhook_tx, chainhook_rx) = channel();
        std::thread::spawn(move || {
            while let Ok(msg) = chainhook_rx.recv() {
                match msg {
                    ChainhookEvent::PerformRequest(request_builder) => {
                        match nestable_block_on(request_builder.send()) {
                            Ok(r) => {
                                if !r.status().is_success() {
                                    let body = nestable_block_on(r.text()).unwrap();
                                    println!(
                                        "{} unable to invoke chainhook ({})",
                                        red!("error:"),
                                        body
                                    );
                                    std::process::exit(1);
                                }
                            }
                            Err(e) => {
                                println!(
                                    "{} unable to invoke chainhook ({})",
                                    red!("error:"),
                                    e.to_string()
                                );
                                std::process::exit(1);
                            }
                        }
                    }
                    ChainhookEvent::Exit => {
                        break;
                    }
                }
            }
        });
        Some(chainhook_tx)
    } else {
        None
    };

    let success = if flags.watch.is_some() {
        run_tests_with_watch(
            flags,
            test_flags,
            allow_wallets,
            Some(cache),
            display_costs_report,
            stacks_chainhooks,
            mine_block_delay,
            chainhook_tx.clone(),
        )
        .await
        .map_err(|e| (e, 0))?;
        0
    } else {
        run_tests(
            flags,
            test_flags,
            allow_wallets,
            Some(cache),
            display_costs_report,
            coverage_report,
            stacks_chainhooks,
            mine_block_delay,
            chainhook_tx.clone(),
        )
        .await?
    };

    if let Some(ref chainhook_tx) = chainhook_tx {
        let _ = chainhook_tx.send(ChainhookEvent::Exit);
    }

    Ok(success)
}

// pub fn is_supported_ext(path: &Path) -> bool {
//     if let Some(ext) = fs_util::get_extension(path) {
//         matches!(ext.as_str(), "ts" | "js" | "clar")
//     } else {
//         false
//     }
// }

fn extract_files_from_regex_blocks(
    specifier: &ModuleSpecifier,
    source: &str,
    media_type: MediaType,
    file_line_index: usize,
    blocks_regex: &Regex,
    lines_regex: &Regex,
) -> Result<Vec<File>, AnyError> {
    let files = blocks_regex
        .captures_iter(source)
        .filter_map(|block| {
            if block.get(1) == None {
                return None;
            }

            let maybe_attributes: Option<Vec<_>> = block
                .get(1)
                .map(|attributes| attributes.as_str().split(' ').collect());

            let file_media_type = if let Some(attributes) = maybe_attributes {
                if attributes.contains(&"ignore") {
                    return None;
                }

                match attributes.get(0) {
                    Some(&"js") => MediaType::JavaScript,
                    Some(&"javascript") => MediaType::JavaScript,
                    Some(&"mjs") => MediaType::Mjs,
                    Some(&"cjs") => MediaType::Cjs,
                    Some(&"jsx") => MediaType::Jsx,
                    Some(&"ts") => MediaType::TypeScript,
                    Some(&"typescript") => MediaType::TypeScript,
                    Some(&"mts") => MediaType::Mts,
                    Some(&"cts") => MediaType::Cts,
                    Some(&"tsx") => MediaType::Tsx,
                    Some(&"") => media_type,
                    _ => MediaType::Unknown,
                }
            } else {
                media_type
            };

            if file_media_type == MediaType::Unknown {
                return None;
            }

            let line_offset = source[0..block.get(0).unwrap().start()]
                .chars()
                .filter(|c| *c == '\n')
                .count();

            let line_count = block.get(0).unwrap().as_str().split('\n').count();

            let body = block.get(2).unwrap();
            let text = body.as_str();

            // TODO(caspervonb) generate an inline source map
            let mut file_source = String::new();
            for line in lines_regex.captures_iter(text) {
                let text = line.get(1).unwrap();
                writeln!(file_source, "{}", text.as_str()).unwrap();
            }

            let file_specifier = deno_core::resolve_url_or_path(&format!(
                "{}${}-{}{}",
                specifier,
                file_line_index + line_offset + 1,
                file_line_index + line_offset + line_count + 1,
                file_media_type.as_ts_extension(),
            ))
            .unwrap();

            Some(File {
                local: file_specifier.to_file_path().unwrap(),
                maybe_types: None,
                media_type: file_media_type,
                source: file_source.into(),
                specifier: file_specifier,
                maybe_headers: None,
            })
        })
        .collect();

    Ok(files)
}

fn extract_files_from_source_comments(
    specifier: &ModuleSpecifier,
    source: Arc<str>,
    media_type: MediaType,
) -> Result<Vec<File>, AnyError> {
    let parsed_source = deno_ast::parse_module(deno_ast::ParseParams {
        specifier: specifier.as_str().to_string(),
        text_info: deno_ast::SourceTextInfo::new(source),
        media_type,
        capture_tokens: false,
        maybe_syntax: None,
        scope_analysis: false,
    })?;
    let comments = parsed_source.comments().get_vec();
    let blocks_regex = Regex::new(r"```([^\r\n]*)\r?\n([\S\s]*?)```")?;
    let lines_regex = Regex::new(r"(?:\* ?)(?:\# ?)?(.*)")?;

    let files = comments
        .iter()
        .filter(|comment| {
            if comment.kind != CommentKind::Block || !comment.text.starts_with('*') {
                return false;
            }

            true
        })
        .flat_map(|comment| {
            extract_files_from_regex_blocks(
                specifier,
                &comment.text,
                media_type,
                parsed_source.text_info().line_index(comment.start()),
                &blocks_regex,
                &lines_regex,
            )
        })
        .flatten()
        .collect();

    Ok(files)
}

fn extract_files_from_fenced_blocks(
    specifier: &ModuleSpecifier,
    source: &str,
    media_type: MediaType,
) -> Result<Vec<File>, AnyError> {
    // The pattern matches code blocks as well as anything in HTML comment syntax,
    // but it stores the latter without any capturing groups. This way, a simple
    // check can be done to see if a block is inside a comment (and skip typechecking)
    // or not by checking for the presence of capturing groups in the matches.
    let blocks_regex = Regex::new(r"(?s)<!--.*?-->|```([^\r\n]*)\r?\n([\S\s]*?)```")?;
    let lines_regex = Regex::new(r"(?:\# ?)?(.*)")?;

    extract_files_from_regex_blocks(
        specifier,
        source,
        media_type,
        /* file line index */ 0,
        &blocks_regex,
        &lines_regex,
    )
}

async fn fetch_inline_files(
    ps: ProcState,
    specifiers: Vec<ModuleSpecifier>,
) -> Result<Vec<File>, AnyError> {
    let mut files = Vec::new();
    for specifier in specifiers {
        let mut fetch_permissions = Permissions::allow_all();
        let file = ps
            .file_fetcher
            .fetch(&specifier, &mut fetch_permissions)
            .await?;

        let inline_files = if file.media_type == MediaType::Unknown {
            extract_files_from_fenced_blocks(&file.specifier, &file.source, file.media_type)
        } else {
            extract_files_from_source_comments(
                &file.specifier,
                file.source.clone(),
                file.media_type,
            )
        };

        files.extend(inline_files?);
    }

    Ok(files)
}

/// Type check a collection of module and document specifiers.
pub async fn check_specifiers(
    ps: &ProcState,
    permissions: Permissions,
    specifiers: Vec<(ModuleSpecifier, TestMode)>,
) -> Result<(), AnyError> {
    let lib = ps.options.ts_type_lib_window();
    let inline_files = fetch_inline_files(
        ps.clone(),
        specifiers
            .iter()
            .filter_map(|(specifier, mode)| {
                if *mode != TestMode::Executable {
                    Some(specifier.clone())
                } else {
                    None
                }
            })
            .collect(),
    )
    .await?;

    if !inline_files.is_empty() {
        let specifiers = inline_files
            .iter()
            .map(|file| file.specifier.clone())
            .collect();

        for file in inline_files {
            println!("caching {}", file.specifier);
            ps.file_fetcher.insert_cached(file);
        }

        ps.prepare_module_load(
            specifiers,
            false,
            lib,
            Permissions::allow_all(),
            permissions.clone(),
            false,
        )
        .await?;
    }

    let module_specifiers = specifiers
        .iter()
        .filter_map(|(specifier, mode)| {
            if *mode != TestMode::Documentation {
                Some(specifier.clone())
            } else {
                None
            }
        })
        .collect();

    ps.prepare_module_load(
        module_specifiers,
        false,
        lib,
        Permissions::allow_all(),
        permissions,
        true,
    )
    .await?;

    Ok(())
}

/// Test a collection of specifiers with test modes concurrently.
async fn test_specifiers(
    ps: ProcState,
    permissions: Permissions,
    specifiers_with_mode: Vec<(ModuleSpecifier, TestMode)>,
    options: TestSpecifierOptions,
    allow_wallets: bool,
    deployment_cache: Option<DeploymentCache>,
    stacks_chainhooks: Vec<StacksChainhookSpecification>,
    mine_block_delay: u16,
    chainhook_tx: Option<Sender<ChainhookEvent>>,
) -> Result<(bool, Vec<SessionArtifacts>), AnyError> {
    let log_level = ps.options.log_level();
    let specifiers_with_mode = if let Some(seed) = options.shuffle {
        let mut rng = SmallRng::seed_from_u64(seed);
        let mut specifiers_with_mode = specifiers_with_mode.clone();
        specifiers_with_mode.sort_by_key(|(specifier, _)| specifier.clone());
        specifiers_with_mode.shuffle(&mut rng);
        specifiers_with_mode
    } else {
        specifiers_with_mode
    };

    let (sender, mut receiver) = unbounded_channel::<TestEvent>();
    let sender = TestEventSender::new(sender);
    let concurrent_jobs = options.concurrent_jobs;
    let fail_fast = options.fail_fast;

    let join_handles = specifiers_with_mode.iter().map(move |(specifier, mode)| {
        let ps = ps.clone();
        let permissions = permissions.clone();
        let specifier = specifier.clone();
        let mode = mode.clone();
        let mut sender = sender.clone();
        let options = options.clone();
        let deployment_cache = deployment_cache.clone();
        let stacks_chainhooks = stacks_chainhooks.clone();
        let chainhook_tx = chainhook_tx.clone();

        tokio::task::spawn_blocking(move || {
            let origin = specifier.to_string();
            let channel = sender.clone();
            let file_result = run_local(api_v1::run_bridge(
                ps,
                permissions,
                specifier,
                mode,
                options,
                channel,
                allow_wallets,
                deployment_cache,
                stacks_chainhooks,
                mine_block_delay,
                chainhook_tx,
            ));

            match file_result {
                Err(error) if error.is::<JsError>() => {
                    let _ = sender.send(TestEvent::UncaughtError(
                        origin,
                        Box::new(error.downcast::<JsError>().unwrap().clone()),
                    ));
                    Ok(vec![])
                }
                Err(error) => Err(error),
                Ok(artifacts) => Ok(artifacts),
            }
        })
    });

    let join_stream = stream::iter(join_handles)
        .buffer_unordered(concurrent_jobs.get())
        .collect::<Vec<Result<Result<Vec<SessionArtifacts>, AnyError>, tokio::task::JoinError>>>();

    let mut reporter = Box::new(PrettyTestReporter::new(
        concurrent_jobs.get() > 1,
        log_level != Some(Level::Error),
    ));

    let handler = {
        tokio::task::spawn(async move {
            let earlier = Instant::now();
            let mut tests = IndexMap::new();
            let mut test_steps = IndexMap::new();
            let mut tests_with_result = HashSet::new();
            let mut summary = TestSummary::new();
            let mut used_only = false;

            while let Some(event) = receiver.recv().await {
                match event {
                    TestEvent::Register(description) => {
                        reporter.report_register(&description);
                        tests.insert(description.id, description);
                    }

                    TestEvent::Plan(plan) => {
                        summary.total += plan.total;
                        summary.filtered_out += plan.filtered_out;

                        if plan.used_only {
                            used_only = true;
                        }

                        reporter.report_plan(&plan);
                    }

                    TestEvent::Wait(id) => {
                        reporter.report_wait(tests.get(&id).unwrap());
                    }

                    TestEvent::Output(output) => {
                        reporter.report_output(&output);
                    }

                    TestEvent::Result(id, result, elapsed) => {
                        if tests_with_result.insert(id) {
                            let description = tests.get(&id).unwrap().clone();
                            match &result {
                                TestResult::Ok => {
                                    summary.passed += 1;
                                }
                                TestResult::Ignored => {
                                    summary.ignored += 1;
                                }
                                TestResult::Failed(error) => {
                                    summary.failed += 1;
                                    summary.failures.push((description.clone(), error.clone()));
                                }
                                TestResult::Cancelled => {
                                    unreachable!("should be handled in TestEvent::UncaughtError");
                                }
                            }
                            reporter.report_result(&description, &result, elapsed);
                        }
                    }

                    TestEvent::UncaughtError(origin, error) => {
                        reporter.report_uncaught_error(&origin, &error);
                        summary.failed += 1;
                        summary.uncaught_errors.push((origin.clone(), error));
                        for desc in tests.values() {
                            if desc.origin == origin && tests_with_result.insert(desc.id) {
                                summary.failed += 1;
                                reporter.report_result(desc, &TestResult::Cancelled, 0);
                            }
                        }
                    }

                    TestEvent::StepRegister(description) => {
                        reporter.report_step_register(&description);
                        test_steps.insert(description.id, description);
                    }

                    TestEvent::StepWait(id) => {
                        reporter.report_step_wait(test_steps.get(&id).unwrap());
                    }

                    TestEvent::StepResult(id, result, duration) => {
                        match &result {
                            TestStepResult::Ok => {
                                summary.passed_steps += 1;
                            }
                            TestStepResult::Ignored => {
                                summary.ignored_steps += 1;
                            }
                            TestStepResult::Failed(_) => {
                                summary.failed_steps += 1;
                            }
                            TestStepResult::Pending(_) => {
                                summary.pending_steps += 1;
                            }
                        }

                        reporter.report_step_result(
                            test_steps.get(&id).unwrap(),
                            &result,
                            duration,
                            &tests,
                            &test_steps,
                        );
                    }
                }

                if let Some(x) = fail_fast {
                    if summary.failed >= x.get() {
                        break;
                    }
                }
            }

            let elapsed = Instant::now().duration_since(earlier);
            reporter.report_summary(&summary, &elapsed);

            if used_only {
                return Err(generic_error(
                    "Test failed because the \"only\" option was used",
                ));
            }

            if summary.failed > 0 {
                return Err(generic_error("Test failed"));
            }

            Ok(())
        })
    };

    let (mut join_results, result) = future::join(join_stream, handler).await;
    let result = result.expect("unable to join thread");
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
        Ok((result.is_ok(), reports))
    }
}

/// Collects specifiers marking them with the appropriate test mode while maintaining the natural
/// input order.
///
/// - Specifiers matching the `is_supported_test_ext` predicate are marked as
/// `TestMode::Documentation`.
/// - Specifiers matching the `is_supported_test_path` are marked as `TestMode::Executable`.
/// - Specifiers matching both predicates are marked as `TestMode::Both`
fn collect_specifiers_with_test_mode(
    include: Vec<String>,
    ignore: Vec<PathBuf>,
    include_inline: bool,
) -> Result<Vec<(ModuleSpecifier, TestMode)>, AnyError> {
    let module_specifiers = collect_specifiers(include.clone(), &ignore, is_supported_test_path)?;

    if include_inline {
        return collect_specifiers(include, &ignore, is_supported_test_ext).map(|specifiers| {
            specifiers
                .into_iter()
                .map(|specifier| {
                    let mode = if module_specifiers.contains(&specifier) {
                        TestMode::Both
                    } else {
                        TestMode::Documentation
                    };

                    (specifier, mode)
                })
                .collect()
        });
    }

    let specifiers_with_mode = module_specifiers
        .into_iter()
        .map(|specifier| (specifier, TestMode::Executable))
        .collect();

    Ok(specifiers_with_mode)
}

/// Collects module and document specifiers with test modes via
/// `collect_specifiers_with_test_mode` which are then pre-fetched and adjusted
/// based on the media type.
///
/// Specifiers that do not have a known media type that can be executed as a
/// module are marked as `TestMode::Documentation`. Type definition files
/// cannot be run, and therefore need to be marked as `TestMode::Documentation`
/// as well.
async fn fetch_specifiers_with_test_mode(
    ps: &ProcState,
    include: Vec<String>,
    ignore: Vec<PathBuf>,
    include_inline: bool,
) -> Result<Vec<(ModuleSpecifier, TestMode)>, AnyError> {
    let maybe_test_config = ps.options.to_test_config()?;

    let mut include_files = include.clone();
    let mut exclude_files = ignore.clone();

    if let Some(test_config) = maybe_test_config.as_ref() {
        if include_files.is_empty() {
            include_files = test_config
                .files
                .include
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>();
        }

        if exclude_files.is_empty() {
            exclude_files = test_config
                .files
                .exclude
                .iter()
                .filter_map(|s| specifier_to_file_path(s).ok())
                .collect::<Vec<_>>();
        }
    }

    if include_files.is_empty() {
        include_files.push(".".to_string());
    }

    let mut specifiers_with_mode =
        collect_specifiers_with_test_mode(include_files, exclude_files, include_inline)?;
    for (specifier, mode) in &mut specifiers_with_mode {
        let file = ps
            .file_fetcher
            .fetch(specifier, &mut Permissions::allow_all())
            .await?;

        if file.media_type == MediaType::Unknown || file.media_type == MediaType::Dts {
            *mode = TestMode::Documentation
        }
    }

    Ok(specifiers_with_mode)
}

pub async fn run_tests(
    flags: Flags,
    test_flags: TestFlags,
    allow_wallets: bool,
    deployment_cache: Option<DeploymentCache>,
    display_costs_report: bool,
    coverage_report: Option<PathBuf>,
    stacks_chainhooks: Vec<StacksChainhookSpecification>,
    mine_block_delay: u16,
    chainhook_tx: Option<Sender<ChainhookEvent>>,
) -> Result<usize, (AnyError, usize)> {
    let ps = ProcState::build(flags).await.map_err(|e| (e, 0))?;
    let permissions = Permissions::from_options(&ps.options.permissions_options());
    let specifiers_with_mode = fetch_specifiers_with_test_mode(
        &ps,
        test_flags.include,
        test_flags.ignore.clone(),
        test_flags.doc,
    )
    .await
    .map_err(|e| (e, 0))?;

    if !test_flags.allow_none && specifiers_with_mode.is_empty() {
        return Err((generic_error("No test modules found"), 0));
    }

    check_specifiers(&ps, permissions.clone(), specifiers_with_mode.clone())
        .await
        .map_err(|e| (e, 0))?;

    if test_flags.no_run {
        return Ok(0);
    }

    let compat = ps.options.compat();
    let (success, artifacts) = test_specifiers(
        ps,
        permissions,
        specifiers_with_mode,
        TestSpecifierOptions {
            compat_mode: compat,
            concurrent_jobs: test_flags.concurrent_jobs,
            fail_fast: test_flags.fail_fast,
            filter: TestFilter::from_flag(&test_flags.filter),
            shuffle: test_flags.shuffle,
            trace_ops: test_flags.trace_ops,
        },
        allow_wallets,
        deployment_cache.clone(),
        stacks_chainhooks,
        mine_block_delay,
        chainhook_tx,
    )
    .await
    .map_err(|e| (e, 0))?;

    if !success {
        return Err((AnyError::msg("Test suite failed"), artifacts.len()));
    }

    if display_costs_report {
        costs::display_costs_report(&artifacts)
    }

    if let Some(ref cache) = deployment_cache {
        if let Some(filename) = coverage_report {
            let mut coverage_reporter = CoverageReporter::new();
            for (contract_id, analysis_artifacts) in cache.contracts_artifacts.iter() {
                coverage_reporter
                    .asts
                    .insert(contract_id.clone(), analysis_artifacts.ast.clone());
            }
            for (contract_id, (_, contract_location)) in cache.deployment.contracts.iter() {
                coverage_reporter
                    .contract_paths
                    .insert(contract_id.name.to_string(), contract_location.to_string());
            }
            for artifact in artifacts.iter() {
                let mut coverage_reports = artifact.coverage_reports.clone();
                coverage_reporter.reports.append(&mut coverage_reports);
            }
            coverage_reporter
                .write_lcov_file(&filename)
                .map_err(|e| (AnyError::from(e), 0))?;
        }
    }

    Ok(artifacts.len())
}

pub async fn run_tests_with_watch(
    flags: Flags,
    test_flags: TestFlags,
    allow_wallets: bool,
    deployment_cache: Option<DeploymentCache>,
    display_costs_report: bool,
    stacks_chainhooks: Vec<StacksChainhookSpecification>,
    mine_block_delay: u16,
    chainhook_tx: Option<Sender<ChainhookEvent>>,
) -> Result<(), AnyError> {
    let ps = ProcState::build(flags).await?;
    let permissions = Permissions::from_options(&ps.options.permissions_options());

    let include = test_flags.include;
    let ignore = test_flags.ignore.clone();
    let paths_to_watch: Vec<_> = include.iter().map(PathBuf::from).collect();
    let no_check = ps.options.type_check_mode() == TypeCheckMode::None;

    let resolver = |changed: Option<Vec<PathBuf>>| {
        let paths_to_watch = paths_to_watch.clone();
        let paths_to_watch_clone = paths_to_watch.clone();

        let files_changed = changed.is_some();
        let include = include.clone();
        let ignore = ignore.clone();
        let ps = ps.clone();

        async move {
            let test_modules = if test_flags.doc {
                collect_specifiers(include.clone(), &ignore, is_supported_test_ext)
            } else {
                collect_specifiers(include.clone(), &ignore, is_supported_test_path)
            }?;

            let mut paths_to_watch = paths_to_watch_clone;
            let mut modules_to_reload = if files_changed {
                Vec::new()
            } else {
                test_modules
                    .iter()
                    .map(|url| (url.clone(), ModuleKind::Esm))
                    .collect()
            };
            let graph = ps
                .create_graph(
                    test_modules
                        .iter()
                        .map(|s| (s.clone(), ModuleKind::Esm))
                        .collect(),
                )
                .await?;
            graph_valid(&graph, !no_check, ps.options.check_js())?;

            // TODO(@kitsonk) - This should be totally derivable from the graph.
            for specifier in test_modules {
                fn get_dependencies<'a>(
                    graph: &'a deno_graph::ModuleGraph,
                    maybe_module: Option<&'a deno_graph::Module>,
                    // This needs to be accessible to skip getting dependencies if they're already there,
                    // otherwise this will cause a stack overflow with circular dependencies
                    output: &mut HashSet<&'a ModuleSpecifier>,
                    no_check: bool,
                ) {
                    if let Some(module) = maybe_module {
                        for dep in module.dependencies.values() {
                            if let Some(specifier) = &dep.get_code() {
                                if !output.contains(specifier) {
                                    output.insert(specifier);
                                    get_dependencies(graph, graph.get(specifier), output, no_check);
                                }
                            }
                            if !no_check {
                                if let Some(specifier) = &dep.get_type() {
                                    if !output.contains(specifier) {
                                        output.insert(specifier);
                                        get_dependencies(
                                            graph,
                                            graph.get(specifier),
                                            output,
                                            no_check,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }

                // This test module and all it's dependencies
                let mut modules = HashSet::new();
                modules.insert(&specifier);
                get_dependencies(&graph, graph.get(&specifier), &mut modules, no_check);

                paths_to_watch.extend(
                    modules
                        .iter()
                        .filter_map(|specifier| specifier.to_file_path().ok()),
                );

                if let Some(changed) = &changed {
                    for path in changed.iter().filter_map(|path| {
                        deno_core::resolve_url_or_path(&path.to_string_lossy()).ok()
                    }) {
                        if modules.contains(&&path) {
                            modules_to_reload.push((specifier, ModuleKind::Esm));
                            break;
                        }
                    }
                }
            }

            Ok((paths_to_watch, modules_to_reload))
        }
        .map(move |result| {
            if files_changed && matches!(result, Ok((_, ref modules)) if modules.is_empty()) {
                ResolutionResult::Ignore
            } else {
                match result {
                    Ok((paths_to_watch, modules_to_reload)) => ResolutionResult::Restart {
                        paths_to_watch,
                        result: Ok(modules_to_reload),
                    },
                    Err(e) => ResolutionResult::Restart {
                        paths_to_watch,
                        result: Err(e),
                    },
                }
            }
        })
    };

    let cli_options = ps.options.clone();
    let operation = |modules_to_reload: Vec<(ModuleSpecifier, ModuleKind)>| {
        let cli_options = cli_options.clone();
        let filter = test_flags.filter.clone();
        let include = include.clone();
        let ignore = ignore.clone();
        let permissions = permissions.clone();
        let ps = ps.clone();
        let deployment_cache = deployment_cache.clone();
        let stacks_chainhooks = stacks_chainhooks.clone();
        let chainhook_tx = chainhook_tx.clone();
        async move {
            let specifiers_with_mode = fetch_specifiers_with_test_mode(
                &ps,
                include.clone(),
                ignore.clone(),
                test_flags.doc,
            )
            .await?
            .iter()
            .filter(|(specifier, _)| contains_specifier(&modules_to_reload, specifier))
            .cloned()
            .collect::<Vec<(ModuleSpecifier, TestMode)>>();

            check_specifiers(&ps, permissions.clone(), specifiers_with_mode.clone()).await?;

            if test_flags.no_run {
                return Ok(());
            }

            let (_failed, artifacts) = test_specifiers(
                ps,
                permissions.clone(),
                specifiers_with_mode,
                TestSpecifierOptions {
                    compat_mode: cli_options.compat(),
                    concurrent_jobs: test_flags.concurrent_jobs,
                    fail_fast: test_flags.fail_fast,
                    filter: TestFilter::from_flag(&filter),
                    shuffle: test_flags.shuffle,
                    trace_ops: test_flags.trace_ops,
                },
                allow_wallets,
                deployment_cache,
                stacks_chainhooks,
                mine_block_delay,
                chainhook_tx,
            )
            .await?;

            if display_costs_report {
                costs::display_costs_report(&artifacts)
            }

            Ok(())
        }
    };

    file_watcher::watch_func(
        resolver,
        operation,
        file_watcher::PrintConfig {
            job_name: "Test".to_string(),
            clear_screen: !cli_options.no_clear_screen(),
        },
    )
    .await?;

    Ok(())
}

// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

pub mod args;
pub mod auth_tokens;
pub mod cache;
pub mod cdp;
pub mod checksum;
pub mod compat;
pub mod deno_dir;
pub mod diagnostics;
pub mod display;
pub mod emit;
pub mod errors;
pub mod file_fetcher;
pub mod file_watcher;
pub mod fmt_errors;
pub mod fs_util;
pub mod graph_util;
pub mod http_cache;
pub mod http_util;
pub mod lockfile;
pub mod logger;
pub mod module_loader;
pub mod ops;
pub mod proc_state;
pub mod resolver;
pub mod text_encoding;
pub mod tools;
pub mod tsc;
pub mod unix_util;
pub mod version;
pub mod windows_util;

use fmt_errors::format_js_error;
use module_loader::CliModuleLoader;
use proc_state::ProcState;

use super::deno_runtime::colors;
use super::deno_runtime::ops::worker_host::CreateWebWorkerCb;
use super::deno_runtime::ops::worker_host::PreloadModuleCb;
use super::deno_runtime::permissions::Permissions;
use super::deno_runtime::web_worker::WebWorker;
use super::deno_runtime::web_worker::WebWorkerOptions;
use super::deno_runtime::worker::MainWorker;
use super::deno_runtime::worker::WorkerOptions;
use super::deno_runtime::BootstrapOptions;
use deno_core::error::AnyError;
use deno_core::error::JsError;
use deno_core::futures::future::LocalFutureObj;
use deno_core::located_script_name;
use deno_core::serde_json;
use deno_core::v8_set_flags;
use deno_core::Extension;
use deno_core::ModuleSpecifier;
use std::io::Write;
use std::iter::once;
use std::sync::Arc;

fn create_web_worker_preload_module_callback(ps: ProcState) -> Arc<PreloadModuleCb> {
    let compat = ps.options.compat();

    Arc::new(move |mut worker| {
        let fut = async move {
            if compat {
                worker.execute_side_module(&compat::GLOBAL_URL).await?;
                worker.execute_side_module(&compat::MODULE_URL).await?;
            }

            Ok(worker)
        };
        LocalFutureObj::new(Box::new(fut))
    })
}

fn create_web_worker_callback(
    ps: ProcState,
    stdio: super::deno_runtime::ops::io::Stdio,
) -> Arc<CreateWebWorkerCb> {
    Arc::new(move |args| {
        let maybe_inspector_server = ps.maybe_inspector_server.clone();

        let module_loader =
            CliModuleLoader::new_for_worker(ps.clone(), args.parent_permissions.clone());
        let create_web_worker_cb = create_web_worker_callback(ps.clone(), stdio.clone());
        let preload_module_cb = create_web_worker_preload_module_callback(ps.clone());

        let extensions = ops::cli_exts(ps.clone());

        let options = WebWorkerOptions {
            bootstrap: BootstrapOptions {
                args: ps.options.argv().clone(),
                cpu_count: std::thread::available_parallelism()
                    .map(|p| p.get())
                    .unwrap_or(1),
                debug_flag: ps
                    .options
                    .log_level()
                    .map_or(false, |l| l == log::Level::Debug),
                enable_testing_features: ps.options.enable_testing_features(),
                location: Some(args.main_module.clone()),
                no_color: !colors::use_color(),
                is_tty: colors::is_tty(),
                runtime_version: version::deno(),
                ts_version: version::TYPESCRIPT.to_string(),
                unstable: ps.options.unstable(),
                user_agent: version::get_user_agent(),
            },
            extensions,
            unsafely_ignore_certificate_errors: ps
                .options
                .unsafely_ignore_certificate_errors()
                .map(ToOwned::to_owned),
            root_cert_store: Some(ps.root_cert_store.clone()),
            seed: ps.options.seed(),
            create_web_worker_cb,
            preload_module_cb,
            format_js_error_fn: Some(Arc::new(format_js_error)),
            source_map_getter: Some(Box::new(module_loader.clone())),
            module_loader,
            worker_type: args.worker_type,
            maybe_inspector_server,
            get_error_class_fn: Some(&errors::get_error_class_name),
            blob_store: ps.blob_store.clone(),
            broadcast_channel: ps.broadcast_channel.clone(),
            shared_array_buffer_store: Some(ps.shared_array_buffer_store.clone()),
            compiled_wasm_module_store: Some(ps.compiled_wasm_module_store.clone()),
            stdio: stdio.clone(),
        };

        WebWorker::bootstrap_from_options(
            args.name,
            args.permissions,
            args.main_module,
            args.worker_id,
            options,
        )
    })
}

pub fn create_main_worker(
    ps: &ProcState,
    main_module: ModuleSpecifier,
    permissions: Permissions,
    mut custom_extensions: Vec<Extension>,
    stdio: super::deno_runtime::ops::io::Stdio,
) -> MainWorker {
    let module_loader = CliModuleLoader::new(ps.clone());

    let maybe_inspector_server = ps.maybe_inspector_server.clone();
    let should_break_on_first_statement = ps.options.inspect_brk().is_some();

    let create_web_worker_cb = create_web_worker_callback(ps.clone(), stdio.clone());
    let web_worker_preload_module_cb = create_web_worker_preload_module_callback(ps.clone());

    let maybe_storage_key = ps.options.resolve_storage_key(&main_module);
    let origin_storage_dir = maybe_storage_key.map(|key| {
        ps.dir
            .root
            // TODO(@crowlKats): change to origin_data for 2.0
            .join("location_data")
            .join(checksum::gen(&[key.as_bytes()]))
    });

    let mut extensions = ops::cli_exts(ps.clone());
    extensions.append(&mut custom_extensions);

    let options = WorkerOptions {
        bootstrap: BootstrapOptions {
            args: ps.options.argv().clone(),
            cpu_count: std::thread::available_parallelism()
                .map(|p| p.get())
                .unwrap_or(1),
            debug_flag: ps
                .options
                .log_level()
                .map_or(false, |l| l == log::Level::Debug),
            enable_testing_features: ps.options.enable_testing_features(),
            location: ps.options.location_flag().map(ToOwned::to_owned),
            no_color: !colors::use_color(),
            is_tty: colors::is_tty(),
            runtime_version: version::deno(),
            ts_version: version::TYPESCRIPT.to_string(),
            unstable: ps.options.unstable(),
            user_agent: version::get_user_agent(),
        },
        extensions,
        unsafely_ignore_certificate_errors: ps
            .options
            .unsafely_ignore_certificate_errors()
            .map(ToOwned::to_owned),
        root_cert_store: Some(ps.root_cert_store.clone()),
        seed: ps.options.seed(),
        source_map_getter: Some(Box::new(module_loader.clone())),
        format_js_error_fn: Some(Arc::new(format_js_error)),
        create_web_worker_cb,
        web_worker_preload_module_cb,
        maybe_inspector_server,
        should_break_on_first_statement,
        module_loader,
        get_error_class_fn: Some(&errors::get_error_class_name),
        origin_storage_dir,
        blob_store: ps.blob_store.clone(),
        broadcast_channel: ps.broadcast_channel.clone(),
        shared_array_buffer_store: Some(ps.shared_array_buffer_store.clone()),
        compiled_wasm_module_store: Some(ps.compiled_wasm_module_store.clone()),
        stdio,
    };

    MainWorker::bootstrap_from_options(main_module, permissions, options)
}

pub fn write_to_stdout_ignore_sigpipe(bytes: &[u8]) -> Result<(), std::io::Error> {
    use std::io::ErrorKind;

    match std::io::stdout().write_all(bytes) {
        Ok(()) => Ok(()),
        Err(e) => match e.kind() {
            ErrorKind::BrokenPipe => Ok(()),
            _ => Err(e),
        },
    }
}

pub fn write_json_to_stdout<T>(value: &T) -> Result<(), AnyError>
where
    T: ?Sized + serde::ser::Serialize,
{
    let mut writer = std::io::BufWriter::new(std::io::stdout());
    serde_json::to_writer_pretty(&mut writer, value)?;
    writeln!(&mut writer)?;
    Ok(())
}

fn init_v8_flags(v8_flags: &[String]) {
    let v8_flags_includes_help = v8_flags
        .iter()
        .any(|flag| flag == "-help" || flag == "--help");
    // Keep in sync with `standalone.rs`.
    let v8_flags = once("UNUSED_BUT_NECESSARY_ARG0".to_owned())
        .chain(v8_flags.iter().cloned())
        .collect::<Vec<_>>();
    let unrecognized_v8_flags = v8_set_flags(v8_flags)
        .into_iter()
        .skip(1)
        .collect::<Vec<_>>();
    if !unrecognized_v8_flags.is_empty() {
        for f in unrecognized_v8_flags {
            eprintln!("error: V8 did not recognize flag '{}'", f);
        }
        eprintln!("\nFor a list of V8 flags, use '--v8-flags=--help'");
        std::process::exit(1);
    }
    if v8_flags_includes_help {
        std::process::exit(0);
    }
}


fn unwrap_or_exit<T>(result: Result<T, AnyError>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => {
            let error_string = match error.downcast_ref::<JsError>() {
                Some(e) => format_js_error(e),
                None => format!("{:?}", error),
            };
            eprintln!(
                "{}: {}",
                colors::red_bold("error"),
                error_string.trim_start_matches("error: ")
            );
            std::process::exit(1);
        }
    }
}

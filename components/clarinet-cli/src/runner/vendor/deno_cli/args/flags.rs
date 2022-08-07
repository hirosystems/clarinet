// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

use clap::Arg;
use clap::ArgMatches;
use clap::ColorChoice;
use clap::Command;
use clap::ValueHint;
use deno_core::serde::Deserialize;
use deno_core::serde::Serialize;
use deno_core::url::Url;
use super::super::super::deno_runtime::permissions::PermissionsOptions;
use log::debug;
use log::Level;
use once_cell::sync::Lazy;
use std::env;
use std::net::SocketAddr;
use std::num::NonZeroU32;
use std::num::NonZeroU8;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::str::FromStr;
use super::super::version;

use super::flags_allow_net;

static LONG_VERSION: Lazy<String> = Lazy::new(|| {
  format!(
    "{} ({}, {})\nv8 {}\ntypescript {}",
    version::deno(),
    if version::is_canary() {
      "canary"
    } else {
      env!("PROFILE")
    },
    env!("TARGET"),
    deno_core::v8_version(),
    version::TYPESCRIPT
  )
});

static SHORT_VERSION: Lazy<String> = Lazy::new(|| {
  super::super::version::deno()
    .split('+')
    .next()
    .unwrap()
    .to_string()
});

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct BenchFlags {
  pub ignore: Vec<PathBuf>,
  pub include: Option<Vec<String>>,
  pub filter: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct BundleFlags {
  pub source_file: String,
  pub out_file: Option<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct CacheFlags {
  pub files: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct CheckFlags {
  pub files: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct CompileFlags {
  pub source_file: String,
  pub output: Option<PathBuf>,
  pub args: Vec<String>,
  pub target: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct CompletionsFlags {
  pub buf: Box<[u8]>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct CoverageFlags {
  pub files: Vec<PathBuf>,
  pub output: Option<PathBuf>,
  pub ignore: Vec<PathBuf>,
  pub include: Vec<String>,
  pub exclude: Vec<String>,
  pub lcov: bool,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct DocFlags {
  pub private: bool,
  pub json: bool,
  pub source_file: Option<String>,
  pub filter: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct EvalFlags {
  pub print: bool,
  pub code: String,
  pub ext: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct FmtFlags {
  pub check: bool,
  pub files: Vec<PathBuf>,
  pub ignore: Vec<PathBuf>,
  pub ext: String,
  pub use_tabs: Option<bool>,
  pub line_width: Option<NonZeroU32>,
  pub indent_width: Option<NonZeroU8>,
  pub single_quote: Option<bool>,
  pub prose_wrap: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct InfoFlags {
  pub json: bool,
  pub file: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct InstallFlags {
  pub module_url: String,
  pub args: Vec<String>,
  pub name: Option<String>,
  pub root: Option<PathBuf>,
  pub force: bool,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct UninstallFlags {
  pub name: String,
  pub root: Option<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct LintFlags {
  pub files: Vec<PathBuf>,
  pub ignore: Vec<PathBuf>,
  pub rules: bool,
  pub maybe_rules_tags: Option<Vec<String>>,
  pub maybe_rules_include: Option<Vec<String>>,
  pub maybe_rules_exclude: Option<Vec<String>>,
  pub json: bool,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct ReplFlags {
  pub eval_files: Option<Vec<String>>,
  pub eval: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct RunFlags {
  pub script: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct TaskFlags {
  pub cwd: Option<String>,
  pub task: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct TestFlags {
  pub ignore: Vec<PathBuf>,
  pub doc: bool,
  pub no_run: bool,
  pub fail_fast: Option<NonZeroUsize>,
  pub allow_none: bool,
  pub include: Vec<String>,
  pub filter: Option<String>,
  pub shuffle: Option<u64>,
  pub concurrent_jobs: NonZeroUsize,
  pub trace_ops: bool,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct UpgradeFlags {
  pub dry_run: bool,
  pub force: bool,
  pub canary: bool,
  pub version: Option<String>,
  pub output: Option<PathBuf>,
  pub ca_file: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct VendorFlags {
  pub specifiers: Vec<String>,
  pub output_path: Option<PathBuf>,
  pub force: bool,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum DenoSubcommand {
  Bench(BenchFlags),
  Bundle(BundleFlags),
  Cache(CacheFlags),
  Check(CheckFlags),
  Compile(CompileFlags),
  Completions(CompletionsFlags),
  Coverage(CoverageFlags),
  Doc(DocFlags),
  Eval(EvalFlags),
  Fmt(FmtFlags),
  Info(InfoFlags),
  Install(InstallFlags),
  Uninstall(UninstallFlags),
  Lsp,
  Lint(LintFlags),
  Repl(ReplFlags),
  Run(RunFlags),
  Task(TaskFlags),
  Test(TestFlags),
  Types,
  Upgrade(UpgradeFlags),
  Vendor(VendorFlags),
}

impl Default for DenoSubcommand {
  fn default() -> DenoSubcommand {
    DenoSubcommand::Repl(ReplFlags {
      eval_files: None,
      eval: None,
    })
  }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TypeCheckMode {
  /// Type-check all modules.
  All,
  /// Skip type-checking of all modules. The default value for "deno run" and
  /// several other subcommands.
  None,
  /// Only type-check local modules. The default value for "deno test" and
  /// several other subcommands.
  Local,
}

impl Default for TypeCheckMode {
  fn default() -> Self {
    Self::None
  }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ConfigFlag {
  Discover,
  Path(String),
  Disabled,
}

impl Default for ConfigFlag {
  fn default() -> Self {
    Self::Discover
  }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Flags {
  /// Vector of CLI arguments - these are user script arguments, all Deno
  /// specific flags are removed.
  pub argv: Vec<String>,
  pub subcommand: DenoSubcommand,

  pub allow_all: bool,
  pub allow_env: Option<Vec<String>>,
  pub allow_hrtime: bool,
  pub allow_net: Option<Vec<String>>,
  pub allow_ffi: Option<Vec<PathBuf>>,
  pub allow_read: Option<Vec<PathBuf>>,
  pub allow_run: Option<Vec<String>>,
  pub allow_write: Option<Vec<PathBuf>>,
  pub ca_stores: Option<Vec<String>>,
  pub ca_file: Option<String>,
  pub cache_blocklist: Vec<String>,
  /// This is not exposed as an option in the CLI, it is used internally when
  /// the language server is configured with an explicit cache option.
  pub cache_path: Option<PathBuf>,
  pub cached_only: bool,
  pub type_check_mode: TypeCheckMode,
  pub config_flag: ConfigFlag,
  pub coverage_dir: Option<String>,
  pub enable_testing_features: bool,
  pub ignore: Vec<PathBuf>,
  pub import_map_path: Option<String>,
  pub inspect_brk: Option<SocketAddr>,
  pub inspect: Option<SocketAddr>,
  pub location: Option<Url>,
  pub lock_write: bool,
  pub lock: Option<PathBuf>,
  pub log_level: Option<Level>,
  pub no_remote: bool,
  /// If true, a list of Node built-in modules will be injected into
  /// the import map.
  pub compat: bool,
  pub no_prompt: bool,
  pub reload: bool,
  pub seed: Option<u64>,
  pub unstable: bool,
  pub unsafely_ignore_certificate_errors: Option<Vec<String>>,
  pub v8_flags: Vec<String>,
  pub version: bool,
  pub watch: Option<Vec<PathBuf>>,
  pub no_clear_screen: bool,
}

fn join_paths(allowlist: &[PathBuf], d: &str) -> String {
  allowlist
    .iter()
    .map(|path| path.to_str().unwrap().to_string())
    .collect::<Vec<String>>()
    .join(d)
}

impl Flags {
  /// Return list of permission arguments that are equivalent
  /// to the ones used to create `self`.
  pub fn to_permission_args(&self) -> Vec<String> {
    let mut args = vec![];

    if self.allow_all {
      args.push("--allow-all".to_string());
      return args;
    }

    match &self.allow_read {
      Some(read_allowlist) if read_allowlist.is_empty() => {
        args.push("--allow-read".to_string());
      }
      Some(read_allowlist) => {
        let s = format!("--allow-read={}", join_paths(read_allowlist, ","));
        args.push(s);
      }
      _ => {}
    }

    match &self.allow_write {
      Some(write_allowlist) if write_allowlist.is_empty() => {
        args.push("--allow-write".to_string());
      }
      Some(write_allowlist) => {
        let s = format!("--allow-write={}", join_paths(write_allowlist, ","));
        args.push(s);
      }
      _ => {}
    }

    match &self.allow_net {
      Some(net_allowlist) if net_allowlist.is_empty() => {
        args.push("--allow-net".to_string());
      }
      Some(net_allowlist) => {
        let s = format!("--allow-net={}", net_allowlist.join(","));
        args.push(s);
      }
      _ => {}
    }

    match &self.unsafely_ignore_certificate_errors {
      Some(ic_allowlist) if ic_allowlist.is_empty() => {
        args.push("--unsafely-ignore-certificate-errors".to_string());
      }
      Some(ic_allowlist) => {
        let s = format!(
          "--unsafely-ignore-certificate-errors={}",
          ic_allowlist.join(",")
        );
        args.push(s);
      }
      _ => {}
    }

    match &self.allow_env {
      Some(env_allowlist) if env_allowlist.is_empty() => {
        args.push("--allow-env".to_string());
      }
      Some(env_allowlist) => {
        let s = format!("--allow-env={}", env_allowlist.join(","));
        args.push(s);
      }
      _ => {}
    }

    match &self.allow_run {
      Some(run_allowlist) if run_allowlist.is_empty() => {
        args.push("--allow-run".to_string());
      }
      Some(run_allowlist) => {
        let s = format!("--allow-run={}", run_allowlist.join(","));
        args.push(s);
      }
      _ => {}
    }

    match &self.allow_ffi {
      Some(ffi_allowlist) if ffi_allowlist.is_empty() => {
        args.push("--allow-ffi".to_string());
      }
      Some(ffi_allowlist) => {
        let s = format!("--allow-ffi={}", join_paths(ffi_allowlist, ","));
        args.push(s);
      }
      _ => {}
    }

    if self.allow_hrtime {
      args.push("--allow-hrtime".to_string());
    }

    args
  }

  /// Extract path arguments for config search paths.
  /// If it returns Some(vec), the config should be discovered
  /// from the current dir after trying to discover from each entry in vec.
  /// If it returns None, the config file shouldn't be discovered at all.
  pub fn config_path_args(&self) -> Option<Vec<PathBuf>> {
    use DenoSubcommand::*;
    if let Fmt(FmtFlags { files, .. }) = &self.subcommand {
      Some(files.clone())
    } else if let Lint(LintFlags { files, .. }) = &self.subcommand {
      Some(files.clone())
    } else if let Run(RunFlags { script }) = &self.subcommand {
      if let Ok(module_specifier) = deno_core::resolve_url_or_path(script) {
        if module_specifier.scheme() == "file" {
          if let Ok(p) = module_specifier.to_file_path() {
            Some(vec![p])
          } else {
            Some(vec![])
          }
        } else {
          // When the entrypoint doesn't have file: scheme (it's the remote
          // script), then we don't auto discover config file.
          None
        }
      } else {
        Some(vec![])
      }
    } else {
      Some(vec![])
    }
  }

  pub fn permissions_options(&self) -> PermissionsOptions {
    PermissionsOptions {
      allow_env: self.allow_env.clone(),
      allow_hrtime: self.allow_hrtime,
      allow_net: self.allow_net.clone(),
      allow_ffi: self.allow_ffi.clone(),
      allow_read: self.allow_read.clone(),
      allow_run: self.allow_run.clone(),
      allow_write: self.allow_write.clone(),
      prompt: !self.no_prompt,
    }
  }
}

static ENV_VARIABLES_HELP: &str = r#"ENVIRONMENT VARIABLES:
    DENO_AUTH_TOKENS     A semi-colon separated list of bearer tokens and
                         hostnames to use when fetching remote modules from
                         private repositories
                         (e.g. "abcde12345@deno.land;54321edcba@github.com")
    DENO_TLS_CA_STORE    Comma-separated list of order dependent certificate
                         stores. Possible values: "system", "mozilla".
                         Defaults to "mozilla".
    DENO_CERT            Load certificate authority from PEM encoded file
    DENO_DIR             Set the cache directory
    DENO_INSTALL_ROOT    Set deno install's output directory
                         (defaults to $HOME/.deno/bin)
    DENO_NO_PROMPT       Set to disable permission prompts on access
                         (alternative to passing --no-prompt on invocation)
    DENO_WEBGPU_TRACE    Directory to use for wgpu traces
    DENO_JOBS            Number of parallel workers used for the --parallel
                         flag with the test subcommand. Defaults to number
                         of available CPUs.
    HTTP_PROXY           Proxy address for HTTP requests
                         (module downloads, fetch)
    HTTPS_PROXY          Proxy address for HTTPS requests
                         (module downloads, fetch)
    NO_COLOR             Set to disable color
    NO_PROXY             Comma-separated list of hosts which do not use a proxy
                         (module downloads, fetch)"#;

static DENO_HELP: Lazy<String> = Lazy::new(|| {
  format!(
    "A modern JavaScript and TypeScript runtime

Docs: https://deno.land/manual@v{}
Modules: https://deno.land/std/ https://deno.land/x/
Bugs: https://github.com/denoland/deno/issues

To start the REPL:

  deno

To execute a script:

  deno run https://deno.land/std/examples/welcome.ts

To evaluate code in the shell:

  deno eval \"console.log(30933 + 404)\"
",
    SHORT_VERSION.as_str()
  )
});

/// Main entry point for parsing deno's command line flags.
pub fn flags_from_vec(args: Vec<String>) -> clap::Result<Flags> {
  let version = version::deno();
  let app = clap_root(&version);
  let matches = app.clone().try_get_matches_from(&args)?;

  let mut flags = Flags::default();

  if matches.is_present("unstable") {
    flags.unstable = true;
  }
  if matches.is_present("log-level") {
    flags.log_level = match matches.value_of("log-level").unwrap() {
      "debug" => Some(Level::Debug),
      "info" => Some(Level::Info),
      _ => unreachable!(),
    };
  }
  if matches.is_present("quiet") {
    flags.log_level = Some(Level::Error);
  }

  match matches.subcommand() {
    Some(("bench", m)) => bench_parse(&mut flags, m),
    Some(("bundle", m)) => bundle_parse(&mut flags, m),
    Some(("cache", m)) => cache_parse(&mut flags, m),
    Some(("check", m)) => check_parse(&mut flags, m),
    Some(("compile", m)) => compile_parse(&mut flags, m),
    Some(("coverage", m)) => coverage_parse(&mut flags, m),
    Some(("doc", m)) => doc_parse(&mut flags, m),
    Some(("eval", m)) => eval_parse(&mut flags, m),
    Some(("fmt", m)) => fmt_parse(&mut flags, m),
    Some(("info", m)) => info_parse(&mut flags, m),
    Some(("install", m)) => install_parse(&mut flags, m),
    Some(("lint", m)) => lint_parse(&mut flags, m),
    Some(("lsp", m)) => lsp_parse(&mut flags, m),
    Some(("repl", m)) => repl_parse(&mut flags, m),
    Some(("run", m)) => run_parse(&mut flags, m),
    Some(("task", m)) => task_parse(&mut flags, m, &args),
    Some(("test", m)) => test_parse(&mut flags, m),
    Some(("types", m)) => types_parse(&mut flags, m),
    Some(("uninstall", m)) => uninstall_parse(&mut flags, m),
    Some(("upgrade", m)) => upgrade_parse(&mut flags, m),
    Some(("vendor", m)) => vendor_parse(&mut flags, m),
    _ => handle_repl_flags(
      &mut flags,
      ReplFlags {
        eval_files: None,
        eval: None,
      },
    ),
  }

  Ok(flags)
}

fn handle_repl_flags(flags: &mut Flags, repl_flags: ReplFlags) {
  flags.subcommand = DenoSubcommand::Repl(repl_flags);
  flags.allow_net = Some(vec![]);
  flags.allow_env = Some(vec![]);
  flags.allow_run = Some(vec![]);
  flags.allow_read = Some(vec![]);
  flags.allow_write = Some(vec![]);
  flags.allow_ffi = Some(vec![]);
  flags.allow_hrtime = true;
}

fn clap_root(version: &str) -> Command {
  clap::Command::new("deno")
    .bin_name("deno")
    .color(ColorChoice::Never)
    .max_term_width(80)
    .version(version)
    .long_version(LONG_VERSION.as_str())
    .arg(
      Arg::new("unstable")
        .long("unstable")
        .help("Enable unstable features and APIs")
        .global(true),
    )
    .arg(
      Arg::new("log-level")
        .short('L')
        .long("log-level")
        .help("Set log level")
        .hide(true)
        .takes_value(true)
        .possible_values(&["debug", "info"])
        .global(true),
    )
    .arg(
      Arg::new("quiet")
        .short('q')
        .long("quiet")
        .help("Suppress diagnostic output")
        .global(true),
    )
    .subcommand(bench_subcommand())
    .subcommand(bundle_subcommand())
    .subcommand(cache_subcommand())
    .subcommand(check_subcommand())
    .subcommand(compile_subcommand())
    .subcommand(completions_subcommand())
    .subcommand(coverage_subcommand())
    .subcommand(doc_subcommand())
    .subcommand(eval_subcommand())
    .subcommand(fmt_subcommand())
    .subcommand(info_subcommand())
    .subcommand(install_subcommand())
    .subcommand(uninstall_subcommand())
    .subcommand(lsp_subcommand())
    .subcommand(lint_subcommand())
    .subcommand(repl_subcommand())
    .subcommand(run_subcommand())
    .subcommand(task_subcommand())
    .subcommand(test_subcommand())
    .subcommand(types_subcommand())
    .subcommand(upgrade_subcommand())
    .subcommand(vendor_subcommand())
    .long_about(DENO_HELP.as_str())
    .after_help(ENV_VARIABLES_HELP)
}

fn bench_subcommand<'a>() -> Command<'a> {
  runtime_args(Command::new("bench"), true, false)
    .trailing_var_arg(true)
    .arg(
      Arg::new("ignore")
        .long("ignore")
        .takes_value(true)
        .use_value_delimiter(true)
        .require_equals(true)
        .help("Ignore files"),
    )
    .arg(
      Arg::new("filter")
        .allow_hyphen_values(true)
        .long("filter")
        .takes_value(true)
        .help("Run benchmarks with this string or pattern in the bench name"),
    )
    .arg(
      Arg::new("files")
        .help("List of file names to run")
        .takes_value(true)
        .multiple_values(true)
        .multiple_occurrences(true),
    )
    .arg(watch_arg(false))
    .arg(no_clear_screen_arg())
    .arg(script_arg().last(true))
    .about("Run benchmarks")
    .long_about(
      "Run benchmarks using Deno's built-in bench tool.

Evaluate the given modules, run all benches declared with 'Deno.bench()' \
and report results to standard output:

  deno bench src/fetch_bench.ts src/signal_bench.ts

Directory arguments are expanded to all contained files matching the \
glob {*_,*.,}bench.{js,mjs,ts,jsx,tsx}:

  deno bench src/",
    )
}

fn bundle_subcommand<'a>() -> Command<'a> {
  compile_args(Command::new("bundle"))
    .arg(
      Arg::new("source_file")
        .takes_value(true)
        .required(true)
        .value_hint(ValueHint::FilePath),
    )
    .arg(
      Arg::new("out_file")
        .takes_value(true)
        .required(false)
        .value_hint(ValueHint::FilePath),
    )
    .arg(watch_arg(false))
    .arg(no_clear_screen_arg())
    .about("Bundle module and dependencies into single file")
    .long_about(
      "Output a single JavaScript file with all dependencies.

  deno bundle https://deno.land/std/examples/colors.ts colors.bundle.js

If no output file is given, the output is written to standard output:

  deno bundle https://deno.land/std/examples/colors.ts",
    )
}

fn cache_subcommand<'a>() -> Command<'a> {
  compile_args(Command::new("cache"))
    .arg(
      Arg::new("file")
        .takes_value(true)
        .required(true)
        .min_values(1)
        .value_hint(ValueHint::FilePath),
    )
    .about("Cache the dependencies")
    .long_about(
      "Cache and compile remote dependencies recursively.

Download and compile a module with all of its static dependencies and save \
them in the local cache, without running any code:

  deno cache https://deno.land/std/http/file_server.ts

Future runs of this module will trigger no downloads or compilation unless \
--reload is specified.",
    )
}

fn check_subcommand<'a>() -> Command<'a> {
  compile_args_without_check_args(Command::new("check"))
  .arg(
    Arg::new("remote")
      .long("remote")
      .help("Type-check all modules, including remote")
    )
    .arg(
      Arg::new("file")
        .takes_value(true)
        .required(true)
        .min_values(1)
        .value_hint(ValueHint::FilePath),
    )
    .about("Type-check the dependencies")
    .long_about(
      "Download and type-check without execution.

  deno check https://deno.land/std/http/file_server.ts

Unless --reload is specified, this command will not re-download already cached dependencies.",
    )
}

fn compile_subcommand<'a>() -> Command<'a> {
  runtime_args(Command::new("compile"), true, false)
    .trailing_var_arg(true)
    .arg(script_arg().required(true))
    .arg(
      Arg::new("output")
        .long("output")
        .short('o')
        .help("Output file (defaults to $PWD/<inferred-name>)")
        .takes_value(true)
        .value_hint(ValueHint::FilePath),
    )
    .arg(
      Arg::new("target")
        .long("target")
        .help("Target OS architecture")
        .takes_value(true)
        .possible_values(&[
          "x86_64-unknown-linux-gnu",
          "x86_64-pc-windows-msvc",
          "x86_64-apple-darwin",
          "aarch64-apple-darwin",
        ]),
    )
    .about("UNSTABLE: Compile the script into a self contained executable")
    .long_about(
      "UNSTABLE: Compiles the given script into a self contained executable.

  deno compile -A https://deno.land/std/http/file_server.ts
  deno compile --output color_util https://deno.land/std/examples/colors.ts

Any flags passed which affect runtime behavior, such as '--unstable', \
'--allow-*', '--v8-flags', etc. are encoded into the output executable and \
used at runtime as if they were passed to a similar 'deno run' command.

The executable name is inferred by default: Attempt to take the file stem of \
the URL path. The above example would become 'file_server'. If the file stem \
is something generic like 'main', 'mod', 'index' or 'cli', and the path has no \
parent, take the file name of the parent path. Otherwise settle with the \
generic name. If the resulting name has an '@...' suffix, strip it.

Cross-compiling to different target architectures is supported using the \
`--target` flag. On the first invocation with deno will download proper \
binary and cache it in $DENO_DIR. The aarch64-apple-darwin target is not \
supported in canary.
",
    )
}

fn completions_subcommand<'a>() -> Command<'a> {
  Command::new("completions")
    .disable_help_subcommand(true)
    .arg(
      Arg::new("shell")
        .possible_values(&["bash", "fish", "powershell", "zsh", "fig"])
        .required(true),
    )
    .about("Generate shell completions")
    .long_about(
      "Output shell completion script to standard output.

  deno completions bash > /usr/local/etc/bash_completion.d/deno.bash
  source /usr/local/etc/bash_completion.d/deno.bash",
    )
}

fn coverage_subcommand<'a>() -> Command<'a> {
  Command::new("coverage")
    .about("Print coverage reports")
    .long_about(
      "Print coverage reports from coverage profiles.

Collect a coverage profile with deno test:

  deno test --coverage=cov_profile

Print a report to stdout:

  deno coverage cov_profile

Include urls that start with the file schema:

  deno coverage --include=\"^file:\" cov_profile

Exclude urls ending with test.ts and test.js:

  deno coverage --exclude=\"test\\.(ts|js)\" cov_profile

Include urls that start with the file schema and exclude files ending with \
test.ts and test.js, for an url to match it must match the include pattern and \
not match the exclude pattern:

  deno coverage --include=\"^file:\" --exclude=\"test\\.(ts|js)\" cov_profile

Write a report using the lcov format:

  deno coverage --lcov --output=cov.lcov cov_profile/

Generate html reports from lcov:

  genhtml -o html_cov cov.lcov
",
    )
    .arg(
      Arg::new("ignore")
        .long("ignore")
        .takes_value(true)
        .use_value_delimiter(true)
        .require_equals(true)
        .help("Ignore coverage files")
        .value_hint(ValueHint::AnyPath),
    )
    .arg(
      Arg::new("include")
        .long("include")
        .takes_value(true)
        .value_name("regex")
        .multiple_values(true)
        .multiple_occurrences(true)
        .require_equals(true)
        .default_value(r"^file:")
        .help("Include source files in the report"),
    )
    .arg(
      Arg::new("exclude")
        .long("exclude")
        .takes_value(true)
        .value_name("regex")
        .multiple_values(true)
        .multiple_occurrences(true)
        .require_equals(true)
        .default_value(r"test\.(js|mjs|ts|jsx|tsx)$")
        .help("Exclude source files from the report"),
    )
    .arg(
      Arg::new("lcov")
        .long("lcov")
        .help("Output coverage report in lcov format")
        .takes_value(false),
    )
    .arg(
      Arg::new("output")
        .requires("lcov")
        .long("output")
        .help("Output file (defaults to stdout) for lcov")
        .long_help(
          "Exports the coverage report in lcov format to the given file. \
    Filename should be passed along with '=' For example '--output=foo.lcov' \
    If no --output arg is specified then the report is written to stdout.",
        )
        .takes_value(true)
        .require_equals(true)
        .value_hint(ValueHint::FilePath),
    )
    .arg(
      Arg::new("files")
        .takes_value(true)
        .multiple_values(true)
        .multiple_occurrences(true)
        .required(true)
        .value_hint(ValueHint::AnyPath),
    )
}

fn doc_subcommand<'a>() -> Command<'a> {
  Command::new("doc")
    .about("Show documentation for a module")
    .long_about(
      "Show documentation for a module.

Output documentation to standard output:

    deno doc ./path/to/module.ts

Output private documentation to standard output:

    deno doc --private ./path/to/module.ts

Output documentation in JSON format:

    deno doc --json ./path/to/module.ts

Target a specific symbol:

    deno doc ./path/to/module.ts MyClass.someField

Show documentation for runtime built-ins:

    deno doc
    deno doc --builtin Deno.Listener",
    )
    .arg(import_map_arg())
    .arg(reload_arg())
    .arg(
      Arg::new("json")
        .long("json")
        .help("Output documentation in JSON format")
        .takes_value(false),
    )
    .arg(
      Arg::new("private")
        .long("private")
        .help("Output private documentation")
        .takes_value(false),
    )
    // TODO(nayeemrmn): Make `--builtin` a proper option. Blocked by
    // https://github.com/clap-rs/clap/issues/1794. Currently `--builtin` is
    // just a possible value of `source_file` so leading hyphens must be
    // enabled.
    .allow_hyphen_values(true)
    .arg(
      Arg::new("source_file")
        .takes_value(true)
        .value_hint(ValueHint::FilePath),
    )
    .arg(
      Arg::new("filter")
        .help("Dot separated path to symbol")
        .takes_value(true)
        .required(false)
        .conflicts_with("json"),
    )
}

fn eval_subcommand<'a>() -> Command<'a> {
  runtime_args(Command::new("eval"), false, true)
    .about("Eval script")
    .long_about(
      "Evaluate JavaScript from the command line.

  deno eval \"console.log('hello world')\"

To evaluate as TypeScript:

  deno eval --ext=ts \"const v: string = 'hello'; console.log(v)\"

This command has implicit access to all permissions (--allow-all).",
    )
    .arg(
      // TODO(@satyarohith): remove this argument in 2.0.
      Arg::new("ts")
        .long("ts")
        .short('T')
        .help("Treat eval input as TypeScript")
        .takes_value(false)
        .multiple_occurrences(false)
        .multiple_values(false)
        .hide(true),
    )
    .arg(
      Arg::new("ext")
        .long("ext")
        .help("Set standard input (stdin) content type")
        .takes_value(true)
        .default_value("js")
        .possible_values(&["ts", "tsx", "js", "jsx"]),
    )
    .arg(
      Arg::new("print")
        .long("print")
        .short('p')
        .help("print result to stdout")
        .takes_value(false)
        .multiple_occurrences(false)
        .multiple_values(false),
    )
    .arg(
      Arg::new("code_arg")
        .multiple_values(true)
        .multiple_occurrences(true)
        .help("Code arg")
        .value_name("CODE_ARG")
        .required(true),
    )
}

fn fmt_subcommand<'a>() -> Command<'a> {
  Command::new("fmt")
    .about("Format source files")
    .long_about(
      "Auto-format JavaScript, TypeScript, Markdown, and JSON files.

  deno fmt
  deno fmt myfile1.ts myfile2.ts
  deno fmt --check

Format stdin and write to stdout:

  cat file.ts | deno fmt -

Ignore formatting code by preceding it with an ignore comment:

  // deno-fmt-ignore

Ignore formatting a file by adding an ignore comment at the top of the file:

  // deno-fmt-ignore-file",
    )
    .arg(config_arg())
    .arg(no_config_arg())
    .arg(
      Arg::new("check")
        .long("check")
        .help("Check if the source files are formatted")
        .takes_value(false),
    )
    .arg(
      Arg::new("ext")
        .long("ext")
        .help("Set standard input (stdin) content type")
        .takes_value(true)
        .default_value("ts")
        .possible_values(&["ts", "tsx", "js", "jsx", "md", "json", "jsonc"]),
    )
    .arg(
      Arg::new("ignore")
        .long("ignore")
        .takes_value(true)
        .use_value_delimiter(true)
        .require_equals(true)
        .help("Ignore formatting particular source files")
        .value_hint(ValueHint::AnyPath),
    )
    .arg(
      Arg::new("files")
        .takes_value(true)
        .multiple_values(true)
        .multiple_occurrences(true)
        .required(false)
        .value_hint(ValueHint::AnyPath),
    )
    .arg(watch_arg(false))
    .arg(no_clear_screen_arg())
    .arg(
      Arg::new("options-use-tabs")
        .long("options-use-tabs")
        .help("Use tabs instead of spaces for indentation. Defaults to false."),
    )
    .arg(
      Arg::new("options-line-width")
        .long("options-line-width")
        .help("Define maximum line width. Defaults to 80.")
        .takes_value(true)
        .validator(|val: &str| match val.parse::<NonZeroUsize>() {
          Ok(_) => Ok(()),
          Err(_) => {
            Err("options-line-width should be a non zero integer".to_string())
          }
        }),
    )
    .arg(
      Arg::new("options-indent-width")
        .long("options-indent-width")
        .help("Define indentation width. Defaults to 2.")
        .takes_value(true)
        .validator(|val: &str| match val.parse::<NonZeroUsize>() {
          Ok(_) => Ok(()),
          Err(_) => {
            Err("options-indent-width should be a non zero integer".to_string())
          }
        }),
    )
    .arg(
      Arg::new("options-single-quote")
        .long("options-single-quote")
        .help("Use single quotes. Defaults to false."),
    )
    .arg(
      Arg::new("options-prose-wrap")
        .long("options-prose-wrap")
        .takes_value(true)
        .possible_values(&["always", "never", "preserve"])
        .help("Define how prose should be wrapped. Defaults to always."),
    )
}

fn info_subcommand<'a>() -> Command<'a> {
  Command::new("info")
    .about("Show info about cache or info related to source file")
    .long_about(
      "Information about a module or the cache directories.

Get information about a module:

  deno info https://deno.land/std/http/file_server.ts

The following information is shown:

local: Local path of the file.
type: JavaScript, TypeScript, or JSON.
emit: Local path of compiled source code. (TypeScript only.)
dependencies: Dependency tree of the source file.

Without any additional arguments, 'deno info' shows:

DENO_DIR: Directory containing Deno-managed files.
Remote modules cache: Subdirectory containing downloaded remote modules.
TypeScript compiler cache: Subdirectory containing TS compiler output.",
    )
    .arg(Arg::new("file").takes_value(true).required(false).value_hint(ValueHint::FilePath))
    .arg(reload_arg().requires("file"))
    .arg(ca_file_arg())
    .arg(
      location_arg()
        .conflicts_with("file")
        .help("Show files used for origin bound APIs like the Web Storage API when running a script with '--location=<HREF>'")
    )
    // TODO(lucacasonato): remove for 2.0
    .arg(no_check_arg().hide(true))
    .arg(no_config_arg())
    .arg(config_arg())
    .arg(import_map_arg())
    .arg(
      Arg::new("json")
        .long("json")
        .help("UNSTABLE: Outputs the information in JSON format")
        .takes_value(false),
    )
}

fn install_subcommand<'a>() -> Command<'a> {
  runtime_args(Command::new("install"), true, true)
    .trailing_var_arg(true)
    .arg(Arg::new("cmd").required(true).multiple_values(true).value_hint(ValueHint::FilePath))
    .arg(
      Arg::new("name")
        .long("name")
        .short('n')
        .help("Executable file name")
        .takes_value(true)
        .required(false))
    .arg(
      Arg::new("root")
        .long("root")
        .help("Installation root")
        .takes_value(true)
        .multiple_occurrences(false)
        .multiple_values(false)
        .value_hint(ValueHint::DirPath))
    .arg(
      Arg::new("force")
        .long("force")
        .short('f')
        .help("Forcefully overwrite existing installation")
        .takes_value(false))
    .about("Install script as an executable")
    .long_about(
      "Installs a script as an executable in the installation root's bin directory.

  deno install --allow-net --allow-read https://deno.land/std/http/file_server.ts
  deno install https://deno.land/std/examples/colors.ts

To change the executable name, use -n/--name:

  deno install --allow-net --allow-read -n serve https://deno.land/std/http/file_server.ts

The executable name is inferred by default:
  - Attempt to take the file stem of the URL path. The above example would
    become 'file_server'.
  - If the file stem is something generic like 'main', 'mod', 'index' or 'cli',
    and the path has no parent, take the file name of the parent path. Otherwise
    settle with the generic name.
  - If the resulting name has an '@...' suffix, strip it.

To change the installation root, use --root:

  deno install --allow-net --allow-read --root /usr/local https://deno.land/std/http/file_server.ts

The installation root is determined, in order of precedence:
  - --root option
  - DENO_INSTALL_ROOT environment variable
  - $HOME/.deno

These must be added to the path manually if required.")
}

fn uninstall_subcommand<'a>() -> Command<'a> {
  Command::new("uninstall")
    .trailing_var_arg(true)
    .arg(
      Arg::new("name")
        .required(true)
        .multiple_occurrences(false)
        .allow_hyphen_values(true))
    .arg(
      Arg::new("root")
        .long("root")
        .help("Installation root")
        .takes_value(true)
        .multiple_occurrences(false)
        .value_hint(ValueHint::DirPath))
    .about("Uninstall a script previously installed with deno install")
    .long_about(
      "Uninstalls an executable script in the installation root's bin directory.

  deno uninstall serve

To change the installation root, use --root:

  deno uninstall --root /usr/local serve

The installation root is determined, in order of precedence:
  - --root option
  - DENO_INSTALL_ROOT environment variable
  - $HOME/.deno")
}

static LSP_HELP: Lazy<String> = Lazy::new(|| {
  format!(
    "The 'deno lsp' subcommand provides a way for code editors and IDEs to
interact with Deno using the Language Server Protocol. Usually humans do not
use this subcommand directly. For example, 'deno lsp' can provide IDEs with
go-to-definition support and automatic code formatting.

How to connect various editors and IDEs to 'deno lsp':
https://deno.land/manual@v{}/getting_started/setup_your_environment#editors-and-ides",
    SHORT_VERSION.as_str()
  )
});

fn lsp_subcommand<'a>() -> Command<'a> {
  Command::new("lsp")
    .about("Start the language server")
    .long_about(LSP_HELP.as_str())
}

fn lint_subcommand<'a>() -> Command<'a> {
  Command::new("lint")
    .about("Lint source files")
    .long_about(
      "Lint JavaScript/TypeScript source code.

  deno lint
  deno lint myfile1.ts myfile2.js

Print result as JSON:

  deno lint --json

Read from stdin:

  cat file.ts | deno lint -
  cat file.ts | deno lint --json -

List available rules:

  deno lint --rules

Ignore diagnostics on the next line by preceding it with an ignore comment and
rule name:

  // deno-lint-ignore no-explicit-any
  // deno-lint-ignore require-await no-empty

Names of rules to ignore must be specified after ignore comment.

Ignore linting a file by adding an ignore comment at the top of the file:

  // deno-lint-ignore-file
",
    )
    .arg(Arg::new("rules").long("rules").help("List available rules"))
    .arg(
      Arg::new("rules-tags")
        .long("rules-tags")
        .require_equals(true)
        .takes_value(true)
        .use_value_delimiter(true)
        .conflicts_with("rules")
        .help("Use set of rules with a tag"),
    )
    .arg(
      Arg::new("rules-include")
        .long("rules-include")
        .require_equals(true)
        .takes_value(true)
        .use_value_delimiter(true)
        .conflicts_with("rules")
        .help("Include lint rules"),
    )
    .arg(
      Arg::new("rules-exclude")
        .long("rules-exclude")
        .require_equals(true)
        .takes_value(true)
        .use_value_delimiter(true)
        .conflicts_with("rules")
        .help("Exclude lint rules"),
    )
    .arg(no_config_arg())
    .arg(config_arg())
    .arg(
      Arg::new("ignore")
        .long("ignore")
        .takes_value(true)
        .use_value_delimiter(true)
        .require_equals(true)
        .help("Ignore linting particular source files")
        .value_hint(ValueHint::AnyPath),
    )
    .arg(
      Arg::new("json")
        .long("json")
        .help("Output lint result in JSON format")
        .takes_value(false),
    )
    .arg(
      Arg::new("files")
        .takes_value(true)
        .multiple_values(true)
        .multiple_occurrences(true)
        .required(false)
        .value_hint(ValueHint::AnyPath),
    )
    .arg(watch_arg(false))
    .arg(no_clear_screen_arg())
}

fn repl_subcommand<'a>() -> Command<'a> {
  runtime_args(Command::new("repl"), false, true)
    .about("Read Eval Print Loop")
    .arg(
      Arg::new("eval-file")
        .long("eval-file")
        .min_values(1)
        .takes_value(true)
        .use_value_delimiter(true)
        .require_equals(true)
        .help("Evaluates the provided file(s) as scripts when the REPL starts. Accepts file paths and URLs.")
        .value_hint(ValueHint::AnyPath),
    )
    .arg(
      Arg::new("eval")
        .long("eval")
        .help("Evaluates the provided code when the REPL starts.")
        .takes_value(true)
        .value_name("code"),
    )
    .arg(unsafely_ignore_certificate_errors_arg())
}

fn run_subcommand<'a>() -> Command<'a> {
  runtime_args(Command::new("run"), true, true)
    .arg(
      watch_arg(true)
        .conflicts_with("inspect")
        .conflicts_with("inspect-brk"),
    )
    .arg(no_clear_screen_arg())
    .trailing_var_arg(true)
    .arg(script_arg().required(true))
    .about("Run a JavaScript or TypeScript program")
    .long_about(
      "Run a JavaScript or TypeScript program

By default all programs are run in sandbox without access to disk, network or
ability to spawn subprocesses.

  deno run https://deno.land/std/examples/welcome.ts

Grant all permissions:

  deno run -A https://deno.land/std/http/file_server.ts

Grant permission to read from disk and listen to network:

  deno run --allow-read --allow-net https://deno.land/std/http/file_server.ts

Grant permission to read allow-listed files from disk:

  deno run --allow-read=/etc https://deno.land/std/http/file_server.ts

Specifying the filename '-' to read the file from stdin.

  curl https://deno.land/std/examples/welcome.ts | deno run -",
    )
}

fn task_subcommand<'a>() -> Command<'a> {
  Command::new("task")
    .trailing_var_arg(true)
    .arg(config_arg())
    .arg(
      Arg::new("cwd")
        .long("cwd")
        .value_name("DIR")
        .help("Specify the directory to run the task in")
        .takes_value(true)
        .value_hint(ValueHint::DirPath)
    )
    // Ideally the task name and trailing arguments should be two separate clap
    // arguments, but there is a bug in clap that's preventing us from doing
    // this (https://github.com/clap-rs/clap/issues/1538). Once that's fixed,
    // then we can revert this back to what it used to be.
    .arg(Arg::new("task_name_and_args")
        .multiple_values(true)
        .multiple_occurrences(true)
        .allow_hyphen_values(true)
    .help("Task to be executed with any additional arguments passed to the task"))
    .about("Run a task defined in the configuration file")
    .long_about(
      "Run a task defined in the configuration file

  deno task build",
    )
}

fn test_subcommand<'a>() -> Command<'a> {
  runtime_args(Command::new("test"), true, true)
    .trailing_var_arg(true)
    .arg(
      Arg::new("ignore")
        .long("ignore")
        .takes_value(true)
        .use_value_delimiter(true)
        .require_equals(true)
        .help("Ignore files")
        .value_hint(ValueHint::AnyPath),
    )
    .arg(
      Arg::new("no-run")
        .long("no-run")
        .help("Cache test modules, but don't run tests")
        .takes_value(false),
    )
    .arg(
      Arg::new("trace-ops")
        .long("trace-ops")
        .help("Enable tracing of async ops. Useful when debugging leaking ops in test, but impacts test execution time.")
        .takes_value(false),
    )
    .arg(
      Arg::new("doc")
        .long("doc")
        .help("UNSTABLE: type-check code blocks")
        .takes_value(false),
    )
    .arg(
      Arg::new("fail-fast")
        .long("fail-fast")
        .alias("failfast")
        .help("Stop after N errors. Defaults to stopping after first failure.")
        .min_values(0)
        .required(false)
        .takes_value(true)
        .require_equals(true)
        .value_name("N")
        .validator(|val: &str| match val.parse::<NonZeroUsize>() {
          Ok(_) => Ok(()),
          Err(_) => Err("fail-fast should be a non zero integer".to_string()),
        }),
    )
    .arg(
      Arg::new("allow-none")
        .long("allow-none")
        .help("Don't return error code if no test files are found")
        .takes_value(false),
    )
    .arg(
      Arg::new("filter")
        .allow_hyphen_values(true)
        .long("filter")
        .takes_value(true)
        .help("Run tests with this string or pattern in the test name"),
    )
    .arg(
      Arg::new("shuffle")
        .long("shuffle")
        .value_name("NUMBER")
        .help("(UNSTABLE): Shuffle the order in which the tests are run")
        .min_values(0)
        .max_values(1)
        .require_equals(true)
        .takes_value(true)
        .validator(|val: &str| match val.parse::<u64>() {
          Ok(_) => Ok(()),
          Err(_) => Err("Shuffle seed should be a number".to_string()),
        }),
    )
    .arg(
      Arg::new("coverage")
        .long("coverage")
        .require_equals(true)
        .takes_value(true)
        .value_name("DIR")
        .conflicts_with("inspect")
        .conflicts_with("inspect-brk")
        .help("UNSTABLE: Collect coverage profile data into DIR"),
    )
    .arg(
      Arg::new("parallel")
        .long("parallel")
        .help("Run test modules in parallel. Parallelism defaults to the number of available CPUs or the value in the DENO_JOBS environment variable.")
        .conflicts_with("jobs")
        .takes_value(false)
    )
    .arg(
      Arg::new("jobs")
        .short('j')
        .long("jobs")
        .help("deprecated: Number of parallel workers, defaults to number of available CPUs when no value is provided. Defaults to 1 when the option is not present.")
        .hide(true)
        .min_values(0)
        .max_values(1)
        .takes_value(true)
        .validator(|val: &str| match val.parse::<NonZeroUsize>() {
          Ok(_) => Ok(()),
          Err(_) => Err("jobs should be a non zero unsigned integer".to_string()),
        }),
    )
    .arg(
      Arg::new("files")
        .help("List of file names to run")
        .takes_value(true)
        .multiple_values(true)
        .multiple_occurrences(true)
        .value_hint(ValueHint::AnyPath),
    )
    .arg(
      watch_arg(false)
        .conflicts_with("no-run")
        .conflicts_with("coverage"),
    )
    .arg(no_clear_screen_arg())
    .arg(script_arg().last(true))
    .about("Run tests")
    .long_about(
      "Run tests using Deno's built-in test runner.

Evaluate the given modules, run all tests declared with 'Deno.test()' and
report results to standard output:

  deno test src/fetch_test.ts src/signal_test.ts

Directory arguments are expanded to all contained files matching the glob
{*_,*.,}test.{js,mjs,ts,jsx,tsx}:

  deno test src/",
    )
}

fn types_subcommand<'a>() -> Command<'a> {
  Command::new("types")
    .about("Print runtime TypeScript declarations")
    .long_about(
      "Print runtime TypeScript declarations.

  deno types > lib.deno.d.ts

The declaration file could be saved and used for typing information.",
    )
}

fn upgrade_subcommand<'a>() -> Command<'a> {
  Command::new("upgrade")
    .about("Upgrade deno executable to given version")
    .long_about(
      "Upgrade deno executable to the given version.
Defaults to latest.

The version is downloaded from
https://github.com/denoland/deno/releases
and is used to replace the current executable.

If you want to not replace the current Deno executable but instead download an
update to a different location, use the --output flag

  deno upgrade --output $HOME/my_deno",
    )
    .arg(
      Arg::new("version")
        .long("version")
        .help("The version to upgrade to")
        .takes_value(true),
    )
    .arg(
      Arg::new("output")
        .long("output")
        .help("The path to output the updated version to")
        .takes_value(true)
        .value_hint(ValueHint::FilePath),
    )
    .arg(
      Arg::new("dry-run")
        .long("dry-run")
        .help("Perform all checks without replacing old exe"),
    )
    .arg(
      Arg::new("force")
        .long("force")
        .short('f')
        .help("Replace current exe even if not out-of-date"),
    )
    .arg(
      Arg::new("canary")
        .long("canary")
        .help("Upgrade to canary builds"),
    )
    .arg(ca_file_arg())
}

fn vendor_subcommand<'a>() -> Command<'a> {
  Command::new("vendor")
    .about("Vendor remote modules into a local directory")
    .long_about(
      "Vendor remote modules into a local directory.

Analyzes the provided modules along with their dependencies, downloads
remote modules to the output directory, and produces an import map that
maps remote specifiers to the downloaded files.

  deno vendor main.ts
  deno run --import-map vendor/import_map.json main.ts

Remote modules and multiple modules may also be specified:

  deno vendor main.ts test.deps.ts https://deno.land/std/path/mod.ts",
    )
    .arg(
      Arg::new("specifiers")
        .takes_value(true)
        .multiple_values(true)
        .multiple_occurrences(true)
        .required(true),
    )
    .arg(
      Arg::new("output")
        .long("output")
        .help("The directory to output the vendored modules to")
        .takes_value(true)
        .value_hint(ValueHint::DirPath),
    )
    .arg(
      Arg::new("force")
        .long("force")
        .short('f')
        .help(
          "Forcefully overwrite conflicting files in existing output directory",
        )
        .takes_value(false),
    )
    .arg(no_config_arg())
    .arg(config_arg())
    .arg(import_map_arg())
    .arg(lock_arg())
    .arg(reload_arg())
    .arg(ca_file_arg())
}

fn compile_args(app: Command) -> Command {
  app
    .arg(import_map_arg())
    .arg(no_remote_arg())
    .arg(no_config_arg())
    .arg(config_arg())
    .arg(no_check_arg())
    .arg(check_arg())
    .arg(reload_arg())
    .arg(lock_arg())
    .arg(lock_write_arg())
    .arg(ca_file_arg())
}

fn compile_args_without_check_args(app: Command) -> Command {
  app
    .arg(import_map_arg())
    .arg(no_remote_arg())
    .arg(config_arg())
    .arg(no_config_arg())
    .arg(reload_arg())
    .arg(lock_arg())
    .arg(lock_write_arg())
    .arg(ca_file_arg())
}

fn permission_args(app: Command) -> Command {
  app
    .arg(
      Arg::new("allow-read")
        .long("allow-read")
        .min_values(0)
        .takes_value(true)
        .use_value_delimiter(true)
        .require_equals(true)
        .help("Allow file system read access")
        .value_hint(ValueHint::AnyPath),
    )
    .arg(
      Arg::new("allow-write")
        .long("allow-write")
        .min_values(0)
        .takes_value(true)
        .use_value_delimiter(true)
        .require_equals(true)
        .help("Allow file system write access")
        .value_hint(ValueHint::AnyPath),
    )
    .arg(
      Arg::new("allow-net")
        .long("allow-net")
        .min_values(0)
        .takes_value(true)
        .use_value_delimiter(true)
        .require_equals(true)
        .help("Allow network access")
        .validator(flags_allow_net::validator),
    )
    .arg(unsafely_ignore_certificate_errors_arg())
    .arg(
      Arg::new("allow-env")
        .long("allow-env")
        .min_values(0)
        .takes_value(true)
        .use_value_delimiter(true)
        .require_equals(true)
        .help("Allow environment access")
        .validator(|keys| {
          for key in keys.split(',') {
            if key.is_empty() || key.contains(&['=', '\0'] as &[char]) {
              return Err(format!("invalid key \"{}\"", key));
            }
          }
          Ok(())
        }),
    )
    .arg(
      Arg::new("allow-run")
        .long("allow-run")
        .min_values(0)
        .takes_value(true)
        .use_value_delimiter(true)
        .require_equals(true)
        .help("Allow running subprocesses"),
    )
    .arg(
      Arg::new("allow-ffi")
        .long("allow-ffi")
        .min_values(0)
        .takes_value(true)
        .use_value_delimiter(true)
        .require_equals(true)
        .help("Allow loading dynamic libraries")
        .value_hint(ValueHint::AnyPath),
    )
    .arg(
      Arg::new("allow-hrtime")
        .long("allow-hrtime")
        .help("Allow high resolution time measurement"),
    )
    .arg(
      Arg::new("allow-all")
        .short('A')
        .long("allow-all")
        .help("Allow all permissions"),
    )
    .arg(Arg::new("prompt").long("prompt").hide(true).help(
      "deprecated: Fallback to prompt if required permission wasn't passed",
    ))
    .arg(
      Arg::new("no-prompt")
        .long("no-prompt")
        .help("Always throw if required permission wasn't passed"),
    )
}

fn runtime_args(
  app: Command,
  include_perms: bool,
  include_inspector: bool,
) -> Command {
  let app = compile_args(app);
  let app = if include_perms {
    permission_args(app)
  } else {
    app
  };
  let app = if include_inspector {
    inspect_args(app)
  } else {
    app
  };
  app
    .arg(cached_only_arg())
    .arg(location_arg())
    .arg(v8_flags_arg())
    .arg(seed_arg())
    .arg(enable_testing_features_arg())
    .arg(compat_arg())
}

fn inspect_args(app: Command) -> Command {
  app
    .arg(
      Arg::new("inspect")
        .long("inspect")
        .value_name("HOST:PORT")
        .help("Activate inspector on host:port (default: 127.0.0.1:9229)")
        .min_values(0)
        .max_values(1)
        .require_equals(true)
        .takes_value(true)
        .validator(inspect_arg_validate),
    )
    .arg(
      Arg::new("inspect-brk")
        .long("inspect-brk")
        .value_name("HOST:PORT")
        .help(
          "Activate inspector on host:port and break at start of user script",
        )
        .min_values(0)
        .max_values(1)
        .require_equals(true)
        .takes_value(true)
        .validator(inspect_arg_validate),
    )
}

static IMPORT_MAP_HELP: Lazy<String> = Lazy::new(|| {
  format!(
    "Load import map file from local file or remote URL.
  Docs: https://deno.land/manual@v{}/linking_to_external_code/import_maps
  Specification: https://wicg.github.io/import-maps/
  Examples: https://github.com/WICG/import-maps#the-import-map",
    SHORT_VERSION.as_str()
  )
});

fn import_map_arg<'a>() -> Arg<'a> {
  Arg::new("import-map")
    .long("import-map")
    .alias("importmap")
    .value_name("FILE")
    .help("Load import map file")
    .long_help(IMPORT_MAP_HELP.as_str())
    .takes_value(true)
    .value_hint(ValueHint::FilePath)
}

fn reload_arg<'a>() -> Arg<'a> {
  Arg::new("reload")
    .short('r')
    .min_values(0)
    .takes_value(true)
    .use_value_delimiter(true)
    .require_equals(true)
    .long("reload")
    .help("Reload source code cache (recompile TypeScript)")
    .value_name("CACHE_BLOCKLIST")
    .long_help(
      "Reload source code cache (recompile TypeScript)
--reload
  Reload everything
--reload=https://deno.land/std
  Reload only standard modules
--reload=https://deno.land/std/fs/utils.ts,https://deno.land/std/fmt/colors.ts
  Reloads specific modules",
    )
    .value_hint(ValueHint::FilePath)
}

fn ca_file_arg<'a>() -> Arg<'a> {
  Arg::new("cert")
    .long("cert")
    .value_name("FILE")
    .help("Load certificate authority from PEM encoded file")
    .takes_value(true)
    .value_hint(ValueHint::FilePath)
}

fn cached_only_arg<'a>() -> Arg<'a> {
  Arg::new("cached-only")
    .long("cached-only")
    .help("Require that remote dependencies are already cached")
}

fn location_arg<'a>() -> Arg<'a> {
  Arg::new("location")
    .long("location")
    .takes_value(true)
    .value_name("HREF")
    .validator(|href| {
      let url = Url::parse(href);
      if url.is_err() {
        return Err("Failed to parse URL".to_string());
      }
      let mut url = url.unwrap();
      if !["http", "https"].contains(&url.scheme()) {
        return Err("Expected protocol \"http\" or \"https\"".to_string());
      }
      url.set_username("").unwrap();
      url.set_password(None).unwrap();
      Ok(())
    })
    .help("Value of 'globalThis.location' used by some web APIs")
    .value_hint(ValueHint::Url)
}

fn enable_testing_features_arg<'a>() -> Arg<'a> {
  Arg::new("enable-testing-features-do-not-use")
    .long("enable-testing-features-do-not-use")
    .help("INTERNAL: Enable internal features used during integration testing")
    .hide(true)
}

fn v8_flags_arg<'a>() -> Arg<'a> {
  Arg::new("v8-flags")
    .long("v8-flags")
    .takes_value(true)
    .use_value_delimiter(true)
    .require_equals(true)
    .help("Set V8 command line options")
    .long_help("To see a list of all available flags use --v8-flags=--help.")
}

fn seed_arg<'a>() -> Arg<'a> {
  Arg::new("seed")
    .long("seed")
    .value_name("NUMBER")
    .help("Set the random number generator seed")
    .takes_value(true)
    .validator(|val| match val.parse::<u64>() {
      Ok(_) => Ok(()),
      Err(_) => Err("Seed should be a number".to_string()),
    })
}

static COMPAT_HELP: Lazy<String> = Lazy::new(|| {
  format!(
    "See https://deno.land/manual@v{}/node/compatibility_mode",
    SHORT_VERSION.as_str()
  )
});

fn compat_arg<'a>() -> Arg<'a> {
  Arg::new("compat")
    .long("compat")
    .requires("unstable")
    .help("UNSTABLE: Node compatibility mode.")
    .long_help(COMPAT_HELP.as_str())
}

fn watch_arg<'a>(takes_files: bool) -> Arg<'a> {
  let arg = Arg::new("watch")
    .long("watch")
    .help("Watch for file changes and restart automatically");

  if takes_files {
    arg
      .value_name("FILES")
      .min_values(0)
      .takes_value(true)
      .use_value_delimiter(true)
      .require_equals(true)
      .long_help(
        "Watch for file changes and restart process automatically.
Local files from entry point module graph are watched by default.
Additional paths might be watched by passing them as arguments to this flag.",
      )
      .value_hint(ValueHint::AnyPath)
  } else {
    arg.long_help(
      "Watch for file changes and restart process automatically. \
      Only local files from entry point module graph are watched.",
    )
  }
}

fn no_clear_screen_arg<'a>() -> Arg<'a> {
  Arg::new("no-clear-screen")
    .requires("watch")
    .long("no-clear-screen")
    .help("Do not clear terminal screen when under watch mode")
}

fn no_check_arg<'a>() -> Arg<'a> {
  Arg::new("no-check")
    .takes_value(true)
    .require_equals(true)
    .min_values(0)
    .value_name("NO_CHECK_TYPE")
    .long("no-check")
    .help("Skip type-checking modules")
    .long_help(
      "Skip type-checking. If the value of '--no-check=remote' is supplied, \
      diagnostic errors from remote modules will be ignored.",
    )
}

fn check_arg<'a>() -> Arg<'a> {
  Arg::new("check")
    .conflicts_with("no-check")
    .long("check")
    .takes_value(true)
    .require_equals(true)
    .min_values(0)
    .value_name("CHECK_TYPE")
    .help("Type-check modules")
    .long_help(
      "Type-check modules.

Deno does not type-check modules automatically from v1.23 onwards. Pass this \
flag to enable type-checking or use the 'deno check' subcommand.

If the value of '--check=all' is supplied, diagnostic errors from remote modules
will be included.",
    )
}

fn script_arg<'a>() -> Arg<'a> {
  Arg::new("script_arg")
    .multiple_values(true)
    .multiple_occurrences(true)
    // NOTE: these defaults are provided
    // so `deno run --v8-flags=--help` works
    // without specifying file to run.
    .default_value_ifs(&[
      ("v8-flags", Some("--help"), Some("_")),
      ("v8-flags", Some("-help"), Some("_")),
    ])
    .help("Script arg")
    .value_name("SCRIPT_ARG")
    .value_hint(ValueHint::FilePath)
}

fn lock_arg<'a>() -> Arg<'a> {
  Arg::new("lock")
    .long("lock")
    .value_name("FILE")
    .help("Check the specified lock file")
    .takes_value(true)
    .value_hint(ValueHint::FilePath)
}

fn lock_write_arg<'a>() -> Arg<'a> {
  Arg::new("lock-write")
    .long("lock-write")
    .requires("lock")
    .help("Write lock file (use with --lock)")
}

static CONFIG_HELP: Lazy<String> = Lazy::new(|| {
  format!(
    "The configuration file can be used to configure different aspects of \
      deno including TypeScript, linting, and code formatting. Typically the \
      configuration file will be called `deno.json` or `deno.jsonc` and \
      automatically detected; in that case this flag is not necessary. \
      See https://deno.land/manual@v{}/getting_started/configuration_file",
    SHORT_VERSION.as_str()
  )
});

fn config_arg<'a>() -> Arg<'a> {
  Arg::new("config")
    .short('c')
    .long("config")
    .value_name("FILE")
    .help("Specify the configuration file")
    .long_help(CONFIG_HELP.as_str())
    .takes_value(true)
    .value_hint(ValueHint::FilePath)
}

fn no_config_arg<'a>() -> Arg<'a> {
  Arg::new("no-config")
    .long("no-config")
    .help("Disable automatic loading of the configuration file.")
    .conflicts_with("config")
}

fn no_remote_arg<'a>() -> Arg<'a> {
  Arg::new("no-remote")
    .long("no-remote")
    .help("Do not resolve remote modules")
}

fn unsafely_ignore_certificate_errors_arg<'a>() -> Arg<'a> {
  Arg::new("unsafely-ignore-certificate-errors")
    .long("unsafely-ignore-certificate-errors")
    .min_values(0)
    .takes_value(true)
    .use_value_delimiter(true)
    .require_equals(true)
    .value_name("HOSTNAMES")
    .help("DANGER: Disables verification of TLS certificates")
    .validator(flags_allow_net::validator)
}

fn bench_parse(flags: &mut Flags, matches: &clap::ArgMatches) {
  flags.type_check_mode = TypeCheckMode::Local;

  runtime_args_parse(flags, matches, true, false);

  // NOTE: `deno bench` always uses `--no-prompt`, tests shouldn't ever do
  // interactive prompts, unless done by user code
  flags.no_prompt = true;

  let ignore = match matches.values_of("ignore") {
    Some(f) => f.map(PathBuf::from).collect(),
    None => vec![],
  };

  let filter = matches.value_of("filter").map(String::from);

  if matches.is_present("script_arg") {
    let script_arg: Vec<String> = matches
      .values_of("script_arg")
      .unwrap()
      .map(String::from)
      .collect();

    for v in script_arg {
      flags.argv.push(v);
    }
  }

  let include = if matches.is_present("files") {
    let files: Vec<String> = matches
      .values_of("files")
      .unwrap()
      .map(String::from)
      .collect();
    Some(files)
  } else {
    None
  };

  watch_arg_parse(flags, matches, false);
  flags.subcommand = DenoSubcommand::Bench(BenchFlags {
    include,
    ignore,
    filter,
  });
}

fn bundle_parse(flags: &mut Flags, matches: &clap::ArgMatches) {
  flags.type_check_mode = TypeCheckMode::Local;

  compile_args_parse(flags, matches);

  let source_file = matches.value_of("source_file").unwrap().to_string();

  let out_file = if let Some(out_file) = matches.value_of("out_file") {
    flags.allow_write = Some(vec![]);
    Some(PathBuf::from(out_file))
  } else {
    None
  };

  watch_arg_parse(flags, matches, false);

  flags.subcommand = DenoSubcommand::Bundle(BundleFlags {
    source_file,
    out_file,
  });
}

fn cache_parse(flags: &mut Flags, matches: &clap::ArgMatches) {
  compile_args_parse(flags, matches);
  let files = matches
    .values_of("file")
    .unwrap()
    .map(String::from)
    .collect();
  flags.subcommand = DenoSubcommand::Cache(CacheFlags { files });
}

fn check_parse(flags: &mut Flags, matches: &clap::ArgMatches) {
  flags.type_check_mode = TypeCheckMode::Local;
  compile_args_without_no_check_parse(flags, matches);
  let files = matches
    .values_of("file")
    .unwrap()
    .map(String::from)
    .collect();
  if matches.is_present("remote") {
    flags.type_check_mode = TypeCheckMode::All;
  }
  flags.subcommand = DenoSubcommand::Check(CheckFlags { files });
}

fn compile_parse(flags: &mut Flags, matches: &clap::ArgMatches) {
  flags.type_check_mode = TypeCheckMode::Local;
  runtime_args_parse(flags, matches, true, false);

  let mut script: Vec<String> = matches
    .values_of("script_arg")
    .unwrap()
    .map(String::from)
    .collect();
  assert!(!script.is_empty());
  let args = script.split_off(1);
  let source_file = script[0].to_string();
  let output = matches.value_of("output").map(PathBuf::from);
  let target = matches.value_of("target").map(String::from);

  flags.subcommand = DenoSubcommand::Compile(CompileFlags {
    source_file,
    output,
    args,
    target,
  });
}

fn coverage_parse(flags: &mut Flags, matches: &clap::ArgMatches) {
  let files = match matches.values_of("files") {
    Some(f) => f.map(PathBuf::from).collect(),
    None => vec![],
  };
  let ignore = match matches.values_of("ignore") {
    Some(f) => f.map(PathBuf::from).collect(),
    None => vec![],
  };
  let include = match matches.values_of("include") {
    Some(f) => f.map(String::from).collect(),
    None => vec![],
  };
  let exclude = match matches.values_of("exclude") {
    Some(f) => f.map(String::from).collect(),
    None => vec![],
  };
  let lcov = matches.is_present("lcov");
  let output = matches.value_of("output").map(PathBuf::from);
  flags.subcommand = DenoSubcommand::Coverage(CoverageFlags {
    files,
    output,
    ignore,
    include,
    exclude,
    lcov,
  });
}

fn doc_parse(flags: &mut Flags, matches: &clap::ArgMatches) {
  import_map_arg_parse(flags, matches);
  reload_arg_parse(flags, matches);

  let source_file = matches.value_of("source_file").map(String::from);
  let private = matches.is_present("private");
  let json = matches.is_present("json");
  let filter = matches.value_of("filter").map(String::from);
  flags.subcommand = DenoSubcommand::Doc(DocFlags {
    source_file,
    json,
    filter,
    private,
  });
}

fn eval_parse(flags: &mut Flags, matches: &clap::ArgMatches) {
  runtime_args_parse(flags, matches, false, true);
  flags.allow_net = Some(vec![]);
  flags.allow_env = Some(vec![]);
  flags.allow_run = Some(vec![]);
  flags.allow_read = Some(vec![]);
  flags.allow_write = Some(vec![]);
  flags.allow_ffi = Some(vec![]);
  flags.allow_hrtime = true;
  // TODO(@satyarohith): remove this flag in 2.0.
  let as_typescript = matches.is_present("ts");
  let ext = if as_typescript {
    "ts".to_string()
  } else {
    matches.value_of("ext").unwrap().to_string()
  };

  let print = matches.is_present("print");
  let mut code: Vec<String> = matches
    .values_of("code_arg")
    .unwrap()
    .map(String::from)
    .collect();
  assert!(!code.is_empty());
  let code_args = code.split_off(1);
  let code = code[0].to_string();
  for v in code_args {
    flags.argv.push(v);
  }
  flags.subcommand = DenoSubcommand::Eval(EvalFlags { print, code, ext });
}

fn fmt_parse(flags: &mut Flags, matches: &clap::ArgMatches) {
  config_args_parse(flags, matches);
  watch_arg_parse(flags, matches, false);

  let files = match matches.values_of("files") {
    Some(f) => f.map(PathBuf::from).collect(),
    None => vec![],
  };
  let ignore = match matches.values_of("ignore") {
    Some(f) => f.map(PathBuf::from).collect(),
    None => vec![],
  };
  let ext = matches.value_of("ext").unwrap().to_string();

  let use_tabs = if matches.is_present("options-use-tabs") {
    Some(true)
  } else {
    None
  };
  let line_width = if matches.is_present("options-line-width") {
    Some(
      matches
        .value_of("options-line-width")
        .unwrap()
        .parse()
        .unwrap(),
    )
  } else {
    None
  };
  let indent_width = if matches.is_present("options-indent-width") {
    Some(
      matches
        .value_of("options-indent-width")
        .unwrap()
        .parse()
        .unwrap(),
    )
  } else {
    None
  };
  let single_quote = if matches.is_present("options-single-quote") {
    Some(true)
  } else {
    None
  };
  let prose_wrap = if matches.is_present("options-prose-wrap") {
    Some(matches.value_of("options-prose-wrap").unwrap().to_string())
  } else {
    None
  };

  flags.subcommand = DenoSubcommand::Fmt(FmtFlags {
    check: matches.is_present("check"),
    ext,
    files,
    ignore,
    use_tabs,
    line_width,
    indent_width,
    single_quote,
    prose_wrap,
  });
}

fn info_parse(flags: &mut Flags, matches: &clap::ArgMatches) {
  reload_arg_parse(flags, matches);
  config_args_parse(flags, matches);
  import_map_arg_parse(flags, matches);
  location_arg_parse(flags, matches);
  ca_file_arg_parse(flags, matches);
  let json = matches.is_present("json");
  flags.subcommand = DenoSubcommand::Info(InfoFlags {
    file: matches.value_of("file").map(|f| f.to_string()),
    json,
  });
}

fn install_parse(flags: &mut Flags, matches: &clap::ArgMatches) {
  runtime_args_parse(flags, matches, true, true);

  let root = if matches.is_present("root") {
    let install_root = matches.value_of("root").unwrap();
    Some(PathBuf::from(install_root))
  } else {
    None
  };

  let force = matches.is_present("force");
  let name = matches.value_of("name").map(|s| s.to_string());
  let cmd_values = matches.values_of("cmd").unwrap();
  let mut cmd = vec![];
  for value in cmd_values {
    cmd.push(value.to_string());
  }

  let module_url = cmd[0].to_string();
  let args = cmd[1..].to_vec();

  flags.subcommand = DenoSubcommand::Install(InstallFlags {
    name,
    module_url,
    args,
    root,
    force,
  });
}

fn uninstall_parse(flags: &mut Flags, matches: &clap::ArgMatches) {
  let root = if matches.is_present("root") {
    let install_root = matches.value_of("root").unwrap();
    Some(PathBuf::from(install_root))
  } else {
    None
  };

  let name = matches.value_of("name").unwrap().to_string();
  flags.subcommand = DenoSubcommand::Uninstall(UninstallFlags { name, root });
}

fn lsp_parse(flags: &mut Flags, _matches: &clap::ArgMatches) {
  flags.subcommand = DenoSubcommand::Lsp;
}

fn lint_parse(flags: &mut Flags, matches: &clap::ArgMatches) {
  config_args_parse(flags, matches);
  watch_arg_parse(flags, matches, false);
  let files = match matches.values_of("files") {
    Some(f) => f.map(PathBuf::from).collect(),
    None => vec![],
  };
  let ignore = match matches.values_of("ignore") {
    Some(f) => f.map(PathBuf::from).collect(),
    None => vec![],
  };
  let rules = matches.is_present("rules");
  let maybe_rules_tags = matches
    .values_of("rules-tags")
    .map(|f| f.map(String::from).collect());

  let maybe_rules_include = matches
    .values_of("rules-include")
    .map(|f| f.map(String::from).collect());

  let maybe_rules_exclude = matches
    .values_of("rules-exclude")
    .map(|f| f.map(String::from).collect());

  let json = matches.is_present("json");
  flags.subcommand = DenoSubcommand::Lint(LintFlags {
    files,
    rules,
    maybe_rules_tags,
    maybe_rules_include,
    maybe_rules_exclude,
    ignore,
    json,
  });
}

fn repl_parse(flags: &mut Flags, matches: &clap::ArgMatches) {
  runtime_args_parse(flags, matches, false, true);
  unsafely_ignore_certificate_errors_parse(flags, matches);

  let eval_files: Option<Vec<String>> = matches
    .values_of("eval-file")
    .map(|values| values.map(String::from).collect());

  handle_repl_flags(
    flags,
    ReplFlags {
      eval_files,
      eval: matches.value_of("eval").map(ToOwned::to_owned),
    },
  );
}

fn run_parse(flags: &mut Flags, matches: &clap::ArgMatches) {
  runtime_args_parse(flags, matches, true, true);

  let mut script: Vec<String> = matches
    .values_of("script_arg")
    .unwrap()
    .map(String::from)
    .collect();
  assert!(!script.is_empty());
  let script_args = script.split_off(1);
  let script = script[0].to_string();
  for v in script_args {
    flags.argv.push(v);
  }

  watch_arg_parse(flags, matches, true);
  flags.subcommand = DenoSubcommand::Run(RunFlags { script });
}

fn task_parse(
  flags: &mut Flags,
  matches: &clap::ArgMatches,
  raw_args: &[String],
) {
  flags.config_flag = if let Some(config) = matches.value_of("config") {
    ConfigFlag::Path(config.to_string())
  } else {
    ConfigFlag::Discover
  };

  let mut task_flags = TaskFlags {
    cwd: None,
    task: String::new(),
  };

  if let Some(cwd) = matches.value_of("cwd") {
    task_flags.cwd = Some(cwd.to_string());
  }

  if let Some(mut index) = matches.index_of("task_name_and_args") {
    index += 1; // skip `task`

    // temporary workaround until https://github.com/clap-rs/clap/issues/1538 is fixed
    while index < raw_args.len() {
      match raw_args[index].as_str() {
        "-c" | "--config" => {
          flags.config_flag = ConfigFlag::Path(raw_args[index + 1].to_string());
          index += 2;
        }
        "--cwd" => {
          task_flags.cwd = Some(raw_args[index + 1].to_string());
          index += 2;
        }
        "--no-config" => {
          flags.config_flag = ConfigFlag::Disabled;
          index += 1;
        }
        "-q" | "--quiet" => {
          flags.log_level = Some(Level::Error);
          index += 1;
        }
        _ => break,
      }
    }

    if index < raw_args.len() {
      task_flags.task = raw_args[index].to_string();
      index += 1;

      if index < raw_args.len() {
        flags
          .argv
          .extend(raw_args[index..].iter().map(String::from));
      }
    }
  }

  flags.subcommand = DenoSubcommand::Task(task_flags);
}

fn test_parse(flags: &mut Flags, matches: &clap::ArgMatches) {
  flags.type_check_mode = TypeCheckMode::Local;
  runtime_args_parse(flags, matches, true, true);
  // NOTE: `deno test` always uses `--no-prompt`, tests shouldn't ever do
  // interactive prompts, unless done by user code
  flags.no_prompt = true;

  let ignore = match matches.values_of("ignore") {
    Some(f) => f.map(PathBuf::from).collect(),
    None => vec![],
  };

  let no_run = matches.is_present("no-run");
  let trace_ops = matches.is_present("trace-ops");
  let doc = matches.is_present("doc");
  let allow_none = matches.is_present("allow-none");
  let filter = matches.value_of("filter").map(String::from);

  let fail_fast = if matches.is_present("fail-fast") {
    if let Some(value) = matches.value_of("fail-fast") {
      Some(value.parse().unwrap())
    } else {
      Some(NonZeroUsize::new(1).unwrap())
    }
  } else {
    None
  };

  let shuffle = if matches.is_present("shuffle") {
    let value = if let Some(value) = matches.value_of("shuffle") {
      value.parse::<u64>().unwrap()
    } else {
      rand::random::<u64>()
    };

    Some(value)
  } else {
    None
  };

  if matches.is_present("script_arg") {
    let script_arg: Vec<String> = matches
      .values_of("script_arg")
      .unwrap()
      .map(String::from)
      .collect();

    for v in script_arg {
      flags.argv.push(v);
    }
  }

  let concurrent_jobs = if matches.is_present("parallel") {
    if let Ok(value) = env::var("DENO_JOBS") {
      value
        .parse::<NonZeroUsize>()
        .unwrap_or(NonZeroUsize::new(1).unwrap())
    } else {
      std::thread::available_parallelism()
        .unwrap_or(NonZeroUsize::new(1).unwrap())
    }
  } else if matches.is_present("jobs") {
    println!(
      "{}",
      super::super::super::deno_runtime::colors::yellow("Warning: --jobs flag is deprecated. Use the --parallel flag with possibly the 'DENO_JOBS' environment variable."),
    );
    if let Some(value) = matches.value_of("jobs") {
      value.parse().unwrap()
    } else {
      std::thread::available_parallelism()
        .unwrap_or(NonZeroUsize::new(1).unwrap())
    }
  } else {
    NonZeroUsize::new(1).unwrap()
  };

  let include: Vec<String> = if matches.is_present("files") {
    matches
      .values_of("files")
      .unwrap()
      .map(String::from)
      .collect::<Vec<_>>()
  } else {
    Vec::new()
  };

  flags.coverage_dir = matches.value_of("coverage").map(String::from);
  watch_arg_parse(flags, matches, false);
  flags.subcommand = DenoSubcommand::Test(TestFlags {
    no_run,
    doc,
    fail_fast,
    include,
    ignore,
    filter,
    shuffle,
    allow_none,
    concurrent_jobs,
    trace_ops,
  });
}

fn types_parse(flags: &mut Flags, _matches: &clap::ArgMatches) {
  flags.subcommand = DenoSubcommand::Types;
}

fn upgrade_parse(flags: &mut Flags, matches: &clap::ArgMatches) {
  ca_file_arg_parse(flags, matches);

  let dry_run = matches.is_present("dry-run");
  let force = matches.is_present("force");
  let canary = matches.is_present("canary");
  let version = matches.value_of("version").map(|s| s.to_string());
  let output = if matches.is_present("output") {
    let install_root = matches.value_of("output").unwrap();
    Some(PathBuf::from(install_root))
  } else {
    None
  };
  let ca_file = matches.value_of("cert").map(|s| s.to_string());
  flags.subcommand = DenoSubcommand::Upgrade(UpgradeFlags {
    dry_run,
    force,
    canary,
    version,
    output,
    ca_file,
  });
}

fn vendor_parse(flags: &mut Flags, matches: &clap::ArgMatches) {
  ca_file_arg_parse(flags, matches);
  config_args_parse(flags, matches);
  import_map_arg_parse(flags, matches);
  lock_arg_parse(flags, matches);
  reload_arg_parse(flags, matches);

  flags.subcommand = DenoSubcommand::Vendor(VendorFlags {
    specifiers: matches
      .values_of("specifiers")
      .map(|p| p.map(ToString::to_string).collect())
      .unwrap_or_default(),
    output_path: matches.value_of("output").map(PathBuf::from),
    force: matches.is_present("force"),
  });
}

fn compile_args_parse(flags: &mut Flags, matches: &clap::ArgMatches) {
  import_map_arg_parse(flags, matches);
  no_remote_arg_parse(flags, matches);
  config_args_parse(flags, matches);
  no_check_arg_parse(flags, matches);
  check_arg_parse(flags, matches);
  reload_arg_parse(flags, matches);
  lock_args_parse(flags, matches);
  ca_file_arg_parse(flags, matches);
}

fn compile_args_without_no_check_parse(
  flags: &mut Flags,
  matches: &clap::ArgMatches,
) {
  import_map_arg_parse(flags, matches);
  no_remote_arg_parse(flags, matches);
  config_args_parse(flags, matches);
  reload_arg_parse(flags, matches);
  lock_args_parse(flags, matches);
  ca_file_arg_parse(flags, matches);
}

fn permission_args_parse(flags: &mut Flags, matches: &clap::ArgMatches) {
  unsafely_ignore_certificate_errors_parse(flags, matches);
  if let Some(read_wl) = matches.values_of("allow-read") {
    let read_allowlist: Vec<PathBuf> = read_wl.map(PathBuf::from).collect();
    flags.allow_read = Some(read_allowlist);
  }

  if let Some(write_wl) = matches.values_of("allow-write") {
    let write_allowlist: Vec<PathBuf> = write_wl.map(PathBuf::from).collect();
    flags.allow_write = Some(write_allowlist);
  }

  if let Some(net_wl) = matches.values_of("allow-net") {
    let net_allowlist: Vec<String> =
      flags_allow_net::parse(net_wl.map(ToString::to_string).collect())
        .unwrap();
    flags.allow_net = Some(net_allowlist);
  }

  if let Some(env_wl) = matches.values_of("allow-env") {
    let env_allowlist: Vec<String> = env_wl
      .map(|env: &str| {
        if cfg!(windows) {
          env.to_uppercase()
        } else {
          env.to_string()
        }
      })
      .collect();
    flags.allow_env = Some(env_allowlist);
    debug!("env allowlist: {:#?}", &flags.allow_env);
  }

  if let Some(run_wl) = matches.values_of("allow-run") {
    let run_allowlist: Vec<String> = run_wl.map(ToString::to_string).collect();
    flags.allow_run = Some(run_allowlist);
    debug!("run allowlist: {:#?}", &flags.allow_run);
  }

  if let Some(ffi_wl) = matches.values_of("allow-ffi") {
    let ffi_allowlist: Vec<PathBuf> = ffi_wl.map(PathBuf::from).collect();
    flags.allow_ffi = Some(ffi_allowlist);
    debug!("ffi allowlist: {:#?}", &flags.allow_ffi);
  }

  if matches.is_present("allow-hrtime") {
    flags.allow_hrtime = true;
  }
  if matches.is_present("allow-all") {
    flags.allow_all = true;
    flags.allow_read = Some(vec![]);
    flags.allow_env = Some(vec![]);
    flags.allow_net = Some(vec![]);
    flags.allow_run = Some(vec![]);
    flags.allow_write = Some(vec![]);
    flags.allow_ffi = Some(vec![]);
    flags.allow_hrtime = true;
  }
  #[cfg(not(test))]
  let has_no_prompt_env = env::var("DENO_NO_PROMPT") == Ok("1".to_string());
  #[cfg(test)]
  let has_no_prompt_env = false;
  if has_no_prompt_env || matches.is_present("no-prompt") {
    flags.no_prompt = true;
  }
}
fn unsafely_ignore_certificate_errors_parse(
  flags: &mut Flags,
  matches: &clap::ArgMatches,
) {
  if let Some(ic_wl) = matches.values_of("unsafely-ignore-certificate-errors") {
    let ic_allowlist: Vec<String> =
      flags_allow_net::parse(ic_wl.map(ToString::to_string).collect()).unwrap();
    flags.unsafely_ignore_certificate_errors = Some(ic_allowlist);
  }
}
fn runtime_args_parse(
  flags: &mut Flags,
  matches: &clap::ArgMatches,
  include_perms: bool,
  include_inspector: bool,
) {
  compile_args_parse(flags, matches);
  cached_only_arg_parse(flags, matches);
  if include_perms {
    permission_args_parse(flags, matches);
  }
  if include_inspector {
    inspect_arg_parse(flags, matches);
  }
  location_arg_parse(flags, matches);
  v8_flags_arg_parse(flags, matches);
  seed_arg_parse(flags, matches);
  compat_arg_parse(flags, matches);
  enable_testing_features_arg_parse(flags, matches);
}

fn inspect_arg_parse(flags: &mut Flags, matches: &clap::ArgMatches) {
  let default = || "127.0.0.1:9229".parse::<SocketAddr>().unwrap();
  flags.inspect = if matches.is_present("inspect") {
    if let Some(host) = matches.value_of("inspect") {
      Some(host.parse().unwrap())
    } else {
      Some(default())
    }
  } else {
    None
  };
  flags.inspect_brk = if matches.is_present("inspect-brk") {
    if let Some(host) = matches.value_of("inspect-brk") {
      Some(host.parse().unwrap())
    } else {
      Some(default())
    }
  } else {
    None
  };
}

fn import_map_arg_parse(flags: &mut Flags, matches: &clap::ArgMatches) {
  flags.import_map_path = matches.value_of("import-map").map(ToOwned::to_owned);
}

fn reload_arg_parse(flags: &mut Flags, matches: &ArgMatches) {
  if let Some(cache_bl) = matches.values_of("reload") {
    let raw_cache_blocklist: Vec<String> =
      cache_bl.map(ToString::to_string).collect();
    if raw_cache_blocklist.is_empty() {
      flags.reload = true;
    } else {
      flags.cache_blocklist = resolve_urls(raw_cache_blocklist);
      debug!("cache blocklist: {:#?}", &flags.cache_blocklist);
      flags.reload = false;
    }
  }
}

fn ca_file_arg_parse(flags: &mut Flags, matches: &clap::ArgMatches) {
  flags.ca_file = matches.value_of("cert").map(ToOwned::to_owned);
}

fn enable_testing_features_arg_parse(
  flags: &mut Flags,
  matches: &clap::ArgMatches,
) {
  if matches.is_present("enable-testing-features-do-not-use") {
    flags.enable_testing_features = true
  }
}

fn cached_only_arg_parse(flags: &mut Flags, matches: &ArgMatches) {
  if matches.is_present("cached-only") {
    flags.cached_only = true;
  }
}

fn location_arg_parse(flags: &mut Flags, matches: &clap::ArgMatches) {
  flags.location = matches
    .value_of("location")
    .map(|href| Url::parse(href).unwrap());
}

fn v8_flags_arg_parse(flags: &mut Flags, matches: &ArgMatches) {
  if let Some(v8_flags) = matches.values_of("v8-flags") {
    flags.v8_flags = v8_flags.map(String::from).collect();
  }
}

fn seed_arg_parse(flags: &mut Flags, matches: &ArgMatches) {
  if matches.is_present("seed") {
    let seed_string = matches.value_of("seed").unwrap();
    let seed = seed_string.parse::<u64>().unwrap();
    flags.seed = Some(seed);

    flags.v8_flags.push(format!("--random-seed={}", seed));
  }
}

fn compat_arg_parse(flags: &mut Flags, matches: &ArgMatches) {
  if matches.is_present("compat") {
    flags.compat = true;
  }
}

fn no_check_arg_parse(flags: &mut Flags, matches: &clap::ArgMatches) {
  if let Some(cache_type) = matches.value_of("no-check") {
    match cache_type {
      "remote" => flags.type_check_mode = TypeCheckMode::Local,
      _ => debug!(
        "invalid value for 'no-check' of '{}' using default",
        cache_type
      ),
    }
  } else if matches.is_present("no-check") {
    flags.type_check_mode = TypeCheckMode::None;
  }
}

fn check_arg_parse(flags: &mut Flags, matches: &clap::ArgMatches) {
  if let Some(cache_type) = matches.value_of("check") {
    match cache_type {
      "all" => flags.type_check_mode = TypeCheckMode::All,
      _ => debug!(
        "invalid value for 'check' of '{}' using default",
        cache_type
      ),
    }
  } else if matches.is_present("check") {
    flags.type_check_mode = TypeCheckMode::Local;
  }
}

fn lock_args_parse(flags: &mut Flags, matches: &clap::ArgMatches) {
  lock_arg_parse(flags, matches);
  if matches.is_present("lock-write") {
    flags.lock_write = true;
  }
}

fn lock_arg_parse(flags: &mut Flags, matches: &clap::ArgMatches) {
  if matches.is_present("lock") {
    let lockfile = matches.value_of("lock").unwrap();
    flags.lock = Some(PathBuf::from(lockfile));
  }
}

fn config_args_parse(flags: &mut Flags, matches: &ArgMatches) {
  flags.config_flag = if matches.is_present("no-config") {
    ConfigFlag::Disabled
  } else if let Some(config) = matches.value_of("config") {
    ConfigFlag::Path(config.to_string())
  } else {
    ConfigFlag::Discover
  };
}

fn no_remote_arg_parse(flags: &mut Flags, matches: &clap::ArgMatches) {
  if matches.is_present("no-remote") {
    flags.no_remote = true;
  }
}

fn inspect_arg_validate(val: &str) -> Result<(), String> {
  match val.parse::<SocketAddr>() {
    Ok(_) => Ok(()),
    Err(e) => Err(e.to_string()),
  }
}

fn watch_arg_parse(
  flags: &mut Flags,
  matches: &clap::ArgMatches,
  allow_extra: bool,
) {
  if allow_extra {
    if let Some(f) = matches.values_of("watch") {
      flags.watch = Some(f.map(PathBuf::from).collect());
    }
  } else if matches.is_present("watch") {
    flags.watch = Some(vec![]);
  }

  if matches.is_present("no-clear-screen") {
    flags.no_clear_screen = true;
  }
}

// TODO(ry) move this to utility module and add test.
/// Strips fragment part of URL. Panics on bad URL.
pub fn resolve_urls(urls: Vec<String>) -> Vec<String> {
  let mut out: Vec<String> = vec![];
  for urlstr in urls.iter() {
    if let Ok(mut url) = Url::from_str(urlstr) {
      url.set_fragment(None);
      let mut full_url = String::from(url.as_str());
      if full_url.len() > 1 && full_url.ends_with('/') {
        full_url.pop();
      }
      out.push(full_url);
    } else {
      panic!("Bad Url: {}", urlstr);
    }
  }
  out
}

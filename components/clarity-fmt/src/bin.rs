pub mod linter;

use clap::{Parser, Subcommand};
use clarinet_files::FileLocation;
use clarity_repl::{
    clarity::EvaluationResult,
    repl::{Session, SessionSettings},
};
use linter::{ClarityLinter, Settings};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Opts {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand, PartialEq, Clone, Debug)]
enum Command {
    /// Format file
    #[clap(name = "fix", bin_name = "fix")]
    Fix(Fix),
}

#[derive(Parser, PartialEq, Clone, Debug)]
struct Fix {
    /// File path
    pub file_path: String,
}

pub fn main() {
    let opts: Opts = match Opts::try_parse() {
        Ok(opts) => opts,
        Err(e) => {
            println!("{}", e);
            std::process::exit(1);
        }
    };

    match opts.command {
        Command::Fix(cmd) => {
            let file = FileLocation::from_path_string(&cmd.file_path).unwrap();
            let snippet = file.read_content_as_utf8().unwrap();
            let mut session = Session::new(SessionSettings::default());
            let contract_analysis = match session.eval(snippet, None, false) {
                Ok(execution) => match execution.result {
                    EvaluationResult::Contract(evaluation) => evaluation.contract.analysis,
                    _ => {
                        println!("empty contract");
                        std::process::exit(1);
                    }
                },
                Err(e) => {
                    println!("{:?}", e);
                    std::process::exit(1);
                }
            };

            let settings = Settings::default();
            let linter = ClarityLinter::new(settings);

            let res = linter.run(&contract_analysis);

            println!("{}", res)
        }
    };
}

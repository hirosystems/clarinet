use crate::repl::{settings::SessionSettings, Session};

use clarity::vm::EvaluationResult;
use rustyline::error::ReadlineError;
use rustyline::Editor;

const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
const HISTORY_FILE: Option<&'static str> = option_env!("CLARITY_REPL_HISTORY_FILE");

enum Input<'a> {
    Incomplete(char),
    Complete(Vec<&'a str>),
}

fn complete_input(str: &str) -> Result<Input, (char, char)> {
    let mut forms: Vec<&str> = vec![];
    let mut paren_count = 0;
    let mut last_pos = 0;

    let mut brackets = vec![];
    let mut skip_next = false;
    let mut in_string = false;

    for (pos, character) in str.char_indices() {
        match character {
            '\\' => skip_next = true,
            '"' => {
                if skip_next {
                    skip_next = false
                } else {
                    in_string = !in_string
                }
            }
            '(' | '{' => {
                if !in_string {
                    brackets.push(character);
                    // skip whitespace between the previous form's
                    // closing paren (if there is one) and the current
                    // form's opening paren
                    match (character, paren_count) {
                        ('(', 0) => {
                            paren_count += 1;
                            last_pos = pos
                        }
                        ('(', _) => paren_count += 1,
                        _ => {}
                    }
                }
            }
            ')' | '}' => {
                if !in_string {
                    match (brackets.pop(), character) {
                        (Some('('), '}') => return Err((')', '}')),
                        (Some('{'), ')') => return Err(('}', ')')),
                        _ => {}
                    };
                    if character == ')' {
                        paren_count -= 1;
                        if paren_count == 0 {
                            forms.push(&str[last_pos..pos + 1]);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    match brackets.last() {
        Some(char) => Ok(Input::Incomplete(*char)),
        _ => Ok(Input::Complete(forms)),
    }
}

pub struct Terminal {
    pub session: Session,
    pub session_wasm: Option<Session>,
}

impl Terminal {
    pub fn new(
        session_settings: SessionSettings,
        session_wasm_settings: Option<SessionSettings>,
    ) -> Terminal {
        let session = Session::new(session_settings);
        let session_wasm = session_wasm_settings.map(Session::new);
        Terminal {
            session,
            session_wasm,
        }
    }

    pub fn load(session: Session, session_wasm: Option<Session>) -> Terminal {
        Terminal {
            session,
            session_wasm,
        }
    }

    pub fn start(&mut self) -> bool {
        println!("{}", green!(format!("clarity-repl v{}", VERSION.unwrap())));
        println!("{}", black!("Enter \"::help\" for usage hints."));
        println!("{}", black!("Connected to a transient in-memory database."));

        let output = match self.session.display_digest() {
            Ok(output) => output,
            Err(e) => {
                println!("{}", e);
                std::process::exit(1);
            }
        };
        println!("{}", output);
        let mut editor = Editor::<()>::new();
        let mut ctrl_c_acc = 0;
        let mut input_buffer = vec![];
        let mut prompt = String::from(">> ");

        editor
            .load_history(HISTORY_FILE.unwrap_or("history.txt"))
            .ok();
        let reload = loop {
            let readline = editor.readline(prompt.as_str());
            match readline {
                Ok(command) => {
                    ctrl_c_acc = 0;
                    input_buffer.push(command);
                    let input = input_buffer.join(" ");
                    match complete_input(&input) {
                        Ok(Input::Complete(_)) => {
                            let (reload, output, result) = self.session.handle_command(&input);

                            if let Some(session_wasm) = &mut self.session_wasm {
                                let (_, _, result_wasm) = session_wasm.handle_command(&input);

                                if let (Some(result), Some(result_wasm)) = (result, result_wasm) {
                                    match (result, result_wasm) {
                                        (Ok(result), Ok(result_wasm)) => {
                                            let value = match result.result {
                                                EvaluationResult::Contract(contract_result) => {
                                                    contract_result.result
                                                }
                                                EvaluationResult::Snippet(snippet_result) => {
                                                    Some(snippet_result.result)
                                                }
                                            };
                                            let value_wasm = match result_wasm.result {
                                                EvaluationResult::Contract(contract_result) => {
                                                    contract_result.result
                                                }
                                                EvaluationResult::Snippet(snippet_result) => {
                                                    Some(snippet_result.result)
                                                }
                                            };
                                            if value != value_wasm {
                                                dbg!(value);
                                                dbg!(value_wasm);
                                                print_clarity_wasm_warning();
                                            };
                                        }
                                        (Ok(result), Err(error_wasm)) => {
                                            dbg!(result);
                                            dbg!(error_wasm);
                                            print_clarity_wasm_warning();
                                        }
                                        (Err(error), Ok(result_wasm)) => {
                                            dbg!(error);
                                            dbg!(result_wasm);
                                            print_clarity_wasm_warning();
                                        }
                                        (Err(error), Err(error_wasm)) => {
                                            if error != error_wasm {
                                                dbg!(error);
                                                dbg!(error_wasm);
                                                print_clarity_wasm_warning();
                                            }
                                        }
                                    };
                                }
                            }

                            for line in output {
                                println!("{}", line);
                            }
                            prompt = String::from(">> ");
                            self.session.executed.push(input.to_string());
                            editor.add_history_entry(input);
                            input_buffer.clear();
                            if reload {
                                break true;
                            }
                        }
                        Ok(Input::Incomplete(str)) => {
                            prompt = format!("{}.. ", str);
                        }
                        Err((expected, got)) => {
                            println!("Error: expected closing {}, got {}", expected, got);
                            prompt = String::from(">> ");
                            input_buffer.pop();
                        }
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    ctrl_c_acc += 1;
                    if ctrl_c_acc == 2 {
                        break false;
                    } else {
                        println!("{}", yellow!("Hit CTRL-C a second time to quit."));
                    }
                }
                Err(ReadlineError::Eof) => {
                    println!("CTRL-D");
                    break false;
                }
                Err(err) => {
                    println!("Error: {:?}", err);
                    break false;
                }
            }
        };
        editor
            .save_history(HISTORY_FILE.unwrap_or("history.txt"))
            .unwrap();
        reload
    }
}

pub fn print_clarity_wasm_warning() {
    println!("{} https://github.com/stacks-network/clarity-wasm/issues/new/choose {}",
        yellow!("It appears that Clarity-Wasm is returning an unexpected result.\nPlease help improve the Stacks network by reporting this issue at"),
        yellow!("and include the errors above along with the source code that triggered this.\n")
    );
}

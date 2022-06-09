use crate::repl::{settings::SessionSettings, Session};

use ansi_term::{Colour, Style};
use rustyline::error::ReadlineError;
use rustyline::Editor;
use std::io::{stdin, stdout, Write};

const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
const HISTORY_FILE: Option<&'static str> = option_env!("CLARITY_REPL_HISTORY_FILE");

fn complete_input(str: &str) -> Result<Option<char>, (char, char)> {
    let mut brackets = vec![];
    for character in str.chars() {
        match character {
            '(' | '{' => brackets.push(character),
            ')' | '}' => match (brackets.pop(), character) {
                (Some('('), '}') => return Err((')', '}')),
                (Some('{'), ')') => return Err(('}', ')')),
                _ => {}
            },
            _ => {}
        }
    }
    match brackets.last() {
        Some(char) => Ok(Some(*char)),
        _ => Ok(None),
    }
}

pub struct Terminal {
    pub session: Session,
}

impl Terminal {
    pub fn new(session_settings: SessionSettings) -> Terminal {
        let mut session = Session::new(session_settings);
        session.is_interactive = true;
        Terminal { session }
    }

    pub fn load(mut session: Session) -> Terminal {
        session.is_interactive = true;
        Terminal { session }
    }

    pub fn start(&mut self) {
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
        loop {
            let readline = editor.readline(prompt.as_str());
            match readline {
                Ok(command) => {
                    ctrl_c_acc = 0;
                    input_buffer.push(command);
                    let input = input_buffer.join("\n");
                    match complete_input(&input) {
                        Ok(None) => {
                            let output = self.session.handle_command(&input);
                            for line in output {
                                println!("{}", line);
                            }
                            prompt = String::from(">> ");
                            self.session.executed.push(input.clone());
                            editor.add_history_entry(&input);
                            input_buffer.clear();
                        }
                        Ok(Some(str)) => {
                            prompt = format!("{}.. ", str);
                        }
                        Err((expected, got)) => {
                            println!("Error: expected closing {}, got {}", expected, got);
                            input_buffer.pop();
                        }
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    ctrl_c_acc += 1;
                    if ctrl_c_acc == 2 {
                        break;
                    } else {
                        println!("{}", yellow!("Hit CTRL-C a second time to quit."));
                    }
                }
                Err(ReadlineError::Eof) => {
                    println!("CTRL-D");
                    break;
                }
                Err(err) => {
                    println!("Error: {:?}", err);
                    break;
                }
            }
        }
        editor
            .save_history(HISTORY_FILE.unwrap_or("history.txt"))
            .unwrap();
    }
}

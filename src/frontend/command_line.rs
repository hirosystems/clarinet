
use crate::repl::Session;

use std::io::{Write, stdout, stdin};
use termion::{color, style};
use rustyline::error::ReadlineError;
use rustyline::Editor;

pub struct CommandLine <'a> {
    session: Session<'a>,
}

impl <'a> CommandLine <'a> {
    pub fn new() -> CommandLine<'a> {
        CommandLine {
            session: Session::new()
        }
    }

    pub fn start(&mut self) {
        println!("{}clarity-repl v1.0{}", color::Fg(color::LightGreen), color::Fg(color::LightBlack));
        println!("Enter \".help\" for usage hints.");
        println!("Connected to a transient in-memory database.{}", color::Fg(color::White));

        let mut editor = Editor::<()>::new();
        let mut ctrl_c_acc = 0;
        loop {
            let readline = editor.readline(">> ");
            match readline {
                Ok(command) => {
                    match command.as_str() {
                        ".help" => self.display_help(),
                        snippet => {
                            let result = self.session.interpret(snippet.to_string());
                            match result {
                                Ok(result) => println!("{}", result),
                                Err(error) => println!("{}{}{}", color::Fg(color::LightRed), error, color::Fg(color::LightBlack))
                            }
                        }
                    }
                    ctrl_c_acc = 0;
                    editor.add_history_entry(command.as_str());
                },
                Err(ReadlineError::Interrupted) => {
                    ctrl_c_acc += 1;
                    if ctrl_c_acc == 2 {
                        break
                    } else {
                        println!("Hit CTRL-C a second time to quit.");
                    }
                },
                Err(ReadlineError::Eof) => {
                    println!("CTRL-D");
                    break
                },
                Err(err) => {
                    println!("Error: {:?}", err);
                    break
                }
            }
        }
        editor.save_history("history.txt").unwrap();
    }

    pub fn display_help(&self) {
        let help = 
".help\tDisplay help";
        println!("{}", help);
    }
}

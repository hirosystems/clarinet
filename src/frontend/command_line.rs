
use crate::repl::Session;

use std::io::{Write, stdout, stdin};
use ansi_term::{Style, Colour};
use rustyline::error::ReadlineError;
use rustyline::Editor;

pub struct CommandLine {
    session: Session,
}

impl CommandLine {
    pub fn new() -> CommandLine {
        CommandLine {
            session: Session::new()
        }
    }

    pub fn start(&mut self) {

        let light_green = Colour::Green.bold();
        let light_red = Colour::Red.bold();
        let light_black = Colour::Black.bold();

        println!("{}", light_green.paint("clarity-repl v1.0"));
        println!("{}", light_black.paint("Enter \".help\" for usage hints."));
        println!("{}", light_black.paint("Connected to a transient in-memory database."));

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
                                Err((message, diagnostic)) => {
                                    println!("{}", light_red.paint(message));
                                    if let Some(diagnostic) = diagnostic {
                                        if diagnostic.spans.len() > 0 {
                                            let lines = snippet.lines();
                                            let mut formatted_lines: Vec<String> = lines.map(|l| l.to_string()).collect();
                                            for span in diagnostic.spans {
                                                let first_line = span.start_line as usize - 1;
                                                let last_line = span.end_line as usize - 1;
                                                let mut pass = vec![];
    
                                                for (line_index, line) in formatted_lines.iter().enumerate() {
                                                    if line_index >= first_line && line_index <= last_line {
                                                        let (begin, end) = match (line_index == first_line, line_index == last_line) {
                                                            (true, true) => (span.start_column as usize - 1, span.end_column as usize - 1), // One line
                                                            (true, false) => (span.start_column as usize - 1, line.len() - 1),              // Multiline, first line
                                                            (false, false) => (0, line.len() - 1),                                          // Multiline, in between
                                                            (false, true) => (0, span.end_column as usize - 1),                             // Multiline, last line 
                                                        };
                                                        
                                                        let error_style = light_red.underline();
                                                        let formatted_line = format!("{}{}{}", &line[..begin], error_style.paint(&line[begin..=end]), &line[(end + 1)..]);
                                                        pass.push(formatted_line);
                                                    } else {
                                                        pass.push(line.clone());
                                                    }
                                                }
                                                formatted_lines = pass;
                                            }
                                            println!("{}", formatted_lines.join("\n"));
                                        }
                                    }
                                }
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

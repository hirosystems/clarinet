
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
            let readline = editor.readline("◇ ");
            match readline {
                Ok(command) => {
                    match command.as_str() {
                        ".help" => self.display_help(),
                        cmd if cmd.starts_with(".functions") => self.display_functions(),
                        cmd if cmd.starts_with(".doc") => self.display_doc(cmd),
                        snippet => {
                            let result = self.session.interpret(snippet.to_string());
                            match result {
                                Ok(result) => println!("{}", light_green.paint(result)),
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
        let help_colour = Colour::Yellow;
        let coming_soon_colour = Colour::Black.bold();

        println!("{}", help_colour.paint(".help\t\t\t\tDisplay help"));
        println!("{}", help_colour.paint(".functions\t\t\t\tDisplay all the native functions available in clarity"));
        println!("{}", help_colour.paint(".doc <function> \t\tDisplay documentation for a given native function fn-name"));
        println!("{}", coming_soon_colour.paint(".mint-stx <principal>\t\tMint STX balance for a given principal [coming soon]"));
        println!("{}", coming_soon_colour.paint(".get-block-height\t\tGet current block height [coming soon]"));
        println!("{}", coming_soon_colour.paint(".set-block-height <number>\tSet current block height [coming soon]"));
    }

    pub fn display_functions(&self) {
        let help_colour = Colour::Yellow;
        let api_reference_index = self.session.get_api_reference_index();
        println!("{}", help_colour.paint(api_reference_index.join("\n")));
    }

    pub fn display_doc(&self, command: &str) {
        let help_colour = Colour::Yellow;
        let help_accent_colour = Colour::Yellow.bold();
        let keyword = {
            let mut s = command.to_string();
            s = s.replace(".doc", "");
            s = s.replace(" ", "");
            s
        };
        match self.session.lookup_api_reference(&keyword) {
            Some(doc) => println!("{}", help_colour.paint(doc)),
            None => println!("{}", help_colour.paint("Function unknown")),
        };
    }
}

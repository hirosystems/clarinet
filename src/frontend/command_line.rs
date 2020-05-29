
use crate::repl::Session;

// use termion::event::{Key, Event, MouseEvent};
// use termion::input::{TermRead, MouseTerminal};
// use termion::raw::IntoRawMode;
use std::io::{Write, stdout, stdin};
use termion::{color, style};
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
        println!("{}clarity-repl v0.1{}", color::Fg(color::Green), color::Fg(color::White));
        let mut rl = Editor::<()>::new();
        if rl.load_history("history.txt").is_err() {
            println!("No previous history.");
        }
        let mut ctrl_c_acc = 0;
        loop {
            let readline = rl.readline(">> ");
            match readline {
                Ok(command) => {
                    ctrl_c_acc = 0;
                    rl.add_history_entry(command.as_str());
                    let res = self.session.interpret(command);
                    println!("{}", res);
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
        rl.save_history("history.txt").unwrap();
        }
}


        // for c in stdin.keys() {
        //     write!(stdout,
        //            "{}{}",
        //            termion::cursor::Goto(1, 1),
        //            termion::clear::CurrentLine)
        //             .unwrap();
    
        //     match c.unwrap() {
        //         Key::Char('q') => break,
        //         Key::Char(c) => println!("{}", c),
        //         Key::Alt(c) => println!("^{}", c),
        //         Key::Ctrl(c) => println!("*{}", c),
        //         Key::Esc => println!("ESC"),
        //         Key::Left => println!("←"),
        //         Key::Right => println!("→"),
        //         Key::Up => println!("↑"),
        //         Key::Down => println!("↓"),
        //         Key::Backspace => println!("×"),
        //         _ => {}
        //     }
        //     stdout.flush().unwrap();
        // }
    
    

        // let stdin = io::stdin();
        // for line in stdin.lock().lines() {
        //     let snippet = line.unwrap();

        //     let contract_identifier = QualifiedContractIdentifier::transient();
        //     let mut contract_ast = match ast::build_ast(&contract_identifier, &snippet, &mut ()) {
        //         Ok(res) => res,
        //         Err(parse_error) => {
        //             println!("Parse error: {:?}", parse_error);
        //             continue
        //         }
        //     };
    
        //     let mut db = AnalysisDatabase::new();
        //     let result = analysis::run_analysis(
        //         &contract_identifier, 
        //         &mut contract_ast.expressions,
        //         &mut db, 
        //         false,
        //         LimitedCostTracker::new_max_limit());
        
        //     println!("{:?}", result);
        // }
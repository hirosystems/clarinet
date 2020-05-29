
use crate::repl::Session;

use termion::event::{Key, Event, MouseEvent};
use termion::input::{TermRead, MouseTerminal};
use termion::raw::IntoRawMode;
use std::io::{Write, stdout, stdin};
use termion::{color, style};

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
        let stdin = stdin();
        let mut stdout = stdout().into_raw_mode().unwrap();

        for c in stdin.events() {
            let evt = c.unwrap();
            match evt {
                Event::Key(Key::Ctrl('c')) | Event::Key(Key::Ctrl('d')) => break,
                Event::Key(Key::Char(k)) => {
                    write!(stdout,"{}", k).unwrap();
                },
                Event::Key(Key::Backspace) => {
                    
                    // write!(stdout,"{}", k).unwrap();
                }
                e => {
                    println!("=> {:?}", e);
                }
            }
            stdout.flush().unwrap();
        }
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
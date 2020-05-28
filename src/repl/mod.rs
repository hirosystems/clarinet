use std::collections::VecDeque;
use std::io::{self, BufRead};

use super::clarity::analysis::AnalysisDatabase;
use super::clarity::types::QualifiedContractIdentifier;
use super::clarity::{ast, analysis};
use super::clarity::costs::LimitedCostTracker;

enum Command {
}

pub struct Session {
    session_id: u32,
    started_at: u32,
    commands: VecDeque<Command>,
    defined_functions: VecDeque<Command>,
    defined_contracts: VecDeque<Command>,
}

impl Session {

    pub fn new() -> Session {
        Session {
            session_id: 0,
            started_at: 0,
            commands: VecDeque::new(),
            defined_functions: VecDeque::new(),
            defined_contracts: VecDeque::new(),
        }
    }

    pub fn start(&mut self) {
        println!("clarity-repl v0.1");
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            let snippet = line.unwrap();

            let contract_identifier = QualifiedContractIdentifier::transient();
            let mut contract_ast = match ast::build_ast(&contract_identifier, &snippet, &mut ()) {
                Ok(res) => res,
                Err(parse_error) => {
                    println!("Parse error: {:?}", parse_error);
                    continue
                }
            };
    
            let mut db = AnalysisDatabase::new();
            let result = analysis::run_analysis(
                &contract_identifier, 
                &mut contract_ast.expressions,
                &mut db, 
                false,
                LimitedCostTracker::new_max_limit());
        
            println!("{:?}", result);
        }
    }
}
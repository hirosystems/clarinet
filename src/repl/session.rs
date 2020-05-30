use std::collections::VecDeque;
use crate::clarity::types::QualifiedContractIdentifier;
use super::ClarityInterpreter;
use crate::clarity::diagnostic::Diagnostic;

enum Command {
    LoadLocalContract(String),
    LoadDeployContract(String),
    UnloadContract(String),
    ExecuteSnippet(String),
    OpenSession,
    CloseSession,
}

pub struct Session {
    session_id: u32,
    started_at: u32,
    commands: VecDeque<Command>,
    defined_functions: VecDeque<Command>,
    defined_contracts: VecDeque<Command>,
    interpreter: ClarityInterpreter,
}

impl Session {

    pub fn new() -> Session {
        Session {
            session_id: 0,
            started_at: 0,
            commands: VecDeque::new(),
            defined_functions: VecDeque::new(),
            defined_contracts: VecDeque::new(),
            interpreter: ClarityInterpreter::new(),
        }
    }

    pub fn interpret(&mut self, snippet: String) -> Result<String, (String, Option<Diagnostic>)> {
    
        let contract_identifier = QualifiedContractIdentifier::transient();

        self.interpreter.run(snippet, contract_identifier)
    }
}

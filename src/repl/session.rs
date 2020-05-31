use std::collections::{VecDeque, HashMap};
use crate::clarity::types::QualifiedContractIdentifier;
use super::ClarityInterpreter;
use crate::clarity::diagnostic::Diagnostic;
use crate::clarity::functions::NativeFunctions;
use crate::clarity::functions::define::DefineFunctions;
use crate::clarity::variables::NativeVariables;
use crate::clarity::docs::{
    make_api_reference, 
    make_define_reference, 
    make_keyword_reference
};

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
    api_reference: HashMap<String, String>,
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
            api_reference: build_api_reference(),
        }
    }

    pub fn interpret(&mut self, snippet: String) -> Result<String, (String, Option<Diagnostic>)> {
    
        let contract_identifier = QualifiedContractIdentifier::transient();

        self.interpreter.run(snippet, contract_identifier)
    }

    pub fn lookup_api_reference(&self, keyword: &str) -> Option<&String> {
        self.api_reference.get(keyword)
    }

    pub fn get_api_reference_index(&self) -> Vec<String> {
        let mut keys = self.api_reference.iter()
            .map(|(k, _)| k.to_string())
            .collect::<Vec<String>>();
        keys.sort();
        keys
    }
}

fn build_api_reference() -> HashMap<String, String> {
    let mut api_reference = HashMap::new();
    for func in NativeFunctions::ALL.iter() {
        let api = make_api_reference(&func);
        let description = {
            let mut s = api.description.to_string();
            s = s.replace("\n", " ");
            s
        };
        let doc = format!("Usage\n{}\n\nDescription\n{}\n\nExamples\n{}",
            api.signature, description, api.example);
        api_reference.insert(api.name, doc);
    }

    for func in DefineFunctions::ALL.iter() {
        let api = make_define_reference(&func);
        let description = {
            let mut s = api.description.to_string();
            s = s.replace("\n", " ");
            s
        };
        let doc = format!("Usage\n{}\n\nDescription\n{}\n\nExamples\n{}",
            api.signature, description, api.example);
        api_reference.insert(api.name, doc);
    }

    for func in NativeVariables::ALL.iter() {
        let api = make_keyword_reference(&func);
        let description = {
            let mut s = api.description.to_string();
            s = s.replace("\n", " ");
            s
        };
        let doc = format!("Description\n{}\n\nExamples\n{}",
            description, api.example);
        api_reference.insert(api.name.to_string(), doc);
    }
    api_reference
}

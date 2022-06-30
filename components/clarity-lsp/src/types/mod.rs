use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionItemKind,
    pub detail: Option<String>,
    pub markdown_documentation: Option<String>,
    pub insert_text: Option<String>,
    pub insert_text_format: InsertTextFormat,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum CompletionItemKind {
    Module,
    Event,
    Function,
    Class,
    Field,
    TypeParameter,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum InsertTextFormat {
    Snippet,
    PlainText,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
enum Symbol {
    PublicFunction,
    ReadonlyFunction,
    PrivateFunction,
    ImportedTrait,
    LocalVariable,
    Constant,
    DataMap,
    DataVar,
    FungibleToken,
    NonFungibleToken,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct CompletionMaps {
    pub inter_contract: Vec<CompletionItem>,
    pub intra_contract: Vec<CompletionItem>,
    pub data_fields: Vec<CompletionItem>,
}

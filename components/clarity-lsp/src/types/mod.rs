use lsp_types::CompletionItem;
use serde::{Deserialize, Serialize};

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

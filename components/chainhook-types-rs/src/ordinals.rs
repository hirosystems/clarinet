use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OrdinalOperation {
    InscriptionRevealed(OrdinalInscriptionRevealData),
    InscriptionTransferred(OrdinalInscriptionTransferData),
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct OrdinalInscriptionTransferData {
    pub ordinal_number: u64,
    pub destination: OrdinalInscriptionTransferDestination,
    pub satpoint_pre_transfer: String,
    pub satpoint_post_transfer: String,
    pub post_transfer_output_value: Option<u64>,
    pub tx_index: usize,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum OrdinalInscriptionTransferDestination {
    Transferred(String),
    SpentInFees,
    Burnt(String),
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum OrdinalInscriptionCurseType {
    DuplicateField,
    IncompleteField,
    NotAtOffsetZero,
    NotInFirstInput,
    Pointer,
    Pushnum,
    Reinscription,
    Stutter,
    UnrecognizedEvenField,
    Generic,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct OrdinalInscriptionCharms {
    pub coin: bool,
    pub cursed: bool,
    pub epic: bool,
    pub legendary: bool,
    pub lost: bool,
    pub nineball: bool,
    pub rare: bool,
    pub reinscription: bool,
    pub unbound: bool,
    pub uncommon: bool,
    pub vindicated: bool,
    pub mythic: bool,
    pub burned: bool,
    pub palindrome: bool,
}

impl OrdinalInscriptionCharms {
    pub fn none() -> Self {
        OrdinalInscriptionCharms {
            coin: false,
            cursed: false,
            epic: false,
            legendary: false,
            lost: false,
            nineball: false,
            rare: false,
            reinscription: false,
            unbound: false,
            uncommon: false,
            vindicated: false,
            mythic: false,
            burned: false,
            palindrome: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct OrdinalInscriptionRevealData {
    pub content_bytes: String,
    pub content_type: String,
    pub content_length: usize,
    pub inscription_number: OrdinalInscriptionNumber,
    pub inscription_fee: u64,
    pub inscription_output_value: u64,
    pub inscription_id: String,
    pub inscription_input_index: usize,
    pub inscription_pointer: Option<u64>,
    pub inscriber_address: Option<String>,
    pub delegate: Option<String>,
    pub metaprotocol: Option<String>,
    pub metadata: Option<Value>,
    pub parents: Vec<String>,
    pub ordinal_number: u64,
    pub ordinal_block_height: u64,
    pub ordinal_offset: u64,
    pub tx_index: usize,
    pub transfers_pre_inscription: u32,
    pub satpoint_post_inscription: String,
    pub curse_type: Option<OrdinalInscriptionCurseType>,
    pub charms: OrdinalInscriptionCharms,
}

impl OrdinalInscriptionNumber {
    pub fn zero() -> Self {
        OrdinalInscriptionNumber {
            jubilee: 0,
            classic: 0,
        }
    }
}

impl OrdinalInscriptionRevealData {
    pub fn get_inscription_number(&self) -> i64 {
        self.inscription_number.jubilee
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct OrdinalInscriptionNumber {
    pub classic: i64,
    pub jubilee: i64,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Brc20TokenDeployData {
    pub tick: String,
    pub max: String,
    pub lim: String,
    pub dec: String,
    pub address: String,
    pub inscription_id: String,
    pub self_mint: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Brc20BalanceData {
    pub tick: String,
    pub amt: String,
    pub address: String,
    pub inscription_id: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Brc20TransferData {
    pub tick: String,
    pub amt: String,
    pub sender_address: String,
    pub receiver_address: String,
    pub inscription_id: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Brc20Operation {
    Deploy(Brc20TokenDeployData),
    Mint(Brc20BalanceData),
    Transfer(Brc20BalanceData),
    TransferSend(Brc20TransferData),
}

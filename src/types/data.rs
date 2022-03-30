use super::events::StacksTransactionEvent;
use serde::{self, Deserialize, Serialize};
use std::collections::HashSet;

/// BlockIdentifier uniquely identifies a block in a particular network.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct BlockIdentifier {
    /// Also known as the block height.
    pub index: u64,
    pub hash: String,
}

/// StacksBlock contain an array of Transactions that occurred at a particular
/// BlockIdentifier. A hard requirement for blocks returned by Rosetta
/// implementations is that they MUST be _inalterable_: once a client has
/// requested and received a block identified by a specific BlockIndentifier,
/// all future calls for that same BlockIdentifier must return the same block
/// contents.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct StacksBlockData {
    pub block_identifier: BlockIdentifier,
    pub parent_block_identifier: BlockIdentifier,
    /// The timestamp of the block in milliseconds since the Unix Epoch. The
    /// timestamp is stored in milliseconds because some blockchains produce
    /// blocks more often than once a second.
    pub timestamp: i64,
    pub transactions: Vec<StacksTransactionData>,
    pub metadata: StacksBlockMetadata,
}

/// StacksMicroblockData contain an array of Transactions that occurred at a particular
/// BlockIdentifier. A hard requirement for blocks returned by Rosetta
/// implementations is that they MUST be _inalterable_: once a client has
/// requested and received a block identified by a specific BlockIndentifier,
/// all future calls for that same BlockIdentifier must return the same block
/// contents.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct StacksMicroblockData {
    pub block_identifier: BlockIdentifier,
    pub parent_block_identifier: BlockIdentifier,
    pub transactions: Vec<StacksTransactionData>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct StacksMicroblocksTrail {
    pub microblocks: Vec<StacksMicroblockData>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct StacksBlockMetadata {
    pub bitcoin_anchor_block_identifier: BlockIdentifier,
    pub pox_cycle_index: u32,
    pub pox_cycle_position: u32,
    pub pox_cycle_length: u32,
}

/// BitcoinBlock contain an array of Transactions that occurred at a particular
/// BlockIdentifier. A hard requirement for blocks returned by Rosetta
/// implementations is that they MUST be _inalterable_: once a client has
/// requested and received a block identified by a specific BlockIndentifier,
/// all future calls for that same BlockIdentifier must return the same block
/// contents.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct BitcoinBlockData {
    pub block_identifier: BlockIdentifier,
    pub parent_block_identifier: BlockIdentifier,
    /// The timestamp of the block in milliseconds since the Unix Epoch. The
    /// timestamp is stored in milliseconds because some blockchains produce
    /// blocks more often than once a second.
    pub timestamp: u32,
    pub transactions: Vec<BitcoinTransactionData>,
    pub metadata: BitcoinBlockMetadata,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct BitcoinBlockMetadata {}

/// The timestamp of the block in milliseconds since the Unix Epoch. The
/// timestamp is stored in milliseconds because some blockchains produce blocks
/// more often than once a second.
#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct Timestamp(i64);

/// Transactions contain an array of Operations that are attributable to the
/// same TransactionIdentifier.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct StacksTransactionData {
    pub transaction_identifier: TransactionIdentifier,
    pub operations: Vec<Operation>,
    /// Transactions that are related to other transactions should include the
    /// transaction_identifier of these transactions in the metadata.
    pub metadata: StacksTransactionMetadata,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum StacksTransactionKind {
    ContractCall,
    ContractDeployment(StacksContractDeploymentData),
    NativeTokenTransfer,
    Coinbase,
    Other,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct StacksContractDeploymentData {
    pub contract_identifier: String,
    pub code: String,
}

/// Extra data for Transaction
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct StacksTransactionMetadata {
    pub success: bool,
    pub raw_tx: String,
    pub result: String,
    pub sender: String,
    pub fee: u64,
    pub kind: StacksTransactionKind,
    pub execution_cost: Option<StacksTransactionExecutionCost>,
    pub receipt: StacksTransactionReceipt,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sponsor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct StacksTransactionExecutionCost {
    pub write_length: u64,
    pub write_count: u64,
    pub read_length: u64,
    pub read_count: u64,
    pub runtime: u64,
}

/// Extra event data for Transaction
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct StacksTransactionReceipt {
    pub mutated_contracts_radius: HashSet<String>,
    pub mutated_assets_radius: HashSet<String>,
    pub events: Vec<StacksTransactionEvent>,
}

impl StacksTransactionReceipt {
    pub fn new(
        mutated_contracts_radius: HashSet<String>,
        mutated_assets_radius: HashSet<String>,
        events: Vec<StacksTransactionEvent>,
    ) -> StacksTransactionReceipt {
        StacksTransactionReceipt {
            mutated_contracts_radius,
            mutated_assets_radius,
            events,
        }
    }
}

/// Transactions contain an array of Operations that are attributable to the
/// same TransactionIdentifier.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct BitcoinTransactionData {
    pub transaction_identifier: TransactionIdentifier,
    pub operations: Vec<Operation>,
    /// Transactions that are related to other transactions should include the
    /// transaction_identifier of these transactions in the metadata.
    pub metadata: BitcoinTransactionMetadata,
}

/// Extra data for Transaction
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct BitcoinTransactionMetadata {}

/// The transaction_identifier uniquely identifies a transaction in a particular
/// network and block or in the mempool.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct TransactionIdentifier {
    /// Any transactions that are attributable only to a block (ex: a block
    /// event) should use the hash of the block as the identifier.
    pub hash: String,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, strum::EnumIter, strum::IntoStaticStr,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OperationType {
    Credit,
    Debit,
    Lock,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct OperationMetadata {
    /// Has to be specified for ADD_KEY, REMOVE_KEY, and STAKE operations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_key: Option<PublicKey>,
    // TODO(lgalabru): ???
    //#[serde(skip_serializing_if = "Option::is_none")]
    // pub access_key: Option<TODO>,
    /// Has to be specified for DEPLOY_CONTRACT operation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    /// Has to be specified for FUNCTION_CALL operation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method_name: Option<String>,
    /// Has to be specified for FUNCTION_CALL operation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<String>,
}

/// PublicKey contains a public key byte array for a particular CurveType
/// encoded in hex. Note that there is no PrivateKey struct as this is NEVER the
/// concern of an implementation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PublicKey {
    /// Hex-encoded public key bytes in the format specified by the CurveType.
    pub hex_bytes: Option<String>,
    pub curve_type: CurveType,
}

/// CurveType is the type of cryptographic curve associated with a PublicKey.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CurveType {
    /// `y (255-bits) || x-sign-bit (1-bit)` - `32 bytes` (https://ed25519.cr.yp.to/ed25519-20110926.pdf)
    Edwards25519,
    /// SEC compressed - `33 bytes` (https://secg.org/sec1-v2.pdf#subsubsection.2.3.3)
    Secp256k1,
}

/// Operations contain all balance-changing information within a transaction.
/// They are always one-sided (only affect 1 AccountIdentifier) and can
/// succeed or fail independently from a Transaction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Operation {
    pub operation_identifier: OperationIdentifier,

    /// Restrict referenced related_operations to identifier indexes < the
    /// current operation_identifier.index. This ensures there exists a clear
    /// DAG-structure of relations. Since operations are one-sided, one could
    /// imagine relating operations in a single transfer or linking operations
    /// in a call tree.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub related_operations: Option<Vec<OperationIdentifier>>,

    /// The network-specific type of the operation. Ensure that any type that
    /// can be returned here is also specified in the NetworkStatus. This can
    /// be very useful to downstream consumers that parse all block data.
    #[serde(rename = "type")]
    pub type_: OperationType,

    /// The network-specific status of the operation. Status is not defined on
    /// the transaction object because blockchains with smart contracts may have
    /// transactions that partially apply. Blockchains with atomic transactions
    /// (all operations succeed or all operations fail) will have the same
    /// status for each operation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<OperationStatusKind>,

    pub account: AccountIdentifier,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<Amount>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<OperationMetadata>,
}

/// The operation_identifier uniquely identifies an operation within a
/// transaction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperationIdentifier {
    /// The operation index is used to ensure each operation has a unique
    /// identifier within a transaction. This index is only relative to the
    /// transaction and NOT GLOBAL. The operations in each transaction should
    /// start from index 0. To clarify, there may not be any notion of an
    /// operation index in the blockchain being described.
    pub index: u32,

    /// Some blockchains specify an operation index that is essential for
    /// client use. For example, Bitcoin uses a network_index to identify
    /// which UTXO was used in a transaction.  network_index should not be
    /// populated if there is no notion of an operation index in a blockchain
    /// (typically most account-based blockchains).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network_index: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize, strum::EnumIter)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OperationStatusKind {
    Success,
}

/// The account_identifier uniquely identifies an account within a network. All
/// fields in the account_identifier are utilized to determine this uniqueness
/// (including the metadata field, if populated).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct AccountIdentifier {
    /// The address may be a cryptographic public key (or some encoding of it)
    /// or a provided username.
    pub address: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_account: Option<SubAccountIdentifier>,
    /* Rosetta Spec also optionally provides:
     *
     * /// Blockchains that utilize a username model (where the address is not a
     * /// derivative of a cryptographic public key) should specify the public
     * /// key(s) owned by the address in metadata.
     * #[serde(skip_serializing_if = "Option::is_none")]
     * pub metadata: Option<serde_json::Value>, */
}

/// An account may have state specific to a contract address (ERC-20 token)
/// and/or a stake (delegated balance). The sub_account_identifier should
/// specify which state (if applicable) an account instantiation refers to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct SubAccountIdentifier {
    /// The SubAccount address may be a cryptographic value or some other
    /// identifier (ex: bonded) that uniquely specifies a SubAccount.
    pub address: SubAccount,
    /* Rosetta Spec also optionally provides:
     *
     * /// If the SubAccount address is not sufficient to uniquely specify a
     * /// SubAccount, any other identifying information can be stored here.  It is
     * /// important to note that two SubAccounts with identical addresses but
     * /// differing metadata will not be considered equal by clients.
     * #[serde(skip_serializing_if = "Option::is_none")]
     * pub metadata: Option<serde_json::Value>, */
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SubAccount {
    LiquidBalanceForStorage,
    Locked,
}

/// Amount is some Value of a Currency. It is considered invalid to specify a
/// Value without a Currency.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Amount {
    /// Value of the transaction in atomic units represented as an
    /// arbitrary-sized signed integer.  For example, 1 BTC would be represented
    /// by a value of 100000000.
    pub value: u64,

    pub currency: Currency,
    /* Rosetta Spec also optionally provides:
     *
     * #[serde(skip_serializing_if = "Option::is_none")]
     * pub metadata: Option<serde_json::Value>, */
}

/// Currency is composed of a canonical Symbol and Decimals. This Decimals value
/// is used to convert an Amount.Value from atomic units (Satoshis) to standard
/// units (Bitcoins).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Currency {
    /// Canonical symbol associated with a currency.
    pub symbol: String,

    /// Number of decimal places in the standard unit representation of the
    /// amount.  For example, BTC has 8 decimals. Note that it is not possible
    /// to represent the value of some currency in atomic units that is not base
    /// 10.
    pub decimals: u32,

    /// Any additional information related to the currency itself.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<CurrencyMetadata>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CurrencyStandard {
    Sip09,
    Sip10,
    None,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CurrencyMetadata {
    pub asset_class_identifier: String,
    pub asset_identifier: Option<String>,
    pub standard: CurrencyStandard,
}

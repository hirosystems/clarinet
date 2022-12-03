/**
 * BitcoinChainUpdate provide informations about new blocks and confirmed blocks.
 * @export
 * @interface BitcoinChainUpdate
 */
export interface BitcoinChainUpdate {
  /**
   * @type {Array<Block>}
   * @memberof BitcoinChainUpdate
   */
  new_blocks: Array<Block>;
  /**
   * @type {Array<Block>}
   * @memberof BitcoinChainUpdate
   */
  confirmed_blocks: Array<Block>;
}

/**
 * StacksChainUpdate provide informations about new blocks and confirmed blocks.
 * @export
 * @interface StacksChainUpdate
 */
export interface StacksChainUpdate {
  /**
   * @type {Array<StacksBlockUpdate>}
   * @memberof StacksChainUpdate
   */
  new_blocks: Array<StacksBlockUpdate>;
  /**
   * @type {Array<Block>}
   * @memberof StacksChainUpdate
   */
  confirmed_blocks: Array<Block>;
}

/**
 * StacksBlockUpdate provide informations about new blocks and confirmed blocks.
 * @export
 * @interface StacksBlockUpdate
 */
export interface StacksBlockUpdate {
  /**
   * @type {Block}
   * @memberof StacksBlockUpdate
   */
  block: Block;
  /**
   * @type {Array<Block>}
   * @memberof StacksBlockUpdate
   */
  parent_microblocks_to_rollback: Array<Block>;
  /**
   * @type {Array<Block>}
   * @memberof StacksBlockUpdate
   */
  parent_microblocks_to_apply: Array<Block>;
}

export interface BitcoinChainEvent {
  apply: Block[];
  rollback: Block[];
  chainhook: {
    uuid: string;
    predicate: BitcoinPredicate;
  };
}

export interface StacksChainEvent {
  apply: Block[];
  rollback: Block[];
  chainhook: {
    uuid: string;
    predicate: StacksPredicate;
  };
}

export interface StacksChainhook {
  uuid: string;
  predicate: StacksPredicate;
}

export interface StacksPredicate {
  type: StacksPredicateType;
  rule:
    | StacksContractCallBasedPredicate
    | StacksPrintEventBasedPredicate
    | StacksFtEventBasedPredicate
    | StacksNftEventBasedPredicate
    | StacksStxEventBasedPredicate;
}

export enum StacksPredicateType {
  ContractCall = "contract_call",
  PrintEvent = "print_event",
  FtEvent = "ft_event",
  NftEvent = "nft_event",
  StxEvent = "stx_event",
}

export interface BitcoinChainhook {
  uuid: string;
  predicate: BitcoinPredicate;
}

export enum BitcoinPredicateScope {
  Inputs = "inputs",
  Outputs = "outputs",
}

export interface BitcoinPredicate {
  scope: BitcoinPredicateScope;
  type: BitcoinPredicateType;
  rule: BitcoinPredicateMatchingRule;
}

export enum BitcoinPredicateType {
  Hex = "hex",
  P2pkh = "p2pkh",
  P2sh = "p2sh",
  P2wpkh = "p2wpkh",
  P2wsh = "p2wsh",
  Script = "script",
}

export interface BitcoinPredicateMatchingRule {
  equals?: string;
  starts_with?: string;
  ends_with?: string;
}

export interface StacksPrintEventBasedPredicate {
  contract_identifier: string;
  contains: string;
}

export interface StacksFtEventBasedPredicate {
  asset_identifier: string;
  actions: string[];
}

export interface StacksContractCallBasedPredicate {
  contract_identifier: string;
  method: string;
}

export interface StacksPrintEventBasedPredicate {
  contract_identifier: string;
  contains: string;
}

export interface StacksFtEventBasedPredicate {
  asset_identifier: string;
  actions: string[];
}

export interface StacksNftEventBasedPredicate {
  asset_identifier: string;
  actions: string[];
}

export interface StacksStxEventBasedPredicate {
  actions: string[];
}

/**
 * In blockchains with sharded state, the SubNetworkIdentifier is required to query some object on a specific shard. This identifier is optional for all non-sharded blockchains.
 * @export
 * @interface SubNetworkIdentifier
 */
export interface SubNetworkIdentifier {
  /**
   * @type {string}
   * @memberof SubNetworkIdentifier
   */
  network: string;
  /**
   * @type {object}
   * @memberof SubNetworkIdentifier
   */
  metadata?: object;
}

/**
 * The network_identifier specifies which network a particular object is associated with.
 * @export
 * @interface NetworkIdentifier
 */
export interface NetworkIdentifier {
  /**
   * @type {string}
   * @memberof NetworkIdentifier
   */
  blockchain: string;
  /**
   * If a blockchain has a specific chain-id or network identifier, it should go in this field. It is up to the client to determine which network-specific identifier is mainnet or testnet.
   * @type {string}
   * @memberof NetworkIdentifier
   */
  network: string;
  /**
   * @type {SubNetworkIdentifier}
   * @memberof NetworkIdentifier
   */
  sub_network_identifier?: SubNetworkIdentifier;
}

/**
 * Used by RelatedTransaction to indicate the direction of the relation (i.e. cross-shard/cross-network sends may reference `backward` to an earlier transaction and async execution may reference `forward`). Can be used to indicate if a transaction relation is from child to parent or the reverse.
 * @export
 * @enum {string}
 */
export enum Direction {
  forward = "forward",
  backward = "backward",
}

/**
 * The related_transaction allows implementations to link together multiple transactions. An unpopulated network identifier indicates that the related transaction is on the same network.
 * @export
 * @interface RelatedTransaction
 */
export interface RelatedTransaction {
  /**
   * @type {NetworkIdentifier}
   * @memberof RelatedTransaction
   */
  network_identifier?: NetworkIdentifier;
  /**
   * @type {TransactionIdentifier}
   * @memberof RelatedTransaction
   */
  transaction_identifier: TransactionIdentifier;
  /**
   * @type {Direction}
   * @memberof RelatedTransaction
   */
  direction: Direction;
}

/**
 * The transaction_identifier uniquely identifies a transaction in a particular network and block or in the mempool.
 * @export
 * @interface TransactionIdentifier
 */
export interface TransactionIdentifier {
  /**
   * Any transactions that are attributable only to a block (ex: a block event) should use the hash of the block as the identifier.
   * @type {string}
   * @memberof TransactionIdentifier
   */
  hash: string;
}

/**
 * StacksTransactionMetadata contain an specific data about Stacks transactions.
 * @export
 * @interface StacksTransactionMetadata
 */
export interface StacksTransactionMetadata {
  /**
   * @type {boolean}
   * @memberof StacksTransactionMetadata
   */
  success: boolean;
  /**
   * @type {string}
   * @memberof StacksTransactionMetadata
   */
  result: string;
  /**
   * @type {string[]}
   * @memberof StacksTransactionMetadata
   */
  events: string[];
  /**
   * @type {string}
   * @memberof StacksTransactionMetadata
   */
  description: string;
  /**
   * @type {string}
   * @memberof StacksTransactionMetadata
   */
  raw_tx: string;
  /**
   * @type {string}
   * @memberof StacksTransactionMetadata
   */
  sender: string;
  /**
   * @type {string}
   * @memberof StacksTransactionMetadata
   */
  sponsor?: string;
  /**
   * @type {number}
   * @memberof StacksTransactionMetadata
   */
  fee: number;
  /**
   * @type {number}
   * @memberof StacksTransactionMetadata
   */
  nonce: number;
  /**
   * @type {StacksTransactionKind}
   * @memberof StacksTransactionMetadata
   */
  kind: StacksTransactionKind;
  /**
   * @type {StacksTransactionReceipt}
   * @memberof StacksTransactionMetadata
   */
  receipt: StacksTransactionReceipt;
  /**
   * @type {StacksTransactionExecutionCost}
   * @memberof StacksTransactionMetadata
   */
  execution_cost?: StacksTransactionExecutionCost;
  /**
   * @type {AnchorBlockPosition | MicroBlockPosition}
   * @memberof StacksTransactionMetadata
   */
  position: AnchorBlockPosition | MicroBlockPosition;
   /**
   * @type {string}
   * @memberof StacksTransactionMetadata
   */
  proof?: string;
}

/**
 * MicroBlockPosition
 * @export
 * @interface MicroBlockPosition
 */
 export interface MicroBlockPosition {
  micro_block_identifier: BlockIdentifier,
  index: number
}

/**
 * AnchorBlockPosition
 * @export
 * @interface AnchorBlockPosition
 */
 export interface AnchorBlockPosition {
  index: number
}

export interface StacksTransactionReceipt {
  /**
   * @type {string[]}
   * @memberof StacksTransactionReceipt
   */
  mutated_contracts_radius: string[];
  /**
   * @type {string[]}
   * @memberof StacksTransactionReceipt
   */
  mutated_assets_radius: string[];
  /**
   * @type {StacksTransactionEvent[]}
   * @memberof StacksTransactionReceipt
   */
  events: StacksTransactionEvent[];
}

export interface StacksTransactionEvent {
  type: StacksTransactionEventType;
  data:
    | StacksSTXTransferEventData
    | StacksSTXMintEventData
    | StacksSTXLockEventData
    | StacksSTXBurnEventData
    | StacksNFTTransferEventData
    | StacksNFTMintEventData
    | StacksNFTBurnEventData
    | StacksFTTransferEventData
    | StacksFTMintEventData
    | StacksFTBurnEventData
    | StacksDataVarSetEventData
    | StacksDataMapInsertEventData
    | StacksDataMapUpdateEventData
    | StacksDataMapDeleteEventData
    | StacksSmartContractEventData;
}

export interface StacksContractDeploymentData {
  /**
   * @type {string}
   * @memberof StacksContractDeploymentData
   */
  contract_identifier: string;
  /**
   * @type {string[]}
   * @memberof StacksContractDeploymentData
   */
  code: string[];
}

export interface StacksTransactionExecutionCost {
  /**
   * @type {number}
   * @memberof StacksTransactionExecutionCost
   */
  write_length: number;
  /**
   * @type {number}
   * @memberof StacksTransactionExecutionCost
   */
  write_count: number;
  /**
   * @type {number}
   * @memberof StacksTransactionExecutionCost
   */
  read_length: number;
  /**
   * @type {number}
   * @memberof StacksTransactionExecutionCost
   */
  read_count: number;
  /**
   * @type {number}
   * @memberof StacksTransactionExecutionCost
   */
  runtime: number;
}

export enum StacksTransactionKind {
  ContractCall = "ContractCall",
  ContractDeployment = "ContractDeployment",
  NativeTokenTransfer = "NativeTokenTransfer",
  Coinbase = "Coinbase",
  Other = "Other",
}

export enum StacksTransactionEventType {
  StacksSTXTransferEvent = "STXTransferEvent",
  StacksSTXMintEvent = "STXMintEvent",
  StacksSTXLockEvent = "STXLockEvent",
  StacksSTXBurnEvent = "STXBurnEvent",
  StacksNFTTransferEvent = "NFTTransferEvent",
  StacksNFTMintEvent = "NFTMintEvent",
  StacksNFTBurnEvent = "NFTBurnEvent",
  StacksFTTransferEvent = "FTTransferEvent",
  StacksFTMintEvent = "FTMintEvent",
  StacksFTBurnEvent = "FTBurnEvent",
  StacksDataVarSetEvent = "DataVarSetEvent",
  StacksDataMapInsertEvent = "DataMapInsertEvent",
  StacksDataMapUpdateEvent = "DataMapUpdateEvent",
  StacksDataMapDeleteEvent = "DataMapDeleteEvent",
  StacksSmartContractEvent = "SmartContractEvent",
}

export interface StacksSTXTransferEventData {
  sender: string;
  recipient: string;
  amount: string;
  memo?: string;
}

export interface StacksSTXMintEventData {
  recipient: string;
  amount: string;
}

export interface StacksSTXLockEventData {
  locked_amount: string;
  unlock_height: string;
  locked_address: string;
}

export interface StacksSTXBurnEventData {
  sender: string;
  amount: string;
}

export interface StacksNFTTransferEventData {
  asset_class_identifier: string;
  asset_identifier: string;
  sender: string;
  recipient: string;
}

export interface StacksNFTMintEventData {
  asset_class_identifier: string;
  asset_identifier: string;
  recipient: string;
}

export interface StacksNFTBurnEventData {
  asset_class_identifier: string;
  asset_identifier: string;
  sender: string;
}

export interface StacksFTTransferEventData {
  asset_identifier: string;
  sender: string;
  recipient: string;
  amount: string;
}

export interface StacksFTMintEventData {
  asset_identifier: string;
  recipient: string;
  amount: string;
}

export interface StacksFTBurnEventData {
  asset_identifier: string;
  sender: string;
  amount: string;
}

export interface StacksDataVarSetEventData {
  contract_identifier: string;
  var: string;
  new_value: string;
}

export interface StacksDataMapInsertEventData {
  contract_identifier: String;
  map: string;
  inserted_key: string;
  inserted_value: string;
}

export interface StacksDataMapUpdateEventData {
  contract_identifier: string;
  map: string;
  key: string;
  new_value: string;
}

export interface StacksDataMapDeleteEventData {
  contract_identifier: string;
  map: string;
  deleted_key: string;
}

export interface StacksSmartContractEventData {
  contract_identifier: string;
  topic: string;
  value: string;
}

/**
 * BitcoinTransactionMetadata contain an specific data about Bitcoin transactions.
 * @export
 * @interface BitcoinTransactionMetadata
 */
export interface BitcoinTransactionMetadata {
  inputs: Input[];
  outputs: Output[];
  /**
   * @type {string}
   * @memberof StacksTransactionMetadata
   */
  proof?: string;
}

export interface Input {
  previous_output: string;
  script_sig: string;
  sequence: number;
  witness: any[];
}

export interface Output {
  value: number;
  script_pubkey: string;
}

/**
 * StacksTransaction contain an array of Operations that are attributable to the same TransactionIdentifier.
 * @export
 * @interface StacksTransaction
 */
export interface StacksTransaction {
  /**
   * @type {TransactionIdentifier}
   * @memberof Transaction
   */
  transaction_identifier: TransactionIdentifier;
  /**
   * @type {Array<Operation>}
   * @memberof Transaction
   */
  operations: Array<Operation>;
  /**
   * @type {Array<RelatedTransaction>}
   * @memberof Transaction
   */
  related_transactions?: Array<RelatedTransaction>;
  /**
   * Transactions that are related to other transactions (like a cross-shard transaction) should include the tranaction_identifier of these transactions in the metadata.
   * @type {object}
   * @memberof StacksTransactionMetadata
   */
  metadata: StacksTransactionMetadata;
}

/**
 * BitcoinTransaction contain an array of Operations that are attributable to the same TransactionIdentifier.
 * @export
 * @interface BitcoinTransaction
 */
export interface BitcoinTransaction {
  /**
   * @type {TransactionIdentifier}
   * @memberof Transaction
   */
  transaction_identifier: TransactionIdentifier;
  /**
   * @type {Array<Operation>}
   * @memberof Transaction
   */
  operations: Array<Operation>;
  /**
   * @type {Array<RelatedTransaction>}
   * @memberof Transaction
   */
  related_transactions?: Array<RelatedTransaction>;
  /**
   * Transactions that are related to other transactions (like a cross-shard transaction) should include the tranaction_identifier of these transactions in the metadata.
   * @type {object}
   * @memberof BitcoinTransactionMetadata
   */
  metadata: BitcoinTransactionMetadata;
}

/**
 * Transactions contain an array of Operations that are attributable to the same TransactionIdentifier.
 * @export
 * @interface Transaction
 */
export interface Transaction {
  /**
   * @type {TransactionIdentifier}
   * @memberof Transaction
   */
  transaction_identifier: TransactionIdentifier;
  /**
   * @type {Array<Operation>}
   * @memberof Transaction
   */
  operations: Array<Operation>;
  /**
   * @type {Array<RelatedTransaction>}
   * @memberof Transaction
   */
  related_transactions?: Array<RelatedTransaction>;
  /**
   * Transactions that are related to other transactions (like a cross-shard transaction) should include the tranaction_identifier of these transactions in the metadata.
   * @type {object}
   * @memberof Transaction
   */
  metadata?: StacksTransactionMetadata | BitcoinTransactionMetadata;
}

/**
 * StacksBlockMetadata contains specific data about Stacks blocks.
 * @export
 * @interface StacksBlockMetadata
 */
export interface StacksBlockMetadata {
  /**
   * @type {BlockIdentifier}
   * @memberof StacksBlockMetadata
   */
  bitcoin_anchor_block_identifier: BlockIdentifier;
  /**
   * @type {BlockIdentifier}
   * @memberof StacksBlockMetadata
   */
  confirm_microblock_identifier?: BlockIdentifier;
  /**
   * @type {number}
   * @memberof StacksBlockMetadata
   */
  pox_cycle_index: number;
  /**
   * @type {number}
   * @memberof StacksBlockMetadata
   */
  pox_cycle_position: number;
  /**
   * @type {number}
   * @memberof StacksBlockMetadata
   */
  pox_cycle_length: number;
}

/**
 * BitcoinBlockMetadata contains specific data about Bitcoin blocks.
 * @export
 * @interface BitcoinBlockMetadata
 */
export interface BitcoinBlockMetadata {}

/**
 * The block_identifier uniquely identifies a block in a particular network.
 * @export
 * @interface BlockIdentifier
 */
export interface BlockIdentifier {
  /**
   * This is also known as the block height.
   * @type {number}
   * @memberof BlockIdentifier
   */
  index: number;
  /**
   * @type {string}
   * @memberof BlockIdentifier
   */
  hash: string;
}

/**
 * Blocks contain an array of Transactions that occurred at a particular BlockIdentifier. A hard requirement for blocks returned by Rosetta implementations is that they MUST be _inalterable_: once a client has requested and received a block identified by a specific BlockIndentifier, all future calls for that same BlockIdentifier must return the same block contents.
 * @export
 * @interface Block
 */
export interface Block {
  /**
   * @type {BlockIdentifier}
   * @memberof Block
   */
  block_identifier: BlockIdentifier;
  /**
   * @type {BlockIdentifier}
   * @memberof Block
   */
  parent_block_identifier: BlockIdentifier;
  /**
   * The timestamp of the block in milliseconds since the Unix Epoch. The timestamp is stored in milliseconds because some blockchains produce blocks more often than once a second.
   * @type {number}
   * @memberof Block
   */
  timestamp: number;
  /**
   * @type {Array<Transaction>}
   * @memberof Block
   */
  transactions: Array<Transaction>;
  /**
   * @type {object}
   * @memberof Block
   */
  metadata?: StacksBlockMetadata | BitcoinBlockMetadata;
}

/**
 * The operation_identifier uniquely identifies an operation within a transaction.
 * @export
 * @interface OperationIdentifier
 */
export interface OperationIdentifier {
  /**
   * The operation index is used to ensure each operation has a unique identifier within a transaction. This index is only relative to the transaction and NOT GLOBAL. The operations in each transaction should start from index 0. To clarify, there may not be any notion of an operation index in the blockchain being described.
   * @type {number}
   * @memberof OperationIdentifier
   */
  index: number;
  /**
   * Some blockchains specify an operation index that is essential for client use. For example, Bitcoin uses a network_index to identify which UTXO was used in a transaction. network_index should not be populated if there is no notion of an operation index in a blockchain (typically most account-based blockchains).
   * @type {number}
   * @memberof OperationIdentifier
   */
  network_index?: number;
}

/**
 * The account_identifier uniquely identifies an account within a network. All fields in the account_identifier are utilized to determine this uniqueness (including the metadata field, if populated).
 * @export
 * @interface AccountIdentifier
 */
export interface AccountIdentifier {
  /**
   * The address may be a cryptographic public key (or some encoding of it) or a provided username.
   * @type {string}
   * @memberof AccountIdentifier
   */
  address: string;
  /**
   * @type {SubAccountIdentifier}
   * @memberof AccountIdentifier
   */
  sub_account?: SubAccountIdentifier;
  /**
   * Blockchains that utilize a username model (where the address is not a derivative of a cryptographic public key) should specify the public key(s) owned by the address in metadata.
   * @type {object}
   * @memberof AccountIdentifier
   */
  metadata?: object;
}

/**
 * An account may have state specific to a contract address (SIP-10 token) and/or a stake (delegated balance). The sub_account_identifier should specify which state (if applicable) an account instantiation refers to.
 * @export
 * @interface SubAccountIdentifier
 */
export interface SubAccountIdentifier {
  /**
   * The SubAccount address may be a cryptographic value or some other identifier (ex: bonded) that uniquely specifies a SubAccount.
   * @type {string}
   * @memberof SubAccountIdentifier
   */
  address: string;
  /**
   * If the SubAccount address is not sufficient to uniquely specify a SubAccount, any other identifying information can be stored here. It is important to note that two SubAccounts with identical addresses but differing metadata will not be considered equal by clients.
   * @type {object}
   * @memberof SubAccountIdentifier
   */
  metadata?: object;
}

/**
 * Operations contain all balance-changing information within a transaction. They are always one-sided (only affect 1 AccountIdentifier) and can succeed or fail independently from a Transaction. Operations are used both to represent on-chain data (Data API) and to construct new transactions (Construction API), creating a standard interface for reading and writing to blockchains.
 * @export
 * @interface Operation
 */
export interface Operation {
  /**
   * @type {OperationIdentifier}
   * @memberof Operation
   */
  operation_identifier: OperationIdentifier;
  /**
   * Restrict referenced related_operations to identifier indices < the current operation_identifier.index. This ensures there exists a clear DAG-structure of relations. Since operations are one-sided, one could imagine relating operations in a single transfer or linking operations in a call tree.
   * @type {Array<OperationIdentifier>}
   * @memberof Operation
   */
  related_operations?: Array<OperationIdentifier>;
  /**
   * Type is the network-specific type of the operation. Ensure that any type that can be returned here is also specified in the NetworkOptionsResponse. This can be very useful to downstream consumers that parse all block data.
   * @type {string}
   * @memberof Operation
   */
  type: string;
  /**
   * Status is the network-specific status of the operation. Status is not defined on the transaction object because blockchains with smart contracts may have transactions that partially apply (some operations are successful and some are not). Blockchains with atomic transactions (all operations succeed or all operations fail) will have the same status for each operation. On-chain operations (operations retrieved in the `/block` and `/block/transaction` endpoints) MUST have a populated status field (anything on-chain must have succeeded or failed). However, operations provided during transaction construction (often times called "intent" in the documentation) MUST NOT have a populated status field (operations yet to be included on-chain have not yet succeeded or failed).
   * @type {string}
   * @memberof Operation
   */
  status?: string;
  /**
   * @type {AccountIdentifier}
   * @memberof Operation
   */
  account?: AccountIdentifier;
  /**
   * @type {Amount}
   * @memberof Operation
   */
  amount?: Amount;
  /**
   * @type {CoinChange}
   * @memberof Operation
   */
  coin_change?: CoinChange;
  /**
   * @type {object}
   * @memberof Operation
   */
  metadata?: object;
}

/**
 * Amount is some Value of a Currency. It is considered invalid to specify a Value without a Currency.
 * @export
 * @interface Amount
 */
export interface Amount {
  /**
   * Value of the transaction in atomic units represented as an arbitrary-sized signed integer. For example, 1 BTC would be represented by a value of 100000000.
   * @type {string}
   * @memberof Amount
   */
  value: string;
  /**
   * @type {Currency}
   * @memberof Amount
   */
  currency: Currency;
  /**
   * @type {object}
   * @memberof Amount
   */
  metadata?: object;
}

/**
 * Currency is composed of a canonical Symbol and Decimals. This Decimals value is used to convert an Amount.Value from atomic units (Satoshis) to standard units (Bitcoins).
 * @export
 * @interface Currency
 */
export interface Currency {
  /**
   * Canonical symbol associated with a currency.
   * @type {string}
   * @memberof Currency
   */
  symbol: string;
  /**
   * Number of decimal places in the standard unit representation of the amount. For example, BTC has 8 decimals. Note that it is not possible to represent the value of some currency in atomic units that is not base 10.
   * @type {number}
   * @memberof Currency
   */
  decimals: number;
  /**
   * Any additional information related to the currency itself. For example, it would be useful to populate this object with the contract address of an SIP-10 token.
   * @type {object}
   * @memberof Currency
   */
  metadata?: object;
}

/**
 * CoinIdentifier uniquely identifies a Coin.
 * @export
 * @interface CoinIdentifier
 */
export interface CoinIdentifier {
  /**
   * Identifier should be populated with a globally unique identifier of a Coin. In Bitcoin, this identifier would be transaction_hash:index.
   * @type {string}
   * @memberof CoinIdentifier
   */
  identifier: string;
}

/**
 * CoinChange is used to represent a change in state of a some coin identified by a coin_identifier. This object is part of the Operation model and must be populated for UTXO-based blockchains. Coincidentally, this abstraction of UTXOs allows for supporting both account-based transfers and UTXO-based transfers on the same blockchain (when a transfer is account-based, don't populate this model).
 * @export
 * @interface CoinChange
 */
export interface CoinChange {
  /**
   * @type {CoinIdentifier}
   * @memberof CoinChange
   */
  coin_identifier: CoinIdentifier;
  /**
   * @type {CoinAction}
   * @memberof CoinChange
   */
  coin_action: CoinAction;
}

/**
 * CoinActions are different state changes that a Coin can undergo. When a Coin is created, it is coin_created. When a Coin is spent, it is coin_spent. It is assumed that a single Coin cannot be created or spent more than once.
 * @export
 * @enum {string}
 */
export enum CoinAction {
  created = "coin_created",
  spent = "coin_spent",
}

/**
 * Contract interfaces are ABI returned by the stacks node
 * @export
 * @interface StacksContractInterface
 */
export interface StacksContractInterface {
  /**
   * List of defined methods
   * @type {Array<object>}
   * @memberof ContractInterfaceResponse
   */
  functions: Array<object>;
  /**
   * List of defined variables
   * @type {Array<DataVarField>}
   * @memberof ContractInterfaceResponse
   */
  variables: Array<DataVarField>;
  /**
   * List of defined data-maps
   * @type {Array<DataMapField>}
   * @memberof ContractInterfaceResponse
   */
  maps: Array<DataMapField>;
  /**
   * List of fungible tokens in the contract
   * @type {Array<DataFtField>}
   * @memberof ContractInterfaceResponse
   */
  fungible_tokens: Array<DataFtField>;
  /**
   * List of non-fungible tokens in the contract
   * @type {Array<DataNftField>}
   * @memberof ContractInterfaceResponse
   */
  non_fungible_tokens: Array<DataNftField>;
}

/**
 * DataVarField describes clarity data-var metadata.
 * @export
 * @interface DataVarField
 */
export interface DataVarField {
  /**
   * Name of var
   * @type {string}
   * @memberof DataVarField
   */
  name: string;
}

/**
 * DataMapField describes clarity data-map metadata.
 * @export
 * @interface DataMapField
 */
export interface DataMapField {
  /**
   * Name of map
   * @type {string}
   * @memberof DataMapField
   */
  name: string;
}

/**
 * DataMapField describes clarity fungible token metadata.
 * @export
 * @interface DataFtField
 */
export interface DataFtField {
  /**
   * Name of fungible token
   * @type {string}
   * @memberof DataFtField
   */
  name: string;
}

/**
 * DataMapField describes clarity non fungible token metadata.
 * @export
 * @interface DataNftField
 */
export interface DataNftField {
  /**
   * Name of non fungible token
   * @type {string}
   * @memberof DataNftField
   */
  name: string;
}

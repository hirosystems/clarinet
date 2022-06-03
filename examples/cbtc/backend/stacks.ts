export interface StacksChainEvent {
  apply: Apply[];
  hook:  Hook;
}

export interface Apply {
  transaction:      Transaction;
  proof:            null;
  block_identifier: BlockIdentifier;
  confirmations:    number;
}

export interface BlockIdentifier {
  index: number;
  hash:  string;
}

export interface Transaction {
  transaction_identifier: TransactionIdentifier;
  operations:             Operation[];
  metadata:               TransactionMetadata;
}

export interface TransactionMetadata {
  success:        boolean;
  raw_tx:         string;
  result:         string;
  sender:         string;
  fee:            number;
  kind:           Kind;
  execution_cost: ExecutionCost;
  receipt:        Receipt;
  description:    string;
}

export interface ExecutionCost {
  write_length: number;
  write_count:  number;
  read_length:  number;
  read_count:   number;
  runtime:      number;
}

export interface Kind {
  ContractCall: ContractCall;
}

export interface ContractCall {
  contract_identifier: string;
  method:              string;
  args:                string[];
}

export interface Receipt {
  mutated_contracts_radius: string[];
  mutated_assets_radius:    string[];
  contract_calls_stack:     any[];
  events:                   Event[];
}

export interface Event {
  FTBurnEvent?: FTBurnEvent;
}

export interface FTBurnEvent {
  asset_identifier: string;
  sender:           string;
  amount:           string;
}

export interface Operation {
  operation_identifier: OperationIdentifier;
  type:                 string;
  status:               string;
  account:              Account;
  amount:               Amount;
}

export interface Account {
  address: string;
}

export interface Amount {
  value:    number;
  currency: Currency;
}

export interface Currency {
  symbol:   string;
  decimals: number;
  metadata: CurrencyMetadata;
}

export interface CurrencyMetadata {
  asset_class_identifier: string;
  asset_identifier:       null;
  standard:               string;
}

export interface OperationIdentifier {
  index: number;
}

export interface TransactionIdentifier {
  hash: string;
}

export interface Hook {
  uuid:      string;
  predicate: Predicate;
}

export interface Predicate {
  ft_event: FtEvent;
}

export interface FtEvent {
  asset_identifier: string;
  actions:          string[];
}
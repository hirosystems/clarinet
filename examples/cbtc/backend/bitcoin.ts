export interface BitcoinChainEvent {
    apply:     Apply[];
    hook_uuid: string;
}

export interface Apply {
    transaction:      Transaction;
    proof:            string;
    block_identifier: BlockIdentifier;
    confirmations:    number;
}

export interface BlockIdentifier {
    index: number;
    hash:  string;
}

export interface Transaction {
    transaction_identifier: TransactionIdentifier;
    operations:             any[];
    metadata:               Metadata;
}

export interface Metadata {
    inputs:  Input[];
    outputs: Output[];
}

export interface Input {
    previous_output: string;
    script_sig:      string;
    sequence:        number;
    witness:         any[];
}

export interface Output {
    value:         number;
    script_pubkey: string;
}

export interface TransactionIdentifier {
    hash: string;
}
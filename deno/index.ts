interface Transaction {
    output: any;
}

interface Block {
    transactions: Array<Transaction>;
}

interface Chain {
    mineBlock(): Promise<Block>;
}

interface Account {
    label: string;
}

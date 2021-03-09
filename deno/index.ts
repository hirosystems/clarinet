// export function mineBlock(transactions: Array<Transaction>): void;

export class Transaction {
    output: any;
}

export class Block {
    transactions: Array<Transaction>;
}


export class Account {
    label: string;
}

export class Chain {
    name: string;

    constructor(name: string) {
      this.name = name;
    }
  
    mineBlock(transactions: Array<Transaction>) {
      return globalThis.mineBlock(transactions);
    }
}

type TestFunction = (chain: Chain, accounts: Array<Account>) => void | Promise<void>;

interface UnitTestOptions {
    name: string;
    fn: TestFunction
}

export class Clarinet {

    static test(options: UnitTestOptions) {
        Deno.test({
            name: options.name,
            async fn() {
                let result: any = await globalThis.setupChain();
                await options.fn(result.chain, result.accounts);
            },
        })
    }
}

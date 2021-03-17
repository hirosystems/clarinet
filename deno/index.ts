export class Tx {
    sender: string;
    contract: string;
    method: string;
    args: Array<string>;

    constructor(contract: string, method: string, args: Array<string>, sender: string) {
        this.contract = contract;
        this.method = method;
        this.args = args;
        this.sender = sender;
    }
}

export interface TxReceipt {
    output: string;
}

export interface Block {
    height: number,
    receipts: Array<TxReceipt>;
}

export interface Account {
    address: string;
    balance: number;
    name: string;
    mnemonic: string;
    derivation: string;
}

export interface Chain {
    sessionId: number;
}

export class Chain {
    sessionId: number;

    constructor(sessionId: number) {
        this.sessionId = sessionId;
    }

    mineBlock(transactions: Array<Tx>) {
        let result = (Deno as any).core.jsonOpSync("mine_block", {
            sessionId: this.sessionId,
            transactions: transactions
        });
        let block: Block = {
            height: result.block_height,
            receipts: result.receipts
        };
        return block;
    }

    mineEmptyBlock(count: number) {
        (Deno as any).core.jsonOpSync("mine_empty_blocks", {
            sessionId: this.sessionId,
            count: count,
        });
        return
    }

    callReadOnlyFn(contract: string, method: string, args: Array<any>, sender: string) {
        let result = (Deno as any).core.jsonOpSync("call_read_only_fn", {
            sessionId: this.sessionId,
            contract: contract,
            method: method,
            args: args,
            sender: sender,
        });
        return result;
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
                (Deno as any).core.ops();
                let result = (Deno as any).core.jsonOpSync("setup_chain", {});
                let chain = new Chain(result['session_id']);
                let accounts: Array<Account> = result['accounts'];
                await options.fn(chain, accounts);
            },
        })
    }
}

export namespace types {
    export function uint(val: number) {
        return `u${val}`;
    }
}

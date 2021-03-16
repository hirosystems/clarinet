export interface Transaction {
    output: any;
}

export interface Block {
    transactions: Array<Transaction>;
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
                let chain: Chain = {
                    sessionId: result['session_id']
                };
                let accounts: Array<Account> = result['accounts'];
                await options.fn(chain, accounts);
            },
        })
    }
}

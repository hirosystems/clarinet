export class Tx {
    type: number;
    sender: string;
    contractCall?: TxContractCall;
    trasnferStx?: TxTransfer;

    constructor(type: number, sender: string) {
        this.type = type;
        this.sender = sender;
    }

    static transferSTX(amount: number, recipient: string, sender: string) {
        let tx = new Tx(1, sender);
        tx.trasnferStx = {
            recipient,
            amount
        };
        return tx;
    }

    static contractCall(contract: string, method: string, args: Array<string>, sender: string) {
        let tx = new Tx(2, sender);
        tx.contractCall = {
            contract,
            method,
            args
        }
        return tx;
    }
}

export interface TxContractCall {
    contract: string;
    method: string;
    args: Array<string>;
}

export interface TxTransfer {
    amount: number;
    recipient: string;
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

    const byteToHex: any = [];
    for (let n = 0; n <= 0xff; ++n) {
        const hexOctet = n.toString(16).padStart(2, "0");
        byteToHex.push(hexOctet);
    }

    function serializeTuple(input: Object) {
        let items: Array<string> = [];
        for (var [key, value] of Object.entries(input)) {
            if (typeof value === 'object') {
                items.push(`${key}: { ${serializeTuple(value)} }`);
            } else if (Array.isArray(value)) {
                // todo(ludo): not supported, should panic
            } else {
                items.push(`${key}: ${value}`);
            }
        }
        return items.join(", ");
    }

    function isObject(obj: any) {
        return typeof obj === 'object' && !Array.isArray(obj);
    }

    export function bool(val: boolean) {
        return `${val}`;
    }

    export function int(val: number) {
        return `${val}`;
    }

    export function uint(val: number) {
        return `u${val}`;
    }

    export function ascii(val: string) {
        return `"${val}"`;
    }

    export function utf8(val: string) {
        return `u"${val}"`;
    }

    export function buff(val: ArrayBuffer) {
        const buff = new Uint8Array(val);
        const hexOctets = new Array(buff.length);
    
        for (let i = 0; i < buff.length; ++i)
            hexOctets[i] = byteToHex[buff[i]];
        
        return `0x${hexOctets.join("")}`;
    }

    export function list(val: Array<any>) {
        return `(list ${val.join(" ")})`;
    }

    export function principal(val: string) {
        return `'${val}`;
    }

    export function tuple(val: Object) {
        return `{ ${serializeTuple(val)} }`;
    }
}
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
    result: string;
    events: Array<any>;
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

    getAssetsMaps() {
        let result = (Deno as any).core.jsonOpSync("get_assets_maps", {
            sessionId: this.sessionId,
        });
        return result;
    }
}

type TestFunction = (chain: Chain, accounts: Map<string, Account>) => void | Promise<void>;

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
                let accounts: Map<string, Account> = new Map();
                for (let account of result['accounts']) {
                    accounts.set(account.name, account);
                } 
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

    export function ok(val: string) {
        return `(ok ${val})`;
    }

    export function err(val: string) {
        return `(err ${val})`;
    }

    export function some(val: string) {
        return `(some ${val})`;
    }

    export function none() {
        return `none`;
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

declare global {
    interface String {
        /**
         * Lorem ipsum
         * @param value
         */
        expectOk(): String;
        expectErr(): String;
        expectSome(): String;
        expectNone(): void;
        expectBool(value: boolean): boolean;
        expectUint(value: number): number;
        expectInt(value: number): number;
        expectBuff(value: ArrayBuffer): ArrayBuffer;
        expectAscii(value: String): String;
        expectUtf8(value: String): String;
        expectPrincipal(value: String): String;
        expectList(): Array<String>;
        expectTuple(): Object;
    }
}

function consume(src: String, token: String, wrapped: boolean) {
    let dst = (' ' + src).slice(1);
    let size = token.length;
    if (wrapped) {
        size += 2;
    }
    if (dst.length < size) {
        throw new Error(`Expected ${green(token.toString())}, got ${red(src.toString())}`);
    }
    if (wrapped) {
        dst = dst.substring(1, dst.length - 1);
    }
    let res = dst.slice(0, token.length);
    if (res !== token) {
        throw new Error(`Expected ${green(token.toString())}, got ${red(src.toString())}`);
    }
    let leftPad = 0;
    if (dst.charAt(token.length) === ' ') {
        leftPad = 1;
    }
    let remainder = dst.substring(token.length + leftPad);
    return remainder;
}
  
String.prototype.expectOk = function () {
    return consume(this, "ok", true);
};
  
String.prototype.expectErr = function () {
    return consume(this, "err", true);
};

String.prototype.expectSome = function () {
    return consume(this, "some", true);
};

String.prototype.expectNone = function () {
    return consume(this, "none", false);
};

String.prototype.expectBool = function (value: boolean) {
    try {
        consume(this, `${value}`, false)
    } catch (error) {
        throw error;
    }
    return value;
};

String.prototype.expectUint = function (value: number) {
    try {
        consume(this, `u${value}`, false)
    } catch (error) {
        throw error;
    }
    return value;
};
  
String.prototype.expectInt = function (value: number) {
    try {
        consume(this, `${value}`, false)
    } catch (error) {
        throw error;
    }
    return value;
};

String.prototype.expectBuff = function (value: ArrayBuffer) {
    let buffer = types.buff(value);
    if (this !== buffer) {
        throw new Error(`Expected ${green(buffer)}, got ${red(this.toString())}`);
    }
    return value;
};

String.prototype.expectAscii = function (value: string) {
    try {
        consume(this, `"${value}"`, false)
    } catch (error) {
        throw error;
    }
    return value;
};

String.prototype.expectUtf8 = function (value: string) {
    try {
        consume(this, `u"${value}"`, false)
    } catch (error) {
        throw error;
    }
    return value;
};

String.prototype.expectPrincipal = function (value: string) {
    try {
        consume(this, `${value}`, false)
    } catch (error) {
        throw error;
    }
    return value;
};

String.prototype.expectList = function () {
    if (this.charAt(0) !== "[" || this.charAt(this.length - 1) !== "]") {
        throw new Error(`Expected ${green("(list ...)")}, got ${red(this.toString())}`);
    }

    let stack = [];
    let elements = [];
    let start = 1;
    for (var i = 0; i < this.length; i++) {
        if (this.charAt(i) === "," && stack.length == 1) {
            elements.push(this.substring(start, i));
            start = i + 2;
        }
        if (["(", "[", "{"].includes(this.charAt(i))) {
            stack.push(this.charAt(i));
        }
        if (this.charAt(i) === ")" && stack[stack.length - 1] === "(") {
            stack.pop();
        }
        if (this.charAt(i) === "}" && stack[stack.length - 1] === "{") {
            stack.pop();
        }
        if (this.charAt(i) === "]" && stack[stack.length - 1] === "[") {
            stack.pop();
        }
    }
    let remainder = this.substring(start, this.length-1);
    if (remainder.length > 0) {
        elements.push(remainder);
    }
    return elements;
};

String.prototype.expectTuple = function () {
    if (this.charAt(0) !== "{" || this.charAt(this.length - 1) !== "}") {
        throw new Error(`Expected ${green("(tuple ...)")}, got ${red(this.toString())}`);
    }

    let start = 1;
    let stack = [];
    let elements = [];
    for (var i = 0; i < this.length; i++) {
        if (this.charAt(i) === "," && stack.length == 1) {
            elements.push(this.substring(start, i));
            start = i + 2;
        }
        if (["(", "[", "{"].includes(this.charAt(i))) {
            stack.push(this.charAt(i));
        }
        if (this.charAt(i) === ")" && stack[stack.length - 1] === "(") {
            stack.pop();
        }
        if (this.charAt(i) === "}" && stack[stack.length - 1] === "{") {
            stack.pop();
        }
        if (this.charAt(i) === "]" && stack[stack.length - 1] === "[") {
            stack.pop();
        }
    }
    let remainder = this.substring(start, this.length-1);
    if (remainder.length > 0) {
        elements.push(remainder);
    }
    
    let tuple: {[key: string]: String} = {};
    for (let element of elements) {
        for (var i = 0; i < element.length; i++) {
            if (element.charAt(i) === ":") {
                let key: string = element.substring(0, i);
                let value: string = element.substring(i + 2, element.length);
                tuple[key] = value;
                break;
            }
        }
    }

    return tuple;
};

const noColor = globalThis.Deno?.noColor ?? true;

interface Code {
  open: string;
  close: string;
  regexp: RegExp;
}

let enabled = !noColor;

function code(open: number[], close: number): Code {
  return {
    open: `\x1b[${open.join(";")}m`,
    close: `\x1b[${close}m`,
    regexp: new RegExp(`\\x1b\\[${close}m`, "g"),
  };
}

function run(str: string, code: Code): string {
  return enabled
    ? `${code.open}${str.replace(code.regexp, code.open)}${code.close}`
    : str;
}

export function red(str: string): string {
  return run(str, code([31], 39));
}

export function green(str: string): string {
  return run(str, code([32], 39));
}

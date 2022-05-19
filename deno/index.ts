export class Tx {
  type: number;
  sender: string;
  contractCall?: TxContractCall;
  transferStx?: TxTransfer;
  deployContract?: TxDeployContract;

  constructor(type: number, sender: string) {
    this.type = type;
    this.sender = sender;
  }

  static transferSTX(amount: number, recipient: string, sender: string) {
    let tx = new Tx(1, sender);
    tx.transferStx = {
      recipient,
      amount,
    };
    return tx;
  }

  static contractCall(
    contract: string,
    method: string,
    args: Array<string>,
    sender: string,
  ) {
    let tx = new Tx(2, sender);
    tx.contractCall = {
      contract,
      method,
      args,
    };
    return tx;
  }

  static deployContract(name: string, code: string, sender: string) {
    let tx = new Tx(3, sender);
    tx.deployContract = {
      name,
      code,
    };
    return tx;
  }
}

export interface TxContractCall {
  contract: string;
  method: string;
  args: Array<string>;
}

export interface TxDeployContract {
  code: string;
  name: string;
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
  height: number;
  receipts: Array<TxReceipt>;
}

export interface Account {
  address: string;
  balance: number;
  name: string;
}

export interface Chain {
  sessionId: number;
}

export interface ReadOnlyFn {
  session_id: number;
  result: string;
  events: Array<any>;
}

export interface EmptyBlock {
  session_id: number;
  block_height: number;
}

export interface AssetsMaps {
  session_id: number;
  assets: {
    [name: string]: {
      [owner: string]: number;
    };
  };
}

export class Chain {
  sessionId: number;
  blockHeight: number = 1;

  constructor(sessionId: number) {
    this.sessionId = sessionId;
  }

  mineBlock(transactions: Array<Tx>): Block {
    let result = JSON.parse((Deno as any).core.opSync("api/v1/mine_block", {
      sessionId: this.sessionId,
      transactions: transactions,
    }));
    this.blockHeight = result.block_height;
    let block: Block = {
      height: result.block_height,
      receipts: result.receipts,
    };
    return block;
  }

  mineEmptyBlock(count: number): EmptyBlock {
    let result = JSON.parse(
      (Deno as any).core.opSync("api/v1/mine_empty_blocks", {
        sessionId: this.sessionId,
        count: count,
      }),
    );
    this.blockHeight = result.block_height;
    let emptyBlock: EmptyBlock = {
      session_id: result.session_id,
      block_height: result.block_height,
    };
    return emptyBlock;
  }

  mineEmptyBlockUntil(targetBlockHeight: number): EmptyBlock {
    let count = targetBlockHeight - this.blockHeight;
    if (count < 0) {
      throw new Error(
        `Chain tip cannot be moved from ${this.blockHeight} to ${targetBlockHeight}`,
      );
    }
    return this.mineEmptyBlock(count);
  }

  callReadOnlyFn(
    contract: string,
    method: string,
    args: Array<any>,
    sender: string,
  ): ReadOnlyFn {
    let result = JSON.parse(
      (Deno as any).core.opSync("api/v1/call_read_only_fn", {
        sessionId: this.sessionId,
        contract: contract,
        method: method,
        args: args,
        sender: sender,
      }),
    );
    let readOnlyFn: ReadOnlyFn = {
      session_id: result.session_id,
      result: result.result,
      events: result.events,
    };
    return readOnlyFn;
  }

  getAssetsMaps(): AssetsMaps {
    let result = JSON.parse(
      (Deno as any).core.opSync("api/v1/get_assets_maps", {
        sessionId: this.sessionId,
      }),
    );
    let assetsMaps: AssetsMaps = {
      session_id: result.session_id,
      assets: result.assets,
    };
    return assetsMaps;
  }
}

type PreDeploymentFunction = (
  chain: Chain,
  accounts: Map<string, Account>,
) => void | Promise<void>;

type TestFunction = (
  chain: Chain,
  accounts: Map<string, Account>,
  contracts: Map<string, Contract>,
) => void | Promise<void>;
type PreSetupFunction = () => Array<Tx>;

interface UnitTestOptions {
  name: string;
  only?: true;
  ignore?: true;
  deploymentPath?: string;
  preDeployment?: PreDeploymentFunction;
  fn: TestFunction;
}

export interface Contract {
  contract_id: string;
  source: string;
  contract_interface: any;
}

export interface StacksNode {
  url: string;
}

type ScriptFunction = (
  accounts: Map<string, Account>,
  contracts: Map<string, Contract>,
  node: StacksNode,
) => void | Promise<void>;

interface ScriptOptions {
  fn: ScriptFunction;
}

export class Clarinet {
  static test(options: UnitTestOptions) {
    Deno.test({
      name: options.name,
      only: options.only,
      ignore: options.ignore,
      async fn() {
        (Deno as any).core.ops();

        let hasPreDeploymentSteps = options.preDeployment !== undefined;

        let result = JSON.parse(
          (Deno as any).core.opSync("api/v1/new_session", {
            name: options.name,
            loadDeployment: !hasPreDeploymentSteps,
            deploymentPath: options.deploymentPath,
          }),
        );

        if (options.preDeployment) {
          let chain = new Chain(result["session_id"]);
          let accounts: Map<string, Account> = new Map();
          for (let account of result["accounts"]) {
            accounts.set(account.name, account);
          }
          await options.preDeployment(chain, accounts);

          result = JSON.parse(
            (Deno as any).core.opSync("api/v1/load_deployment", {
              sessionId: chain.sessionId,
              deploymentPath: options.deploymentPath,
            }),
          );
        }

        let chain = new Chain(result["session_id"]);
        let accounts: Map<string, Account> = new Map();
        for (let account of result["accounts"]) {
          accounts.set(account.name, account);
        }
        let contracts: Map<string, Contract> = new Map();
        for (let contract of result["contracts"]) {
          contracts.set(contract.contract_id, contract);
        }
        await options.fn(chain, accounts, contracts);

        JSON.parse((Deno as any).core.opSync("api/v1/terminate_session", {
          sessionId: chain.sessionId,
        }));
      },
    });
  }

  static run(options: ScriptOptions) {
    Deno.test({
      name: "running script",
      async fn() {
        (Deno as any).core.ops();
        let result = JSON.parse(
          (Deno as any).core.opSync("api/v1/new_session", {
            name: "running script",
            loadDeployment: true,
            deploymentPath: undefined,
          }),
        );
        let accounts: Map<string, Account> = new Map();
        for (let account of result["accounts"]) {
          accounts.set(account.name, account);
        }
        let contracts: Map<string, Contract> = new Map();
        for (let contract of result["contracts"]) {
          contracts.set(contract.contract_id, contract);
        }
        let stacks_node: StacksNode = {
          url: result["stacks_node_url"],
        };
        await options.fn(accounts, contracts, stacks_node);
      },
    });
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
      if (typeof value === "object") {
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
    return typeof obj === "object" && !Array.isArray(obj);
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

  export function int(val: number | bigint) {
    return `${val}`;
  }

  export function uint(val: number | bigint) {
    return `u${val}`;
  }

  export function ascii(val: string) {
    return `"${val}"`;
  }

  export function utf8(val: string) {
    return `u"${val}"`;
  }

  export function buff(val: ArrayBuffer | string) {
    const buff = typeof val == "string"
      ? new TextEncoder().encode(val)
      : new Uint8Array(val);

    const hexOctets = new Array(buff.length);

    for (let i = 0; i < buff.length; ++i) {
      hexOctets[i] = byteToHex[buff[i]];
    }

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
    expectOk(): String;
    expectErr(): String;
    expectSome(): String;
    expectNone(): void;
    expectBool(value: boolean): boolean;
    expectUint(value: number | bigint): bigint;
    expectInt(value: number | bigint): bigint;
    expectBuff(value: ArrayBuffer): ArrayBuffer;
    expectAscii(value: String): String;
    expectUtf8(value: String): String;
    expectPrincipal(value: String): String;
    expectList(): Array<String>;
    expectTuple(): Record<string, string>;
  }

  interface Array<T> {
    expectSTXTransferEvent(
      amount: Number | bigint,
      sender: String,
      recipient: String,
    ): Object;
    expectFungibleTokenTransferEvent(
      amount: Number | bigint,
      sender: String,
      recipient: String,
      assetId: String,
    ): Object;
    expectFungibleTokenMintEvent(
      amount: Number | bigint,
      recipient: String,
      assetId: String,
    ): Object;
    expectFungibleTokenBurnEvent(
      amount: Number | bigint,
      sender: String,
      assetId: String,
    ): Object;
    expectPrintEvent(
      contract_identifier: string,
      value: string,
    ): Object;
    expectNonFungibleTokenTransferEvent(
      tokenId: String,
      sender: String,
      recipient: String,
      assetAddress: String,
      assetId: String,
    ): Object;
    expectNonFungibleTokenMintEvent(
      tokenId: String,
      recipient: String,
      assetAddress: String,
      assetId: String,
    ): Object;
    expectNonFungibleTokenBurnEvent(
      tokenId: String,
      sender: String,
      assetAddress: String,
      assetId: String,
    ): Object;
    // expectEvent(sel: (e: Object) => Object): Object;
  }
}

function consume(src: String, expectation: String, wrapped: boolean) {
  let dst = (" " + src).slice(1);
  let size = expectation.length;
  if (!wrapped && src !== expectation) {
    throw new Error(
      `Expected ${green(expectation.toString())}, got ${red(src.toString())}`,
    );
  }
  if (wrapped) {
    size += 2;
  }
  if (dst.length < size) {
    throw new Error(
      `Expected ${green(expectation.toString())}, got ${red(src.toString())}`,
    );
  }
  if (wrapped) {
    dst = dst.substring(1, dst.length - 1);
  }
  let res = dst.slice(0, expectation.length);
  if (res !== expectation) {
    throw new Error(
      `Expected ${green(expectation.toString())}, got ${red(src.toString())}`,
    );
  }
  let leftPad = 0;
  if (dst.charAt(expectation.length) === " ") {
    leftPad = 1;
  }
  let remainder = dst.substring(expectation.length + leftPad);
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
    consume(this, `${value}`, false);
  } catch (error) {
    throw error;
  }
  return value;
};

String.prototype.expectUint = function (value: number | bigint): bigint {
  try {
    consume(this, `u${value}`, false);
  } catch (error) {
    throw error;
  }
  return BigInt(value);
};

String.prototype.expectInt = function (value: number | bigint): bigint {
  try {
    consume(this, `${value}`, false);
  } catch (error) {
    throw error;
  }
  return BigInt(value);
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
    consume(this, `"${value}"`, false);
  } catch (error) {
    throw error;
  }
  return value;
};

String.prototype.expectUtf8 = function (value: string) {
  try {
    consume(this, `u"${value}"`, false);
  } catch (error) {
    throw error;
  }
  return value;
};

String.prototype.expectPrincipal = function (value: string) {
  try {
    consume(this, `${value}`, false);
  } catch (error) {
    throw error;
  }
  return value;
};

String.prototype.expectList = function () {
  if (this.charAt(0) !== "[" || this.charAt(this.length - 1) !== "]") {
    throw new Error(
      `Expected ${green("(list ...)")}, got ${red(this.toString())}`,
    );
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
  let remainder = this.substring(start, this.length - 1);
  if (remainder.length > 0) {
    elements.push(remainder);
  }
  return elements;
};

String.prototype.expectTuple = function () {
  if (this.charAt(0) !== "{" || this.charAt(this.length - 1) !== "}") {
    throw new Error(
      `Expected ${green("(tuple ...)")}, got ${red(this.toString())}`,
    );
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
  let remainder = this.substring(start, this.length - 1);
  if (remainder.length > 0) {
    elements.push(remainder);
  }

  let tuple: Record<string, string> = {};
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

Array.prototype.expectSTXTransferEvent = function (
  amount: Number | bigint,
  sender: String,
  recipient: String,
) {
  for (let event of this) {
    try {
      let e: any = {};
      e["amount"] = event.stx_transfer_event.amount.expectInt(amount);
      e["sender"] = event.stx_transfer_event.sender.expectPrincipal(sender);
      e["recipient"] = event.stx_transfer_event.recipient.expectPrincipal(
        recipient,
      );
      return e;
    } catch (error) {
      continue;
    }
  }
  throw new Error(`Unable to retrieve expected STXTransferEvent`);
};

Array.prototype.expectFungibleTokenTransferEvent = function (
  amount: Number,
  sender: String,
  recipient: String,
  assetId: String,
) {
  for (let event of this) {
    try {
      let e: any = {};
      e["amount"] = event.ft_transfer_event.amount.expectInt(amount);
      e["sender"] = event.ft_transfer_event.sender.expectPrincipal(sender);
      e["recipient"] = event.ft_transfer_event.recipient.expectPrincipal(
        recipient,
      );
      if (event.ft_transfer_event.asset_identifier.endsWith(assetId)) {
        e["assetId"] = event.ft_transfer_event.asset_identifier;
      } else {
        continue;
      }
      return e;
    } catch (error) {
      continue;
    }
  }
  throw new Error(
    `Unable to retrieve expected FungibleTokenTransferEvent(${amount}, ${sender}, ${recipient}, ${assetId})\n${
      JSON.stringify(this)
    }`,
  );
};

Array.prototype.expectFungibleTokenMintEvent = function (
  amount: Number | bigint,
  recipient: String,
  assetId: String,
) {
  for (let event of this) {
    try {
      let e: any = {};
      e["amount"] = event.ft_mint_event.amount.expectInt(amount);
      e["recipient"] = event.ft_mint_event.recipient.expectPrincipal(recipient);
      if (event.ft_mint_event.asset_identifier.endsWith(assetId)) {
        e["assetId"] = event.ft_mint_event.asset_identifier;
      } else {
        continue;
      }
      return e;
    } catch (error) {
      continue;
    }
  }
  throw new Error(`Unable to retrieve expected FungibleTokenMintEvent`);
};

Array.prototype.expectFungibleTokenBurnEvent = function (
  amount: Number | bigint,
  sender: String,
  assetId: String,
) {
  for (let event of this) {
    try {
      let e: any = {};
      e["amount"] = event.ft_burn_event.amount.expectInt(amount);
      e["sender"] = event.ft_burn_event.sender.expectPrincipal(sender);
      if (event.ft_burn_event.asset_identifier.endsWith(assetId)) {
        e["assetId"] = event.ft_burn_event.asset_identifier;
      } else {
        continue;
      }
      return e;
    } catch (error) {
      continue;
    }
  }
  throw new Error(`Unable to retrieve expected FungibleTokenBurnEvent`);
};

Array.prototype.expectPrintEvent = function (
  contract_identifier: string,
  value: string,
) {
  for (let event of this) {
    try {
      let e: any = {};
      e["contract_identifier"] = event.contract_event.contract_identifier
        .expectPrincipal(
          contract_identifier,
        );

      if (event.contract_event.topic.endsWith("print")) {
        e["topic"] = event.contract_event.topic;
      } else {
        continue;
      }

      if (event.contract_event.value.endsWith(value)) {
        e["value"] = event.contract_event.value;
      } else {
        continue;
      }
      return e;
    } catch (error) {
      continue;
    }
  }
  throw new Error(`Unable to retrieve expected PrintEvent`);
};
// Array.prototype.expectEvent = function(sel: (e: Object) => Object) {
//     for (let event of this) {
//         try {
//             sel(event);
//             return event as Object;
//         } catch (error) {
//             continue;
//         }
//     }
//     throw new Error(`Unable to retrieve expected PrintEvent`);
// }
Array.prototype.expectNonFungibleTokenTransferEvent = function (
  tokenId: String,
  sender: String,
  recipient: String,
  assetAddress: String,
  assetId: String,
) {
  for (let event of this) {
    try {
      let e: any = {};
      if (event.nft_transfer_event.value === tokenId) {
        e["tokenId"] = event.nft_transfer_event.value;
      } else {
        continue;
      }
      e["sender"] = event.nft_transfer_event.sender.expectPrincipal(sender);
      e["recipient"] = event.nft_transfer_event.recipient.expectPrincipal(
        recipient,
      );
      if (
        event.nft_transfer_event.asset_identifier ===
          `${assetAddress}::${assetId}`
      ) {
        e["assetId"] = event.nft_transfer_event.asset_identifier;
      } else {
        continue;
      }
      return e;
    } catch (error) {
      continue;
    }
  }
  throw new Error(`Unable to retrieve expected NonFungibleTokenTransferEvent`);
};

Array.prototype.expectNonFungibleTokenMintEvent = function (
  tokenId: String,
  recipient: String,
  assetAddress: String,
  assetId: String,
) {
  for (let event of this) {
    try {
      let e: any = {};
      if (event.nft_mint_event.value === tokenId) {
        e["tokenId"] = event.nft_mint_event.value;
      } else {
        continue;
      }
      e["recipient"] = event.nft_mint_event.recipient.expectPrincipal(
        recipient,
      );
      if (
        event.nft_mint_event.asset_identifier === `${assetAddress}::${assetId}`
      ) {
        e["assetId"] = event.nft_mint_event.asset_identifier;
      } else {
        continue;
      }
      return e;
    } catch (error) {
      continue;
    }
  }
  throw new Error(`Unable to retrieve expected NonFungibleTokenMintEvent`);
};

Array.prototype.expectNonFungibleTokenBurnEvent = function (
  tokenId: String,
  sender: String,
  assetAddress: String,
  assetId: String,
) {
  for (let event of this) {
    try {
      let e: any = {};
      if (event.nft_burn_event.value === tokenId) {
        e["tokenId"] = event.nft_burn_event.value;
      } else {
        continue;
      }
      e["sender"] = event.nft_burn_event.sender.expectPrincipal(sender);
      if (
        event.nft_burn_event.asset_identifier === `${assetAddress}::${assetId}`
      ) {
        e["assetId"] = event.nft_burn_event.asset_identifier;
      } else {
        continue;
      }
      return e;
    } catch (error) {
      continue;
    }
  }
  throw new Error(`Unable to retrieve expected NonFungibleTokenBurnEvent`);
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

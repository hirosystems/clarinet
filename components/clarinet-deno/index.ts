// deno-lint-ignore-file ban-ts-comment no-namespace

import {
  ExpectFungibleTokenBurnEvent,
  ExpectFungibleTokenMintEvent,
  ExpectFungibleTokenTransferEvent,
  ExpectNonFungibleTokenBurnEvent,
  ExpectNonFungibleTokenMintEvent,
  ExpectNonFungibleTokenTransferEvent,
  ExpectPrintEvent,
  ExpectSTXTransferEvent,
} from "./types.ts";

export * from "./types.ts";

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
    const tx = new Tx(1, sender);
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
    sender: string
  ) {
    const tx = new Tx(2, sender);
    tx.contractCall = {
      contract,
      method,
      args,
    };
    return tx;
  }

  static deployContract(name: string, code: string, sender: string) {
    const tx = new Tx(3, sender);
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
  events: Array<unknown>;
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
  events: Array<unknown>;
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
  blockHeight = 1;

  constructor(sessionId: number) {
    this.sessionId = sessionId;
  }

  mineBlock(transactions: Array<Tx>): Block {
    const result = JSON.parse(
      // @ts-ignore
      Deno.core.opSync("api/v1/mine_block", {
        sessionId: this.sessionId,
        transactions: transactions,
      })
    );
    this.blockHeight = result.block_height;
    const block: Block = {
      height: result.block_height,
      receipts: result.receipts,
    };
    return block;
  }

  mineEmptyBlock(count: number): EmptyBlock {
    const result = JSON.parse(
      // @ts-ignore
      Deno.core.opSync("api/v1/mine_empty_blocks", {
        sessionId: this.sessionId,
        count: count,
      })
    );
    this.blockHeight = result.block_height;
    const emptyBlock: EmptyBlock = {
      session_id: result.session_id,
      block_height: result.block_height,
    };
    return emptyBlock;
  }

  mineEmptyBlockUntil(targetBlockHeight: number): EmptyBlock {
    const count = targetBlockHeight - this.blockHeight;
    if (count < 0) {
      throw new Error(
        `Chain tip cannot be moved from ${this.blockHeight} to ${targetBlockHeight}`
      );
    }
    return this.mineEmptyBlock(count);
  }

  callReadOnlyFn(
    contract: string,
    method: string,
    args: Array<unknown>,
    sender: string
  ): ReadOnlyFn {
    const result = JSON.parse(
      // @ts-ignore
      Deno.core.opSync("api/v1/call_read_only_fn", {
        sessionId: this.sessionId,
        contract: contract,
        method: method,
        args: args,
        sender: sender,
      })
    );
    const readOnlyFn: ReadOnlyFn = {
      session_id: result.session_id,
      result: result.result,
      events: result.events,
    };
    return readOnlyFn;
  }

  getAssetsMaps(): AssetsMaps {
    const result = JSON.parse(
      // @ts-ignore
      Deno.core.opSync("api/v1/get_assets_maps", {
        sessionId: this.sessionId,
      })
    );
    const assetsMaps: AssetsMaps = {
      session_id: result.session_id,
      assets: result.assets,
    };
    return assetsMaps;
  }
}

type PreDeploymentFunction = (
  chain: Chain,
  accounts: Map<string, Account>
) => void | Promise<void>;

type TestFunction = (
  chain: Chain,
  accounts: Map<string, Account>,
  contracts: Map<string, Contract>
) => void | Promise<void>;

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
  contract_interface: unknown;
}

export interface StacksNode {
  url: string;
}

type ScriptFunction = (
  accounts: Map<string, Account>,
  contracts: Map<string, Contract>,
  node: StacksNode
) => void | Promise<void>;

interface ScriptOptions {
  fn: ScriptFunction;
}

export class Clarinet {
  static test(options: UnitTestOptions) {
    // @ts-ignore
    Deno.test({
      name: options.name,
      only: options.only,
      ignore: options.ignore,
      async fn() {
        const hasPreDeploymentSteps = options.preDeployment !== undefined;

        let result = JSON.parse(
          // @ts-ignore
          Deno.core.opSync("api/v1/new_session", {
            name: options.name,
            loadDeployment: !hasPreDeploymentSteps,
            deploymentPath: options.deploymentPath,
          })
        );

        if (options.preDeployment) {
          const chain = new Chain(result.session_id);
          const accounts: Map<string, Account> = new Map();
          for (const account of result.accounts) {
            accounts.set(account.name, account);
          }
          await options.preDeployment(chain, accounts);

          result = JSON.parse(
            // @ts-ignore
            Deno.core.opSync("api/v1/load_deployment", {
              sessionId: chain.sessionId,
              deploymentPath: options.deploymentPath,
            })
          );
        }

        const chain = new Chain(result.session_id);
        const accounts: Map<string, Account> = new Map();
        for (const account of result.accounts) {
          accounts.set(account.name, account);
        }
        const contracts: Map<string, Contract> = new Map();
        for (const contract of result.contracts) {
          contracts.set(contract.contract_id, contract);
        }
        await options.fn(chain, accounts, contracts);

        JSON.parse(
          // @ts-ignore
          Deno.core.opSync("api/v1/terminate_session", {
            sessionId: chain.sessionId,
          })
        );
      },
    });
  }

  static run(options: ScriptOptions) {
    // @ts-ignore
    Deno.test({
      name: "running script",
      async fn() {
        const result = JSON.parse(
          // @ts-ignore
          Deno.core.opSync("api/v1/new_session", {
            name: "running script",
            loadDeployment: true,
            deploymentPath: undefined,
          })
        );
        const accounts: Map<string, Account> = new Map();
        for (const account of result.accounts) {
          accounts.set(account.name, account);
        }
        const contracts: Map<string, Contract> = new Map();
        for (const contract of result.contracts) {
          contracts.set(contract.contract_id, contract);
        }
        const stacks_node: StacksNode = {
          url: result.stacks_node_url,
        };
        await options.fn(accounts, contracts, stacks_node);
      },
    });
  }
}

export namespace types {
  const byteToHex: string[] = [];
  for (let n = 0; n <= 0xff; ++n) {
    const hexOctet = n.toString(16).padStart(2, "0");
    byteToHex.push(hexOctet);
  }

  function serializeTuple(input: Record<string, unknown>) {
    const items: Array<string> = [];
    for (const [key, value] of Object.entries(input)) {
      if (Array.isArray(value)) {
        throw new Error("Tuple value can't be an array");
      } else if (!!value && typeof value === "object") {
        items.push(
          `${key}: { ${serializeTuple(value as Record<string, unknown>)} }`
        );
      } else {
        items.push(`${key}: ${value}`);
      }
    }
    return items.join(", ");
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
    return JSON.stringify(val);
  }

  export function utf8(val: string) {
    return `u${JSON.stringify(val)}`;
  }

  export function buff(val: ArrayBuffer | string) {
    const buff =
      typeof val == "string"
        ? new TextEncoder().encode(val)
        : new Uint8Array(val);

    const hexOctets = new Array(buff.length);

    for (let i = 0; i < buff.length; ++i) {
      hexOctets[i] = byteToHex[buff[i]];
    }

    return `0x${hexOctets.join("")}`;
  }

  export function list(val: Array<unknown>) {
    return `(list ${val.join(" ")})`;
  }

  export function principal(val: string) {
    return `'${val}`;
  }

  export function tuple(val: Record<string, unknown>) {
    return `{ ${serializeTuple(val)} }`;
  }
}

declare global {
  interface String {
    expectOk(): string;
    expectErr(): string;
    expectSome(): string;
    expectNone(): void;
    expectBool(value: boolean): boolean;
    expectUint(value: number | bigint): bigint;
    expectInt(value: number | bigint): bigint;
    expectBuff(value: ArrayBuffer): ArrayBuffer;
    expectAscii(value: string): string;
    expectUtf8(value: string): string;
    expectPrincipal(value: string): string;
    expectList(): Array<string>;
    expectTuple(): Record<string, string>;
  }

  interface Array<T> {
    expectSTXTransferEvent(
      amount: number | bigint,
      sender: string,
      recipient: string
    ): ExpectSTXTransferEvent;
    expectFungibleTokenTransferEvent(
      amount: number | bigint,
      sender: string,
      recipient: string,
      assetId: string
    ): ExpectFungibleTokenTransferEvent;
    expectFungibleTokenMintEvent(
      amount: number | bigint,
      recipient: string,
      assetId: string
    ): ExpectFungibleTokenMintEvent;
    expectFungibleTokenBurnEvent(
      amount: number | bigint,
      sender: string,
      assetId: string
    ): ExpectFungibleTokenBurnEvent;
    expectPrintEvent(
      contractIdentifier: string,
      value: string
    ): ExpectPrintEvent;
    expectNonFungibleTokenTransferEvent(
      tokenId: string,
      sender: string,
      recipient: string,
      assetAddress: string,
      assetId: string
    ): ExpectNonFungibleTokenTransferEvent;
    expectNonFungibleTokenMintEvent(
      tokenId: string,
      recipient: string,
      assetAddress: string,
      assetId: string
    ): ExpectNonFungibleTokenMintEvent;
    expectNonFungibleTokenBurnEvent(
      tokenId: string,
      sender: string,
      assetAddress: string,
      assetId: string
    ): ExpectNonFungibleTokenBurnEvent;
  }
}

// deno-lint-ignore ban-types
function consume(src: String, expectation: string, wrapped: boolean) {
  let dst = (" " + src).slice(1);
  let size = expectation.length;
  if (!wrapped && src !== expectation) {
    throw new Error(
      `Expected ${green(expectation.toString())}, got ${red(src.toString())}`
    );
  }
  if (wrapped) {
    size += 2;
  }
  if (dst.length < size) {
    throw new Error(
      `Expected ${green(expectation.toString())}, got ${red(src.toString())}`
    );
  }
  if (wrapped) {
    dst = dst.substring(1, dst.length - 1);
  }
  const res = dst.slice(0, expectation.length);
  if (res !== expectation) {
    throw new Error(
      `Expected ${green(expectation.toString())}, got ${red(src.toString())}`
    );
  }
  let leftPad = 0;
  if (dst.charAt(expectation.length) === " ") {
    leftPad = 1;
  }
  const remainder = dst.substring(expectation.length + leftPad);
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
  const buffer = types.buff(value);
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
      `Expected ${green("(list ...)")}, got ${red(this.toString())}`
    );
  }

  const stack = [];
  const elements = [];
  let start = 1;
  for (let i = 0; i < this.length; i++) {
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
  const remainder = this.substring(start, this.length - 1);
  if (remainder.length > 0) {
    elements.push(remainder);
  }
  return elements;
};

String.prototype.expectTuple = function () {
  if (this.charAt(0) !== "{" || this.charAt(this.length - 1) !== "}") {
    throw new Error(
      `Expected ${green("(tuple ...)")}, got ${red(this.toString())}`
    );
  }

  let start = 1;
  const stack = [];
  const elements = [];
  for (let i = 0; i < this.length; i++) {
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
  const remainder = this.substring(start, this.length - 1);
  if (remainder.length > 0) {
    elements.push(remainder);
  }

  const tuple: Record<string, string> = {};
  for (const element of elements) {
    for (let i = 0; i < element.length; i++) {
      if (element.charAt(i) === ":") {
        const key = element.substring(0, i).trim();
        const value = element.substring(i + 2).trim();
        tuple[key] = value;
        break;
      }
    }
  }

  return tuple;
};

Array.prototype.expectSTXTransferEvent = function (amount, sender, recipient) {
  for (const event of this) {
    try {
      const { stx_transfer_event } = event;
      return {
        amount: stx_transfer_event.amount.expectInt(amount),
        sender: stx_transfer_event.sender.expectPrincipal(sender),
        recipient: stx_transfer_event.recipient.expectPrincipal(recipient),
      };
    } catch (_error) {
      continue;
    }
  }
  throw new Error("Unable to retrieve expected STXTransferEvent");
};

Array.prototype.expectFungibleTokenTransferEvent = function (
  amount,
  sender,
  recipient,
  assetId
) {
  for (const event of this) {
    try {
      const { ft_transfer_event } = event;
      if (!ft_transfer_event.asset_identifier.endsWith(assetId)) continue;

      return {
        amount: ft_transfer_event.amount.expectInt(amount),
        sender: ft_transfer_event.sender.expectPrincipal(sender),
        recipient: ft_transfer_event.recipient.expectPrincipal(recipient),
        assetId: ft_transfer_event.asset_identifier,
      };
    } catch (_error) {
      continue;
    }
  }
  throw new Error(
    `Unable to retrieve expected FungibleTokenTransferEvent(${amount}, ${sender}, ${recipient}, ${assetId})\n${JSON.stringify(
      this
    )}`
  );
};

Array.prototype.expectFungibleTokenMintEvent = function (
  amount,
  recipient,
  assetId
) {
  for (const event of this) {
    try {
      const { ft_mint_event } = event;
      if (!ft_mint_event.asset_identifier.endsWith(assetId)) continue;

      return {
        amount: ft_mint_event.amount.expectInt(amount),
        recipient: ft_mint_event.recipient.expectPrincipal(recipient),
        assetId: ft_mint_event.asset_identifier,
      };
    } catch (_error) {
      continue;
    }
  }
  throw new Error("Unable to retrieve expected FungibleTokenMintEvent");
};

Array.prototype.expectFungibleTokenBurnEvent = function (
  amount,
  sender,
  assetId
) {
  for (const event of this) {
    try {
      const { ft_burn_event } = event;
      if (!ft_burn_event.asset_identifier.endsWith(assetId)) continue;

      return {
        amount: ft_burn_event.amount.expectInt(amount),
        sender: ft_burn_event.sender.expectPrincipal(sender),
        assetId: ft_burn_event.asset_identifier,
      };
    } catch (_error) {
      continue;
    }
  }
  throw new Error("Unable to retrieve expected FungibleTokenBurnEvent");
};

Array.prototype.expectPrintEvent = function (contractIdentifier, value) {
  for (const event of this) {
    try {
      const { contract_event } = event;
      if (!contract_event.topic.endsWith("print")) continue;
      if (!contract_event.value.endsWith(value)) continue;

      return {
        contract_identifier:
          contract_event.contract_identifier.expectPrincipal(
            contractIdentifier
          ),
        topic: contract_event.topic,
        value: contract_event.value,
      };
    } catch (error) {
      console.warn(error);
      continue;
    }
  }
  throw new Error("Unable to retrieve expected PrintEvent");
};

Array.prototype.expectNonFungibleTokenTransferEvent = function (
  tokenId,
  sender,
  recipient,
  assetAddress,
  assetId
) {
  for (const event of this) {
    try {
      const { nft_transfer_event } = event;
      if (nft_transfer_event.value !== tokenId) continue;
      if (nft_transfer_event.asset_identifier !== `${assetAddress}::${assetId}`)
        continue;

      return {
        tokenId: nft_transfer_event.value,
        sender: nft_transfer_event.sender.expectPrincipal(sender),
        recipient: nft_transfer_event.recipient.expectPrincipal(recipient),
        assetId: nft_transfer_event.asset_identifier,
      };
    } catch (_error) {
      continue;
    }
  }
  throw new Error("Unable to retrieve expected NonFungibleTokenTransferEvent");
};

Array.prototype.expectNonFungibleTokenMintEvent = function (
  tokenId,
  recipient,
  assetAddress,
  assetId
) {
  for (const event of this) {
    try {
      const { nft_mint_event } = event;
      if (nft_mint_event.value !== tokenId) continue;
      if (nft_mint_event.asset_identifier !== `${assetAddress}::${assetId}`)
        continue;

      return {
        tokenId: nft_mint_event.value,
        recipient: nft_mint_event.recipient.expectPrincipal(recipient),
        assetId: nft_mint_event.asset_identifier,
      };
    } catch (_error) {
      continue;
    }
  }
  throw new Error("Unable to retrieve expected NonFungibleTokenMintEvent");
};

Array.prototype.expectNonFungibleTokenBurnEvent = function (
  tokenId,
  sender,
  assetAddress,
  assetId
) {
  for (const event of this) {
    try {
      if (event.nft_burn_event.value !== tokenId) continue;
      if (
        event.nft_burn_event.asset_identifier !== `${assetAddress}::${assetId}`
      )
        continue;

      return {
        assetId: event.nft_burn_event.asset_identifier,
        tokenId: event.nft_burn_event.value,
        sender: event.nft_burn_event.sender.expectPrincipal(sender),
      };
    } catch (_error) {
      continue;
    }
  }
  throw new Error("Unable to retrieve expected NonFungibleTokenBurnEvent");
};

const noColor = Deno.noColor ?? true;
const enabled = !noColor;

interface Code {
  open: string;
  close: string;
  regexp: RegExp;
}

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

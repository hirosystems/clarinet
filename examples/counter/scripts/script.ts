export interface Account {
  address: string;
  balance: number;
  name: string;
  mnemonic: string;
  derivation: string;
}

export interface Contract {
  address: string;
  name: string;
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
  name: string;
  fn: ScriptFunction;
}

export class Clarinet {
  static run(options: ScriptOptions) {
    Deno.test({
      name: options.name,
      async fn() {
        (Deno as any).core.ops();
        let result = (Deno as any).core.jsonOpSync("setup_chain", {
          name: options.name,
          transactions: [],
        });
        log(`${JSON.stringify(result)}`);
        let accounts: Map<string, Account> = new Map();
        for (let account of result["accounts"]) {
          accounts.set(account.name, account);
        }
        let contracts: Map<string, any> = new Map();
        for (let contract of result["contracts"]) {
          contracts.set(contract.name, contract);
        }
        let stacks_node: StacksNode = {
          url: result["stacks_node_url"]
        };
        await options.fn(accounts, contracts, stacks_node);
      },
    });
  }
}

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

export function log(str: string): string {
  return run(str, code([31], 39));
}

export function log_red(str: string): string {
  return run(str, code([31], 39));
}

export function log_green(str: string): string {
  return run(str, code([32], 39));
}

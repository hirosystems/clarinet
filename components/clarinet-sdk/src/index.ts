import { Cl, ClarityValue } from "@stacks/transactions";

import { vfs } from "./vfs.js";
import type { ContractInterface } from "./contractInterface.js";
import {
  SDK,
  TransactionRes,
  CallContractArgs,
  DeployContractArgs,
  TransferSTXArgs,
  ContractOptions,
} from "@hirosystems/clarinet-sdk-wasm";
import { ContractAST } from "./contractAst.js";

const wasmModule = import("@hirosystems/clarinet-sdk-wasm");

// https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/BigInt#use_within_json
// @ts-ignore
BigInt.prototype.toJSON = function () {
  return this.toString();
};

type ClarityEvent = { event: string; data: { [key: string]: any } };
export type ParsedTransactionRes = {
  result: ClarityValue;
  events: ClarityEvent[];
};

type CallFn = (
  contract: string,
  method: string,
  args: ClarityValue[],
  sender: string,
) => ParsedTransactionRes;

type DeployContractOptions = {
  clarityVersion: 1 | 2;
};
type DeployContract = (
  name: string,
  content: string,
  options: DeployContractOptions | null,
  sender: string,
) => ParsedTransactionRes;

type TransferSTX = (
  amount: number | bigint,
  recipient: string,
  sender: string,
) => ParsedTransactionRes;

type Tx =
  | {
      callPublicFn: {
        contract: string;
        method: string;
        args: ClarityValue[];
        sender: string;
      };
      deployContract?: never;
      transferSTX?: never;
    }
  | {
      callPublicFn?: never;
      deployContract: {
        name: string;
        content: string;
        options: DeployContractOptions | null;
        sender: string;
      };
      transferSTX?: never;
    }
  | {
      callPublicFn?: never;
      deployContradct?: never;
      transferSTX: { amount: number; recipient: string; sender: string };
    };

export const tx = {
  callPublicFn: (contract: string, method: string, args: ClarityValue[], sender: string): Tx => ({
    callPublicFn: { contract, method, args, sender },
  }),
  deployContract: (
    name: string,
    content: string,
    options: DeployContractOptions | null,
    sender: string,
  ): Tx => ({
    deployContract: { name, content, options, sender },
  }),
  transferSTX: (amount: number, recipient: string, sender: string): Tx => ({
    transferSTX: { amount, recipient, sender },
  }),
};

type MineBlock = (txs: Array<Tx>) => ParsedTransactionRes[];
type GetDataVar = (contract: string, dataVar: string) => ClarityValue;
type GetMapEntry = (contract: string, mapName: string, mapKey: ClarityValue) => ClarityValue;
type GetContractAST = (contractId: string) => ContractAST;
type GetContractsInterfaces = () => Map<string, ContractInterface>;

// because the session is wrapped in a proxy the types need to be hardcoded
export type ClarityVM = {
  [K in keyof SDK]: K extends "callReadOnlyFn" | "callPublicFn"
    ? CallFn
    : K extends "deployContract"
    ? DeployContract
    : K extends "transferSTX"
    ? TransferSTX
    : K extends "mineBlock"
    ? MineBlock
    : K extends "getDataVar"
    ? GetDataVar
    : K extends "getMapEntry"
    ? GetMapEntry
    : K extends "getContractAST"
    ? GetContractAST
    : K extends "getContractsInterfaces"
    ? GetContractsInterfaces
    : SDK[K];
};

function parseEvents(events: string): ClarityEvent[] {
  try {
    // @todo: improve type safety
    return JSON.parse(events).map((e: string) => {
      const { event, data } = JSON.parse(e);
      return {
        event: event,
        data: data,
      };
    });
  } catch (e) {
    console.error(`Fail to parse events: ${e}`);
    return [];
  }
}

function parseTxResult(response: TransactionRes): ParsedTransactionRes {
  return {
    result: Cl.deserialize(response.result),
    events: parseEvents(response.events),
  };
}

const getSessionProxy = () => ({
  get(session: SDK, prop: keyof SDK, receiver: any) {
    // some of the WASM methods are proxied here to:
    // - serialize clarity values input argument
    // - deserialize output into clarity values

    if (prop === "callReadOnlyFn" || prop === "callPublicFn") {
      const callFn: CallFn = (contract, method, args, sender) => {
        const response = session[prop](
          new CallContractArgs(
            contract,
            method,
            args.map((a) => Cl.serialize(a)),
            sender,
          ),
        );
        return parseTxResult(response);
      };
      return callFn;
    }

    if (prop === "deployContract") {
      const callDeployContract: DeployContract = (name, content, options, sender) => {
        const rustOptions = options
          ? new ContractOptions(options.clarityVersion)
          : new ContractOptions();

        const response = session.deployContract(
          new DeployContractArgs(name, content, rustOptions, sender),
        );
        return parseTxResult(response);
      };
      return callDeployContract;
    }

    if (prop === "transferSTX") {
      const callTransferSTX: TransferSTX = (amount, ...args) => {
        const response = session.transferSTX(new TransferSTXArgs(BigInt(amount), ...args));
        return parseTxResult(response);
      };
      return callTransferSTX;
    }

    if (prop === "mineBlock") {
      const callMineBlock: MineBlock = (txs) => {
        const serializedTxs = txs.map((tx) => {
          if (tx.callPublicFn) {
            return {
              callPublicFn: {
                ...tx.callPublicFn,
                args_maps: tx.callPublicFn.args.map((a) => Cl.serialize(a)),
              },
            };
          }
          return tx;
        });

        const responses: TransactionRes[] = session.mineBlock(serializedTxs);
        return responses.map(parseTxResult);
      };
      return callMineBlock;
    }

    if (prop === "getDataVar") {
      const getDataVar: GetDataVar = (...args) => {
        const response = session.getDataVar(...args);
        const result = Cl.deserialize(response);
        return result;
      };
      return getDataVar;
    }

    if (prop === "getMapEntry") {
      const getMapEntry: GetMapEntry = (contract, mapName, mapKey) => {
        const response = session.getMapEntry(contract, mapName, Cl.serialize(mapKey));
        const result = Cl.deserialize(response);
        return result;
      };
      return getMapEntry;
    }

    return Reflect.get(session, prop, receiver);
  },
});

// load wasm only once and memoize it
function memoizedInit() {
  let vm: ClarityVM | null = null;
  return async (manifestPath = "./Clarinet.toml") => {
    const module = await wasmModule;

    if (!vm) {
      console.log("init clarity vm");
      vm = new Proxy(new module.SDK(vfs), getSessionProxy()) as unknown as ClarityVM;
    }
    // start a new session
    await vm.initSession(process.cwd(), manifestPath);
    return vm;
  };
}

export const initVM = memoizedInit();

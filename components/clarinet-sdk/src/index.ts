import { Cl, ClarityValue } from "@stacks/transactions";
import {
  SDK,
  TransactionRes,
  CallContractArgs,
  DeployContractArgs,
  TransferSTXArgs,
  ContractOptions,
} from "@hirosystems/clarinet-sdk-wasm";

import { vfs } from "./vfs.js";
import type { ContractInterface } from "./contractInterface.js";
import { ContractAST } from "./contractAst.js";

const wasmModule = import("@hirosystems/clarinet-sdk-wasm");

// https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/BigInt#use_within_json
// @ts-ignore
BigInt.prototype.toJSON = function () {
  return this.toString();
};

export type ClarityEvent = {
  event: string;
  data: { raw_value?: string; value?: ClarityValue; [key: string]: any };
};

export type ParsedTransactionResult = {
  result: ClarityValue;
  events: ClarityEvent[];
};

export type CallFn = (
  contract: string,
  method: string,
  args: ClarityValue[],
  sender: string,
) => ParsedTransactionResult;

export type DeployContractOptions = {
  clarityVersion: 1 | 2;
};
export type DeployContract = (
  name: string,
  content: string,
  options: DeployContractOptions | null,
  sender: string,
) => ParsedTransactionResult;

export type TransferSTX = (
  amount: number | bigint,
  recipient: string,
  sender: string,
) => ParsedTransactionResult;

export type Tx =
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

export type MineBlock = (txs: Array<Tx>) => ParsedTransactionResult[];
export type GetDataVar = (contract: string, dataVar: string) => ClarityValue;
export type GetMapEntry = (contract: string, mapName: string, mapKey: ClarityValue) => ClarityValue;
export type GetContractAST = (contractId: string) => ContractAST;
export type GetContractsInterfaces = () => Map<string, ContractInterface>;
export type GetBlockTime = () => ClarityValue;

// because the session is wrapped in a proxy the types need to be hardcoded
export type Simnet = {
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
    : K extends "getBlockTime"
    ? GetBlockTime
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
      if ("raw_value" in data) {
        data.value = Cl.deserialize(data.raw_value);
      }
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

function parseTxResponse(response: TransactionRes): ParsedTransactionResult {
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
        return parseTxResponse(response);
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
        return parseTxResponse(response);
      };
      return callDeployContract;
    }

    if (prop === "transferSTX") {
      const callTransferSTX: TransferSTX = (amount, ...args) => {
        const response = session.transferSTX(new TransferSTXArgs(BigInt(amount), ...args));
        return parseTxResponse(response);
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
        return responses.map(parseTxResponse);
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

    if (prop === "getBlockTime") {
      const getBlockTime: GetBlockTime = () => {
        const response = session.getBlockTime();
        const result = Cl.deserialize(response);
        return result;
      };
      return getBlockTime;
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
  let simnet: Simnet | null = null;

  return async (manifestPath = "./Clarinet.toml") => {
    if (!simnet) {
      const module = await wasmModule;
      simnet = new Proxy(new module.SDK(vfs), getSessionProxy()) as unknown as Simnet;
    }

    // start a new simnet session
    await simnet.initSession(process.cwd(), manifestPath);
    return simnet;
  };
}

export const initSimnet = memoizedInit();

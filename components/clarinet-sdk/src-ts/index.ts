import { Cl, ClarityValue } from "@stacks/transactions";

import { vfs } from "./vfs";
import type { ContractInterface } from "./contractInterface";

import type { SDK } from "./sdk";
type WASMModule = typeof import("./sdk");
const wasmModule = import("./sdk");

type CallFn = (
  contract: string,
  method: string,
  args: ClarityValue[],
  sender: string
) => {
  result: ClarityValue;
  events: { event: string; data: { [key: string]: any } }[];
};
type GetDataVar = (contract: string, dataVar: string) => ClarityValue;
type GetMapEntry = (contract: string, mapName: string, mapKey: ClarityValue) => ClarityValue;
type GetAssetsMap = () => Map<string, Map<string, bigint>>;
type GetAccounts = () => Map<string, string>;

// because the session is wrapped in a proxy the types need to be hardcoded
export type ClarityVM = {
  [K in keyof SDK]: K extends "callReadOnlyFn" | "callPublicFn"
    ? CallFn
    : K extends "getDataVar"
    ? GetDataVar
    : K extends "getMapEntry"
    ? GetMapEntry
    : K extends "getAccounts"
    ? GetAccounts
    : K extends "getAssetsMap"
    ? GetAssetsMap
    : K extends "getContractsInterfaces"
    ? () => Map<string, ContractInterface>
    : SDK[K];
};

const getSessionProxy = (wasm: WASMModule) => ({
  get(session: SDK, prop: keyof SDK, receiver: any) {
    // some of the WASM methods are proxied here to:
    // - serialize clarity values input argument
    // - desizeialize output into clarity values

    if (prop === "callReadOnlyFn" || prop === "callPublicFn") {
      const callFn: CallFn = (contract, method, args, sender) => {
        const response = session[prop](
          new wasm.CallContractArgs(
            contract,
            method,
            args.map((a) => Cl.serialize(a))
          ),
          sender
        );
        const result = Cl.deserialize(response.result);

        const events = response.events.map((e: { event: string; data: Map<string, any> }) => {
          return {
            event: e.event,
            data: Object.fromEntries(e.data.entries()),
          };
        });

        return { result, events };
      };

      return callFn;
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

    // handle both CJS and ESM context
    // - in CJS: `module.default` is a promise resolving
    // - in ESM: `module` is directly the
    // @ts-ignore
    let wasm: WASMModule =
      typeof module.default === "undefined"
        ? (module as unknown as WASMModule)
        : await module.default;

    if (!vm) {
      console.log("init clarity vm");
      vm = new Proxy(new wasm.SDK(vfs), getSessionProxy(wasm)) as unknown as ClarityVM;
    }
    // start a new session
    await vm.initSession(process.cwd(), manifestPath);
    return vm;
  };
}

export const initVM = memoizedInit();

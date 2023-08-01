import { Cl, ClarityValue } from "@stacks/transactions";

import init, { CallContractArgs, TestVM } from "../pkg/clarinet_sdk";
import wasm from "../pkg/clarinet_sdk_bg.wasm";
import { callVFS } from "./vfs";
import { ContractInterface } from "./contractInterface";

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

// because we use a proxy around the test session,
// the type need to be hardcoded
// @todo: is there a better way than this nested ternary?
export type ClarityVM = {
  [K in keyof TestVM]: K extends "callReadOnlyFn" | "callPublicFn"
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
    : TestVM[K];
};

const sessionProxy = {
  get(session: TestVM, prop: keyof TestVM, receiver: any) {
    if (prop === "callReadOnlyFn" || prop === "callPublicFn") {
      const callFn: CallFn = (contract, method, args, sender) => {
        const response = session[prop](
          new CallContractArgs(
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
};

async function initWASM(): Promise<ClarityVM> {
  // @ts-ignore
  await init(wasm());
  return new Proxy(new TestVM(callVFS), sessionProxy) as unknown as ClarityVM;
}

function memoizedInit() {
  let vm: null | ClarityVM = null;
  return async () => {
    if (!vm) {
      vm = await initWASM();
    }
    return vm;
  };
}

export default memoizedInit();

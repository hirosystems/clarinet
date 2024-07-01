import { SDKOptions } from "@hirosystems/clarinet-sdk-wasm";

export {
  tx,
  type ClarityEvent,
  type ParsedTransactionResult,
  type DeployContractOptions,
  type Tx,
  type TransferSTX,
} from "../../common/src/sdkProxyHelpers.js";

import { vfs } from "./vfs.js";
import { Simnet, getSessionProxy } from "./sdkProxy.js";

export { type Simnet } from "./sdkProxy.js";

const wasmModule = import("@hirosystems/clarinet-sdk-wasm");

// https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/BigInt#use_within_json
// @ts-ignore
BigInt.prototype.toJSON = function () {
  return this.toString();
};

// load wasm only once and memoize it
function memoizedInit() {
  let simnet: Simnet | null = null;

  return async (
    manifestPath = "./Clarinet.toml",
    noCache = false,
    options?: { trackCosts: boolean; trackCoverage: boolean },
  ) => {
    if (noCache || !simnet) {
      const module = await wasmModule;
      let sdkOptions = new SDKOptions(!!options?.trackCosts, !!options?.trackCoverage);
      simnet = new Proxy(new module.SDK(vfs, sdkOptions), getSessionProxy()) as unknown as Simnet;
    }

    // start a new simnet session
    await simnet.initSession(process.cwd(), manifestPath);
    return simnet;
  };
}

export const initSimnet = memoizedInit();

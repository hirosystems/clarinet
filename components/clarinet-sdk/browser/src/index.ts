import init, { SDK } from "@hirosystems/clarinet-sdk-wasm-browser";

import { httpClient } from "../../common/src/httpClient.js";

import { Simnet, getSessionProxy } from "./sdkProxy.js";
import { defaultVfs } from "./defaultVfs.js";

// https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/BigInt#use_within_json
// @ts-ignore
BigInt.prototype.toJSON = function () {
  return this.toString();
};

export {
  tx,
  type ClarityEvent,
  type ParsedTransactionResult,
  type DeployContractOptions,
  type Tx,
  type TransferSTX,
} from "../../common/src/sdkProxyHelpers.js";

export { init, SDK, getSessionProxy, type Simnet };
export { defaultVfs, defaultFileStore } from "./defaultVfs.js";

export const initSimnet = async (virtualFileSystem?: Function) => {
  await init();

  const vfs = virtualFileSystem ? virtualFileSystem : defaultVfs;
  return new Proxy(new SDK(vfs, httpClient), getSessionProxy()) as unknown as Simnet;
};

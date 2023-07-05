import { Cl, ClarityValue } from "@stacks/transactions";

import init, { TestSession } from "../pkg/clarinet_sdk";
import wasm from "../pkg/clarinet_sdk_bg.wasm";
import { callVFS } from "./vfs";

type CallFn = (
  contract: string,
  method: string,
  args: ClarityValue[],
  sender: string
) => {
  result: ClarityValue;
  events: { event: string; data: { [key: string]: any } }[];
};

/**
 * @example
 * const assets = session.getAssetsMap();
 * assert.equal(assets.get("STX")?.get(wallet.address), expectedAmountInUSTX);
 */
export type GetAssetsMap = () => Map<string, Map<string, bigint>>;

export type GetAccounts = () => Map<string, string>;

// because we use a proxy around the test session,
// the type need to be hardcoded
// @todo: is there a better way than this nested ternary?
export type Session = {
  [K in keyof TestSession]: K extends "callReadOnlyFn" | "callPublicFn"
    ? CallFn
    : K extends "getAccounts"
    ? GetAccounts
    : K extends "getAssetsMap"
    ? GetAssetsMap
    : TestSession[K];
};

const sessionProxy = {
  get(session: TestSession, prop: keyof TestSession, receiver: any) {
    if (prop === "callReadOnlyFn" || prop === "callPublicFn") {
      const callFn: CallFn = (contract, method, args, sender) => {
        const response = session[prop](
          contract,
          method,
          args.map((a) => Cl.serialize(a)),
          sender
        );
        const result = Cl.deserialize(response.result);

        const events = response.events.map(
          (e: { event: string; data: Map<string, any> }) => {
            return {
              event: e.event,
              data: Object.fromEntries(e.data.entries()),
            };
          }
        );

        return { result, events };
      };
      return callFn;
    }

    return Reflect.get(session, prop, receiver);
  },
};

export async function main(): Promise<Session> {
  // @ts-ignore
  await init(wasm());

  return new Proxy(
    new TestSession(callVFS),
    sessionProxy
  ) as unknown as Session;
}

import { Cl, Pc, serializeCV } from "@stacks/transactions";
import { expect, it, vi } from "vitest";
import { tx } from "../../common/src/sdkProxyHelpers";
import { createSyncDebugSimnet } from "../src/syncDebugSimnet";
import { syncSend } from "../src/syncDebugSocket";

vi.mock("../src/syncDebugSocket", () => ({ syncSend: vi.fn() }));

it("preserves batch outcomes and transaction options through the synchronous adapter", () => {
  const sender = "ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM";
  const execution = { result: serializeCV(Cl.ok(Cl.bool(true))), events: "[]", costs: "null" };
  vi.mocked(syncSend)
    .mockReturnValueOnce(JSON.stringify({ result: { accounts: { deployer: sender } } }))
    .mockReturnValueOnce(
      JSON.stringify({
        result: {
          results: [
            { status: "rejected", error: "invalid transfer" },
            { status: "included_failure", error: "runtime", execution: null },
            {
              status: "included_failure",
              error: "(err u7)",
              execution: { ...execution, result: serializeCV(Cl.error(Cl.uint(7))) },
            },
            { status: "success", execution },
          ],
        },
      }),
    );
  const simnet = createSyncDebugSimnet();
  const call = tx.callPublicFn("counter", "increment", [Cl.uint(1)], sender, {
    postConditions: [Pc.principal(sender).willSendEq(1).ustx()],
  });
  const results = simnet.mineBlock([
    tx.transferSTX(0, sender, sender),
    call,
    call,
    tx.deployContract("new", "true", { clarityVersion: 3 }, sender),
  ]);
  expect(results).toEqual([
    { status: "rejected", error: "invalid transfer" },
    { status: "included_failure", error: "runtime", execution: null },
    {
      status: "included_failure",
      error: "(err u7)",
      execution: { result: Cl.error(Cl.uint(7)), events: [], costs: null, performance: undefined },
    },
    {
      status: "success",
      execution: { result: Cl.ok(Cl.bool(true)), events: [], costs: null, performance: undefined },
    },
  ]);
  expect(vi.mocked(syncSend).mock.calls[1][0]).toMatchObject({
    method: "mineBlock",
    txs: [
      { type: "transferSTX", amount: 0 },
      {
        type: "callPublicFn",
        args: [serializeCV(Cl.uint(1))],
        postConditions: call.callPublicFn!.postConditions,
      },
      { type: "callPublicFn" },
      { type: "deployContract", options: { clarityVersion: 3 } },
    ],
  });
});

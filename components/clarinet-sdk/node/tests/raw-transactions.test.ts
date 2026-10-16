import { Cl, deserializeTransaction } from "@stacks/transactions";
import { beforeEach, expect, it } from "vitest";
import { initSimnet, type Simnet } from "..";
import fixtures from "./fixtures/raw-transactions/transactions.json";

let simnet: Simnet;
const { sender, sponsor, transactions } = fixtures;
const balance = (address: string) => simnet.getAssetsMap().get("STX")!.get(address)!;
const broadcast = (name: keyof typeof transactions) =>
  simnet.broadcastTransaction(transactions[name].hex);
beforeEach(async () => {
  simnet = await initSimnet("tests/fixtures/Clarinet.toml");
  simnet.setEpoch("3.4");
  simnet.mintSTX(sender, 1000000n);
  simnet.mintSTX(sponsor, 1000000n);
});

it("executes signed bytes, preserves the memo and canonical ID, and rejects replay", () => {
  const height = simnet.blockHeight;
  const result = broadcast("transfer");
  expect(result.status).toBe("success");
  expect(result.transaction).toEqual({ txid: transactions.transfer.txid, fee: 10n, nonce: 0n });
  expect(result.transaction!.txid).toBe(deserializeTransaction(transactions.transfer.hex).txid());
  if (result.status !== "success") throw new Error(result.status);
  expect(result.execution.result).toEqual(Cl.ok(Cl.bool(true)));
  expect(result.execution.performance).toBeUndefined();
  expect(result.execution.events[0].data.memo).toBe("726177206d656d6f".padEnd(68, "0"));
  expect(balance(sender)).toBe(999890n);
  expect(balance(sponsor)).toBe(1000100n);
  expect(simnet.getAccountNonce(sender)).toBe(1n);
  expect(simnet.blockHeight).toBe(height + 1);
  expect(broadcast("transfer").status).toBe("rejected");
  expect(balance(sender)).toBe(999890n);
  expect(simnet.blockHeight).toBe(height + 1);
});

it.each([
  "wrong_nonce",
  "wrong_network",
  "zero",
  "self",
  "insufficient",
  "insufficient_fee",
] as const)("rejects %s without any settlement", (name) => {
  const height = simnet.blockHeight;
  expect(broadcast(name).status).toBe("rejected");
  expect(balance(sender)).toBe(1000000n);
  expect(balance(sponsor)).toBe(1000000n);
  expect(simnet.getAccountNonce(sender)).toBe(0n);
  expect(simnet.blockHeight).toBe(height);
});

it.each(["xyz", "00", transactions.transfer.hex + "00"])("rejects malformed input", (hex) => {
  expect(simnet.broadcastTransaction(hex)).toMatchObject({ status: "rejected", transaction: null });
  expect(simnet.getAccountNonce(sender)).toBe(0n);
});

it("checks signatures on Wasm", () => {
  const tx = deserializeTransaction(transactions.transfer.hex);
  tx.setFee(11n);
  expect(simnet.broadcastTransaction(tx.serialize()).status).toBe("rejected");
  expect(balance(sender)).toBe(1000000n);
});

it.each(["sponsored", "same_sponsor"] as const)("settles %s exactly once per principal", (name) => {
  expect(broadcast(name).status).toBe("success");
  expect(simnet.getAccountNonce(sender)).toBe(1n);
  expect(simnet.getAccountNonce(sponsor)).toBe(name === "sponsored" ? 1n : 0n);
  expect(balance(sender)).toBe(name === "sponsored" ? 999900n : 999890n);
  expect(balance(sponsor)).toBe(name === "sponsored" ? 1000090n : 1000100n);
});

it("deploys and calls through the shared frame and updates contract caches", () => {
  expect(broadcast("deploy").status).toBe("success");
  expect(simnet.getContractSource(`${sender}.raw`)).toBe(fixtures.source);
  expect(simnet.getContractsInterfaces().has(`${sender}.raw`)).toBe(true);
  expect(broadcast("increment").status).toBe("success");
  expect(simnet.getDataVar(`${sender}.raw`, "count")).toEqual(Cl.uint(1));
  expect(balance(sender)).toBe(999980n);
});

it.each([
  ["fail", "abort_by_response"],
  ["crash", "runtime_error"],
  ["abort", "abort_by_post_condition"],
] as const)("keeps fees and nonces but rolls back %s", (name, status) => {
  broadcast("deploy");
  const result = broadcast(name);
  expect(result.status).toBe(status);
  expect(result.transaction).toEqual({ txid: transactions[name].txid, fee: 10n, nonce: 1n });
  expect(simnet.getDataVar(`${sender}.raw`, "count")).toEqual(Cl.uint(0));
  expect(balance(sender)).toBe(999980n);
  expect(balance(sponsor)).toBe(1000000n);
  expect(simnet.getAccountNonce(sender)).toBe(2n);
  if (name === "abort" && result.status === "abort_by_post_condition")
    expect(result.events).toHaveLength(1);
});

it("charges included analysis failures without caching a contract", () => {
  expect(broadcast("bad_deploy").status).toBe("analysis_error");
  expect(balance(sender)).toBe(999990n);
  expect(simnet.getAccountNonce(sender)).toBe(1n);
  expect(simnet.getContractsInterfaces().has(`${sender}.bad`)).toBe(false);
});

it("settles a sponsored call", () => {
  broadcast("deploy");
  expect(broadcast("sponsored_call").status).toBe("success");
  expect(balance(sender)).toBe(999990n);
  expect(balance(sponsor)).toBe(999990n);
  expect(simnet.getAccountNonce(sender)).toBe(2n);
  expect(simnet.getAccountNonce(sponsor)).toBe(1n);
});

it("passes tx-sponsor? to the VM", () => {
  broadcast("deploy");
  const result = broadcast("sponsor_context");
  expect(result.status).toBe("success");
  if (result.status === "success")
    expect(result.execution.result).toEqual(Cl.ok(Cl.some(Cl.principal(sponsor))));
});

it("rejects a sponsor nonce mismatch without touching either account", () => {
  expect(broadcast("bad_sponsor_nonce").status).toBe("rejected");
  expect(balance(sender)).toBe(1000000n);
  expect(balance(sponsor)).toBe(1000000n);
  expect(simnet.getAccountNonce(sender)).toBe(0n);
  expect(simnet.getAccountNonce(sponsor)).toBe(0n);
});

it("retains sponsor fees and both nonces after a post-condition abort", () => {
  broadcast("deploy");
  expect(broadcast("sponsored_abort").status).toBe("abort_by_post_condition");
  expect(balance(sender)).toBe(999990n);
  expect(balance(sponsor)).toBe(999990n);
  expect(simnet.getAccountNonce(sender)).toBe(2n);
  expect(simnet.getAccountNonce(sponsor)).toBe(1n);
});

it("rejects duplicate contracts before parsing and rolls back the fee", () => {
  broadcast("deploy");
  const result = broadcast("duplicate_deploy");
  expect(result.status).toBe("rejected");
  if (result.status === "rejected") expect(result.error).toContain("Duplicate contract");
  expect(balance(sender)).toBe(999990n);
  expect(simnet.getAccountNonce(sender)).toBe(1n);
  expect(simnet.getContractSource(`${sender}.raw`)).toBe(fixtures.source);
});

it("does not cache a deployment aborted by its post-conditions", () => {
  const result = broadcast("abort_deploy");
  expect(result.status).toBe("abort_by_post_condition");
  expect(simnet.getContractsInterfaces().has(`${sender}.aborted`)).toBe(false);
  expect(balance(sender)).toBe(999990n);
  expect(balance(sponsor)).toBe(1000000n);
  expect(simnet.getAccountNonce(sender)).toBe(1n);
});

it("rejects a deployment version that the epoch does not allow", () => {
  simnet.setEpoch("2.5");
  expect(broadcast("deploy").status).toBe("rejected");
  expect(balance(sender)).toBe(1000000n);
  expect(simnet.getAccountNonce(sender)).toBe(0n);
});

import { Cl } from "@stacks/transactions";
import { describe, expect, it } from "vitest";

// test the built package and not the source code
// makes it simpler to handle wasm build
import { initVM, tx } from "../";

const deployerAddr = "ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM";
const wallet1Addr = "ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5";

const manifestPath = "tests/fixtures/Clarinet.toml";

describe("basic vm interactions", async () => {
  it("initialize vm", async () => {
    const vm = await initVM(manifestPath);
    expect(vm.blockHeight).toBe(1);
  });

  it("can mine empty blocks", async () => {
    const vm = await initVM(manifestPath);
    vm.mineEmptyBlock();
    expect(vm.blockHeight).toBe(2);
    vm.mineEmptyBlocks(4);
    expect(vm.blockHeight).toBe(6);
  });

  it("exposes devnet stacks accounts", async () => {
    const vm = await initVM(manifestPath);
    const accounts = vm.getAccounts();

    expect(accounts).toHaveLength(4);
    expect(accounts.get("deployer")).toBe(deployerAddr);
    expect(accounts.get("wallet_1")).toBe(wallet1Addr);
  });

  it("expose assets maps", async () => {
    const vm = await initVM(manifestPath);

    const assets = vm.getAssetsMap();
    expect(assets.get("STX")).toHaveLength(4);
    expect(assets.get("STX")?.get(wallet1Addr)).toBe(100000000000000n);
  });
});

describe("vm can call contracts function", async () => {
  it("can call read only functions", async () => {
    const vm = await initVM(manifestPath);
    const res = vm.callReadOnlyFn("counter", "get-counter", [], wallet1Addr);

    expect(res).toHaveProperty("result");
    expect(res).toHaveProperty("events");
    expect(res.result).toStrictEqual(Cl.ok(Cl.tuple({ counter: Cl.uint(0) })));
  });

  it("does not increase block height when calling read-only functions", async () => {
    const vm = await initVM(manifestPath);
    const initalBH = vm.blockHeight;

    vm.callReadOnlyFn("counter", "get-counter", [], wallet1Addr);
    vm.callReadOnlyFn("counter", "get-counter", [], wallet1Addr);
    expect(vm.blockHeight).toBe(initalBH);
  });

  it("can call public functions", async () => {
    const vm = await initVM(manifestPath);
    const res = vm.callPublicFn("counter", "increment", [], wallet1Addr);

    expect(res).toHaveProperty("result");
    expect(res).toHaveProperty("events");
    expect(res.result).toStrictEqual(Cl.ok(Cl.bool(true)));
  });

  it("can call public functions with arguments", async () => {
    const vm = await initVM(manifestPath);
    const res = vm.callPublicFn("counter", "add", [Cl.uint(2)], wallet1Addr);

    expect(res).toHaveProperty("result");
    expect(res).toHaveProperty("events");
    expect(res.result).toStrictEqual(Cl.ok(Cl.bool(true)));
  });

  it("increases block height when calling public functions", async () => {
    const vm = await initVM(manifestPath);
    const initalBH = vm.blockHeight;

    vm.callPublicFn("counter", "increment", [], wallet1Addr);
    vm.callPublicFn("counter", "increment", [], wallet1Addr);
    expect(vm.blockHeight).toBe(initalBH + 2);
  });

  it("can call public functions in the same block", async () => {
    const vm = await initVM(manifestPath);
    const initalBH = vm.blockHeight;

    const res = vm.mineBlock([
      tx.callPublicFn("counter", "increment", [], wallet1Addr),
      tx.callPublicFn("counter", "increment", [], wallet1Addr),
    ]);

    expect(res).toHaveLength(2);
    expect(res[0]).toHaveProperty("events");
    expect(res[1]).toHaveProperty("events");
    expect(res[0].result).toStrictEqual(Cl.ok(Cl.bool(true)));
    expect(res[1].result).toStrictEqual(Cl.ok(Cl.bool(true)));

    const counterVal = vm.callReadOnlyFn("counter", "get-counter", [], wallet1Addr);
    expect(counterVal.result).toStrictEqual(Cl.ok(Cl.tuple({ counter: Cl.uint(2) })));

    expect(vm.blockHeight).toStrictEqual(initalBH + 1);
  });

  it("can get updated assets map", async () => {
    const vm = await initVM(manifestPath);

    vm.callPublicFn("counter", "increment", [], wallet1Addr);
    vm.callPublicFn("counter", "increment", [], wallet1Addr);

    const assets = vm.getAssetsMap();
    const STX = assets.get("STX")!;
    expect(STX).toHaveLength(5);
    expect(STX.get(wallet1Addr)).toStrictEqual(99999998000000n);
    expect(STX.get(`${deployerAddr}.counter`)).toStrictEqual(2000000n);
  });
});

describe("vm can read contracts data vars and maps", async () => {
  it("can get data-vars", async () => {
    const vm = await initVM(manifestPath);

    const counter = vm.getDataVar("counter", "counter");
    expect(counter).toStrictEqual(Cl.uint(0));
  });

  it("can get map entry", async () => {
    const vm = await initVM(manifestPath);

    // add a participant in the map
    vm.callPublicFn("counter", "increment", [], wallet1Addr);

    const p = vm.getMapEntry("counter", "participants", Cl.standardPrincipal(wallet1Addr));
    expect(p).toStrictEqual(Cl.some(Cl.bool(true)));
  });
});

describe("vm can get contracts info and deploy contracts", async () => {
  it("can get contract interfaces", async () => {
    const vm = await initVM(manifestPath);

    const contractInterfaces = vm.getContractsInterfaces();
    expect(contractInterfaces).toHaveLength(1);

    let counterInterface = contractInterfaces.get(`${deployerAddr}.counter`);
    expect(counterInterface).not.toBeNull();
    expect(counterInterface?.functions).toHaveLength(4);
    expect(counterInterface?.variables).toHaveLength(2);
    expect(counterInterface?.maps).toHaveLength(1);
  });

  it("can deploy contracts as snippets", async () => {
    const vm = await initVM(manifestPath);

    const res = vm.deployContract("temp", "(+ 24 18)", deployerAddr);
    expect(res.result).toStrictEqual(Cl.int(42));

    const contractInterfaces = vm.getContractsInterfaces();
    expect(contractInterfaces).toHaveLength(1);
  });

  it("can deploy contracts", async () => {
    const vm = await initVM(manifestPath);

    const deployRes = vm.deployContract(
      "op",
      "(define-public (add (a uint) (b uint)) (ok (+ a b)))",
      deployerAddr
    );
    expect(deployRes.result).toStrictEqual(Cl.bool(true));

    const contractInterfaces = vm.getContractsInterfaces();
    expect(contractInterfaces).toHaveLength(2);

    const addRes = vm.callPublicFn("op", "add", [Cl.uint(13), Cl.uint(29)], wallet1Addr);
    expect(addRes.result).toStrictEqual(Cl.ok(Cl.uint(42)));
  });
});

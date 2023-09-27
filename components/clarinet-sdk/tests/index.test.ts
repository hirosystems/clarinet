import { Cl } from "@stacks/transactions";
import { describe, expect, it } from "vitest";

// test the built package and not the source code
// makes it simpler to handle wasm build
import { initVM, tx } from "../";

const deployerAddr = "ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM";
const address1 = "ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5";

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
    expect(accounts.get("wallet_1")).toBe(address1);
  });

  it("expose assets maps", async () => {
    const vm = await initVM(manifestPath);

    const assets = vm.getAssetsMap();
    expect(assets.get("STX")).toHaveLength(4);
    expect(assets.get("STX")?.get(address1)).toBe(100000000000000n);
  });

  it("can get and set epoch", async () => {
    const vm = await initVM(manifestPath);

    // should be 2.4 by default
    expect(vm.currentEpoch).toBe("2.4");

    vm.setEpoch("2.0");
    expect(vm.currentEpoch).toBe("2.0");

    // @ts-ignore
    // "0" is an invalid epoch
    // it logs that 0 is invalid and defaults to 2.4
    vm.setEpoch("0");
    expect(vm.currentEpoch).toBe("2.4");
  });
});

describe("vm can call contracts function", async () => {
  it("can call read only functions", async () => {
    const vm = await initVM(manifestPath);
    const res = vm.callReadOnlyFn("counter", "get-count", [], address1);

    expect(res).toHaveProperty("result");
    expect(res).toHaveProperty("events");
    expect(res.result).toStrictEqual(Cl.ok(Cl.tuple({ count: Cl.uint(0) })));
  });

  it("does not increase block height when calling read-only functions", async () => {
    const vm = await initVM(manifestPath);
    const initalBH = vm.blockHeight;

    vm.callReadOnlyFn("counter", "get-count", [], address1);
    vm.callReadOnlyFn("counter", "get-count", [], address1);
    expect(vm.blockHeight).toBe(initalBH);
  });

  it("can call public functions", async () => {
    const vm = await initVM(manifestPath);
    const res = vm.callPublicFn("counter", "increment", [], address1);

    expect(res).toHaveProperty("result");
    expect(res).toHaveProperty("events");
    expect(res.result).toStrictEqual(Cl.ok(Cl.bool(true)));
  });

  it("can call public functions with arguments", async () => {
    const vm = await initVM(manifestPath);
    const res = vm.callPublicFn("counter", "add", [Cl.uint(2)], address1);

    expect(res).toHaveProperty("result");
    expect(res).toHaveProperty("events");
    expect(res.result).toStrictEqual(Cl.ok(Cl.bool(true)));
  });

  it("increases block height when calling public functions", async () => {
    const vm = await initVM(manifestPath);
    const initalBH = vm.blockHeight;

    vm.callPublicFn("counter", "increment", [], address1);
    vm.callPublicFn("counter", "increment", [], address1);
    expect(vm.blockHeight).toBe(initalBH + 2);
  });

  it("can call public functions in the same block", async () => {
    const vm = await initVM(manifestPath);
    const initalBH = vm.blockHeight;

    const res = vm.mineBlock([
      tx.callPublicFn("counter", "increment", [], address1),
      tx.callPublicFn("counter", "increment", [], address1),
    ]);

    expect(res).toHaveLength(2);
    expect(res[0]).toHaveProperty("events");
    expect(res[1]).toHaveProperty("events");
    expect(res[0].result).toStrictEqual(Cl.ok(Cl.bool(true)));
    expect(res[1].result).toStrictEqual(Cl.ok(Cl.bool(true)));

    const counterVal = vm.callReadOnlyFn("counter", "get-count", [], address1);
    expect(counterVal.result).toStrictEqual(Cl.ok(Cl.tuple({ count: Cl.uint(2) })));

    expect(vm.blockHeight).toStrictEqual(initalBH + 1);
  });

  it("can get updated assets map", async () => {
    const vm = await initVM(manifestPath);

    vm.callPublicFn("counter", "increment", [], address1);
    vm.callPublicFn("counter", "increment", [], address1);

    const assets = vm.getAssetsMap();
    const STX = assets.get("STX")!;
    expect(STX).toHaveLength(5);
    expect(STX.get(address1)).toStrictEqual(99999998000000n);
    expect(STX.get(`${deployerAddr}.counter`)).toStrictEqual(2000000n);
  });
});

describe("vm can read contracts data vars and maps", async () => {
  it("can get data-vars", async () => {
    const vm = await initVM(manifestPath);

    const counter = vm.getDataVar("counter", "count");
    expect(counter).toStrictEqual(Cl.uint(0));
  });

  it("can get map entry", async () => {
    const vm = await initVM(manifestPath);

    // add a participant in the map
    vm.callPublicFn("counter", "increment", [], address1);

    const p = vm.getMapEntry("counter", "participants", Cl.standardPrincipal(address1));
    expect(p).toStrictEqual(Cl.some(Cl.bool(true)));
  });
});

describe("vm can get contracts info and deploy contracts", async () => {
  it("can get contract interfaces", async () => {
    const vm = await initVM(manifestPath);

    const contractInterfaces = vm.getContractsInterfaces();
    expect(contractInterfaces).toHaveLength(1);

    const counterInterface = contractInterfaces.get(`${deployerAddr}.counter`);
    expect(counterInterface).not.toBeNull();
    expect(counterInterface?.functions).toHaveLength(4);
    expect(counterInterface?.variables).toHaveLength(2);
    expect(counterInterface?.maps).toHaveLength(1);
  });

  it("can get contract source", async () => {
    const vm = await initVM(manifestPath);

    const counterSource = vm.getContractSource(`${deployerAddr}.counter`);
    expect(counterSource?.startsWith("(define-data-var count")).toBe(true);

    const counterSourceShortAddr = vm.getContractSource("counter");
    expect(counterSourceShortAddr).toBe(counterSource);

    const noSource = vm.getContractSource(`${deployerAddr}.not-counter`);
    expect(noSource).toBeUndefined();
  });

  it("can get contract ast", async () => {
    const vm = await initVM(manifestPath);

    const counterAst = vm.getContractAST(`${deployerAddr}.counter`);
    expect(counterAst).toBeDefined();
    expect(counterAst.expressions).toHaveLength(7);

    const getWithShortAddr = vm.getContractAST("counter");
    expect(getWithShortAddr).toBeDefined();
  });

  it("can deploy contracts as snippets", async () => {
    const vm = await initVM(manifestPath);

    const res = vm.deployContract("temp", "(+ 24 18)", null, deployerAddr);
    expect(res.result).toStrictEqual(Cl.int(42));

    const contractInterfaces = vm.getContractsInterfaces();
    expect(contractInterfaces).toHaveLength(1);
  });

  it("can deploy contracts", async () => {
    const vm = await initVM(manifestPath);

    const source = "(define-public (add (a uint) (b uint)) (ok (+ a b)))\n";
    const deployRes = vm.deployContract("op", source, null, deployerAddr);
    expect(deployRes.result).toStrictEqual(Cl.bool(true));

    const contractInterfaces = vm.getContractsInterfaces();
    expect(contractInterfaces).toHaveLength(2);

    const addRes = vm.callPublicFn("op", "add", [Cl.uint(13), Cl.uint(29)], address1);
    expect(addRes.result).toStrictEqual(Cl.ok(Cl.uint(42)));

    const opSource = vm.getContractSource("op");
    expect(opSource).toBe(source);

    const opASt = vm.getContractAST("op");
    expect(opASt).toBeDefined();
    expect(opASt.expressions).toHaveLength(1);
  });

  it("can deploy contract with a given clarity_version", async () => {
    const vm = await initVM(manifestPath);

    const source = "(define-public (add (a uint) (b uint)) (ok (+ a b)))\n";

    vm.deployContract("contract1", source, { clarityVersion: 1 }, deployerAddr);
    const contract1Interface = vm.getContractsInterfaces().get(`${vm.deployer}.contract1`)!;
    expect(contract1Interface.epoch).toBe("Epoch24");
    expect(contract1Interface.clarity_version).toBe("Clarity1");

    vm.deployContract("contract2", source, { clarityVersion: 2 }, deployerAddr);
    const contract2Interface = vm.getContractsInterfaces().get(`${vm.deployer}.contract2`)!;
    expect(contract2Interface.epoch).toBe("Epoch24");
    expect(contract2Interface.clarity_version).toBe("Clarity2");

    vm.setEpoch("2.0");
    vm.deployContract("contract3", source, { clarityVersion: 1 }, deployerAddr);
    const contract3Interface = vm.getContractsInterfaces().get(`${vm.deployer}.contract3`)!;
    expect(contract3Interface.epoch).toBe("Epoch20");
    expect(contract3Interface.clarity_version).toBe("Clarity1");
  });
});

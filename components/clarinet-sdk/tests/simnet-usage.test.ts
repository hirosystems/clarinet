import fs from "node:fs";
import path from "node:path";
import { Cl } from "@stacks/transactions";
import { describe, expect, it, beforeEach, afterEach } from "vitest";

// test the built package and not the source code
// makes it simpler to handle wasm build
import { Simnet, initSimnet, tx } from "../dist/esm";

const deployerAddr = "ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM";
const address1 = "ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5";
const address2 = "ST2CY5V39NHDPWSXMW9QDT3HC3GD6Q6XX4CFRK9AG";

let simnet: Simnet;

function deleteExistingDeploymentPlan() {
  const deploymentPlan = path.join(
    process.cwd(),
    "tests/fixtures/deployments/default.simnet-plan.yaml",
  );
  if (fs.existsSync(deploymentPlan)) {
    fs.unlinkSync(deploymentPlan);
  }
}

beforeEach(async () => {
  deleteExistingDeploymentPlan();
  simnet = await initSimnet("tests/fixtures/Clarinet.toml");
});

afterEach(() => {
  deleteExistingDeploymentPlan();
});

describe("basic simnet interactions", async () => {
  it("initialize simnet", async () => {
    expect(simnet.blockHeight).toBe(1);
  });

  it("can mine empty blocks", async () => {
    const blockHeight = simnet.blockHeight;
    simnet.mineEmptyBlock();
    expect(simnet.blockHeight).toBe(blockHeight + 1);
    simnet.mineEmptyBlocks(4);
    expect(simnet.blockHeight).toBe(blockHeight + 5);
  });

  it("exposes devnet stacks accounts", async () => {
    const accounts = simnet.getAccounts();

    expect(accounts).toHaveLength(4);
    expect(accounts.get("deployer")).toBe(deployerAddr);
    expect(accounts.get("wallet_1")).toBe(address1);
  });

  it("expose assets maps", async () => {
    const assets = simnet.getAssetsMap();
    expect(assets.get("STX")).toHaveLength(4);
    expect(assets.get("STX")?.get(address1)).toBe(100000000000000n);
  });

  it("can get and set epoch", async () => {
    // should be 2.4 by default
    expect(simnet.currentEpoch).toBe("2.4");

    simnet.setEpoch("2.0");
    expect(simnet.currentEpoch).toBe("2.0");

    // @ts-ignore
    // "0" is an invalid epoch
    // it logs that 0 is invalid and defaults to 2.4
    simnet.setEpoch("0");
    expect(simnet.currentEpoch).toBe("2.4");
  });
});

describe("simnet can run arbitrary snippets", async () => {
  it("can run simple snippets", () => {
    const res = simnet.runSnippet("(+ 1 2)");
    expect(res).toStrictEqual(Cl.int(3));
  });

  it("show diagnostic in case of error", () => {
    const res = simnet.runSnippet("(+ 1 u2)");
    console.log("res", res);
    expect(res).toBe("error:\nexpecting expression of type 'int', found 'uint'");
  });
});

describe("simnet can call contracts function", async () => {
  it("can call read only functions", async () => {
    const res = simnet.callReadOnlyFn("counter", "get-count", [], address1);

    expect(res).toHaveProperty("result");
    expect(res).toHaveProperty("events");
    expect(res.result).toStrictEqual(Cl.ok(Cl.tuple({ count: Cl.uint(0) })));
  });

  it("does not increase block height when calling read-only functions", async () => {
    const initalBH = simnet.blockHeight;

    simnet.callReadOnlyFn("counter", "get-count", [], address1);
    simnet.callReadOnlyFn("counter", "get-count", [], address1);
    expect(simnet.blockHeight).toBe(initalBH);
  });

  it("can call public functions", async () => {
    const res = simnet.callPublicFn("counter", "increment", [], address1);

    expect(res).toHaveProperty("result");
    expect(res).toHaveProperty("events");
    expect(res.result).toStrictEqual(Cl.ok(Cl.bool(true)));

    expect(res.events).toHaveLength(2);
    const printEvent = res.events[0];
    expect(printEvent.event).toBe("print_event");
    expect(printEvent.data.value).toStrictEqual(Cl.stringAscii("call increment"));
  });

  it("can call public functions with arguments", async () => {
    const res = simnet.callPublicFn("counter", "add", [Cl.uint(2)], address1);

    expect(res).toHaveProperty("result");
    expect(res).toHaveProperty("events");
    expect(res.result).toStrictEqual(Cl.ok(Cl.bool(true)));
  });

  it("increases block height when calling public functions", async () => {
    const initalBH = simnet.blockHeight;

    simnet.callPublicFn("counter", "increment", [], address1);
    simnet.callPublicFn("counter", "increment", [], address1);
    expect(simnet.blockHeight).toBe(initalBH + 2);
  });

  it("can call public functions in the same block", async () => {
    const initalBH = simnet.blockHeight;

    const res = simnet.mineBlock([
      tx.callPublicFn("counter", "increment", [], address1),
      tx.callPublicFn("counter", "increment", [], address1),
    ]);

    expect(res).toHaveLength(2);
    expect(res[0]).toHaveProperty("events");
    expect(res[1]).toHaveProperty("events");
    expect(res[0].result).toStrictEqual(Cl.ok(Cl.bool(true)));
    expect(res[1].result).toStrictEqual(Cl.ok(Cl.bool(true)));

    const counterVal = simnet.callReadOnlyFn("counter", "get-count", [], address1);
    expect(counterVal.result).toStrictEqual(Cl.ok(Cl.tuple({ count: Cl.uint(2) })));

    expect(simnet.blockHeight).toStrictEqual(initalBH + 1);
  });

  it("can get updated assets map", async () => {
    simnet.callPublicFn("counter", "increment", [], address1);
    simnet.callPublicFn("counter", "increment", [], address1);

    const assets = simnet.getAssetsMap();
    const STX = assets.get("STX")!;
    expect(STX).toHaveLength(5);
    expect(STX.get(address1)).toStrictEqual(99999998000000n);
    expect(STX.get(`${deployerAddr}.counter`)).toStrictEqual(2000000n);
  });

  it("can pass principals as arguments", async () => {
    const to = Cl.standardPrincipal(address2);
    const { result } = simnet.callPublicFn("counter", "transfer-100", [to], address1);
    expect(result).toStrictEqual(Cl.ok(Cl.bool(true)));
  });

  it("can pass traits as arguments", async () => {
    const trait = Cl.contractPrincipal(simnet.deployer, "multiplier-contract");
    const { result } = simnet.callPublicFn("counter", "call-multiply", [trait], address1);
    expect(result).toStrictEqual(Cl.ok(Cl.uint(4)));
  });
});

describe("simnet can read contracts data vars and maps", async () => {
  it("can get data-vars", async () => {
    const counter = simnet.getDataVar("counter", "count");
    expect(counter).toStrictEqual(Cl.uint(0));
  });
  it("can get block time", async () => {
    const bt = simnet.getBlockTime();
    expect(bt).toBeDefined();
  });

  it("can get map entry", async () => {
    // add a participant in the map
    simnet.callPublicFn("counter", "increment", [], address1);

    const p = simnet.getMapEntry("counter", "participants", Cl.standardPrincipal(address1));
    expect(p).toStrictEqual(Cl.some(Cl.bool(true)));
  });
});

describe("simnet can get contracts info and deploy contracts", async () => {
  it("can get contract interfaces", async () => {
    const contractInterfaces = simnet.getContractsInterfaces();
    expect(contractInterfaces).toHaveLength(3);

    const counterInterface = contractInterfaces.get(`${deployerAddr}.counter`);
    expect(counterInterface).not.toBeNull();
    expect(counterInterface?.functions).toHaveLength(6);
    expect(counterInterface?.variables).toHaveLength(2);
    expect(counterInterface?.maps).toHaveLength(1);
  });

  it("can get contract source", async () => {
    const counterSource = simnet.getContractSource(`${deployerAddr}.counter`);
    expect(counterSource?.startsWith("(define-data-var count")).toBe(true);

    const counterSourceShortAddr = simnet.getContractSource("counter");
    expect(counterSourceShortAddr).toBe(counterSource);

    const noSource = simnet.getContractSource(`${deployerAddr}.not-counter`);
    expect(noSource).toBeUndefined();
  });

  it("can get contract ast", async () => {
    const counterAst = simnet.getContractAST(`${deployerAddr}.counter`);
    expect(counterAst).toBeDefined();
    expect(counterAst.expressions).toHaveLength(10);

    const getWithShortAddr = simnet.getContractAST("counter");
    expect(getWithShortAddr).toBeDefined();
  });

  it("can deploy contracts as snippets", async () => {
    const res = simnet.deployContract("temp", "(+ 24 18)", null, deployerAddr);
    expect(res.result).toStrictEqual(Cl.int(42));

    const contractInterfaces = simnet.getContractsInterfaces();
    expect(contractInterfaces).toHaveLength(3);
  });

  it("can deploy contracts", async () => {
    const source = "(define-public (add (a uint) (b uint)) (ok (+ a b)))\n";
    const deployRes = simnet.deployContract("op", source, null, deployerAddr);
    expect(deployRes.result).toStrictEqual(Cl.bool(true));

    const contractInterfaces = simnet.getContractsInterfaces();
    expect(contractInterfaces).toHaveLength(4);

    const addRes = simnet.callPublicFn("op", "add", [Cl.uint(13), Cl.uint(29)], address1);
    expect(addRes.result).toStrictEqual(Cl.ok(Cl.uint(42)));

    const opSource = simnet.getContractSource("op");
    expect(opSource).toBe(source);

    const opASt = simnet.getContractAST("op");
    expect(opASt).toBeDefined();
    expect(opASt.expressions).toHaveLength(1);
  });

  it("can deploy contract with a given clarity_version", async () => {
    const source = "(define-public (add (a uint) (b uint)) (ok (+ a b)))\n";

    simnet.deployContract("contract1", source, { clarityVersion: 1 }, deployerAddr);
    const contract1Interface = simnet.getContractsInterfaces().get(`${simnet.deployer}.contract1`)!;
    expect(contract1Interface.epoch).toBe("Epoch24");
    expect(contract1Interface.clarity_version).toBe("Clarity1");

    simnet.deployContract("contract2", source, { clarityVersion: 2 }, deployerAddr);
    const contract2Interface = simnet.getContractsInterfaces().get(`${simnet.deployer}.contract2`)!;
    expect(contract2Interface.epoch).toBe("Epoch24");
    expect(contract2Interface.clarity_version).toBe("Clarity2");

    simnet.setEpoch("2.0");
    simnet.deployContract("contract3", source, { clarityVersion: 1 }, deployerAddr);
    const contract3Interface = simnet.getContractsInterfaces().get(`${simnet.deployer}.contract3`)!;
    expect(contract3Interface.epoch).toBe("Epoch20");
    expect(contract3Interface.clarity_version).toBe("Clarity1");
  });
});

describe("simnet can transfer stx", () => {
  it("can transfer stx", async () => {
    simnet.transferSTX(1000, address2, address1);
    const stxBalances = simnet.getAssetsMap().get("STX");
    const stxAddress1 = stxBalances?.get(address1);
    const stxAddress2 = stxBalances?.get(address2);
    expect(stxAddress1).toBe(99999999999000n);
    expect(stxAddress2).toBe(100000000001000n);
  });
});

describe("simnet can get session reports", async () => {
  it("can get line coverage", async () => {
    simnet.callPublicFn("counter", "increment", [], address1);
    simnet.callPublicFn("counter", "increment", [], address1);

    const reports = simnet.collectReport();
    expect(reports.coverage.startsWith("TN:")).toBe(true);
    expect(reports.coverage.endsWith("end_of_record\n")).toBe(true);
  });

  it("can get costs", async () => {
    simnet.callPublicFn("counter", "increment", [], address1);

    const reports = simnet.collectReport();
    expect(() => JSON.parse(reports.costs)).not.toThrow();

    const parsedReports = JSON.parse(reports.costs);
    expect(parsedReports).toHaveLength(1);

    const report = parsedReports[0];
    expect(report.contract_id).toBe(`${simnet.deployer}.counter`);
    expect(report.method).toBe("increment");
    expect(report.cost_result.total.write_count).toBe(3);
  });
});

describe("the sdk handles multiple manifests project", () => {
  it("handle invalid project", () => {
    const manifestPath = path.join(process.cwd(), "tests/fixtures/contracts/invalid.clar");
    const expectedErr = `error: unexpected ')'\n--> ${manifestPath}:5:2\n)) ;; extra \`)\`\n`;

    expect(async () => {
      await initSimnet("tests/fixtures/InvalidManifest.toml");
    }).rejects.toThrow(expectedErr);
  });
});

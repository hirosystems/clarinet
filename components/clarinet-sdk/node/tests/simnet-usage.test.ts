import fs from "node:fs";
import path from "node:path";
import { Cl } from "@stacks/transactions";
import { describe, expect, it, beforeEach, afterEach } from "vitest";

// test the built package and not the source code
// makes it simpler to handle wasm build
import { Simnet, initSimnet, tx } from "..";

const deployerAddr = "ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM";
const address1 = "ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5";
const address2 = "ST2CY5V39NHDPWSXMW9QDT3HC3GD6Q6XX4CFRK9AG";

let simnet: Simnet;

const nbOfBootContracts = 24;

const deploymentPlanPath = path.join(
  process.cwd(),
  "tests/fixtures/deployments/default.simnet-plan.yaml",
);

function deleteExistingDeploymentPlan() {
  if (fs.existsSync(deploymentPlanPath)) {
    fs.unlinkSync(deploymentPlanPath);
  }
}

beforeEach(async () => {
  deleteExistingDeploymentPlan();
  simnet = await initSimnet("tests/fixtures/Clarinet.toml", false, {
    trackCosts: true,
    trackCoverage: false,
  });
});

afterEach(() => {
  deleteExistingDeploymentPlan();
});

describe("basic simnet interactions", () => {
  it("initialize simnet", () => {
    expect(simnet.blockHeight).toBe(1);
  });

  it("can run command", () => {
    const r = simnet.executeCommand("::set_epoch 3.1");
    expect(r).toBe("Epoch updated to: 3.1");
  });

  it("can mine empty blocks", () => {
    const blockHeight = simnet.blockHeight;
    simnet.mineEmptyBlock();
    expect(simnet.blockHeight).toBe(blockHeight + 1);
    simnet.mineEmptyBlocks(4);
    expect(simnet.blockHeight).toBe(blockHeight + 5);
  });

  it("can not mine empty stacks block in pre-3.0", () => {
    expect(() => simnet.mineEmptyStacksBlock()).toThrowError(
      "use mineEmptyBurnBlock in epoch lower than 3.0",
    );
  });

  it("exposes devnet stacks accounts", () => {
    const accounts = simnet.getAccounts();

    expect(accounts).toHaveLength(4);
    expect(accounts.get("deployer")).toBe(deployerAddr);
    expect(accounts.get("wallet_1")).toBe(address1);
  });

  it("expose assets maps", () => {
    const assets = simnet.getAssetsMap();
    expect(assets.get("STX")).toHaveLength(4);
    expect(assets.get("STX")?.get(address1)).toBe(100000000000000n);
  });

  it("can get and set epoch", () => {
    // should be 2.4 at the beginning because
    // the latest contract in the manifest is deployed in 2.4
    expect(simnet.currentEpoch).toBe("2.4");

    simnet.setEpoch("2.5");
    expect(simnet.currentEpoch).toBe("2.5");

    // @ts-ignore
    // "0" is an invalid epoch
    // it logs that 0 is invalid and defaults to 3.1
    simnet.setEpoch("0");
    expect(simnet.currentEpoch).toBe("3.1");
  });

  it("can get default clarity version for current epoch", () => {
    const clarityVersion = simnet.getDefaultClarityVersionForCurrentEpoch();
    expect(clarityVersion).toBe("Clarity 2");
  });
});

describe("simnet epoch 3", () => {
  it("can mine empty blocks", () => {
    simnet.setEpoch("3.0");
    const blockHeight = simnet.stacksBlockHeight;
    const burnBlockHeight = simnet.burnBlockHeight;
    simnet.mineEmptyStacksBlock();
    expect(simnet.stacksBlockHeight).toBe(blockHeight + 1);
    expect(simnet.burnBlockHeight).toBe(burnBlockHeight);
    simnet.mineEmptyStacksBlocks(4);
    expect(simnet.stacksBlockHeight).toBe(blockHeight + 5);
    simnet.mineEmptyBurnBlocks(4);
    expect(simnet.burnBlockHeight).toBe(burnBlockHeight + 4);
    expect(simnet.stacksBlockHeight).toBe(blockHeight + 9);
  });
});
describe("simnet can run arbitrary snippets", () => {
  it("can run simple snippets", () => {
    const res = simnet.execute("(+ 1 2)");
    expect(res.result).toStrictEqual(Cl.int(3));
  });

  it("show diagnostic in case of error", () => {
    expect(() => {
      simnet.execute("(+ 1 u2)");
    }).toThrow("error: expecting expression of type 'int', found 'uint'");
  });
});

describe("simnet can call contracts function", () => {
  it("can call read only functions", () => {
    const res = simnet.callReadOnlyFn("counter", "get-count", [], address1);

    expect(res).toHaveProperty("result");
    expect(res).toHaveProperty("events");
    expect(res.result).toStrictEqual(Cl.ok(Cl.tuple({ count: Cl.uint(0) })));
  });

  it("does not increase block height when calling read-only functions", () => {
    const initalBH = simnet.blockHeight;

    simnet.callReadOnlyFn("counter", "get-count", [], address1);
    simnet.callReadOnlyFn("counter", "get-count", [], address1);
    expect(simnet.blockHeight).toBe(initalBH);
  });

  it("can call public functions", () => {
    const res = simnet.callPublicFn("counter", "increment", [], address1);

    expect(res).toHaveProperty("result");
    expect(res).toHaveProperty("events");
    expect(res.result).toStrictEqual(Cl.ok(Cl.bool(true)));

    expect(res.events).toHaveLength(3);
    const printEvent = res.events[0];
    expect(printEvent.event).toBe("print_event");
    expect(printEvent.data.value).toStrictEqual(Cl.stringAscii("call increment"));
  });

  it("reports costs", () => {
    const res = simnet.callPublicFn("counter", "increment", [], address1);

    expect(res).toHaveProperty("costs");
    expect(res.costs).toStrictEqual({
      memory: 417,
      memory_limit: 100000000,
      total: {
        writeLength: 44,
        writeCount: 3,
        readLength: 1466,
        readCount: 8,
        runtime: 15630,
      },
      limit: {
        writeLength: 15000000,
        writeCount: 15000,
        readLength: 100000000,
        readCount: 15000,
        runtime: 5000000000,
      },
    });
  });

  it("can call public functions with arguments", () => {
    const res = simnet.callPublicFn("counter", "add", [Cl.uint(2)], address1);

    expect(res).toHaveProperty("result");
    expect(res).toHaveProperty("events");
    expect(res.result).toStrictEqual(Cl.ok(Cl.bool(true)));
  });

  it("increases block height when calling public functions", () => {
    const initalBH = simnet.blockHeight;

    simnet.callPublicFn("counter", "increment", [], address1);
    simnet.callPublicFn("counter", "increment", [], address1);
    expect(simnet.blockHeight).toBe(initalBH + 2);
  });

  it("can call public functions in the same block", () => {
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

  it("can call private functions", () => {
    const { result, events } = simnet.callPrivateFn("counter", "inner-increment", [], address1);
    expect(events).toHaveLength(1);
    expect(result).toStrictEqual(Cl.bool(true));
  });

  it("can call public and private functions in the same block", () => {
    const initalBH = simnet.blockHeight;

    const res = simnet.mineBlock([
      tx.callPrivateFn("counter", "inner-increment", [], address1),
      tx.callPublicFn("counter", "increment", [], address1),
      tx.callPrivateFn("counter", "inner-increment", [], address1),
    ]);

    expect(res[0].result).toStrictEqual(Cl.bool(true));
    expect(res[1].result).toStrictEqual(Cl.ok(Cl.bool(true)));
    expect(res[2].result).toStrictEqual(Cl.bool(true));

    const counterVal = simnet.callReadOnlyFn("counter", "get-count", [], address1);
    expect(counterVal.result).toStrictEqual(Cl.ok(Cl.tuple({ count: Cl.uint(3) })));

    expect(simnet.blockHeight).toStrictEqual(initalBH + 1);
  });

  it("can not call a public function with callPrivateFn", () => {
    expect(() => {
      simnet.callPrivateFn("counter", "increment", [], address1);
    }).toThrow(/^increment is not a private function$/);
  });

  it("can not call a private function with callPublicFn", () => {
    expect(() => {
      simnet.callPublicFn("counter", "inner-increment", [], address1);
    }).toThrow(/^inner-increment is not a public function$/);
  });

  it("can get updated assets map", () => {
    simnet.callPublicFn("counter", "increment", [], address1);
    simnet.callPublicFn("counter", "increment", [], address1);

    const assets = simnet.getAssetsMap();
    const STX = assets.get("STX")!;
    expect(STX).toHaveLength(5);
    expect(STX.get(address1)).toStrictEqual(99999998000000n);
    expect(STX.get(`${deployerAddr}.counter`)).toStrictEqual(2000000n);
  });

  it("can pass principals as arguments", () => {
    const to = Cl.standardPrincipal(address2);
    const { result } = simnet.callPublicFn("counter", "transfer-100", [to], address1);
    expect(result).toStrictEqual(Cl.ok(Cl.bool(true)));
  });

  it("can pass traits as arguments", () => {
    const trait = Cl.contractPrincipal(simnet.deployer, "multiplier-contract");
    const { result } = simnet.callPublicFn("counter", "call-multiply", [trait], address1);
    expect(result).toStrictEqual(Cl.ok(Cl.uint(4)));
  });
});

describe("mineBlock and callPublicFunction properly handle block height incrementation", () => {
  const expectedReturnedBH = 2;

  it("increases the block height after the call in callPublicFn", () => {
    const { result } = simnet.callPublicFn("block-height-tests", "get-block-height", [], address1);
    expect(result).toStrictEqual(Cl.ok(Cl.uint(expectedReturnedBH)));
  });

  it("increases the block height after the call in mineBlock", () => {
    const [{ result }] = simnet.mineBlock([
      tx.callPublicFn("block-height-tests", "get-block-height", [], address1),
    ]);
    expect(result).toStrictEqual(Cl.ok(Cl.uint(expectedReturnedBH)));
  });
});

describe("simnet can read contracts data vars and maps", () => {
  it("can get data-vars", () => {
    const counter = simnet.getDataVar("counter", "count");
    expect(counter).toStrictEqual(Cl.uint(0));
  });
  it("can get block time", () => {
    const bt = simnet.getBlockTime();
    expect(bt).toBeDefined();
  });

  it("can get map entry", () => {
    // add a participant in the map
    simnet.callPublicFn("counter", "increment", [], address1);

    const p = simnet.getMapEntry("counter", "participants", Cl.standardPrincipal(address1));
    expect(p).toStrictEqual(Cl.some(Cl.bool(true)));
  });
});

describe("simnet can get contracts info and deploy contracts", () => {
  it("can get contract interfaces", () => {
    const contractInterfaces = simnet.getContractsInterfaces();
    expect(contractInterfaces).toHaveLength(nbOfBootContracts + 4);

    const counterInterface = contractInterfaces.get(`${deployerAddr}.counter`);
    expect(counterInterface).not.toBeNull();
    expect(counterInterface?.functions).toHaveLength(7);
    expect(counterInterface?.variables).toHaveLength(2);
    expect(counterInterface?.maps).toHaveLength(1);
  });

  it("can get contract source", () => {
    const counterSource = simnet.getContractSource(`${deployerAddr}.counter`);
    expect(counterSource?.startsWith(";; counter contract")).toBe(true);

    const counterSourceShortAddr = simnet.getContractSource("counter");
    expect(counterSourceShortAddr).toBe(counterSource);

    const noSource = simnet.getContractSource(`${deployerAddr}.not-counter`);
    expect(noSource).toBeUndefined();
  });

  it("can get contract ast", () => {
    const counterAst = simnet.getContractAST(`${deployerAddr}.counter`);

    expect(counterAst).toBeDefined();
    expect(counterAst.expressions).toHaveLength(11);

    const getWithShortAddr = simnet.getContractAST("counter");
    expect(getWithShortAddr).toBeDefined();
  });

  it("can get commets in ast", () => {
    const counterAst = simnet.getContractAST(`${deployerAddr}.counter`);

    expect(counterAst).toBeDefined();
    expect(counterAst.expressions).toHaveLength(11);

    // @ts-ignore
    expect(counterAst.expressions[0].pre_comments[0][0]).toBe("counter contract");
  });

  it("can deploy contracts as snippets", () => {
    simnet.setEpoch("3.0");
    const res = simnet.deployContract("temp", "(+ 24 18)", null, deployerAddr);
    expect(res.result).toStrictEqual(Cl.int(42));

    const contractInterfaces = simnet.getContractsInterfaces();
    expect(contractInterfaces).toHaveLength(nbOfBootContracts + 4);
  });

  it("can deploy contracts", () => {
    simnet.setEpoch("3.0");
    const source = "(define-public (add (a uint) (b uint)) (ok (+ a b)))\n";
    const deployRes = simnet.deployContract("op", source, null, deployerAddr);
    expect(deployRes.result).toStrictEqual(Cl.bool(true));

    const contractInterfaces = simnet.getContractsInterfaces();
    expect(contractInterfaces).toHaveLength(nbOfBootContracts + 5);

    const addRes = simnet.callPublicFn("op", "add", [Cl.uint(13), Cl.uint(29)], address1);
    expect(addRes.result).toStrictEqual(Cl.ok(Cl.uint(42)));

    const opSource = simnet.getContractSource("op");
    expect(opSource).toBe(source);

    const opASt = simnet.getContractAST("op");
    expect(opASt).toBeDefined();
    expect(opASt.expressions).toHaveLength(1);
  });

  it("can deploy contract with a given clarity_version", () => {
    const source = "(define-public (add (a uint) (b uint)) (ok (+ a b)))\n";

    simnet.deployContract("contract1", source, { clarityVersion: 1 }, deployerAddr);
    const contract1Interface = simnet.getContractsInterfaces().get(`${simnet.deployer}.contract1`)!;
    expect(contract1Interface.epoch).toBe("Epoch24");
    expect(contract1Interface.clarity_version).toBe("Clarity1");

    simnet.deployContract("contract2", source, { clarityVersion: 2 }, deployerAddr);
    const contract2Interface = simnet.getContractsInterfaces().get(`${simnet.deployer}.contract2`)!;
    expect(contract2Interface.epoch).toBe("Epoch24");
    expect(contract2Interface.clarity_version).toBe("Clarity2");

    simnet.setEpoch("2.5");
    simnet.deployContract("contract3", source, { clarityVersion: 1 }, deployerAddr);
    const contract3Interface = simnet.getContractsInterfaces().get(`${simnet.deployer}.contract3`)!;
    expect(contract3Interface.epoch).toBe("Epoch25");
    expect(contract3Interface.clarity_version).toBe("Clarity1");
  });
});

describe("simnet can transfer stx", () => {
  it("can transfer stx", () => {
    simnet.transferSTX(1000, address2, address1);
    const stxBalances = simnet.getAssetsMap().get("STX");
    const stxAddress1 = stxBalances?.get(address1);
    const stxAddress2 = stxBalances?.get(address2);
    expect(stxAddress1).toBe(99999999999000n);
    expect(stxAddress2).toBe(100000000001000n);
  });
});

describe("the simnet can execute commands", () => {
  it("can mint_stx", () => {
    const result = simnet.executeCommand(
      "::mint_stx ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM 1000",
    );
    expect(result).toBe("→ ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM: 100000000001000 µSTX");
  });

  it("can get_assets_maps", () => {
    simnet.executeCommand("::mint_stx ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM 1000");
    let result = simnet.executeCommand("::get_assets_maps");
    const expected = [
      "+-------------------------------------------+-----------------+",
      "| Address                                   | uSTX            |",
      "+-------------------------------------------+-----------------+",
      "| ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM | 100000000001000 |",
      "+-------------------------------------------+-----------------+",
      "| ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5 | 100000000000000 |",
      "+-------------------------------------------+-----------------+",
      "| ST2CY5V39NHDPWSXMW9QDT3HC3GD6Q6XX4CFRK9AG | 100000000000000 |",
      "+-------------------------------------------+-----------------+",
      "| STNHKEPYEPJ8ET55ZZ0M5A34J0R3N5FM2CMMMAZ6  | 100000000000000 |",
      "+-------------------------------------------+-----------------+\n",
    ].join("\n");
    expect(result).toBe(expected);
  });
});

// describe("custom manifest path", () => {
//   it("initSimnet handles absolute path", async () => {
//     const manifestPath = path.join(process.cwd(), "tests/fixtures/Clarinet.toml");
//     const simnet = await initSimnet(manifestPath);
//     expect(simnet.blockHeight).toBe(1);
//   });
// });

describe("the sdk handles multiple manifests project", () => {
  it("handle invalid project", async () => {
    // the lsp displays paths with the unix notation, hence why we are hardcoding the contract path with `/`
    const manifestPath = `${process.cwd()}/tests/fixtures/contracts/invalid.clar`;
    const expectedErr = `error: unexpected ')'\n--> ${manifestPath}:5:2\n)) ;; extra \`)\`\n`;

    await expect(async () => {
      await initSimnet("tests/fixtures/InvalidManifest.toml");
    }).rejects.toThrow(expectedErr);
  });
});

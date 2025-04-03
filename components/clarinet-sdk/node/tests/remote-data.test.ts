import fs from "node:fs";
import path from "node:path";
import { Cl } from "@stacks/transactions";
import { describe, expect, it, beforeEach, afterEach, afterAll, beforeAll } from "vitest";

// test the built package and not the source code
// makes it simpler to handle wasm build
import { getSDK } from "..";

const api_url = "https://api.testnet.hiro.so";
const counterAddress = "STJCAB2T9TR2EJM7YS4DM2CGBBVTF7BV237Y8KNV.counter";
const sender = "ST2CY5V39NHDPWSXMW9QDT3HC3GD6Q6XX4CFRK9AG";

const deploymentPlanPath = path.join(
  process.cwd(),
  "tests/fixtures/deployments/default.simnet-plan.yaml",
);
const metadataCachePath = path.join(process.cwd(), "./.cache");

function deleteExistingDeploymentPlan() {
  if (fs.existsSync(deploymentPlanPath)) {
    fs.unlinkSync(deploymentPlanPath);
  }
}

function deleteMetadataFsCache() {
  if (fs.existsSync(metadataCachePath)) {
    fs.rmSync(metadataCachePath, { recursive: true, force: true });
  }
}

beforeEach(() => {
  deleteExistingDeploymentPlan();
});
afterEach(() => {
  deleteExistingDeploymentPlan();
});

beforeAll(() => {
  deleteMetadataFsCache();
});
afterAll(() => {
  deleteMetadataFsCache();
});

describe("simnet remote interactions", async () => {
  const simnet = await getSDK();

  it("can call a remote contract", async () => {
    await simnet.initEmptySession({
      enabled: true,
      api_url: "https://api.testnet.hiro.so",
      initial_height: 56230,
    });
    const { result } = simnet.callReadOnlyFn(counterAddress, "get-count", [], sender);
    expect(result).toStrictEqual(Cl.uint(0));
  });

  it("can call a remote contract", async () => {
    await simnet.initEmptySession({
      enabled: true,
      api_url: "https://api.testnet.hiro.so",
      initial_height: 57000,
    });
    const { result } = simnet.callReadOnlyFn(counterAddress, "get-count", [], sender);
    expect(result).toStrictEqual(Cl.uint(1));
  });

  it("can use at-block", async () => {
    await simnet.initEmptySession({
      enabled: true,
      api_url: "https://api.testnet.hiro.so",
      initial_height: 57000,
    });
    const { result: resultAt56230 } = simnet.callReadOnlyFn(
      counterAddress,
      "get-count-at-block",
      [Cl.uint(56230)],
      sender,
    );
    expect(resultAt56230).toStrictEqual(Cl.ok(Cl.uint(0)));
    const { result: resultAt56300 } = simnet.callReadOnlyFn(
      counterAddress,
      "get-count-at-block",
      [Cl.uint(56300)],
      sender,
    );
    expect(resultAt56300).toStrictEqual(Cl.ok(Cl.uint(1)));
  });

  it("caches metadata", async () => {
    await simnet.initEmptySession({
      enabled: true,
      api_url: "https://api.testnet.hiro.so",
      initial_height: 56230,
    });
    const { result } = simnet.callReadOnlyFn(counterAddress, "get-count", [], sender);
    expect(result).toStrictEqual(Cl.uint(0));

    const cachePath = path.join(process.cwd(), "./.cache/datastore");
    expect(fs.existsSync(cachePath)).toBe(true);
    const files = fs.readdirSync(cachePath);
    expect(files).toHaveLength(6);
    expect(files).toContain(
      "STJCAB2T9TR2EJM7YS4DM2CGBBVTF7BV237Y8KNV_counter_vm-metadata__9__contract_8b1963abdc117b1b925d8f0390bf5001dec17ad91adc5309c00c7d5ac0b5bfd0.json",
    );
  });

  it("throws an error if the contract is not available at a given block height", async () => {
    await simnet.initEmptySession({
      enabled: true,
      api_url: "https://api.testnet.hiro.so",
      // the counter contract is deployed at 41613
      initial_height: 41000,
    });
    expect(() => simnet.callReadOnlyFn(counterAddress, "get-count", [], sender)).toThrowError(
      `Call contract function error: ${counterAddress}::get-count() -> Error calling contract function: Runtime error while interpreting ${counterAddress}: Interpreter(Expect("Failed to read non-consensus contract metadata, even though contract exists in MARF."))`,
    );
  });
});

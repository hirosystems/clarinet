import fs from "node:fs";
import path from "node:path";
import { Cl } from "@stacks/transactions";
import { describe, expect, it, beforeEach, afterEach } from "vitest";

// test the built package and not the source code
// makes it simpler to handle wasm build
import { Simnet, getSDK, initSimnet, tx } from "..";

const deployerAddr = "ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM";
const address1 = "ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5";
const address2 = "ST2CY5V39NHDPWSXMW9QDT3HC3GD6Q6XX4CFRK9AG";

let simnet: Simnet;

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
  simnet = await getSDK();
  await simnet.initEmptySession({
    enabled: true,
    api_url: "http://localhost:3999",
    initial_height: 186,
  });
});

afterEach(() => {
  deleteExistingDeploymentPlan();
});

describe("simnet remote interactions", () => {
  it("can call a remote contract", () => {
    const result = simnet.callReadOnlyFn(`${address1}.counter`, "get-count", [], address2);
    expect(result.result).toStrictEqual(Cl.uint(2));
  });
});

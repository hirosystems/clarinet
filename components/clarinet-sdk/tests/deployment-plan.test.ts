import fs from "node:fs";
import path from "node:path";
import { Cl } from "@stacks/transactions";
import { describe, expect, it, beforeEach, afterEach, assert } from "vitest";

// test the built package and not the source code
// makes it simpler to handle wasm build
import { initSimnet } from "../";

const deploymentPlan = path.join(
  process.cwd(),
  "tests/fixtures/deployments/default.simnet-plan.yaml",
);
const customDeploymentPlan = path.join(
  process.cwd(),
  "tests/fixtures/deployments/custom.simnet-plan.yaml",
);

function deleteExistingDeploymentPlan() {
  if (fs.existsSync(deploymentPlan)) {
    fs.unlinkSync(deploymentPlan);
  }
}

beforeEach(async () => {
  deleteExistingDeploymentPlan();
});

afterEach(() => {
  deleteExistingDeploymentPlan();
});

describe("deployment plans test", async () => {
  it("simnet deployment plan is created if it does not exist", async () => {
    assert(!fs.existsSync(deploymentPlan));
    const simnet = await initSimnet("tests/fixtures/Clarinet.toml");
    // make sure the simnet is running
    expect(simnet.blockHeight).toBe(1);
    assert(fs.existsSync(deploymentPlan));
  });

  it("handle custom deployment plan", async () => {
    deleteExistingDeploymentPlan();
    fs.copyFileSync(customDeploymentPlan, deploymentPlan);
    assert(fs.existsSync(deploymentPlan));

    // init simnet with no cache to load the new deployment plan
    const simnet = await initSimnet("tests/fixtures/Clarinet.toml", true);
    const result = simnet.getDataVar("counter", "count");

    // the count is 2 because the custom deployment plan calls (add u2)
    expect(result).toMatchObject(Cl.uint(2));
  });
});

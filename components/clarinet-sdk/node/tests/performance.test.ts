import fs from "node:fs";
import path from "node:path";
import { Cl } from "@stacks/transactions";
import { describe, expect, it, beforeEach, afterEach } from "vitest";

// test the built package and not the source code
// makes it simpler to handle wasm build
import { Simnet, initSimnet } from "..";

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
  simnet = await initSimnet("tests/fixtures/Clarinet.toml", false, {
    trackCosts: false,
    trackCoverage: false,
    trackPerformance: true,
  });
});

afterEach(() => {
  deleteExistingDeploymentPlan();
});

describe("performance tracking", () => {
  it("can track performance during contract calls", async () => {
    await simnet.enablePerformance("runtime");

    const result = simnet.callReadOnlyFn(
      "counter",
      "get-count",
      [],
      "ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5",
    );

    expect(result.performance).toBeDefined();
    expect(result.performance).not.toBeNull();

    if (result.performance) {
      expect(result.performance).toContain(";");
      expect(result.performance.trim()).not.toBe("");
    }
  });
});

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
    performanceCostField: "runtime",
  });
});

afterEach(() => {
  deleteExistingDeploymentPlan();
  // Clean up any perf.data file that might have been created
  if (fs.existsSync("perf.data")) {
    fs.unlinkSync("perf.data");
  }
});

describe("performance tracking", () => {
  it("can initialize simnet with performance tracking", () => {
    expect(simnet.blockHeight).toBe(1);
  });

  it("can track performance during simple operations", () => {
    // Execute a simple operation to test performance tracking
    const result = simnet.execute("(+ 1 2)");
    expect(Cl.prettyPrint(result.result)).toBe("3");
  });

  it("can track different cost fields", async () => {
    // Test with read_count tracking
    const simnetReadCount = await initSimnet("tests/fixtures/Clarinet.toml", false, {
      trackCosts: false,
      trackCoverage: false,
      trackPerformance: true,
      performanceCostField: "read_count",
    });

    // Execute a simple operation to test read_count tracking
    const result = simnetReadCount.execute("(+ 1 2)");
    expect(Cl.prettyPrint(result.result)).toBe("3");
  });
});

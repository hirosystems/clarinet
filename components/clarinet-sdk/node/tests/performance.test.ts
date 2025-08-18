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
    performanceFilename: "performance-tracking.perf.data",
  });
});

afterEach(() => {
  deleteExistingDeploymentPlan();
});

describe("performance tracking", () => {
  it("can track performance during simple operations", () => {
    const result = simnet.execute("(+ 1 2)");
    expect(Cl.prettyPrint(result.result)).toBe("3");

    const perfData = simnet.collectPerformanceData();
    expect(perfData).toBeDefined();
    expect(perfData).not.toBeNull();

    // The performance data should contain some information
    if (perfData) {
      expect(perfData).toContain(";");
      expect(perfData.trim()).not.toBe("");
    }
  });
});

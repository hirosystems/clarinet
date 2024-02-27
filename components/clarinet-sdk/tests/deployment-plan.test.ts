import { Cl } from "@stacks/transactions";
import { describe, expect, it, beforeEach } from "vitest";

// test the built package and not the source code
// makes it simpler to handle wasm build
import { Simnet, initSimnet, tx } from "../";
import path from "node:path";
import { assert } from "node:console";

const deployerAddr = "ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM";
const address1 = "ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5";
const address2 = "ST2CY5V39NHDPWSXMW9QDT3HC3GD6Q6XX4CFRK9AG";

let simnet: Simnet;

beforeEach(async () => {
  simnet = await initSimnet("tests/fixtures/Clarinet.toml");
});

describe("basic simnet interactions", async () => {
  it("initialize simnet", async () => {
    const { result } = simnet.callReadOnlyFn("counter", "get-count", [], address1);
    expect(result).toBeDefined();
  });
});

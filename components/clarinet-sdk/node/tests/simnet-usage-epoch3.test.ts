import fs from "node:fs";
import path from "node:path";
import { describe, expect, it, beforeEach, afterEach } from "vitest";

// test the built package and not the source code
// makes it simpler to handle wasm build
import { Simnet, initSimnet } from "..";

let simnet: Simnet;

beforeEach(async () => {
  simnet = await initSimnet("tests/fixtures/Clarinet.toml");
  simnet.setEpoch("3.0");
});

describe("basic simnet interactions", () => {
  it("initialize simnet", () => {
    expect(simnet.blockHeight).toBe(1);
  });

  it("can mine empty blocks", () => {
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
  })
})
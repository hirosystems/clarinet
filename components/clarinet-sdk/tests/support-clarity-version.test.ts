import { Cl } from "@stacks/transactions";
import { describe, expect, it, beforeEach } from "vitest";

// test the built package and not the source code
// makes it simpler to handle wasm build
import { Simnet, initSimnet } from "../dist/esm";

let simnet: Simnet;

beforeEach(async () => {
  simnet = await initSimnet("tests/fixtures/Clarinet.toml");
});

describe.only("the sdk handle all clarity version", () => {
  it("handle clarity 1", () => {
    simnet.setEpoch("2.05");
    let resOk = simnet.runSnippet('(index-of "stacks" "s")');
    expect(resOk).toStrictEqual(Cl.some(Cl.uint(0)));

    // `index-of?` was introduced in clarity 2
    let resFail1 = simnet.runSnippet('(index-of? "stacks" "s")');
    expect(resFail1).toBe("error:\nuse of unresolved function 'index-of?'");

    // `tenure-height` was introduced in clarity 3
    let resFail2 = simnet.runSnippet("(print tenure-height)");
    expect(resFail2).toBe("error:\nuse of unresolved variable 'tenure-height'");
  });

  it("handle clarity 2", () => {
    simnet.setEpoch("2.4");
    // `index-of` is still available in clarity 2
    let resOk1 = simnet.runSnippet('(index-of "stacks" "s")');
    expect(resOk1).toStrictEqual(Cl.some(Cl.uint(0)));

    // `index-of?` is available in clarity 2
    let resOk2 = simnet.runSnippet('(index-of? "stacks" "s")');
    expect(resOk2).toStrictEqual(Cl.some(Cl.uint(0)));

    // `block-height` is avaliable in clarity 1 & 2
    let resOk3 = simnet.runSnippet("(print block-height)");
    expect(resOk3).toStrictEqual(Cl.uint(1));

    // `tenure-height` was introduced in clarity 3
    let resFail = simnet.runSnippet("(print tenure-height)");
    expect(resFail).toBe("error:\nuse of unresolved variable 'tenure-height'");
  });

  it("handle clarity 3", () => {
    simnet.setEpoch("3.0");
    // `index-of` is still available in clarity 2
    let resOk1 = simnet.runSnippet('(index-of "stacks" "s")');
    expect(resOk1).toStrictEqual(Cl.some(Cl.uint(0)));

    // `index-of?` is available in clarity 2
    let resOk2 = simnet.runSnippet('(index-of? "stacks" "s")');
    expect(resOk2).toStrictEqual(Cl.some(Cl.uint(0)));

    // `tenure-height` was introduced in clarity 3
    let resOk3 = simnet.runSnippet("(print tenure-height)");
    expect(resOk3).toStrictEqual(Cl.uint(1));

    // `block-height` was removed in clarity 3
    let resFail = simnet.runSnippet("(print block-height)");
    expect(resFail).toBe("error:\nuse of unresolved variable 'block-height'");
  });
});

import { describe, expect, it } from "vitest";

/*
  The test below is an example. Learn more in the clarinet-sdk readme:
  https://github.com/hirosystems/clarinet/blob/develop/components/clarinet-sdk/README.md
*/

describe("example tests", () => {
  it("ensures simnet is well initalise", () => {
    // swappool and it's dependencies makes for 7 contracts
    expect(simnet.getContractsInterfaces()).toHaveLength(7);
  });
});

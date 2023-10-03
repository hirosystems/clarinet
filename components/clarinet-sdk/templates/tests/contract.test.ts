import { beforeAll, describe, expect, it } from "vitest";

const accounts = vm.getAccounts();
const address1 = accounts.get("wallet_1")!;

/*
  The test below is an example. Learn more in the clarinet-sdk readme:
  https://github.com/hirosystems/clarinet/blob/develop/components/clarinet-sdk/README.md
*/

describe("example tests", () => {
  it("ensures vm is well initalise", () => {
    expect(vm.blockHeight).toBe(1);
  });

  // it("shows an example", () => {
  //   const { result } = vm.callReadOnlyFn("counter", "get-counter", [], w1);
  //   expect(result).toBeUint(0);
  // });
});

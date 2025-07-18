import { tx } from "@hirosystems/clarinet-sdk";
import { Cl } from "@stacks/transactions";
import { describe, expect, it } from "vitest";

const accounts = simnet.getAccounts();
const authority = accounts.get("authority")!;
const address2 = accounts.get("wallet_2")!;

describe("Ensure that tokens can be minted and burnt", () => {
  it("Ensure that tokens can be minted by authority", async () => {
    const { result } = simnet.callPublicFn(
      "cbtc-token",
      "mint",
      [Cl.uint(1000), Cl.standardPrincipal(authority)],
      authority,
    );

    expect(result).toBeOk(Cl.bool(true));
  });

  it("Ensure that tokens can't be minted by someone else", async () => {
    const { result } = simnet.callPublicFn(
      "cbtc-token",
      "mint",
      [Cl.uint(1000), Cl.standardPrincipal(authority)],
      address2,
    );

    expect(result).toBeErr(Cl.uint(0));
  });

  it("Ensure that tokens can be burnt by owner", async () => {
    simnet.callPublicFn(
      "cbtc-token",
      "mint",
      [Cl.uint(1000), Cl.standardPrincipal(authority)],
      authority,
    );
    simnet.callPublicFn(
      "cbtc-token",
      "mint",
      [Cl.uint(1000), Cl.standardPrincipal(address2)],
      authority,
    );
    const block = simnet.mineBlock([
      tx.callPublicFn("cbtc-token", "burn", [Cl.uint(1000)], authority),
      tx.callPublicFn("cbtc-token", "burn", [Cl.uint(1000)], address2),
    ]);

    expect(block[0].result).toBeOk(Cl.bool(true));
    expect(block[1].result).toBeOk(Cl.bool(true));
  });
});

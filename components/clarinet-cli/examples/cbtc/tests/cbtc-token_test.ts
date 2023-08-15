import { Clarinet, Tx, Chain, types } from "https://deno.land/x/clarinet@v1.7.1/index.ts";
import { assertEquals } from "https://deno.land/std@0.191.0/testing/asserts.ts";

Clarinet.test({
  name: "Ensure that tokens can be minted and burnt",
  fn(chain: Chain) {
    const block = chain.mineBlock([
      Tx.contractCall(
        "cbtc-token",
        "mint",
        [types.uint(1000), types.principal("ST2CY5V39NHDPWSXMW9QDT3HC3GD6Q6XX4CFRK9AG")],
        "ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5"
      ),
      Tx.contractCall(
        "cbtc-token",
        "burn",
        [types.uint(500)],
        "ST2CY5V39NHDPWSXMW9QDT3HC3GD6Q6XX4CFRK9AG"
      ),
    ]);
    assertEquals(block.receipts.length, 2);
    assertEquals(block.height, 3);
  },
});

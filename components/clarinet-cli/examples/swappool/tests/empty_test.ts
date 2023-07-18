import {
  Clarinet,
  Chain,
  Account,
} from "https://deno.land/x/clarinet@v1.7.0/index.ts";
import { assertEquals } from "https://deno.land/std@0.170.0/testing/asserts.ts";

Clarinet.test({
  name: "Ensure that <...>",
  fn(chain: Chain, accounts: Map<string, Account>) {
    const block = chain.mineBlock([]);

    assertEquals(block.receipts.length, 0);
  },
});

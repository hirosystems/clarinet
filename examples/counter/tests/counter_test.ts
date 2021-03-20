
import { Clarinet, Tx, Chain, Account, types } from 'https://deno.land/x/clarinet@v0.3.0/index.ts';
import { assertEquals } from "https://deno.land/std@0.90.0/testing/asserts.ts";

Clarinet.test({
    name: "Ensure that counter can be incremented multiples per block, accross multiple blocks",
    async fn(chain: Chain, accounts: Array<Account>) {
        let block = chain.mineBlock([
            Tx.contractCall("counter", "increment", [types.uint(1)], accounts[0].address),
            Tx.contractCall("counter", "increment", [types.uint(4)], accounts[1].address),
            Tx.contractCall("counter", "increment", [types.uint(10)], accounts[2].address)
        ]);
        assertEquals(block.height, 2);
        assertEquals(block.receipts[0].result, "(ok u2)");
        assertEquals(block.receipts[1].result, "(ok u6)");
        assertEquals(block.receipts[2].result, "(ok u16)");
        console.log(block);
        
        block = chain.mineBlock([
            Tx.contractCall("counter", "increment", [types.uint(1)], accounts[0].address),
            Tx.contractCall("counter", "increment", [types.uint(4)], accounts[0].address),
            Tx.contractCall("counter", "increment", [types.uint(10)], accounts[0].address)
        ]);
        assertEquals(block.height, 3);
        assertEquals(block.receipts[0].result, "(ok u17)");
        assertEquals(block.receipts[1].result, "(ok u21)");
        assertEquals(block.receipts[2].result, "(ok u31)");
        console.log(block);
    },
});

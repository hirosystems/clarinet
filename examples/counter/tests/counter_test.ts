import { Clarinet, Tx, Chain, Account, types } from 'https://deno.land/x/clarinet@v0.4.0/index.ts';
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
        block.receipts[0].result
            .expectOk()
            .expectUint(2);
        block.receipts[1].result
            .expectOk()
            .expectUint(6);
        block.receipts[2].result
            .expectOk()
            .expectUint(16);
        console.log(block);
        
        block = chain.mineBlock([
            Tx.contractCall("counter", "increment", [types.uint(1)], accounts[0].address),
            Tx.contractCall("counter", "increment", [types.uint(4)], accounts[0].address),
            Tx.contractCall("counter", "increment", [types.uint(10)], accounts[0].address)
        ]);
        assertEquals(block.height, 3);
        block.receipts[0].result
            .expectOk()
            .expectUint(17);
        block.receipts[1].result
            .expectOk()
            .expectUint(21);
        block.receipts[2].result
            .expectOk()
            .expectUint(31);
        console.log(block);

        console.log(chain.callReadOnlyFn("counter", "read-counter", [], accounts[0].address));
        // console.log(chain.callReadOnlyFn("counter", "read-counter", accounts[0].address));
    },
});

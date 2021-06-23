import { Clarinet, Tx, Chain, Account, types } from 'https://deno.land/x/clarinet@v0.12.0/index.ts';
import { assertEquals } from 'https://deno.land/std@0.90.0/testing/asserts.ts';

Clarinet.test({
    name: "A quick demo on how to assert expectations",
    async fn(chain: Chain, accounts: Map<string, Account>) {
        let wallet_1 = accounts.get('wallet_1')!;

        let assetMaps = chain.getAssetsMaps();
        const balance = assetMaps.assets['STX'][wallet_1.address];

        let block = chain.mineBlock([
           Tx.contractCall('billboard', 'set-message', [types.utf8("testing")], wallet_1.address),
           Tx.contractCall('billboard', 'get-message', [], wallet_1.address),
           Tx.contractCall('billboard', 'set-message', [types.utf8("testing...")], wallet_1.address),
           Tx.contractCall('billboard', 'get-message', [], wallet_1.address),
        ]);

        assertEquals(block.receipts.length, 4);
        assertEquals(block.height, 2);

        block.receipts[1].result
            .expectUtf8('testing');

        block.receipts[3].result
            .expectUtf8('testing...');
        
        let [event] = block.receipts[0].events;
        let {sender, recipient, amount} = event.stx_transfer_event;
        sender.expectPrincipal("ST1J4G6RR643BCG8G8SR6M2D9Z9KXT2NJDRK3FBTK");
        recipient.expectPrincipal("ST1HTBVD3JG9C05J7HBJTHGR0GGW7KXW28M5JS8QE.billboard");
        amount.expectInt(100);

        assetMaps = chain.getAssetsMaps();
        assertEquals(assetMaps.assets['STX'][wallet_1.address], balance - 210);
    },
});

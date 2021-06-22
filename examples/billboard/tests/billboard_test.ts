import { Clarinet, Tx, Chain, Account, types } from 'https://deno.land/x/clarinet@v0.6.0/index.ts';
import { assertEquals } from 'https://deno.land/std@0.90.0/testing/asserts.ts';

Clarinet.test({
    name: "Ensure that <...>",
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
        
        assetMaps = chain.getAssetsMaps();
        assertEquals(assetMaps.assets['STX'][wallet_1.address], balance - 210);
    },
});

import { Clarinet, Tx, Chain, Account, types } from 'https://deno.land/x/clarinet@v1.0.0-beta1/index.ts';
import { assertEquals } from 'https://deno.land/std@0.90.0/testing/asserts.ts';

Clarinet.test({
    name: "A quick demo on how to assert expectations",
    async fn(chain: Chain, accounts: Map<string, Account>) {
        let deployer = accounts.get('deployer')!;
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
        sender.expectPrincipal(wallet_1.address);
        recipient.expectPrincipal(`${deployer.address}.billboard`);
        amount.expectInt(100);

        assetMaps = chain.getAssetsMaps();
        assertEquals(assetMaps.assets['STX'][wallet_1.address], balance - 210);
    },
});

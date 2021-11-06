// import { Clarinet, Tx, Chain, Account, Contract, types } from 'https://deno.land/x/clarinet@v0.13.0/index.ts';
import { Clarinet, Tx, Chain, Account, Contract, types } from '../../../deno/index.ts';
import { assertEquals } from "https://deno.land/std@0.90.0/testing/asserts.ts";

Clarinet.test({
    name: "Ensure that nft can be transferred form one account to another",
    async fn(chain: Chain, accounts: Map<string, Account>, contracts: Map<string, Contract>) {
        let deployer = accounts.get("deployer")!;
        let wallet_1 = accounts.get("wallet_1")!;
        let wallet_2 = accounts.get("wallet_2")!;

        
        let block = chain.mineBlock([
            Tx.contractCall("simple-nft", "test-mint", [types.principal(wallet_1.address)], wallet_1.address),
        ]);

        block.receipts[0].result.expectOk().expectBool(true);

        block = chain.mineBlock([
            Tx.contractCall("simple-nft", "transfer", [
                types.uint(1),
                types.principal(wallet_1.address),
                types.principal(wallet_2.address)
            ], wallet_1.address)
        ]);

        block.receipts[0].result.expectOk().expectBool(true);

        block.receipts[0].events.expectNonFungibleTokenTransferEvent(
            types.uint(1),
            wallet_1.address, 
            wallet_2.address, 
            `${deployer.address}.simple-nft`,
            "nft"
        )
    },
});

import { Clarinet, Tx, Chain, Account, types } from 'https://deno.land/x/clarinet@v0.1.2/index.ts';

Clarinet.test({
    name: "Ensure that test 1 are being executed",
    async fn(chain: Chain, accounts: Array<Account>) {
        let [alice, bob, charlie] = accounts;
        let block = chain.mineBlock([
            new Tx("bbtc", "create-box", [types.uint(12), types.uint(12)], alice.address)
        ]);
        console.log(block);
    },
});

Clarinet.test({
    name: "Ensure that test 2 are being executed",
    async fn(chain: Chain, accounts: Array<Account>) {
        console.log(`Test initialized with Chain::${chain.sessionId} and accounts ${JSON.stringify(accounts)}`);
    },
});

Clarinet.test({
    name: "Ensure that tests 3 are being executed",
    async fn(chain: Chain, accounts: Array<Account>) {
        console.log(`Test initialized with Chain::${chain.sessionId} and accounts ${JSON.stringify(accounts)}`);
    },
});

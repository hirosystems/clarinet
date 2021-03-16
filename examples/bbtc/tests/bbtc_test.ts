import { Clarinet, Block, Chain, Account } from 'https://deno.land/x/clarinet@v0.1.1/index.ts';

Clarinet.test({
    name: "Ensure that test 1 are being executed",
    async fn(chain: Chain, accounts: Array<Account>) {
        console.log(`Test initialized with Chain::${chain.sessionId} and accounts ${JSON.stringify(accounts)}`);
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

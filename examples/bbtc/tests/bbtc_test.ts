import { Clarinet, Block, Chain, Account } from './index.ts';

Clarinet.test({
    name: "Ensure that test 1 are being executed",
    async fn(chain: Chain, accounts: Array<Account>) {
        console.log(`Hello ${JSON.stringify(chain)} and accounts ${JSON.stringify(accounts)}`);
    },
});

Clarinet.test({
    name: "Ensure that test 2 are being executed",
    async fn(chain: any, accounts: any) {
        // console.log(`Hello ${chain} and accounts ${accounts}`);
    },
});

Clarinet.test({
    name: "Ensure that tests 3 are being executed",
    async fn(chain: any, accounts: any) {
        // console.log(`Hello ${chain} and accounts ${accounts}`);
    },
});

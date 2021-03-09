import { Clarinet, Block, Chain, Account } from './index.ts';

// Clarinet.test({
//     name: "Ensure that tests are being executed",
//     async fn(chain: Chain, accounts: Array<Account>) {
//         console.log("Hello world :)");
//     },
// });

Clarinet.test({
    name: "Ensure that test 1 are being executed",
    async fn() {
        // console.log(`Hello ${chain} and accounts ${accounts}`);
    },
});

Clarinet.test({
    name: "Ensure that test 2 are being executed",
    async fn() {
        // console.log(`Hello ${chain} and accounts ${accounts}`);
    },
});

Clarinet.test({
    name: "Ensure that tests 3 are being executed",
    async fn() {
        // console.log(`Hello ${chain} and accounts ${accounts}`);
    },
});

// import { assertEquals } from 'https://deno.land/std/testing/asserts.ts';

// Deno.test('maps to a smaller story with formatted date', (chain: Chain, accounts: Array<Account>) => {
//     let block = chain.mineBlock([
//         `(contract-call? 'ST000000000000000000002AMW42H.bbtc create-box ${123} ${123})`,
//     ]);
//     assertEquals(block.transactions.len(), 1);

//     let res = chain.read(`(contract-call? 'ST000000000000000000002AMW42H.bbtc create-box size fee)`);
//     assertEquals(res, 1);

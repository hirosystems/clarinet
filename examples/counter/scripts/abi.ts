import { Clarinet, Contract, Account, StacksNode } from 'https://deno.land/x/clarinet@v0.13.0/index.ts';

Clarinet.run({
    async fn(accounts: Map<string, Account>, contracts: Map<string, Contract>, node: StacksNode) {
        console.log("Contracts");
        for (let contract of contracts) {
            console.log(contract);
        }
        console.log("Accounts");
        for (let account of accounts) {
            console.log(account);
        }

    }
});
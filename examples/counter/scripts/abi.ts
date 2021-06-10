import { Clarinet, Contract, Account, StacksNode, log, log_green, log_red } from './script.ts';

Clarinet.run({
    name: "Generating ABI",
    async fn(accounts: Map<string, Account>, contracts: Map<string, Contract>, node: StacksNode) {
        log_green(`Accounts ${accounts}`);
    }
});
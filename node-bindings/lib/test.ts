import { StacksDevnetOrchestrator } from "./index";

// const devnet = new StacksDevnet({
//     config: {
//         manifestPath: "/Users/ludovic/Coding/clarinet/clarinet-cli/examples/counter/Clarinet.toml",
//     },
//     logger: (msg) => console.log(msg)
// });

const devnet = new StacksDevnetOrchestrator({
    path: "/Users/ludovic/Coding/clarinet/clarinet-cli/examples/counter/Clarinet.toml",
    logs: true,
    accounts: [
        {
            id: "wallet_9",
            mnemonic: "sell invite acquire kitten bamboo drastic jelly vivid peace spawn twice guilt pave pen trash pretty park cube fragile unaware remain midnight betray rebuild",
            balance: 100_000_000,
        }
    ],
    devnet: {
        orchestrator_port: 8000,
        pox_stacking_orders: [
            {
                start_at_cycle: 3,
                duration: 12,
                wallet: "wallet_1",
                slots: 2,
                btc_address: "mr1iPkD9N3RJZZxXRk7xF9d36gffa6exNC"
            }
        ]    
    }
});

console.log(devnet.getStacksNodeUrl())

devnet.start();

let block = devnet.waitForStacksBlock();

console.log(`Hello from JS ${JSON.stringify(block)}`);

devnet.stop();
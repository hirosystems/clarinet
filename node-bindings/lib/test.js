const StacksDevnet = require("./index");

const devnet = new StacksDevnet({
    config: {
        manifestPath: "/Users/ludovic/Coding/clarinet/clarinet-cli/examples/counter/Clarinet.toml",
    },
    logger: (msg) => console.log(msg)
});

devnet.start();

let block = devnet.waitForStacksBlock();

console.log(`Hello from JS ${JSON.stringify(block)}`);

devnet.stop();
const StacksDevnet = require("./index");

const devnet = new StacksDevnet({
    config: {
        manifestPath: "/Users/ludovic/Coding/clarinet/clarinet-cli/examples/counter/Clarinet.toml",
    },
    logger: (msg) => console.log(msg)
});

devnet.start();

var i = 0;
let block = devnet.waitForStacksBlock((block) => {
    i += 1;
    console.log(`${i} Hello from JS ${block}`);
    return i > 5;
});

console.log(block);

devnet.stop();
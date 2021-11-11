"use strict";

const { stacksDevnetNew, stacksDevnetStart, stacksDevnetStop, stacksDevnetWaitForStacksBlock, stacksDevnetWaitForBitcoinBlock } = require('../native/index.node');

class StacksDevnet {
    
    constructor(setup) {
        let manifestPath = setup.manifestPath;
        var logs = setup.logs;
        logs ||= false;
        var accounts = setup.settings.accounts;
        accounts ||= [];
        var devnet = setup.settings.devnet;
        devnet ||= {};
        console.log(manifestPath);
        this.handle = stacksDevnetNew(manifestPath, logs, accounts, devnet);
    }

    start() {
        return stacksDevnetStart.call(this.handle);
    }

    waitForStacksTransaction(txid, num_block_timeout = 5, callback) {
    }

    waitForStacksBlock() {
        return stacksDevnetWaitForStacksBlock.call(this.handle);
    }

    waitForBitcoinBlock(callback) {
        return stacksDevnetWaitForBitcoinBlock.call(this.handle);
    }

    stop() {
        stacksDevnetStop.call(this.handle);
    }
}

module.exports = StacksDevnet;

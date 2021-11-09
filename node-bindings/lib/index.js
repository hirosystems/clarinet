"use strict";

const { stacksDevnetNew, stacksDevnetStart, stacksDevnetStop, stacksDevnetWaitForStacksBlock, stacksDevnetWaitForBitcoinBlock } = require('../native/index.node');

class StacksDevnet {
    
    constructor(setup) {
        this.handle = stacksDevnetNew(setup.config.manifestPath, setup.logger);
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

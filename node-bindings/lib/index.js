"use strict";

const { promisify } = require("util");
const { stacksDevnetNew, stacksDevnetStart, stacksDevnetStop, stacksDevnetWaitForStacksBlock, stacksDevnetWaitForBitcoinBlock, stacksDevnetOnLog } = require('../native/index.node');

class StacksDevnet {
    
    constructor(setup) {
        this.handle = stacksDevnetNew(setup.config.manifestPath, setup.logger);
    }

    start() {
        return stacksDevnetStart.call(this.handle);
    }

    waitForBootCompletion(callback) {
    }

    waitForStacksTransaction(txid, num_block_timeout = 5, callback) {
    }

    waitForStacksBlock(callback) {
        return stacksDevnetWaitForStacksBlock.call(this.handle, callback);
    }

    waitForBitcoinBlock(callback) {
        return stacksDevnetWaitForBitcoinBlock.call(this.handle, callback);
    }

    stop() {
        stacksDevnetStop.call(this.handle);
    }
}

module.exports = StacksDevnet;

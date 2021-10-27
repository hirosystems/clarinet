"use strict";

const { promisify } = require("util");
const { stackDevnetNew, stackDevnetStart, stackDevnetOnStacksBlock, stackDevnetOnBitcoinBlock, stackDevnetTerminate } = require('../native/index.node');

class StacksDevnet {
    
    constructor() {
        this.handle = stackDevnetNew();
    }

    start() {
        return stackDevnetStart.call(this.handle);
    }

    waitForBootCompletion(callback) {
    }

    waitForStacksTransaction(txid, num_block_timeout = 5, callback) {
    }

    waitForStacksBlock(callback) {
        return stackDevnetOnStacksBlock.call(this.handle, callback);
    }

    waitForBitcoinBlock(callback) {
        return stackDevnetOnBitcoinBlock.call(this.handle, callback);
    }

    waitForAttachment(callback) {
    }

    stop() {
        stackDevnetTerminate.call(this.handle);
    }
}

module.exports = StacksDevnet;

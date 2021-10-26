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

    waitStacksBlock(predicate) {
        return stackDevnetOnStacksBlock.call(this.handle, predicate);
    }

    waitBitcoinBlock(predicate) {
        return stackDevnetOnBitcoinBlock.call(this.handle, predicate);
    }

    stop() {
        stackDevnetTerminate.call(this.handle);
    }
}

module.exports = StacksDevnet;

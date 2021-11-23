"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.StacksDevnetOrchestrator = void 0;
var _a = require("../native/index.node"), stacksDevnetNew = _a.stacksDevnetNew, stacksDevnetStart = _a.stacksDevnetStart, stacksDevnetStop = _a.stacksDevnetStop, stacksDevnetWaitForStacksBlock = _a.stacksDevnetWaitForStacksBlock, stacksDevnetWaitForBitcoinBlock = _a.stacksDevnetWaitForBitcoinBlock, stacksDevnetGetStacksNodeUrl = _a.stacksDevnetGetStacksNodeUrl;
var StacksDevnetOrchestrator = /** @class */ (function () {
    /**
     * @summary Construct a new StacksDevnetOrchestrator
     * @param {ClarinetManifest} manifest
     * @memberof StacksDevnetOrchestrator
     */
    function StacksDevnetOrchestrator(manifest) {
        var manifestPath = manifest.path;
        var logs = manifest.logs;
        logs || (logs = false);
        var accounts = manifest.accounts;
        accounts || (accounts = []);
        var devnet = manifest.devnet;
        devnet || (devnet = {});
        this.handle = stacksDevnetNew(manifestPath, logs, accounts, devnet);
    }
    /**
     * @summary Start orchestrating containers
     * @memberof StacksDevnetOrchestrator
     */
    StacksDevnetOrchestrator.prototype.start = function () {
        return stacksDevnetStart.call(this.handle);
    };
    /**
     * @summary Returns the URL of the stacks-node container
     * @memberof StacksDevnetOrchestrator
     */
    StacksDevnetOrchestrator.prototype.getStacksNodeUrl = function () {
        return stacksDevnetGetStacksNodeUrl.call(this.handle);
    };
    /**
     * @summary Wait for the next Stacks block
     * @memberof StacksDevnetOrchestrator
     */
    StacksDevnetOrchestrator.prototype.waitForStacksBlock = function () {
        return stacksDevnetWaitForStacksBlock.call(this.handle);
    };
    /**
     * @summary Wait for the next Bitcoin block
     * @memberof StacksDevnetOrchestrator
     */
    StacksDevnetOrchestrator.prototype.waitForBitcoinBlock = function () {
        return stacksDevnetWaitForBitcoinBlock.call(this.handle);
    };
    /**
     * @summary Terminates the containers
     * @memberof StacksDevnetOrchestrator
     */
    StacksDevnetOrchestrator.prototype.stop = function () {
        stacksDevnetStop.call(this.handle);
    };
    return StacksDevnetOrchestrator;
}());
exports.StacksDevnetOrchestrator = StacksDevnetOrchestrator;
//# sourceMappingURL=index.js.map
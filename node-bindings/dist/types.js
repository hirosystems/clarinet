"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.CoinAction = exports.StacksTransactionEventType = exports.StacksTransactionKind = exports.Direction = void 0;
/**
 * Used by RelatedTransaction to indicate the direction of the relation (i.e. cross-shard/cross-network sends may reference `backward` to an earlier transaction and async execution may reference `forward`). Can be used to indicate if a transaction relation is from child to parent or the reverse.
 * @export
 * @enum {string}
 */
var Direction;
(function (Direction) {
    Direction["forward"] = "forward";
    Direction["backward"] = "backward";
})(Direction = exports.Direction || (exports.Direction = {}));
var StacksTransactionKind;
(function (StacksTransactionKind) {
    StacksTransactionKind["ContractCall"] = "ContractCall";
    StacksTransactionKind["ContractDeployment"] = "ContractDeployment";
    StacksTransactionKind["NativeTokenTransfer"] = "NativeTokenTransfer";
    StacksTransactionKind["Coinbase"] = "Coinbase";
    StacksTransactionKind["Other"] = "Other";
})(StacksTransactionKind = exports.StacksTransactionKind || (exports.StacksTransactionKind = {}));
var StacksTransactionEventType;
(function (StacksTransactionEventType) {
    StacksTransactionEventType["StacksSTXTransferEvent"] = "StacksSTXTransferEvent";
    StacksTransactionEventType["StacksSTXMintEvent"] = "StacksSTXMintEvent";
    StacksTransactionEventType["StacksSTXLockEvent"] = "StacksSTXLockEvent";
    StacksTransactionEventType["StacksSTXBurnEvent"] = "StacksSTXBurnEvent";
    StacksTransactionEventType["StacksNFTTransferEvent"] = "StacksNFTTransferEvent";
    StacksTransactionEventType["StacksNFTMintEvent"] = "StacksNFTMintEvent";
    StacksTransactionEventType["StacksNFTBurnEvent"] = "StacksNFTBurnEvent";
    StacksTransactionEventType["StacksFTTransferEvent"] = "StacksFTTransferEvent";
    StacksTransactionEventType["StacksFTMintEvent"] = "StacksFTMintEvent";
    StacksTransactionEventType["StacksFTBurnEvent"] = "StacksFTBurnEvent";
    StacksTransactionEventType["StacksDataVarSetEvent"] = "StacksDataVarSetEvent";
    StacksTransactionEventType["StacksDataMapInsertEvent"] = "StacksDataMapInsertEvent";
    StacksTransactionEventType["StacksDataMapUpdateEvent"] = "StacksDataMapUpdateEvent";
    StacksTransactionEventType["StacksDataMapDeleteEvent"] = "StacksDataMapDeleteEvent";
    StacksTransactionEventType["StacksSmartContractEvent"] = "StacksSmartContractEvent";
})(StacksTransactionEventType = exports.StacksTransactionEventType || (exports.StacksTransactionEventType = {}));
/**
 * CoinActions are different state changes that a Coin can undergo. When a Coin is created, it is coin_created. When a Coin is spent, it is coin_spent. It is assumed that a single Coin cannot be created or spent more than once.
 * @export
 * @enum {string}
 */
var CoinAction;
(function (CoinAction) {
    CoinAction["created"] = "coin_created";
    CoinAction["spent"] = "coin_spent";
})(CoinAction = exports.CoinAction || (exports.CoinAction = {}));
//# sourceMappingURL=types.js.map
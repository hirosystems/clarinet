# Clarinet SDK

The Clarinet SDK can be used to interact with the simnet from Node.js.

Here is a non-exhaustive list of some of the use-cass:
- Call public and read-only functions from smart contracts
- Get clarity maps or data-var values
- Get contract ABI
- Write unit tests for Clarity smart contracts

## Getting started with the SDK

> The SDK requires Node.js >= 18.0 and NPM to be installed. [Volta](https://volta.sh/) is a great tool to install and manage JS tooling.

The SDK can be installed with NPM. It works in pair with Stacks.js so let's install it as well.

```sh
npm install @hirosystems/clarinet-sdk @stacks/transactions
```

### Usage

Here is a very basic code snippet showing how to use the SDK:

```ts
import { initVM } from "@hirosystems/clarinet-sdk";
import { Cl } from "@stacks/transactions";

async function main() {
  const vm = await initVM();

  const accounts = vm.getAccounts();
  const address1 = accounts.get("wallet_1")!;

  const call = vm.callPublicFn("counter", "add", [Cl.uint(1)], address1);
  console.log(call.result); // Cl.int(Cl.ok(true))

  const counter = vm.getDataVar("counter", "count");
  console.log(counter); // Cl.uint(1)
}

main();
```

By default, the SDK will look for a Clarinet.toml file in the current working directory.  
It's also possible to provide the path to the manifest like so:

```ts
 const vm = await initVM("./path/to/Clarinet.toml");
```


## API references

### `initVM`

```ts
initVM(manifestPath?: string): Promise<ClarityVM>
```

The `initVM` function takes the manifest path (`Clarinet.toml`) as an optional argument. By default, it'll look for a manifest in the current working directory.  
It will often be the first function to call when using the SDK.

```ts
import { initVM } from "@hirosystems/clarinet-sdk";

const vm = await initVM();
// or
const vm = await initVM("./clarity/Clarinet.toml");
```

### ClarityVM properties


#### `ClarityVM.blockHeight`

Returns the current block height of the simnet.

```ts
import { initVM } from "@hirosystems/clarinet-sdk";
const vm = await initVM();

console.log(vm.blockHeight); // 0
```


#### `ClarityVM.deployer`

Returns the default deployer address as defined in the project file `./setting/Devnet.toml`.

```ts
import { initVM } from "@hirosystems/clarinet-sdk";
const vm = await initVM();

console.log(vm.deployer);  // ST1P...GZGM
```

### ClarityVM methods


#### `ClarityVM.getAccounts()`

```ts
getAccounts(): Map<string, string>
```

Get the Stacks addresses defined in the project file `./setting/Devnet.toml`.


```ts
import { initVM } from "@hirosystems/clarinet-sdk";
const vm = await initVM();

const accounts = vm.getAccounts();
const address1 = accounts.get("wallet_1")!;
console.log(address1); // ST1S...YPD5
```


#### `ClarityVM.getAssetsMap()`

```ts
getAssetsMap(): Map<string, Map<string, bigint>>
```

Get a list of assets balances by Stacks addresses. It returns STX balances as well as FTs and NFTs.

```ts
import { initVM } from "@hirosystems/clarinet-sdk";
const vm = await initVM();

const assets = vm.getAssetsMap();
const stxBalances = assets.get("STX")!;

console.log(stxBalances);
// Map(10) {
//   'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM' => 100000000000000n,
//   'ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5' => 100000000000000n,
//   // ...
// }
```


#### `ClarityVM.getDataVar()`

```ts
getDataVar(contract: string, dataVar: string): ClarityValue
```

Get the value of a data-var defined in a contract.

Given a contract with the following definition:

```clar
(define-data-var count uint u0)
```

It can be accessed with:

```ts
import { initVM } from "@hirosystems/clarinet-sdk";
const vm = await initVM();

const counter = vm.getDataVar("counter", "count");
// counter is Cl.uint(0)
```


#### `ClarityVM.getMapEntry()`

```ts
getMapEntry(contract: string, mapName: string, mapKey: ClarityValue): ClarityValue
```

Get the value of a map entry by its key.  
Note that it will always return an optional value (`(some <value>)` or `none`). Just like Clarity `map-get?`.

Given a contract with the following definition:

```clar
(define-map participants principal bool)
```

It can be accessed with:

```ts
import { initVM } from "@hirosystems/clarinet-sdk";
const vm = await initVM();

const accounts = vm.getAccounts();
const address1 = accounts.get("wallet_1")!;

const participated = vm.getMapEntry("counter", "participants", Cl.standardPrincipal(address1));;
// counter is Cl.some(Cl.bool(true|false)) or Cl.none()
```


#### `ClarityVM.callReadOnlyFn()`

```ts
callReadOnlyFn(
  contract: string,     // stacks address of the contract
  method: string,       // read-only function to call
  args: ClarityValue[], // array of Clarity Values
  sender: string        // stacks address of the sender
): ParsedTransactionRes
```

Call read-only functions exposed by a contract. Returns an object with the result of the function call as a Clarity Value.  
It takes function arguments in the form in Clarity Values, available in the package `@stacks/transactions`.

```ts
import { initVM } from "@hirosystems/clarinet-sdk";
import { Cl } from "@stacks/transactions";
const vm = await initVM();

const accounts = vm.getAccounts();
const address1 = accounts.get("wallet_1")!;

const getCounter = vm.callReadOnlyFn("counter", "get-counter", [], address1);
console.log(getCounter.result); // Cl.uint(1)

// With arguments:
const callPOX = vm.callReadOnlyFn("pox-3", "is-pox-active", [Cl.uint(100)], address1);
```

As in many methods of the SDK, the contract address can be just the contract name if deployed by the default deployer.

```ts
vm.callReadOnlyFn("counter", "get-counter", [], address1);
// equivalent
vm.callReadOnlyFn(`${vm.deployer}.counter`, "get-counter", [], address1);
```


#### `ClarityVM.callPublicFn()`

```ts
callPublicFn(
  contract: string,     // stacks address of the contract
  method: string,       // public function to call
  args: ClarityValue[], // array of Clarity Values
  sender: string        // stacks address of the sender
): ParsedTransactionRes
```

Call read-only functions exposed by a contract.
Returns an object with the result of the function call as a Clarity Value and the events fired during the function execution.  
It takes function arguments in the form in Clarity Values, available in the package `@stacks/transactions`.  
It will simulate a block being mined and increase the block height by one.


```ts
import { initVM } from "@hirosystems/clarinet-sdk";
import { Cl } from "@stacks/transactions";
const vm = await initVM();

const accounts = vm.getAccounts();
const address1 = accounts.get("wallet_1")!;

const callAdd = vm.callPublicFn("counter", "add", [Cl.uint(3)], address1);
console.log(callAdd.result); // a Clarity Value such as Cl.bool(true)
console.log(callAdd.events); // and array of events (such as print event, stx stransfer event, etc)
```


#### `ClaritVM.transferSTX()`

```ts
transferSTX(amount: number | bigint, recipient: string, sender: string): ParsedTransactionRes
```

Transfer STX from an address to an other. The amount is in uSTX.  
It will simulate a block being mined and increase the block height by one.

```ts
import { initVM } from "@hirosystems/clarinet-sdk";
const vm = await initVM();

const accounts = vm.getAccounts();
const address1 = accounts.get("wallet_1")!;
const address2 = accounts.get("wallet_2")!;

const transfer = vm.transferSTX(100, address1, address2);
console.log(transfer);
// {
//   result: Cl.ok(Cl.bool(true)),
//   events: [
//     {
//       event: 'stx_transfer_event',
//       data: {
//         amount: '100',
//         memo: '',
//         recipient: 'ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5',
//         sender: 'ST2CY5V39NHDPWSXMW9QDT3HC3GD6Q6XX4CFRK9AG'
//       }
//     }
//   ]
// }
```


#### `ClarityVM.deployContract()`

```ts
deployContract(
  // name of the contract to be deployed
  name: string,
  // content of the contract
  content: string,
  // an object to specify options such as the ClarityVersion
  options: DeployContractOptions | null,
  // sender stacks address
  sender: string
): ParsedTransactionRes
```

Deploy a contract to the VM.  
It will simulate a block being mined and increase the block height by one.


```ts
import { initVM } from "@hirosystems/clarinet-sdk";
import { Cl } from "@stacks/transactions";
const vm = await initVM();

const accounts = vm.getAccounts();
const address1 = accounts.get("wallet_1")!;

const source = "(define-public (add (a uint) (b uint)) (ok (+ a b)))";
const deployRes = vm.deployContract("op", source, vm.deployer);

const addRes = vm.callPublicFn("op", "add", [Cl.uint(1), Cl.uint(1)], address1);
console.log(addRes.result) // Cl.ok(Cl.uint(2))

// specify a clarityVersion
vm.deployContract("contract2", source, { clarityVersion : 2 }, deployerAddr);
```


#### `ClarityVM.mineBlock()`

```ts
mineBlock(txs: Tx[]): ParsedTransactionRes[]
```

The `.callPublicFn()`, `.transferSTX()`, and `.deployContract()` methods all mine one block with only one transaction. It can also be useful to mine a block with multiple transactions. This is what `.mineBlock()` is for.  

It take an array of transaction objects.  
The transactions can be built with the `tx` helper exported by the SDK.
It has three methods `.callPublicFn()`, `.transferSTX()`, and `.deployContract()`, which have the same interface as the `ClarityVM` methods but instead of performing a transaction, it will build a transaction object than can be passed to the `mineBlock()` function.


```ts
// import `tx` as well
import { initVM, tx } from "@hirosystems/clarinet-sdk";
import { Cl } from "@stacks/transactions";
const vm = await initVM();

const accounts = vm.getAccounts();
const address1 = accounts.get("wallet_1")!;
const address2 = accounts.get("wallet_2")!;

const block = vm.mineBlock([
  tx.callPublicFn("counter", "increment", [], address1),
  tx.callPublicFn("counter", "add", [Cl.uint(10)], address1),
  tx.transferSTX(100, address1, address2),
]);

console.log(block[0]); // `increment` response with { result, events}
console.log(block[1]); // `add` response with { result, events}
console.log(block[2]); // `transfer_stx` response with { result, events}
```


#### `ClarityVM.mineEmptyBlock()`

```ts
mineEmptyBlock(): number
```

Mine one empty block and increase the block height by one. Returns the new block height.

```ts
import { initVM } from "@hirosystems/clarinet-sdk";
const vm = await initVM();

console.log(vm.blockHeight); // 0
const newHeight = vm.mineEmptyBlock();
cosole.log(newHeight); // 1
console.log(vm.blockHeight); // 1
```


#### `ClarityVM.mineEmptyBlocks()`

```ts
mineEmptyBlocks(count?: number): number
```

Mine multiple empty blocks to reach a certain block height. Returns the new block height.

```ts
import { initVM } from "@hirosystems/clarinet-sdk";
const vm = await initVM();

console.log(vm.blockHeight); // 0
const newHeight = vm.mineEmptyBlocks(10);
console.log(newHeight); // 10
console.log(vm.blockHeight); // 10
```


#### `ClarityVM.getContractsInterfaces()`

```ts
getContractsInterfaces(): Map<string, ContractInterface>
```

Returns the interfaces of the project contracts.
It returns a Map of Contracts, the keys are the contract addresses.
The interfaces contains informations such as the ABI, the available functions, data-vars and maps, the NFT and FT defined in the contract  .
It can be used to get the list of the contracts and iterate of it.

```ts
import { initVM } from "@hirosystems/clarinet-sdk";
const vm = await initVM();

const contractInterfaces = vm.getContractsInterfaces();
let counterInterface = contractInterfaces.get(`${deployerAddr}.counter`);
console.log(counterInterface?.functions) // array of the functions
```


#### `ClarityVM.getContractSource()`

```ts
getContractSource(contract: string): string | undefined
```

Get the source code of a contract as a string.

```ts
import { initVM } from "@hirosystems/clarinet-sdk";
const vm = await initVM();

const source = "(define-public (add (a uint) (b uint)) (ok (+ a b)))";
vm.deployContract("contract", source, null, deployerAddr);

const contractSource = vm.getContractSource("contract");
console.log(contractSource);
// "(define-public (add (a uint) (b uint)) (ok (+ a b)))"
```


#### `ClarityVM.getContractAST()`

```ts
getContractAST(contractId: string): ContractAST
```

Get the full AST of a Clarity contract.

It throws an error if it fails to get the AST or to encode it JS (which should not happen).  
Note: The `ContractAST` TypeScript is still very simple but will be improved over time.

```ts
import { initVM } from "@hirosystems/clarinet-sdk";
const vm = await initVM();

const counterAst = vm.getContractAST("counter");
```

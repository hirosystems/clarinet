# Clarinet SDK

The Clarinet SDK is a JavaScript library that spawns and interacts with a simulated Clarinet environment, also known as "simnet."

A Simnet is a simulated network that mimics the Stacks blockchain and runs the Clarity VM, without the need for actual Stacks and Bitcoin nodes (unlike Devnet, Testnet and Mainnet).

Here is a non-exhaustive list of some of simnet's use-cases:

- Call public and read-only functions from smart contracts
- Get clarity maps or data-var values
- Get contract interfaces (available functions and data)
- [Write unit tests for Clarity smart contracts](../feature-guides/test-contract-with-clarinet-sdk.md)

## Getting Started With the SDK

> The SDK requires Node.js >= 18.0 and NPM to be installed. [Volta](https://volta.sh/) is a great tool to install and manage JS tooling.

The SDK can be installed with NPM. It works in pair with Stacks.js, so let's install it as well.

```sh
npm install @hirosystems/clarinet-sdk @stacks/transactions
```

### Usage

Here is a very basic code snippet showing how to use the SDK:

```ts
import { initSimnet } from "@hirosystems/clarinet-sdk";
import { Cl } from "@stacks/transactions";

async function main() {
  const simnet = await initSimnet();

  const accounts = simnet.getAccounts();
  const address1 = accounts.get("wallet_1");
  if (!address1) throw new Error("invalid wallet name.");

  const call = simnet.callPublicFn("counter", "add", [Cl.uint(1)], address1);
  console.log(call.result); // Cl.int(Cl.ok(true))
}

main();
```

By default, the SDK will look for a Clarinet.toml file in the current working directory.
It's also possible to provide the path to the manifest like so:

```ts
const simnet = await initSimnet("./path/to/Clarinet.toml");
```

## API References

### `initSimnet`

```ts
initSimnet(manifestPath?: string): Promise<Simnet>
```

The `initSimnet` function takes the manifest path (`Clarinet.toml`) as an optional argument. By default, it'll look for a manifest in the current working directory.
It will often be the first function to call when using the SDK.

```ts
import { initSimnet } from "@hirosystems/clarinet-sdk";

const simnet = await initSimnet();
// or
const simnet = await initSimnet("./clarity/Clarinet.toml");
```

### Simnet Properties

#### `Simnet.blockHeight`

Returns the current block height of the simnet.

```ts
import { initSimnet } from "@hirosystems/clarinet-sdk";
const simnet = await initSimnet();

console.log(simnet.blockHeight); // 0
```

#### `Simnet.deployer`

Returns the default deployer address as defined in the project file `./setting/Devnet.toml`.

```ts
import { initSimnet } from "@hirosystems/clarinet-sdk";
const simnet = await initSimnet();

console.log(simnet.deployer); // ST1P...GZGM
```

### Simnet Methods

#### `Simnet.getAccounts()`

```ts
getAccounts(): Map<string, string>
```

Get the Stacks addresses defined in the project file `./setting/Devnet.toml`.

```ts
import { initSimnet } from "@hirosystems/clarinet-sdk";
const simnet = await initSimnet();

const accounts = simnet.getAccounts();
const address1 = accounts.get("wallet_1")!;
console.log(address1); // ST1S...YPD5
```

#### `Simnet.getAssetsMap()`

```ts
getAssetsMap(): Map<string, Map<string, bigint>>
```

Get a list of asset balances by Stacks addresses. This method returns STX balances as well as FT and NFT balances.

```ts
import { initSimnet } from "@hirosystems/clarinet-sdk";
const simnet = await initSimnet();

const assets = simnet.getAssetsMap();
const stxBalances = assets.get("STX")!;

console.log(stxBalances);
// Map(10) {
//   'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM' => 100000000000000n,
//   'ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5' => 100000000000000n,
//   // ...
// }
```

#### `Simnet.getDataVar()`

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
import { initSimnet } from "@hirosystems/clarinet-sdk";
const simnet = await initSimnet();

const counter = simnet.getDataVar("counter", "count");
// counter is Cl.uint(0)
```

#### `Simnet.getMapEntry()`

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
import { initSimnet } from "@hirosystems/clarinet-sdk";
const simnet = await initSimnet();

const accounts = simnet.getAccounts();
const address1 = accounts.get("wallet_1")!;

const participated = simnet.getMapEntry(
  "counter",
  "participants",
  Cl.standardPrincipal(address1)
);
// counter is Cl.some(Cl.bool(true|false)) or Cl.none()
```

#### `Simnet.callReadOnlyFn()`

```ts
callReadOnlyFn(
  contract: string,     // stacks address of the contract
  method: string,       // read-only function to call
  args: ClarityValue[], // array of Clarity Values
  sender: string        // stacks address of the sender
): ParsedTransactionRes
```

Call read-only functions exposed by a contract. This method returns an object with the result of the function call as a Clarity Value.
It takes function arguments in the form in Clarity Values, available in the package `@stacks/transactions`.

```ts
import { initSimnet } from "@hirosystems/clarinet-sdk";
import { Cl } from "@stacks/transactions";
const simnet = await initSimnet();

const accounts = simnet.getAccounts();
const address1 = accounts.get("wallet_1")!;

const getCounter = simnet.callReadOnlyFn(
  "counter",
  "get-counter",
  [],
  address1
);
console.log(getCounter.result); // Cl.uint(1)

// With arguments:
const callPOX = simnet.callReadOnlyFn(
  "pox-3",
  "is-pox-active",
  [Cl.uint(100)],
  address1
);
```

As in many methods of the SDK, the contract address can be just the contract name, if deployed by the default deployer.

```ts
simnet.callReadOnlyFn("counter", "get-counter", [], address1);
// equivalent
simnet.callReadOnlyFn(
  `${simnet.deployer}.counter`,
  "get-counter",
  [],
  address1
);
```

#### `Simnet.callPublicFn()`

```ts
callPublicFn(
  contract: string,     // stacks address of the contract
  method: string,       // public function to call
  args: ClarityValue[], // array of Clarity Values
  sender: string        // stacks address of the sender
): ParsedTransactionRes
```

Call read-only functions exposed by a contract. This method returns an object with the result of the function call as a Clarity Value and the events fired during the function execution. It takes function arguments in the form in Clarity Values, available in the package `@stacks/transactions`. It will simulate a block being mined and increase the block height by one.

```ts
import { initSimnet } from "@hirosystems/clarinet-sdk";
import { Cl } from "@stacks/transactions";
const simnet = await initSimnet();

const accounts = simnet.getAccounts();
const address1 = accounts.get("wallet_1")!;

const callAdd = simnet.callPublicFn("counter", "add", [Cl.uint(3)], address1);
console.log(callAdd.result); // a Clarity Value such as Cl.bool(true)
console.log(callAdd.events); // and array of events (such as print event, stx stransfer event, etc)
```

#### `Simnet.transferSTX()`

```ts
transferSTX(amount: number | bigint, recipient: string, sender: string): ParsedTransactionRes
```

Transfer STX from an address to an other. The amount is in uSTX. It will simulate a block being mined and increase the block height by one.

```ts
import { initSimnet } from "@hirosystems/clarinet-sdk";
const simnet = await initSimnet();

const accounts = simnet.getAccounts();
const address1 = accounts.get("wallet_1")!;
const address2 = accounts.get("wallet_2")!;

const transfer = simnet.transferSTX(100, address1, address2);
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

#### `Simnet.deployContract()`

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

Deploy a contract to the Simnet. It will simulate a block being mined and increase the block height by one.

```ts
import { initSimnet } from "@hirosystems/clarinet-sdk";
import { Cl } from "@stacks/transactions";
const simnet = await initSimnet();

const accounts = simnet.getAccounts();
const address1 = accounts.get("wallet_1")!;

const source = "(define-public (add (a uint) (b uint)) (ok (+ a b)))";
const deployRes = simnet.deployContract("op", source, simnet.deployer);

const addRes = simnet.callPublicFn(
  "op",
  "add",
  [Cl.uint(1), Cl.uint(1)],
  address1
);
console.log(addRes.result); // Cl.ok(Cl.uint(2))

// specify a clarityVersion
simnet.deployContract("contract2", source, { clarityVersion: 2 }, deployerAddr);
```

#### `Simnet.mineBlock()`

```ts
mineBlock(txs: Tx[]): ParsedTransactionRes[]
```

The `.callPublicFn()`, `.transferSTX()`, and `.deployContract()` methods all mine one block with only one transaction. It can also be useful to mine a block with multiple transactions. This is what `.mineBlock()` is for.

It take an array of transaction objects. The transactions can be built with the `tx` helper exported by the SDK.
It has three methods `.callPublicFn()`, `.transferSTX()`, and `.deployContract()`, which have the same interface as the `Simnet` methods but instead of performing a transaction, it will build a transaction object than can be passed to the `mineBlock()` function.

```ts
// import `tx` as well
import { initSimnet, tx } from "@hirosystems/clarinet-sdk";
import { Cl } from "@stacks/transactions";
const simnet = await initSimnet();

const accounts = simnet.getAccounts();
const address1 = accounts.get("wallet_1")!;
const address2 = accounts.get("wallet_2")!;

const block = simnet.mineBlock([
  tx.callPublicFn("counter", "increment", [], address1),
  tx.callPublicFn("counter", "add", [Cl.uint(10)], address1),
  tx.transferSTX(100, address1, address2),
]);

console.log(block[0]); // `increment` response with { result, events}
console.log(block[1]); // `add` response with { result, events}
console.log(block[2]); // `transfer_stx` response with { result, events}
```

#### `Simnet.mineEmptyBlock()`

```ts
mineEmptyBlock(): number
```

Mine one empty block and increase the block height by one. Returns the new block height.

```ts
import { initSimnet } from "@hirosystems/clarinet-sdk";
const simnet = await initSimnet();

console.log(simnet.blockHeight); // 0
const newHeight = simnet.mineEmptyBlock();
cosole.log(newHeight); // 1
console.log(simnet.blockHeight); // 1
```

#### `Simnet.mineEmptyBlocks()`

```ts
mineEmptyBlocks(count?: number): number
```

Mine multiple empty blocks to reach a certain block height. Returns the new block height.

```ts
import { initSimnet } from "@hirosystems/clarinet-sdk";
const simnet = await initSimnet();

console.log(simnet.blockHeight); // 0
const newHeight = simnet.mineEmptyBlocks(10);
console.log(newHeight); // 10
console.log(simnet.blockHeight); // 10
```

#### `Simnet.getContractsInterfaces()`

```ts
getContractsInterfaces(): Map<string, ContractInterface>
```

Returns the interfaces of the project contracts. This method returns a Map of Contracts; the keys are the contract addresses.
The interfaces contain information such as the available functions, data-vars and maps, NFTs, and the FTs defined in the contract.
It can be used to get the list of the contracts and iterate of it.

```ts
import { initSimnet } from "@hirosystems/clarinet-sdk";
const simnet = await initSimnet();

const contractInterfaces = simnet.getContractsInterfaces();
let counterInterface = contractInterfaces.get(`${deployerAddr}.counter`);
console.log(counterInterface?.functions); // array of the functions
```

#### `Simnet.getContractSource()`

```ts
getContractSource(contract: string): string | undefined
```

Get the source code of a contract as a string.

```ts
import { initSimnet } from "@hirosystems/clarinet-sdk";
const simnet = await initSimnet();

const source = "(define-public (add (a uint) (b uint)) (ok (+ a b)))";
simnet.deployContract("contract", source, null, deployerAddr);

const contractSource = simnet.getContractSource("contract");
console.log(contractSource);
// "(define-public (add (a uint) (b uint)) (ok (+ a b)))"
```

#### `Simnet.getContractAST()`

```ts
getContractAST(contractId: string): ContractAST
```

Get the full AST of a Clarity contract.

It throws an error if it fails to get the AST or to encode it JS (which should not happen).  
Note: The `ContractAST` TypeScript is still very simple but will be improved over time.

```ts
import { initSimnet } from "@hirosystems/clarinet-sdk";
const simnet = await initSimnet();

const counterAst = simnet.getContractAST("counter");
```

# Clarinet SDK

The Clarinet SDK can be used to interact with the simnet from Node.js.

Some of features are:
- call public and read-only functions from smart contracts
- get clarity maps or data-var values
- deploy contracts
- get contract ABI
- write unit tests for Clarity smart contracts

## Core

```
npm install @hirosystems/clarinet-sdk
```

### Usage

```ts
import { initVM } from "clarinet-sdk";
import { Cl } from "@stacks/transactions";

async function main() {
  const vm = await initVM();

  const accounts = vm.getAccounts();
  const w1 = accounts.get("wallet_1")!;

  const call = vm.callPublicFn("counter", "add", [Cl.uint(1)], w1);
  console.log(call.result); // Cl.int(Cl.ok(true))

  const counter = vm.getDataVar("counter", "counter");
  console.log(counter); // Cl.int(2)
}

main();
```

By default, the SDK will look for a Clarinet.toml file in the current working directory.
It's also possible to provide the path to the manifest like so:
```ts
 const vm = await initVM("./path/to/Clarinet.toml");
```

## Tests

<!-- wip -->


## Contributing

Clone the clarinet repo adn switch to the sdk component
```
git clone git@github.com:hirosystems/clarinet.git
cd clarinet/components/clarinet-sdk
```

Open the SDK workspace in VSCode:
```
code ./clarinet-sdk.code-workspace
```

Compile the project (both WASM and JS):
```
npm run build
```

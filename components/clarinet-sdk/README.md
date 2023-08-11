# Clarinet SDK

## Core

### Usage

```ts
import { Cl } from "@stacks/transactions";
import initVM, { ClarityVM } from "@hirosystems/clarinet-sdk";
// available under `obscurity-sdk` right now
// import initVM, { ClarityVM } from "obscurity-sdk";

async function main() {
  const vm = await initVM(); // or await initVM("./path/to/Clarinet.toml")
  const accounts = vm.getAccounts();
  const w1 = accounts.get("wallet_1")!;

  const result = vm.callPublicFn("counter", "increment", [], w1);
  console.log(result) // Cl.int(Cl.ok(true))

  const counter = vm.getDataVar("counter", "counter");
  console.log(counter) // Cl.int(1)
}
```

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

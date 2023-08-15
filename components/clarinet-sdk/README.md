# Clarinet SDK

## Core

### Usage

```ts
import { Cl } from "@stacks/transactions";
import { initVM } from "obscurity-sdk";

async function main() {
  const vm = await initVM();
  const accounts = vm.getAccounts();
  const w1 = accounts.get("wallet_1");
  if (!w1) return;

  const call = vm.callPublicFn("counter", "increment", [Cl.uint(1)], w1);
  console.log(call.result); // Cl.int(Cl.ok(true))

  const counter = vm.getDataVar("counter", "counter");
  console.log(counter); // Cl.int(2)
}

main();
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

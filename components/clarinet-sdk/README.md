# Clarinet SDK

The Clarinet SDK can be used to interact with the simnet from Node.js.

You can use this SDK to:
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

> Note: A bit of boilerplate is needed to setup the testing environment. Soon it will be handled by the clarinet-cli.

The SDK can be used to write unit-tests for Clarinet projects.  
Make sure you are in directory with a Clarinet.toml file and the associated Clarity smart contracts:

```sh
cd ./my-project
ls # here you should see the Clarinet.toml file
```

Let's initialize the Node.js project:
```sh
npm init -y # the -y option sets default properties
npm install @hirosystems/clarinet-sdk @stacks/transactions vite vitest vitest-environment-clarinet
```

Update the package.json file scripts to handle tests:
```json
  "scripts": {
    "test": "vitest run",
    "test:coverage": "vitest run -- --coverage true"
  },
```

The `./.gitignore` file also needs to be updated, add the following lines at the end. It is especially important to ignore `node_modules`.
```
logs
*.log
npm-debug.log*
coverage
*.info
node_modules
```

A config file is needed for Vitest to use the clarinet-environment.
Create the file `vitest.config.js` with the following content:
```js
/// <reference types="vitest" />

import { defineConfig } from "vite";
import { vitestSetupFilePath, getClarinetVitestsArgv } from "@hirosystems/clarinet-sdk/vitest";

export default defineConfig({
  test: {
    environment: "clarinet",
    singleThread: true,
    setupFiles: [vitestSetupFilePath],
    environmentOptions: {
      clarinet: getClarinetVitestsArgv(),
    },
  },
});
```

The set up is ready, let's write the first test. Create a test file in the `unit-tests` directory:

```sh
mkdir unit-tests
touch unit-tests/my-contract.test.js
```

```js
// unit-tests/my-contract.test.js
import { describe, it, expect } from "vitest";
import { Cl } from "@stacks/transactions";

describe("test counter ONE", () => {
  const accounts = vm.getAccounts();
  const w1 = accounts.get("wallet_1");
  if (!w1) throw new Error("wallet_1 does not exist");

  it("adds two numbers", () => {
    const callAdd = vm.callPublicFn("my-contract", "add", [Cl.uint(21), Cl.uint(21)], w1);
    expect(callAdd.result).toBeOk(Cl.uint(42));
  });
});

```

### Notes: 

- This code assumes that you have a contract called `my-contract` with a method `add`.
```clar
;; contracts/my-contract.clar
(define-public (add (n1 uint) (n2 uint))
  (ok (+ n1 n2))
)
```

- You may need to disable the deno extension if it's activated in `.vscode/settings.json`.


### Type checking

You can use TypeScript to write test by installing it and setting up the `tsconfig.json`.

```sh
npm install typescript
touch tsconfig.json
```

```json
{
  "compilerOptions": {
    "target": "ESNext",
    "useDefineForClassFields": true,
    "module": "ESNext",
    "lib": ["ESNext"],
    "skipLibCheck": true,

    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,

    "strict": true,
    "noImplicitAny": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true
  },
  "include": ["node_modules/@hirosystems/clarinet-sdk/vitest-helpers/src", "unit-tests"]
}

```

If you want to write your test in JavaScript but still have a certain level of type safety and autocompletion, VSCode can help you with that. You can create a basic `jsconfig.json` file:

```json
{
  "compilerOptions": {
    "checkJs": true,
    "strict": true
  },
  "include": ["node_modules/@hirosystems/clarinet-sdk/vitest-helpers/src", "unit-tests"]
}
```

## Contributing

Clone the clarinet repo and go to the clarinet-sdk component directory:
```sh
git clone git@github.com:hirosystems/clarinet.git
cd clarinet/components/clarinet-sdk
```

Open the SDK workspace in VSCode:
```sh
code ./clarinet-sdk.code-workspace
```

Compile the project (both WASM and JS):
```sh
npm install
npm run build
```

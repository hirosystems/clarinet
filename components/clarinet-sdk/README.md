# Clarinet SDK

The Clarinet SDK can be used to interact with the simnet from Node.js.

Find the API references of the SDK in [our documentation](https://docs.hiro.so/clarinet/feature-guides/clarinet-js-sdk).  
Learn more about unit testing Clarity smart contracts in [this guide](https://docs.hiro.so/clarinet/feature-guides/test-contract-with-clarinet-sdk).

You can use this SDK to:
- Call public and read-only functions from smart contracts
- Get clarity maps or data-var values
- Get contract interfaces (available functions and data)
- Write unit tests for Clarity smart contracts

## Core

```
npm install @hirosystems/clarinet-sdk
```

### Usage

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

  const counter = simnet.getDataVar("counter", "counter");
  console.log(counter); // Cl.int(2)
}

main();
```


By default, the SDK will look for a Clarinet.toml file in the current working directory.
It's also possible to provide the path to the manifest like so:
```ts
 const simnet = await initSimnet("./path/to/Clarinet.toml");
```

## Tests

The SDK can be used to write unit tests for Clarinet projects.  

You'll need to have Node.js (>= 18) and NPM setup. If you are not sure how to set it up, [Volta](https://volta.sh/) is a nice tool to get started.

In the terminal, run `node --version` to make sure it's available and up to date.

> Note: A bit of boilerplate is needed to setup the testing environment. Soon it will be handled by the clarinet-cli.

Open your terminal and go to a new or existing Clarinet project:

```sh
cd my-project
ls # you should see a Clarinet.toml file in the list
```

Run the following command to setup the testing framework:

```sh
npx @hirosystems/clarinet-sdk
```

Visit the [clarity starter project](https://github.com/hirosystems/clarity-starter/tree/170224c9dd3bde185f194a9036c5970f44c596cd) to see the testing framework in action.


### Type checking

We recommend to use TypeScript to write the unit tests, but it's also possible to do it with JavaScript. To do so, rename your test files to `.test.js` instead of `.test.ts`. You can also delete the `tsconfig.json` and uninstall typescript with `npm uninstall typescript`. 

Note: If you want to write your test in JavaScript but still have a certain level of type safety and autocompletion, VSCode can help you with that. You can create a basic `jsconfig.json` file:

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

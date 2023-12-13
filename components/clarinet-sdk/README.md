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

```console
cd my-project
ls # you should see a Clarinet.toml file in the list
```

Run the following command to setup the testing framework:

```console
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

The clarinet-sdk requires a few steps to be built and tested locally.
We'll look into simplifying this workflow in a future version.

Clone the clarinet repo and `cd` into it:
```console
git clone git@github.com:hirosystems/clarinet.git
cd clarinet
```

Open the SDK workspace in VSCode, it's especially useful to get rust-analyzer
to consider the right files with the right cargo features.
```console
code components/clarinet-sdk/clarinet-sdk.code-workspace
```

The SDK mainly relies on two components:
- the Rust component: `components/clarinet-sdk-wasm`
- the TS component: `components/clarinet-sdk`

To work with these two packages locally, the first one needs to be built with
wasm-pack and linked with: [npm link](https://docs.npmjs.com/cli/v8/commands/npm-link).

Install [wasm-pack](https://rustwasm.github.io/wasm-pack/installer) and run:
```console
cd components/clarinet-sdk-wasm
wasm-pack build --release --target=nodejs --scope hirosystems
cd pkg
npm link
```

Go to the `clarinet-sdk` directory and link the package that was just built.
It will tell npm to use it instead of the published version. You don't need to
repeat the steps everytime the `clarinet-sdk-wasm` changes, it only needs to be
rebuilt with wasm-pack and npm will use it.

Built the TS project:
```console
cd ../../clarinet-sdk
npm link @hirosystems/clarinet-sdk-wasm
```

You can now run `npm test`, it wil be using the local version of `clarinet-sdk-wasm`

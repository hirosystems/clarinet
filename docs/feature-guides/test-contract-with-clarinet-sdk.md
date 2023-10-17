---
title: Unit Tests With the Clarinet SDK
---

The [Clarinet SDK](https://www.npmjs.com/package/@hirosystems/clarinet-sdk) allows you to write unit tests for your Clarity smart contracts. You can theoritically use any JavaScript test framework, but the SDK supports [Vitest](https://vitest.dev/) out of the box.

> Make sure you are using the latest version of Clarinet to follow this guide. See the [getting started](../getting-started.md) guide to know more.

> Take a look at the [API reference guide](../feature-guides/clarinet-js-sdk.md) for more information about the methods and elements of the clarinet-sdk.

## Requirements

The SDK requires Node.js >= 18.0 and NPM to be installed. [Volta](https://volta.sh/) is a great tool to install and manage JS tooling.

To follow this tutorial, you must have the Clarinet CLI installed as well.

## Set Up the Clarity Contract and Unit Tests

Let us consider a `counter` smart contract to understand how to write unit tests for our application requirements.

First, create a new Clarinet project with a `counter` contract.

```console
clarinet new counter
cd counter
clarinet contract new counter
```

Below will be the content of our smart contract.
It keeps track of an initialized value, allows for incrementing and decrementing, and prints actions as a log.

```clarity
;; counter.clar
(define-data-var count uint u1)

(define-public (increment (step uint))
  (let ((new-val (+ (var-get count) step)))
    (var-set count new-val)
    (print { object: "count", action: "incremented", value: new-val })
    (ok new-val)
  )
)

(define-public (decrement (step uint))
  (let ((new-val (- (var-get count) step)))
    (var-set count new-val)
    (print { object: "count", action: "decremented", value: new-val })
    (ok new-val)
  )
)

(define-read-only (read-count)
  (ok (var-get count))
)
```

### Migrating Between Clarinet v1 and Clarinet v2

> Note: Clarinet v2 will be released in October 2023, and will create the right boilerplate files. But if a project has been created with Clarinet v1, the following script prepares the project to run the SDK and Vitest.

Executing this script in a Clarinet v1 project will initialise NPM and Vitest. It will also create a sample test file.

```console
npx @hirosystems/clarinet-sdk@latest
```

This script will ask you if you want to run npm install now; you can press enter to do so.
This can take a few seconds.

The file `tests/counter_test.ts` that was created by `clarinet contract new counter` can be deleted.

You can also have a look at `tests/contract.test.ts`. It's a sample file showing how to use the SDK with Vitest.
It can safely be deleted.

### Unit Tests for `counter` Example

Create a file `tests/counter.test.ts` with the following content:

```ts
import { Cl } from "@stacks/transactions";
import { describe, expect, it } from "vitest";

const accounts = simnet.getAccounts();
const address1 = accounts.get("wallet_1")!;

describe("test `increment` public function", () => {
  it("increments the count by the given value", () => {
    const incrementResponse = simnet.callPublicFn(
      "counter",
      "increment",
      [Cl.uint(1)],
      address1
    );
    console.log(Cl.prettyPrint(incrementResponse.result)); // (ok u2)
    expect(incrementResponse.result).toBeOk(Cl.uint(2));

    const count1 = simnet.getDataVar("counter", "count");
    expect(count1).toBeUint(2);

    simnet.callPublicFn("counter", "increment", [Cl.uint(40)], address1);
    const count2 = simnet.getDataVar("counter", "count");
    expect(count2).toBeUint(42);
  });

  it("sends a print event", () => {
    const incrementResponse = simnet.callPublicFn(
      "counter",
      "increment",
      [Cl.uint(1)],
      address1
    );

    expect(incrementResponse.events).toHaveLength(1);
    const printEvent = incrementResponse.events[0];
    expect(printEvent.event).toBe("print_event");
    expect(printEvent.data.value).toBeTuple({
      object: Cl.stringAscii("count"),
      action: Cl.stringAscii("incremented"),
      value: Cl.uint(2),
    });
  });
});
```

To run the test, go back to your console and run the `npm test` command. It should display a report telling you that tests succeeded.

```sh
npm test
```

There is a very important thing happening under the hood here. The `simnet` object is available globally in the tests, and is automatically initialized before each test.

> You don't need to know much more about that, but if you want to know in detail how it works, you can have a look at the `vitest.config.js` file at the root of you project.

Getting back to the tests, we just implemented two of them:

- The first test checks that the `increment` function returns the new value and saves it to the `count` variable.
- The second test checks that an `print_event` is emitted when the increment function is called.

> You can use `Cl.prettyPrint(value: ClarityValue)` to format any Clarity value into readable Clarity code. It can be useful to debug function results or event values.

Note that we are importing `describe`, `expect` and `it` from Vitest.

- `it` allows us to write a test.
- `describe` is not necessary but allows to organize tests.
- `expect` is use to make assertions on value.

You can learn more about Vitest on their [website](https://vitest.dev).
We also implemented some custom matchers to make assertions on Clarity variables (like `toBeUint`).
The [full list of custom matchers](#custom-vitest-matchers) is available at the end of this guide.

### Comprehensive Unit Tests for `counter`

Let us now write a higher coverage test suite by testing the `decrement` and `get-counter` functions.

These two code blocks can be added at the end of `tests/counter.test.ts`.

```ts
describe("test `decrement` public function", () => {
  it("decrements the count by the given value", () => {
    const decrementResponse = simnet.callPublicFn(
      "counter",
      "decrement",
      [Cl.uint(1)],
      address1
    );
    expect(decrementResponse.result).toBeOk(Cl.uint(0));

    const count1 = simnet.getDataVar("counter", "count");
    expect(count1).toBeUint(0);

    // increase the count so that it can be descreased without going < 0
    simnet.callPublicFn("counter", "increment", [Cl.uint(10)], address1);

    simnet.callPublicFn("counter", "decrement", [Cl.uint(5)], address1);
    const count2 = simnet.getDataVar("counter", "count");
    expect(count2).toBeUint(5);
  });

  it("sends a print event", () => {
    const decrementResponse = simnet.callPublicFn(
      "counter",
      "decrement",
      [Cl.uint(1)],
      address1
    );

    expect(decrementResponse.events).toHaveLength(1);
    const printEvent = decrementResponse.events[0];
    expect(printEvent.event).toBe("print_event");
    expect(printEvent.data.value).toBeTuple({
      object: Cl.stringAscii("count"),
      action: Cl.stringAscii("decremented"),
      value: Cl.uint(0),
    });
  });
});
```

```ts
describe("test `get-count` read only function", () => {
  it("returns the counter value", () => {
    const count1 = simnet.callReadOnlyFn("counter", "read-count", [], address1);
    expect(count1.result).toBeOk(Cl.uint(1));

    simnet.callPublicFn("counter", "increment", [Cl.uint(10)], address1);
    const count2 = simnet.callReadOnlyFn("counter", "read-count", [], address1);
    expect(count2.result).toBeOk(Cl.uint(11));
  });
});
```

## Measure and Increase Code Coverage

To help developers maximize their test coverage, the test framework can produce a `lcov` report, using `--coverage` flag. You can set it in the scripts in the project `package.json`:

```json
  "scripts": {
    "test:coverage": "vitest run -- --coverage",
    // ...
  },
```

Then run the script with the following command. It will produce a file named `./lcov.info`.

```sh
npm run test:coverage
```

From there, you can use the `lcov` tooling suite to produce HTML reports.

```sh
brew install lcov
genhtml --branch-coverage -o coverage lcov.info
open coverage/index.html
```

## Costs Optimization

The test framework can also be used to optimize costs. When you execute a test suite, Clarinet keeps track of all costs being computed when executing the `contract-call`, and displays the most expensive ones in a table.

To help developers maximize their test coverage, the test framework can produce a `lcov` report, using `--coverage` flag. You can set it in the scripts in the project `package.json`:

```json
  "scripts": {
    "test:costs": "vitest run -- --costs",
    // ...
  },
```

And run the script with the following command. It will produce a file named `./costs-reports.json`.

```sh
npm run test:costs
```

For now, there isn't much you can do out of the box with a costs report. But in future versions of the clarinet sdk, we will implement features to help keep track of your costs, such as checking that function calls do not go above a certain threshold.

## Produce Both Coverage and Costs Reports

In your package.json, you should already have a script called `test:reports` like so:

```json
  "scripts": {
    "test:reports": "vitest run -- --coverage --costs",
    // ...
  },
```

Run it to produce both the coverage and the costs reports:

```sh
npm run test:reports
```

## Run Tests in CI

Because the tests only require Node.js and NPM run, they can also be run in GitHub actions and CIs just like any other Node test.

In GitHub, you can directly set up a Node.js workflow like this one:

```yml
name: Test counter contract

on:
  push:
    branches: ["main"]
  pull_request:
    branches: ["main"]

jobs:
  build:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        node-version: [20.x]

    steps:
      - uses: actions/checkout@v3
      - name: Use Node.js ${{ matrix.node-version }}
        uses: actions/setup-node@v3
        with:
          node-version: ${{ matrix.node-version }}
          cache: "npm"
      - run: npm ci
      - run: npm run test:reports
```

## Custom Vitest Matchers

A set of Vitest matchers can be used to make assertions on Clarity values.
They can check the return values of contracts, ensure that the value is actually a Clarity value, and provide nice error messages.

### Check Clarity Type

#### `toHaveClarityType(expectedType: ClarityType)` <!-- omit from toc -->

This matcher can be used to make sure that the value has the right Clarity Type, without checking its value.

```ts
import { ClarityType } from "@stacks/transactions";
import { expect, it } from "vitest";

const address1 = simnet.getAccounts().get("wallet_1");

it("ensures <increment> adds 1", () => {
  const { result } = simnet.callPublicFn("counter", "increment", [], address1);

  // make sure it returns a response ok `(ok ...)`
  expect(result).toHaveClarityType(ClarityType.ResponseOk);
});
```

It can also be used to check any type:

```ts
// uint
expect(result).toHaveClarityType(ClarityType.UInt);

// or tuple
expect(result).toHaveClarityType(ClarityType.Tuple);

// and so one
```

### Response Type

The response type is noted `(response <ok-type> <error-type>)` in Clarity.
It can be `(ok <ok-type>)` or `(err <error-type>)`.
They are called composite types, meaning that they contain another Clarity value.

#### `toBeOk(expected: ClarityValue)` <!-- omit from toc -->

Check that a response is `ok` and has the expected value. Any Clarity value can be passed.

```ts
const decrement = simnet.callPublicFn(
  "counter",
  "decrement",
  [Cl.uint(1)],
  address1
);

// decrement.result is `(ok (uint 0))`
expect(decrement.result).toBeOk(Cl.uint(0));
```

#### `toBeErr(expected: ClarityValue)` <!-- omit from toc -->

Check that a response is `err` and has the expected value. Any Clarity value can be passed.

Consider that the `counter` contract returns and error code 500 `(err u500)` if the value passed to increment is too big:

```ts
const tooBig = 100000;
const increment = simnet.callPublicFn(
  "counter",
  "increment",
  [Cl.uint(toBig)],
  address1
);

// increment.result is `(err u500)`
expect(increment.result).toBeErr(Cl.uint(500));
```

### Optional Type

The option type is noted `(optional <some-type>)` in Clarity.
It can be `(some <some-type>)` or `none`.
Here, `some` is a composite type, meaning that it contains another Clarity value.

#### `toBeSome(expected: ClarityValue)` <!-- omit from toc -->

Consider a billboard smart contract that can contain an optional message:

```ts
const getMessage = simnet.callPublicFn(
  "billboard",
  "get-message",
  [],
  address1
);

// (some u"Hello world")
expect(getMessage.result).toBeSome(Cl.stringUtf8("Hello world"));
```

#### `toBeNone()` <!-- omit from toc -->

Considering the same billboard smart contract but with no saved message:

```ts
const getMessage = simnet.callPublicFn(
  "billboard",
  "get-message",
  [],
  address1
);

// none
expect(getMessage.result).toBeNone();
```

### Simple Clarity Types

Custom assertion matchers are available for all types of Clarity values. They will check that the value has the right type and value.

#### `toBeBool(expected: boolean)` <!-- omit from toc -->

Asserts the value of Clarity boolean (true or false).

```ts
expect(trueResult).toBeBool(true);
expect(falseResult).toBeBool(false);
```

#### `toBeInt(expected: number | bigint)` <!-- omit from toc -->

Asserts the value of a Clarity int.

```ts
expect(result).toBeInt(1);
// it accepts JS bigints
expect(result).toBeInt(1n);
```

#### `toBeUint(expected: number | bigint)` <!-- omit from toc -->

Asserts the value of a Clarity uint.

```ts
expect(result).toBeUint(1);
// it accepts JS bigints
expect(result).toBeUint(1n);
```

#### `toBeAscii(expected: string)` <!-- omit from toc -->

Asserts the value of a Clarity string-ascii.

```ts
expect(result).toBeAscii("Hello wolrd");
```

#### `toBeUtf8(expected: string)` <!-- omit from toc -->

Asserts the value of a Clarity string-utf8.

```ts
expect(result).toBeUtf8("STX");
```

#### `toBePrincipal(expected: string)` <!-- omit from toc -->

Asserts the value of a Clarity principal value. The principal can be a standard or a contract principal.

```ts
expect(standardPrincipal).toBePrincipal(
  "ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM"
);
expect(contractPrincipal).toBePrincipal(
  "ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.counter"
);
```

#### `toBeBuff(expected: Uint8Array)` <!-- omit from toc -->

Asserts the value of a Clarity buffer.  
It takes as an input an ArrayBuffer (`Uint8Array`).

Your test case will ultimately depends on how the Uint8Array is built. `@stacks/transaction` can help building these buffers.

```ts
it.only("can assert buffer values", () => {
  const { result } = simnet.callPublicFn(/* ... */);

  // knowing the expected UintArray
  const value = Uint8Array.from([98, 116, 99]);
  expect(result).toBeBuff(value);

  // knowing the expected string
  const bufferFromAscii = Cl.bufferFromAscii("btc");
  expect(result).toBeBuff(bufferFromAscii.buffer);

  // knowing the expected hex value
  const bufferFromHex = Cl.bufferFromHex("627463");
  console.log(bufferFromHex.buffer);
});
```

### Other Composite Types

`list` and `tuple` are composite types, like `ok`, `err`, and `some`. Meanning that they contain another Clarity value.

#### `toBeList(expected: ClarityValue[])` <!-- omit from toc -->

Check that the value is a `list` containing an array of Clarity values.  
Considering a function that return a list of 3 uints:

```ts
const address1 = simnet.getAccounts().get("wallet_1");

it("can assert list values", () => {
  const { result } = simnet.callReadOnlyFn(
    "counter",
    "func-returning-list-of-uints",
    [],
    address1
  );

  expect(result).toBeList([Cl.uint(1), Cl.uint(2), Cl.uint(3)]);
});
```

#### `toBeTuple(expected: Record<string, ClarityValue>)` <!-- omit from toc -->

Check that the value is a `tuple`, it takes a JavaScript object to check the values. It's used in the [tutorial above](#unit-tests-for-counter-example) to check the value of the print event. It can also be used to check function call result.

The snippet below shows that composite types can be nested:

```ts
const address1 = simnet.getAccounts().get("wallet_1");

it("can assert tuple values", () => {
  const { result } = simnet.callPublicFn(
    "counter",
    "func-returning-tuple",
    [],
    address1
  );

  expect(result).toBeTuple({
    id: Cl.uint(1),
    data: Cl.tuple({
      text: Cl.stringUtf8("Hello world"),
      owner: Cl.standardPrincipal(address1),
    }),
  });
});
```

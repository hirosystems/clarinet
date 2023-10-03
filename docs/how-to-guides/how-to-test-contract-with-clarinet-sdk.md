---
title: Test Contract with clarinet-sdk
---

The [Clarinet JS SDK](https://www.npmjs.com/package/@hirosystems/clarinet-sdk) allows to write unit tests for your Clarity smart contract.  
You can theoritically use any JavaScript test framework, but the SDK supports [Vitest](https://vitest.dev/) out of the box.

*Topics covered in this guide*:

- [Requirements](#requirements)
- [Set up the Clarity contract and unit tests](#set-up-the-clarity-contract-and-unit-tests)
- [Measure and increase code coverage](#measure-and-increase-code-coverage)
- [Costs optimization](#costs-optimization)
- [Produce both coverage and costs reports.](#produce-both-coverage-and-costs-reports)
- [Custom Vitest matchers](#custom-vitest-matchers)
  - [Check clarity type](#check-clarity-type)
  - [Response type](#response-type)
  - [Optional type](#optional-type)
  - [Simple clarity types](#simple-clarity-types)
  - [Other composite types](#other-composite-types)

## Requirements

The SDK requires Node.js >= 18.0 and NPM to be installed. [Volta](https://volta.sh/) is a great tool to install and manage JS tooling.

To follow this tutorial, you must have the Clarinet CLI installed as well.

## Set up the Clarity contract and unit tests

Let us consider a `counter` smart contract to understand how to write unit tests for our application requirements.

Create a new Clarinet project with a `counter` contract.

```console
clarinet new counter
cd counter
clarinet contract new counter
```

And this will be the content of our smart contract.
It keeps track of an initialized value, allows for incrementing and decrementing, and prints actions as a log.

```clarity
;; counter.clar
(define-data-var count uint u1)

(define-public (increment (step uint))
  (let ((new-val (+ step (var-get count)))) 
    (var-set count new-val)
    (print { object: "count", action: "incremented", value: new-val })
    (ok new-val)
  )
)

(define-public (decrement (step uint))
  (let ((new-val (- step (var-get count)))) 
    (var-set count new-val)
    (print { object: "count", action: "decremented", value: new-val })
    (ok new-val)
  )
)

(define-read-only (read-count)
  (ok (var-get count))
)
```

### Migrating between Clarinet 1 and Clarinet 2  <!-- omit from toc -->

> Note: Clarinet 2 will be released in October 2023, and will create the right boilerplate files. But if a project has been created with Clarinet 1, this prepare to project to run the SDK and Vitest.

Executing this script in a Clarinet 1 project will initialise NPM and Vitest. It will also create a sample test file.

```console
npx @hirosystems/clarinet-sdk@latest
```

This script wil lask you if you want to run npm install now, you can press enter to do so.
It can take a few seconds.

The file `tests/counter_test.ts` that was created by `clarinet contract new counter` can be deleted.  

You can have a look at `tests/contract.test.ts`, it's a sample file showing how to use the SDK with Vitest.
It can safely be deleted.


### Unit tests for `counter` example  <!-- omit from toc -->

Create a file `tests/counter.test.ts` with the following content:

```ts
import { Cl } from "@stacks/transactions";
import { describe, expect, it } from "vitest";

const accounts = vm.getAccounts();
const address1 = accounts.get("wallet_1")!;

describe("test increment method", () => {
  it("increments the count by the given value", () => {
    const res1 = vm.callPublicFn("counter", "increment", [Cl.uint(1)], address1);
    expect(res1.result).toBeOk(Cl.uint(2));

    const count1 = vm.getDataVar("counter", "count");
    expect(count1).toBeUint(2);

    vm.callPublicFn("counter", "increment", [Cl.uint(40)], address1);
    const count2 = vm.getDataVar("counter", "count");
    expect(count2).toBeUint(42);
  });

  it("sends a print event", () => {
    const res = vm.callPublicFn("counter", "increment", [Cl.uint(1)], address1);

    expect(res.events).toHaveLength(1);
    const printEvent = res.events[0];
    expect(printEvent.event).toBe("print_event");
    expect(printEvent.data.value).toBeTuple({
      object: Cl.stringAscii("count"),
      action: Cl.stringAscii("incremented"),
      value: Cl.uint(2),
    });
  });
});
```

There is a very important thing happening under the hood. The `vm` object is available globally in the tests, and is automatically initialized before each test.

> You don't need to know much more about that, but if you want to know in details how it works, you can have a look at the `vitest.config.js` file at the root of you project.

We just implement two tests:
- The first one check that the `increment` function return the new value and saves it to the `count` variable.
- The second on 

Note that we are importing `describe`, `expect` and `it` from Vitest.
- `it` allows us to write a test.
- `describe` is not necessary but allows to organize tests.
- `expect` is use to make assertions on value.

You can learn more about Vitest on their [website](https://vitest.dev).
We also implemented some custom matchers to make assertions on Clarity variables (like `toBeUint`).
The [full list of custom matchers](#custom-vitest-matchers) is available at the end of this guide. 


### Comprehensive unit tests for `counter`  <!-- omit from toc -->

Let us now write a higher coverage test suite by testing the `decrement` and `get-counter` functions.

These two code blocks can be added at the end of `tests/counter.test.ts`.

```ts
describe("test `decrement` public function", () => {
  it("decrements the count by the given value", () => {
    const res1 = vm.callPublicFn("counter", "decrement", [Cl.uint(1)], address1);
    expect(res1.result).toBeOk(Cl.uint(0));

    const count1 = vm.getDataVar("counter", "count");
    expect(count1).toBeUint(0);

    // increase the count so that it can be descreased without going < 0
    vm.callPublicFn("counter", "increment", [Cl.uint(10)], address1);

    vm.callPublicFn("counter", "decrement", [Cl.uint(5)], address1);
    const count2 = vm.getDataVar("counter", "count");
    expect(count2).toBeUint(5);
  });

  it("sends a print event", () => {
    const res = vm.callPublicFn("counter", "decrement", [Cl.uint(1)], address1);

    expect(res.events).toHaveLength(1);
    const printEvent = res.events[0];
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
    const res = vm.callReadOnlyFn("counter", "read-count", [], address1);
    expect(res).toBeUint(1);
  });
});
```


## Measure and increase code coverage

To help developers maximizing their test coverage, the test framework can produce a `lcov` report, using `--coverage` flag. You can set it in the scripts in the project `package.json`:

```json
  "scripts": {
    "test:coverage": "vitest run -- --coverage",
    // ...
  },
```

And run the script with the following command. It will produce a file named `./lcov.info`.

```sh
npm run test:coverage
```

From there, you can use the `lcov` tooling suite to produce HTML reports.

```sh
brew install lcov
genhtml --branch-coverage -o coverage lcov.info
open coverage/index.html
```

## Costs optimization

The test framework can also be used to optimize costs. When you execute a test suite, Clarinet keeps track of all costs being computed when executing the `contract-call`, and display the most expensive ones in a table:

To help developers maximizing their test coverage, the test framework can produce a `lcov` report, using `--coverage` flag. You can set it in the scripts in the project `package.json`:

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

For now, there isn't much you can do out of the box with costs reports. But in future version of the clarinet sdk, we will implement features to help keep track on your costs, such as checking that function calls do not go above a certain threshold.

## Produce both coverage and costs reports.

In your package.json, you should already have a script called `test:reports` like so:

```json
  "scripts": {
    "test:reports": "vitest run -- --coverage --costs",
    // ...
  },
```

Run it to produce both the coverage and the costs reports:

```sh
npm run test:coverage
```


## Custom Vitest matchers

A set of Vitest matchers can be used to make assertions on Clarity values.
It makes it to check the return values of contracts, ensure that the value is actually a Clarity value, and providing nice error messages.

### Check clarity type

#### `toHaveClarityType(expectedType: ClarityType)` <!-- omit from toc -->

This matcher can be used to make sure that the value has the right Clarity Type, without checking it's value.

```ts
import { ClarityType } from "@stacks/transactions";
import { expect, it } from "vitest";

const address1 = vm.getAccounts().get("wallet_1");

it("ensures <increment> adds 1", () => {
  const { result } = vm.callPublicFn("counter", "increment", [], address1);

  // make sure it returns a response ok `(ok ...)`
  expect(result).toHaveClarityType(ClarityType.ResponseOk);
});
```

It can be used to check any type
```ts
// uint
expect(result).toHaveClarityType(ClarityType.UInt);

// or tuple
expect(result).toHaveClarityType(ClarityType.Tuple);

// and so one
```

### Response type

The response type is noted `(response <ok-type> <error-type>)` in Clarity.
It can be `(ok <ok-type>)` or `(err <error-type>)`.
They are called composite types, meaning that they contain an other Clarity value.

#### `toBeOk(expected: ClarityValue)` <!-- omit from toc -->

Check that a response is `ok` and has the expected value. Any Clarity value can be passed.

```ts
const decrement = vm.callPublicFn("counter", "decrement", [Cl.uint(1)], address1);

// decrement.result is `(ok (uint 0))`
expect(decrement.result).toBeOk(Cl.uint(0));
```

#### `toBeErr(expected: ClarityValue)` <!-- omit from toc -->

Check that a response is `err` and has the expected value. Any Clarity value can be passed.

Let's consider that are `counter` contract returns and error code 500 `(err u500)` if the value passed to increment is too big;

```ts
const tooBig = 100000;
const increment = vm.callPublicFn("counter", "increment", [Cl.uint(toBig)], address1);

// increment.result is `(err u500)`
expect(increment.result).toBeErr(Cl.uint(500));
```

### Optional type

The option type is noted `(optional <some-type>)` in Clarity.
It can be `(some <some-type>)` or `none`.
Here, `some` is a composite type, meaning that it contains an other Clarity value.

#### `toBeSome(expected: ClarityValue)` <!-- omit from toc -->

Consider a billboard smart contract that can contain an optional message:

```ts
const getMessage = vm.callPublicFn("billboard", "get-message", [], address1);

// (some u"Hello world")
expect(getMessage.result).toBeSome(Cl.stringUtf8("Hello world"));
```

#### `toBeNone()` <!-- omit from toc -->

Considering the same billboard smart contract but with no saved message:

```ts
const getMessage = vm.callPublicFn("billboard", "get-message", [], address1);

// none
expect(getMessage.result).toBeNone();
```

### Simple clarity types

@todo

#### `toBeBool(expected: boolean)` <!-- omit from toc -->

@todo

#### `toBeInt(rexpected: number | bigint)` <!-- omit from toc -->

@todo

#### `toBeUint(expected: number | bigint)` <!-- omit from toc -->

@todo

#### `toBeAscii(expected: string)` <!-- omit from toc -->

@todo

#### `toBeUtf8(expected: string)` <!-- omit from toc -->

@todo

#### `toBePrincipal(expected: string)` <!-- omit from toc -->

@todo

#### `toBeBuff(expected: Uint8Array)` <!-- omit from toc -->

@todo


### Other composite types

`list` and `tuple` are composite types, like `ok`, `err`, and `some`. Meanning that they contain another Clarity value.

#### `toBeList(expected: ClarityValue[])` <!-- omit from toc -->

@todo

#### `toBeTuple(expected: Record<string, ClarityValue>)` <!-- omit from toc -->

@todo


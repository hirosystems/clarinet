# Clarinet SDK

## Core testing lib

This a very early preview.
Expect many breaking changes in the API.

```js
// @ts-check

import { main } from "obscurity-sdk";
import { before, describe, it } from "node:test";
import assert from "node:assert/strict";
import { Cl } from "@stacks/transactions";

describe("test counter", () => {
  const contract = "counter";
  const cost = 1000000n;
  let deployer;
  let contract_addr;
  let sender;
  /** @type import("obscurity-sdk").Session */
  let session;

  before(async () => {
    session = await main();
    await session.initSession(process.cwd(), "./Clarinet.toml");

    const accounts = session.getAccounts();
    deployer = accounts.get("deployer");
    contract_addr = `${deployer}.${contract}`;
    sender = accounts.get("wallet_1");
  });

  it("gets counter value", () => {
    const res = session.callReadOnlyFn(contract, "get-counter", [], sender);

    assert.deepEqual(res.result, Cl.int(0));
    assert.equal(session.blockHeight, 1);
  });

  it("increments counter value", () => {
    const stxBefore = session.getAssetsMap().get("STX");

    const res = session.callPublicFn(contract, "increment", [], sender);

    assert.equal(res.events.length, 2);
    const printEvent = res.events[0];
    assert.equal(printEvent.event, "print_event");
    const stxTransferEvent = res.events[1];
    assert.equal(stxTransferEvent.event, "stx_transfer_event");
    assert.equal(stxTransferEvent.data.amount, cost.toString());

    assert.deepEqual(res.result, Cl.ok(Cl.bool(true)));
    assert.equal(session.blockHeight, 2);

    let counter = session.callReadOnlyFn(contract, "get-counter", [], sender);
    assert.deepEqual(counter.result, Cl.int(1));

    const stxAfter = session.getAssetsMap().get("STX");
    // @ts-ignore
    assert.equal(stxAfter?.get(sender), stxBefore?.get(sender) - cost);
    // @ts-ignore
    assert.equal(stxAfter?.get(contract_addr), cost);
  });

  it("add any value", () => {
    const addRes = session.callPublicFn(contract, "add", [Cl.int(10)], sender);
    assert.deepEqual(addRes.result, Cl.ok(Cl.bool(true)));
    assert.equal(session.blockHeight, 3);

    let res = session.callReadOnlyFn(contract, "get-counter", [], sender);
    assert.deepEqual(res.result, Cl.int(11));
    assert.equal(session.blockHeight, 3);
  });
});
```

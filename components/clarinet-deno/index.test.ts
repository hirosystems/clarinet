import "./index.ts";

import {
  assertArrayIncludes,
  assertEquals,
  assertObjectMatch,
  assertStrictEquals,
  assertThrows,
} from "https://deno.land/std@0.160.0/testing/asserts.ts";

Deno.test("expectOk", () => {
  assertStrictEquals("(ok true)".expectOk(), "true");

  assertThrows(() => "(err u1)".expectOk());
});

Deno.test("expectErr", () => {
  assertStrictEquals("(err u1)".expectErr(), "u1");

  assertThrows(() => "(ok u1)".expectErr());
});

Deno.test("expectSome", () => {
  assertStrictEquals("(some true)".expectSome(), "true");

  assertThrows(() => "none".expectSome());
});

Deno.test("expectNone", () => {
  assertStrictEquals("none".expectNone(), "");

  assertThrows(() => "(some true)".expectNone());
});

Deno.test("expectBool", () => {
  assertStrictEquals("true".expectBool(true), true);
  assertStrictEquals("false".expectBool(false), false);

  assertThrows(() => "true".expectBool(false));
  assertThrows(() => "false".expectBool(true));
});

Deno.test("expectAscii", () => {
  const expect = "hello world";
  assertStrictEquals('"hello world"'.expectAscii(expect), expect);

  // invalid format
  assertThrows(() => "hello world".expectAscii(expect));
  // not equal
  assertThrows(() => '"olleh world"'.expectAscii(expect));
});

Deno.test("expectUtf8", () => {
  const res = 'u"hello world"'.expectUtf8("hello world");
  assertStrictEquals(res, "hello world");
});

Deno.test("expectInt", () => {
  assertStrictEquals("42".expectInt(42), 42n);

  assertThrows(() => "u42".expectInt(42));
});

Deno.test("expectUint", () => {
  assertStrictEquals("u42".expectUint(42), 42n);

  assertThrows(() => "42".expectUint(42));
});

Deno.test("expectPrincicipal", () => {
  const contract = "'.bns";
  assertStrictEquals(contract.expectPrincipal(contract), contract);
  const address = "'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM";
  assertStrictEquals(address.expectPrincipal(address), address);
  const both = `${address}.bns`;
  assertStrictEquals(both.expectPrincipal(both), both);

  const badFormat = ".bns";
  assertThrows(() => badFormat.expectPrincipal("'.bns"));
});

Deno.test("expectBuff", () => {
  const expect = Uint8Array.from([98, 116, 99]);
  assertEquals("0x627463".expectBuff(expect), expect);
});

Deno.test("expectList", () => {
  assertArrayIncludes("[u1, u2, u3]".expectList(), ["u1", "u2", "u3"]);
});

Deno.test("expectTuple", () => {
  assertObjectMatch("{id: u0}".expectTuple(), { id: "u0" });
});

Deno.test("expectPrintEvent", () => {
  const id = "ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.color-vote";
  const events = [
    {
      type: "contract_event",
      contract_event: {
        contract_identifier: id,
        topic: "print",
        value: '"ok"',
      },
    },
  ];

  assertObjectMatch(events.expectPrintEvent(id, '"ok"'), {
    contract_identifier: id,
    topic: "print",
    value: '"ok"',
  });
});

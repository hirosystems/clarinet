import { types } from "./index.ts";

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

Deno.test("types.ok", () => {
  assertStrictEquals(types.ok("2"), "(ok 2)");
  types.ok("2").expectOk().expectInt(2);
});

Deno.test("types.err", () => {
  assertStrictEquals(types.err("u2"), "(err u2)");
  types.err("u2").expectErr().expectUint(2);
});

Deno.test("types.some", () => {
  assertStrictEquals(types.some("true"), "(some true)");
  types.some("true").expectSome().expectBool(true);
});

Deno.test("types.none", () => {
  assertStrictEquals(types.none(), "none");
  types.none().expectNone();
});

Deno.test("types.bool", () => {
  assertStrictEquals(types.bool(true), "true");
  assertStrictEquals(types.bool(false), "false");
  types.bool(true).expectBool(true);
  types.bool(false).expectBool(false);
});

Deno.test("types.ascii", () => {
  assertStrictEquals(types.ascii("hello"), '"hello"');
  types.ascii("hello").expectAscii("hello");
});

Deno.test("types.utf8", () => {
  assertStrictEquals(types.utf8("hello"), 'u"hello"');
  types.utf8("hello").expectUtf8("hello");
});

Deno.test("types.int", () => {
  assertStrictEquals(types.int(2), "2");
  assertStrictEquals(types.int(2n), "2");
  types.int(2).expectInt(2);
});

Deno.test("types.uint", () => {
  assertStrictEquals(types.uint(2), "u2");
  assertStrictEquals(types.uint(2n), "u2");
  types.uint(2).expectUint(2);
});

Deno.test("types.principal", () => {
  const addr = "ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5";
  assertStrictEquals(types.principal(addr), `'${addr}`);
  types.principal(addr).expectPrincipal(`'${addr}`);

  const contract = `${addr}.counter`;
  assertStrictEquals(types.principal(contract), `'${contract}`);
  types.principal(contract).expectPrincipal(`'${contract}`);
});

Deno.test("types.buff", () => {
  assertStrictEquals(types.buff("stx"), "0x737478");
  assertStrictEquals(types.buff(Int8Array.from([115, 116, 120])), "0x737478");
  types.buff("stx").expectBuff(Int8Array.from([115, 116, 120]));
});

Deno.test("types.list", () => {
  assertStrictEquals(types.list([1, 2, 3]), "(list 1 2 3)");
});

Deno.test("types.tuple", () => {
  assertStrictEquals(types.tuple({ id: 1 }), "{ id: 1 }");
  assertObjectMatch(types.tuple({ id: 1 }).expectTuple(), { id: "1" });
});

Deno.test("expectPrintEvent", () => {
  const events = [
    {
      type: "contract_event",
      contract_event: {
        contract_identifier:
          "ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.my-contract",
        topic: "print",
        value: '"hello"',
      },
    },
    {
      type: "stx_transfer_event",
      stx_transfer_event: {
        sender: "ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5",
        recipient: "ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM",
        amount: "1000",
        memo: "",
      },
    },
  ];

  events.expectPrintEvent(
    "ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.my-contract",
    '"hello"'
  );
});

Deno.test("expectSTXTransferEvent", () => {
  const events = [
    {
      type: "contract_event",
      contract_event: {
        contract_identifier:
          "ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.my-contract",
        topic: "print",
        value: '"hello"',
      },
    },
    {
      type: "stx_transfer_event",
      stx_transfer_event: {
        sender: "ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5",
        recipient: "ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM",
        amount: "1000",
        memo: "",
      },
    },
  ];

  events.expectSTXTransferEvent(
    1000,
    "ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5",
    "ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM"
  );
});

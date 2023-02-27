import "./index.ts";
import * as types from "./clarityTypes.ts";

import { assertStrictEquals, assertObjectMatch } from "./deps.test.ts";

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
  assertStrictEquals(types.buff(Uint8Array.from([98, 116, 99])), "0x627463");
  types
    .buff(Uint8Array.from([98, 116, 99]))
    .expectBuff(Uint8Array.from([98, 116, 99]));
});

Deno.test("types.buff (deprecated)", () => {
  assertStrictEquals(types.buff(Int8Array.from([115, 116, 120])), "0x737478");
});

Deno.test("types.list", () => {
  assertStrictEquals(types.list([1, 2, 3]), "(list 1 2 3)");
});

Deno.test("types.tuple", () => {
  assertStrictEquals(types.tuple({ id: 1 }), "{ id: 1 }");
  assertObjectMatch(types.tuple({ id: 1 }).expectTuple(), { id: "1" });
});

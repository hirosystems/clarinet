import { describe, expect, it } from "vitest";
import { formatCV } from "../src/formatCV";
import { Cl } from "@stacks/transactions";

describe("test format of Stacks.js clarity values into clarity style strings", () => {
  it("formats basic types", () => {
    expect(formatCV(Cl.bool(true))).toStrictEqual("true");
    expect(formatCV(Cl.bool(false))).toStrictEqual("false");
    expect(formatCV(Cl.none())).toStrictEqual("none");

    expect(formatCV(Cl.int(1))).toStrictEqual("1");
    expect(formatCV(Cl.int(10n))).toStrictEqual("10");

    expect(formatCV(Cl.stringAscii("hello world!"))).toStrictEqual('"hello world!"');
    expect(formatCV(Cl.stringUtf8("hello world!"))).toStrictEqual('u"hello world!"');
  });

  it("formats principal", () => {
    const addr = "ST2CY5V39NHDPWSXMW9QDT3HC3GD6Q6XX4CFRK9AG";

    expect(formatCV(Cl.standardPrincipal(addr))).toStrictEqual(
      "'ST2CY5V39NHDPWSXMW9QDT3HC3GD6Q6XX4CFRK9AG"
    );
    expect(formatCV(Cl.contractPrincipal(addr, "contract"))).toStrictEqual(
      "'ST2CY5V39NHDPWSXMW9QDT3HC3GD6Q6XX4CFRK9AG.contract"
    );
  });

  it("formats optional some", () => {
    expect(formatCV(Cl.some(Cl.uint(1)))).toStrictEqual("(some u1)");
    expect(formatCV(Cl.some(Cl.stringAscii("btc")))).toStrictEqual('(some "btc")');
    expect(formatCV(Cl.some(Cl.stringUtf8("stx 🚀")))).toStrictEqual('(some u"stx 🚀")');
  });

  it("formats reponse", () => {
    expect(formatCV(Cl.ok(Cl.uint(1)))).toStrictEqual("(ok u1)");
    expect(formatCV(Cl.error(Cl.uint(1)))).toStrictEqual("(err u1)");
    expect(formatCV(Cl.ok(Cl.some(Cl.uint(1))))).toStrictEqual("(ok (some u1))");
    expect(formatCV(Cl.ok(Cl.none()))).toStrictEqual("(ok none)");
  });

  it("formats buffer", () => {
    expect(formatCV(Cl.buffer(Uint8Array.from([98, 116, 99])))).toStrictEqual("0x627463");
    expect(formatCV(Cl.bufferFromAscii("stx"))).toStrictEqual("0x737478");
  });

  it("formats lists", () => {
    expect(formatCV(Cl.list([1, 2, 3].map(Cl.int)))).toStrictEqual("(list 1 2 3)");
    expect(formatCV(Cl.list([1, 2, 3].map(Cl.uint)))).toStrictEqual("(list u1 u2 u3)");
    expect(formatCV(Cl.list(["a", "b", "c"].map(Cl.stringUtf8)))).toStrictEqual(
      '(list u"a" u"b" u"c")'
    );

    expect(formatCV(Cl.list([]))).toStrictEqual("(list)");
  });

  it("can prettify lists on multiple lines", () => {
    const list = Cl.list([1, 2, 3].map(Cl.int));
    expect(formatCV(list)).toStrictEqual("(list 1 2 3)");
    expect(formatCV(list, 2)).toStrictEqual("(list\n  1\n  2\n  3\n)");

    expect(formatCV(Cl.list([]), 2)).toStrictEqual("(list)");
  });

  it("formats tuples", () => {
    expect(formatCV(Cl.tuple({ counter: Cl.uint(10) }))).toStrictEqual("{ counter: u10 }");
    expect(
      formatCV(Cl.tuple({ counter: Cl.uint(10), state: Cl.ok(Cl.stringUtf8("valid")) }))
    ).toStrictEqual('{ counter: u10, state: (ok u"valid") }');

    expect(formatCV(Cl.tuple({}))).toStrictEqual("{}");
  });

  it("can prettify tuples on multiple lines", () => {
    const tuple = Cl.tuple({ counter: Cl.uint(10) });

    expect(formatCV(tuple)).toStrictEqual("{ counter: u10 }");
    expect(formatCV(tuple, 2)).toStrictEqual("{\n  counter: u10\n}");

    expect(formatCV(Cl.tuple({}), 2)).toStrictEqual("{}");
  });

  it("prettifies nested list and tuples", () => {
    // test that the right indentation level is applied for nested composite types
    const addr = "ST2CY5V39NHDPWSXMW9QDT3HC3GD6Q6XX4CFRK9AG";
    const value = Cl.tuple({
      id: Cl.uint(1),
      messageAscii: Cl.stringAscii("hello world"),
      someMessageUtf8: Cl.some(Cl.stringUtf8("hello world")),
      items: Cl.some(
        Cl.list([
          Cl.ok(
            Cl.tuple({
              id: Cl.uint(1),
              owner: Cl.some(Cl.standardPrincipal(addr)),
              valid: Cl.ok(Cl.uint(2)),
              history: Cl.some(Cl.list([Cl.uint(1), Cl.uint(2)])),
            })
          ),
          Cl.ok(
            Cl.tuple({
              id: Cl.uint(2),
              owner: Cl.none(),
              valid: Cl.error(Cl.uint(1000)),
              history: Cl.none(),
            })
          ),
        ])
      ),
    });

    const expected = `{
  id: u1,
  messageAscii: "hello world",
  someMessageUtf8: (some u"hello world"),
  items: (some (list
    (ok {
      id: u1,
      owner: (some 'ST2CY5V39NHDPWSXMW9QDT3HC3GD6Q6XX4CFRK9AG),
      valid: (ok u2),
      history: (some (list
        u1
        u2
      ))
    })
    (ok {
      id: u2,
      owner: none,
      valid: (err u1000),
      history: none
    })
  ))
}`;

    const result = formatCV(value, 2);
    expect(result).toStrictEqual(expected);
  });
});

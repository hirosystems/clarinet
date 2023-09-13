import { describe, it, expect } from "vitest";
import { Cl, ClarityType } from "@stacks/transactions";

import "../src/clarityValuesMatchers";

describe("test clarity values assertions", () => {
  it("tests any CV type", () => {
    // using a native matcher
    expect(Cl.ok(Cl.int(1))).toHaveProperty("type", ClarityType.ResponseOk);
    // custom matchers
    expect(Cl.ok(Cl.int(1))).toHaveClarityType(ClarityType.ResponseOk);
    expect(Cl.error(Cl.int(1))).toHaveClarityType(ClarityType.ResponseErr);
    expect(Cl.some(Cl.int(1))).toHaveClarityType(ClarityType.OptionalSome);
    expect(Cl.none()).toHaveClarityType(ClarityType.OptionalNone);

    expect(Cl.int(1)).toHaveClarityType(ClarityType.Int);
    expect(Cl.uint(1n)).toHaveClarityType(ClarityType.UInt);
    expect(Cl.stringAscii("hello")).toHaveClarityType(ClarityType.StringASCII);
    expect(Cl.stringUtf8("hello")).toHaveClarityType(ClarityType.StringUTF8);

    try {
      expect(1).toHaveClarityType(ClarityType.ResponseOk);
    } catch (e: any) {
      expect(e.message).toBe('actual value must be a Clarity "ResponseOk", received "number"');
    }
    try {
      expect(Cl.uint(1)).toHaveClarityType(ClarityType.ResponseOk);
    } catch (e: any) {
      expect(e.message).toBe('actual value must be a Clarity "ResponseOk", received "UInt"');
    }

    try {
      expect(Cl.ok(Cl.uint(1))).not.toHaveClarityType(ClarityType.ResponseOk);
    } catch (e: any) {
      expect(e.message).toBe('actual value must not be a Clarity "ResponseOk"');
    }
  });

  it("tests ok", () => {
    const okRes = Cl.ok(Cl.uint(1));
    expect(okRes).toEqual(Cl.ok(Cl.uint(1)));
    expect(Cl.ok(Cl.uint(1))).toBeOk(Cl.uint(1));

    expect(Cl.ok(Cl.uint(1))).toBeOk(expect.toBeUint(1));
    expect(Cl.ok(Cl.uint(1))).toBeOk(expect.not.toBeUint(2));
    expect(Cl.error(Cl.uint(1))).not.toBeOk(expect.toBeUint(1));

    const nestedOks = Cl.ok(Cl.ok(Cl.uint(1)));
    expect(nestedOks).toBeOk(expect.toBeOk(expect.toBeUint(1)));

    expect(() => expect(Cl.uint(1)).toBeOk(Cl.uint(1))).toThrow(
      'actual value must be a Clarity "ResponseOk", received "UInt"'
    );

    expect(() => expect(Cl.ok(Cl.uint(1))).toBeOk(Cl.uint(2))).toThrow(
      'expected {"type":1,"value":"1"} to be {"type":1,"value":"2"}'
    );
  });

  it("tests error", () => {
    expect(Cl.error(Cl.uint(1))).toBeErr(expect.toBeUint(1));
  });

  it("tests none", () => {
    expect(Cl.none()).toBeNone();
    expect(Cl.uint(1)).not.toBeNone();

    try {
      expect(Cl.uint(1)).toBeNone();
    } catch (e: any) {
      expect(e.message).toBe('actual value must be a Clarity "OptionalNone", received "UInt"');
    }
  });

  it("tests some", () => {
    expect(Cl.some(Cl.uint(1))).toBeSome(expect.toBeUint(1));
    expect(Cl.some(Cl.uint(1))).toBeSome(Cl.uint(1));

    expect(() => expect(Cl.some(Cl.uint(1))).toBeSome(Cl.uint(2))).toThrow(
      'expected {"type":1,"value":"1"} to be {"type":1,"value":"2"}'
    );
  });

  it("tests bool", () => {
    expect(Cl.bool(true)).toBeBool(true);
    expect(Cl.bool(false)).toBeBool(false);

    try {
      expect(Cl.uint(1)).toBeBool(false);
    } catch (e: any) {
      expect(e.message).toBe('actual value must be a Clarity "BoolFalse", received "UInt"');
    }

    try {
      expect(false).toBeBool(false);
    } catch (e: any) {
      expect(e.message).toBe('actual value must be a Clarity "BoolFalse", received "boolean"');
    }
  });

  it("tests int", () => {
    expect(Cl.int(1)).toBeInt(1);
    expect(Cl.int(1)).toBeInt(1n);

    try {
      expect(Cl.uint(1)).toBeInt(1);
    } catch (e: any) {
      expect(e.message).toBe('actual value must be a Clarity "Int", received "UInt"');
    }

    try {
      expect(Cl.int(2)).toBeInt(1);
    } catch (e: any) {
      expect(e.message).toBe("expected 2 to be 1");
      expect(e.actual).toBe(2n);
      expect(e.expected).toBe(1n);
    }
  });

  it("tests uint", () => {
    expect(Cl.uint(1)).toBeUint(1);
    expect(Cl.uint(1)).toBeUint(1n);

    try {
      expect(Cl.int(1)).toBeUint(1);
    } catch (e: any) {
      expect(e.message).toBe('actual value must be a Clarity "UInt", received "Int"');
    }

    try {
      expect(Cl.uint(2)).toBeUint(1);
    } catch (e: any) {
      expect(e.message).toBe("expected 2 to be 1");
      expect(e.actual).toBe(2n);
      expect(e.expected).toBe(1n);
    }
  });

  it("tests string-ascii", () => {
    expect(Cl.stringAscii("hello world")).toBeAscii("hello world");

    try {
      expect(Cl.int(1)).toBeAscii("hello world");
    } catch (e: any) {
      expect(e.message).toBe('actual value must be a Clarity "StringASCII", received "Int"');
    }

    try {
      expect(Cl.stringAscii("hello")).toBeAscii("hello world");
    } catch (e: any) {
      expect(e.message).toBe("expected hello to be hello world");
      expect(e.actual).toBe("hello");
      expect(e.expected).toBe("hello world");
    }
  });

  it("tests string-utf8", () => {
    expect(Cl.stringUtf8("hello world")).toBeUtf8("hello world");

    try {
      expect(Cl.int(1)).toBeUtf8("hello world");
    } catch (e: any) {
      expect(e.message).toBe('actual value must be a Clarity "StringUTF8", received "Int"');
    }

    try {
      expect(Cl.stringUtf8("hello")).toBeUtf8("hello world");
    } catch (e: any) {
      expect(e.message).toBe("expected hello to be hello world");
    }
  });

  it("tests principal", () => {
    const addr = "ST2CY5V39NHDPWSXMW9QDT3HC3GD6Q6XX4CFRK9AG";
    expect(Cl.standardPrincipal(addr)).toBePrincipal(addr);

    const addr2 = "STNHKEPYEPJ8ET55ZZ0M5A34J0R3N5FM2CMMMAZ6";
    expect(() => expect(Cl.standardPrincipal(addr)).toBePrincipal(addr2)).toThrow(
      `expected ${addr} to be ${addr2}`
    );

    expect(() => expect(Cl.standardPrincipal(addr)).toBePrincipal("INVALID")).toThrow(
      "expected INVALID to be a principal"
    );

    const contractAddress = "ST2CY5V39NHDPWSXMW9QDT3HC3GD6Q6XX4CFRK9AG.contract";
    expect(
      Cl.contractPrincipal("ST2CY5V39NHDPWSXMW9QDT3HC3GD6Q6XX4CFRK9AG", "contract")
    ).toBePrincipal(contractAddress);
  });

  it("tests buffer", () => {
    const val = [98, 116, 99];
    expect(Cl.buffer(Uint8Array.from(val))).toBeBuff(Uint8Array.from(val));
    expect(Cl.buffer(Uint8Array.from(val))).toStrictEqual(Cl.bufferFromUtf8("btc"));

    expect(() => expect(Cl.int(1)).toBeBuff(Uint8Array.from(val))).toThrow(
      'actual value must be a Clarity "Buffer", received "Int"'
    );

    expect(() =>
      expect(Cl.buffer(Uint8Array.from([99, 117, 100]))).toBeBuff(Uint8Array.from(val))
    ).toThrow("the received Buffer does not match the expected one");

    expect(() =>
      expect(Cl.buffer(Uint8Array.from(val))).not.toBeBuff(Uint8Array.from(val))
    ).toThrow("the received Buffer does match the expected one");
  });

  it("test list", () => {
    expect(Cl.list([Cl.uint(1), Cl.uint(2)])).toBeList([Cl.uint(1), Cl.uint(2)]);
    expect(Cl.list([Cl.uint(1), Cl.uint(2)])).toBeList([expect.toBeUint(1), expect.toBeUint(2)]);

    expect(() =>
      expect(Cl.list([Cl.uint(1), Cl.uint(2)])).toBeList([Cl.uint(1), Cl.uint(3)])
    ).toThrow("the received List does not match the expected one");

    expect(() =>
      expect(Cl.list([Cl.uint(1), Cl.uint(2)])).not.toBeList([Cl.uint(1), Cl.uint(2)])
    ).toThrow("the received List does match the expected one");
  });

  it("tests tuple", () => {
    const addr = "ST2CY5V39NHDPWSXMW9QDT3HC3GD6Q6XX4CFRK9AG";
    const tuple = Cl.tuple({
      count: Cl.int(1),
      owner: Cl.standardPrincipal(addr),
    });

    expect(tuple).toHaveClarityType(ClarityType.Tuple);
    expect(tuple).toBeTuple({
      count: Cl.int(1),
      owner: Cl.standardPrincipal(addr),
    });

    expect(tuple).toBeTuple({
      count: expect.toBeInt(1),
      owner: expect.toBePrincipal(addr),
    });

    expect(() =>
      expect(tuple).toBeTuple({
        count: Cl.int(2),
        owner: Cl.standardPrincipal(addr),
      })
    ).toThrow("the received Tuple does not match the expected one");

    expect(() =>
      expect(tuple).not.toBeTuple({
        count: Cl.int(1),
        owner: Cl.standardPrincipal(addr),
      })
    ).toThrow("the received Tuple does match the expected one");
  });

  it("test tuple with complex types", () => {
    const addr = "ST2CY5V39NHDPWSXMW9QDT3HC3GD6Q6XX4CFRK9AG";
    const complexTuple = Cl.tuple({
      id: Cl.uint(1),
      items: Cl.list([
        Cl.tuple({
          id: Cl.uint(1),
          owner: Cl.some(Cl.standardPrincipal(addr)),
          valid: Cl.ok(Cl.uint(2)),
        }),
        Cl.tuple({
          id: Cl.uint(2),
          owner: Cl.none(),
          valid: Cl.error(Cl.uint(1000)),
        }),
      ]),
    });

    // show multiple ways of testing

    // toEqual
    expect(complexTuple).toEqual(complexTuple);

    // toBeTuple with a Cl.tuple
    expect(complexTuple).toBeTuple(complexTuple.data);

    // toBeTuple asymmetric matched
    expect(complexTuple).toBeTuple({
      id: expect.toBeUint(1),
      items: expect.toBeList([
        expect.toBeTuple({
          id: expect.toBeUint(1),
          owner: expect.toBeSome(expect.toBePrincipal(addr)),
          // test for ok and value
          valid: expect.toBeOk(expect.toBeUint(2)),
        }),
        expect.toBeTuple({
          id: expect.toBeUint(2),
          owner: expect.toBeNone(),
          // test for error but not the value within
          valid: expect.toHaveClarityType(ClarityType.ResponseErr),
        }),
      ]),
    });
  });
});

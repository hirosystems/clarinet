import { expect, ExpectStatic, assert } from "vitest";
import {
  Cl,
  ClarityValue,
  ClarityType,
  ResponseOkCV,
  NoneCV,
  SomeCV,
  ResponseErrorCV,
  IntCV,
  UIntCV,
  StringAsciiCV,
  StringUtf8CV,
  ContractPrincipalCV,
  StandardPrincipalCV,
  ListCV,
  TupleCV,
  BufferCV,
  principalToString,
  TrueCV,
  FalseCV,
  BooleanCV,
} from "@stacks/transactions";

import { MatcherState } from "@vitest/expect";

// https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/BigInt#use_within_json
// @ts-ignore
BigInt.prototype.toJSON = function () {
  return this.toString();
};

function notStr(isNot: boolean) {
  return isNot ? "not " : "";
}

function formatMessage(this: any, received: string, expected: string) {
  return `expected ${received} ${notStr(this.isNot)}to be ${expected}`;
}

export class ClarityTypeError extends Error {
  actual?: any;
  expected?: any;

  constructor({ message, actual, expected }: { message: string; actual?: any; expected?: any }) {
    super(message);
    this.actual = actual;
    this.expected = expected;

    Object.setPrototypeOf(this, ClarityTypeError.prototype);
  }
}

type ClarityTypetoValue = {
  [ClarityType.OptionalNone]: NoneCV;
  [ClarityType.OptionalSome]: SomeCV;
  [ClarityType.ResponseOk]: ResponseOkCV;
  [ClarityType.ResponseErr]: ResponseErrorCV;
  [ClarityType.BoolTrue]: TrueCV;
  [ClarityType.BoolFalse]: FalseCV;
  [ClarityType.Int]: IntCV;
  [ClarityType.UInt]: UIntCV;
  [ClarityType.StringASCII]: StringAsciiCV;
  [ClarityType.StringUTF8]: StringUtf8CV;
  [ClarityType.PrincipalStandard]: StandardPrincipalCV;
  [ClarityType.PrincipalContract]: ContractPrincipalCV;
  [ClarityType.List]: ListCV;
  [ClarityType.Tuple]: TupleCV;
  [ClarityType.Buffer]: BufferCV;
};

// the "simple clarity values" are CVs that can't be nested and have `value` property
type SimpleCV = BooleanCV | IntCV | UIntCV | StringAsciiCV | StringUtf8CV;
type SimpleCVTypes =
  | ClarityType.BoolFalse
  | ClarityType.BoolTrue
  | ClarityType.Int
  | ClarityType.UInt
  | ClarityType.StringASCII
  | ClarityType.StringUTF8;

const validClarityTypes = Object.values(ClarityType).filter((t) => typeof t === "number");

function isClarityValue(input: unknown): input is ClarityValue {
  if (!input || typeof input !== "object") return false;
  if (!("type" in input) || typeof input.type !== "number") return false;
  if (!validClarityTypes.includes(input.type)) return false;

  return true;
}

function isClarityValueWithType<T extends ClarityType>(
  input: unknown,
  withType: T
): input is ClarityTypetoValue[T] {
  if (!isClarityValue(input)) return false;
  if (input.type !== withType) return false;

  return true;
}

function checkCVType<T extends ClarityType>(
  actual: unknown,
  expectedType: T,
  isNot: boolean
): actual is ClarityTypetoValue[T] {
  const isCV = isClarityValue(actual);

  if (!isCV) {
    throw new ClarityTypeError({
      message: `actual value must ${notStr(isNot)}be a Clarity "${
        ClarityType[expectedType]
      }", received "${typeof actual}"`,
    });
  }

  const isCVWithType = isClarityValueWithType(actual, expectedType);

  if (!isCVWithType) {
    throw new ClarityTypeError({
      message: `actual value must ${notStr(isNot)}be a Clarity "${
        ClarityType[expectedType]
      }", received "${ClarityType[actual.type]}"`,
      actual: ClarityType[actual.type],
      expected: ClarityType[expectedType],
    });
  }

  return true;
}

function errorToAssertionResult(this: MatcherState, err: any) {
  return {
    pass: false,
    message: () => err.message,
    actual: err.actual,
    expected: err.expected,
  };
}

function getSimpleCVValue(cv: SimpleCV) {
  // int | uint
  if ("value" in cv) return cv.value;
  // stringAscii | stringUtf8
  if ("data" in cv) return cv.data;
  // bool
  return cv.type === ClarityType.BoolTrue;
}

function simpleAssertion(
  this: MatcherState,
  cvType: SimpleCVTypes,
  actual: unknown,
  expected: SimpleCV
) {
  try {
    const isCV = checkCVType(actual, cvType, this.isNot);
    assert(isCV);
  } catch (e: any) {
    return errorToAssertionResult.call(this, e);
  }

  const actualValue = getSimpleCVValue(actual);
  const expectedValue = getSimpleCVValue(expected);

  return {
    pass: this.equals(actual, expected, undefined, true),
    message: () => `expected ${actualValue} ${notStr(this.isNot)}to be ${expectedValue}`,
    actual: actualValue,
    expected: expectedValue,
  };
}

expect.extend({
  toHaveClarityType(actual: unknown, expectedType: ClarityType) {
    try {
      const isCV = checkCVType(actual, expectedType, this.isNot);
      assert(isCV);
    } catch (e: any) {
      return errorToAssertionResult.call(this, e);
    }

    return {
      pass: true,
      message: () =>
        `actual value must ${notStr(this.isNot)}be a Clarity "${ClarityType[expectedType]}"`,
    };
  },

  toBeBool(actual: unknown, expected: boolean) {
    const expectedType = expected ? ClarityType.BoolTrue : ClarityType.BoolFalse;
    return simpleAssertion.call(this, expectedType, actual, Cl.bool(expected));
  },

  toBeInt(actual: unknown, expected: number | bigint) {
    return simpleAssertion.call(this, ClarityType.Int, actual, Cl.int(expected));
  },

  toBeUint(actual: unknown, expected: number | bigint) {
    return simpleAssertion.call(this, ClarityType.UInt, actual, Cl.uint(expected));
  },

  toBeAscii(actual: unknown, expected: string) {
    return simpleAssertion.call(this, ClarityType.StringASCII, actual, Cl.stringAscii(expected));
  },

  toBeUtf8(actual: unknown, expected: string) {
    return simpleAssertion.call(this, ClarityType.StringUTF8, actual, Cl.stringUtf8(expected));
  },

  toBeOk(actual: unknown, expected: ExpectStatic | ClarityValue) {
    const expectedType = ClarityType.ResponseOk;
    try {
      const isCV = checkCVType(actual, expectedType, this.isNot);
      assert(isCV);
    } catch (e: any) {
      return errorToAssertionResult.call(this, e);
    }

    return {
      pass: this.equals(actual.value, expected, undefined, true),
      message: () =>
        formatMessage.call(this, JSON.stringify(actual.value), JSON.stringify(expected)),
      actual: actual.value,
      expected,
    };
  },

  toBeErr(actual: unknown, expected: ExpectStatic | ClarityValue) {
    const expectedType = ClarityType.ResponseErr;
    try {
      const isCV = checkCVType(actual, expectedType, this.isNot);
      assert(isCV);
    } catch (e: any) {
      return errorToAssertionResult.call(this, e);
    }

    return {
      pass: this.equals(actual.value, expected, undefined, true),
      message: () =>
        formatMessage.call(this, JSON.stringify(actual.value), JSON.stringify(expected)),
      actual: actual.value,
      expected,
    };
  },

  toBeSome(actual: unknown, expected: ExpectStatic | ClarityValue) {
    const expectedType = ClarityType.OptionalSome;
    try {
      const isCV = checkCVType(actual, expectedType, this.isNot);
      assert(isCV);
    } catch (e: any) {
      return errorToAssertionResult.call(this, e);
    }

    return {
      pass: this.equals(actual.value, expected, undefined, true),
      message: () =>
        formatMessage.call(
          this,
          JSON.stringify(actual.value),
          isClarityValue(expected) ? JSON.stringify(expected) : expected.toString()
        ),
      actual: actual.value,
      expected,
    };
  },

  toBeNone(actual: unknown) {
    const expectedType = ClarityType.OptionalNone;
    try {
      const isCV = checkCVType(actual, expectedType, this.isNot);
      assert(isCV);
    } catch (e: any) {
      return errorToAssertionResult.call(this, e);
    }

    const expected = Cl.none();
    return {
      pass: this.equals(actual, expected, undefined, true),
      message: () => formatMessage.call(this, "None", "None"),
      actual,
      expected,
    };
  },

  toBePrincipal(actual: unknown, expectedString: string) {
    const isStandard = !expectedString.includes(".");
    let expected: StandardPrincipalCV | ContractPrincipalCV;

    const expectedType = isStandard ? ClarityType.PrincipalStandard : ClarityType.PrincipalContract;
    try {
      const isCV = checkCVType(actual, expectedType, this.isNot);
      assert(isCV);
    } catch (e: any) {
      return errorToAssertionResult.call(this, e);
    }

    const actualString = principalToString(actual);

    try {
      expected = isStandard
        ? Cl.standardPrincipal(expectedString)
        : Cl.contractPrincipal(...(expectedString.split(".") as [string, string]));
    } catch {
      return {
        pass: false,
        message: () => `expected ${expectedString} ${notStr(this.isNot)}to be a principal`,
        actual: actualString,
        expected: expectedString,
      };
    }

    return {
      pass: this.equals(actual, expected, undefined, true),
      message: () => formatMessage.call(this, actualString, expectedString),
      actual: actualString,
      expected: expectedString,
    };
  },

  toBeBuff(actual: unknown, expectedRaw: Uint8Array) {
    const expectedType = ClarityType.Buffer;
    try {
      const isCV = checkCVType(actual, expectedType, this.isNot);
      assert(isCV);
    } catch (e: any) {
      return errorToAssertionResult.call(this, e);
    }

    const expected = Cl.buffer(expectedRaw);
    return {
      pass: this.equals(actual, expected, undefined, true),
      // note: throw a simple message and rely on `actual` and `expected` to display the diff
      message: () => `the received Buffer does ${this.isNot ? "" : "not "}match the expected one`,
      actual,
      expected,
    };
  },

  toBeList(actual: unknown, expected: ExpectStatic[] | ClarityValue[]) {
    const expectedType = ClarityType.List;
    try {
      const isCV = checkCVType(actual, expectedType, this.isNot);
      assert(isCV);
    } catch (e: any) {
      return errorToAssertionResult.call(this, e);
    }
    return {
      pass: this.equals(actual.list, expected, undefined, true),
      // note: throw a simple message and rely on `actual` and `expected` to display the diff
      message: () => `the received List does ${this.isNot ? "" : "not "}match the expected one`,
      actual: actual.list,
      expected,
    };
  },

  toBeTuple(actual: unknown, expected: Record<string, ExpectStatic[] | ClarityValue[]>) {
    const expectedType = ClarityType.Tuple;
    try {
      const isCV = checkCVType(actual, expectedType, this.isNot);
      assert(isCV);
    } catch (e: any) {
      return errorToAssertionResult.call(this, e);
    }

    return {
      pass: this.equals(actual.data, expected, undefined, true),
      // note: throw a simple message and rely on `actual` and `expected` to display the diff
      message: () => `the received Tuple does ${this.isNot ? "" : "not "}match the expected one`,
      actual: actual.data,
      expected,
    };
  },
});

/*
  This file could be implement in stacks.js
  Format stacks.js Clarity Value into Clarity style readable string
  eg:
  `Cl.uint(1)` => u1
  `Cl.list(Cl.uint(1))` => (list u1)
*/

import {
  BufferCV,
  ClarityType,
  ClarityValue,
  ListCV,
  TupleCV,
  principalToString,
} from "@stacks/transactions";

let byteToHex: string[] = [];
for (let n = 0; n <= 0xff; ++n) {
  const hexOctet = n.toString(16).padStart(2, "0");
  byteToHex.push(hexOctet);
}

function formatSpace(space: number, depth: number, end = false) {
  if (!space) return " ";
  return `\n${" ".repeat(space * (depth - (end ? 1 : 0)))}`;
}

function formatBuffer(cv: BufferCV): string {
  const hex = Array.from(cv.buffer).map((n) => byteToHex[n]);
  return `0x${hex.join("")}`;
}

/**
 * formatList
 * @description format List clarity values in clarity style strings
 * with the ability to prettify the result with line break end space indentation
 * @exemple
 * ```
 * formatList(Cl.list([Cl.uint(1)]))
 * // (list u1)
 *
 * formatList(Cl.list([Cl.uint(1)]), 2)
 * // (list
 * //   u1
 * // )
 * ```
 */
function formatList(cv: ListCV, space: number, depth = 1): string {
  if (cv.list.length === 0) return "(list)";

  const spaceBefore = formatSpace(space, depth, false);
  const endSpace = space ? formatSpace(space, depth, true) : "";

  const items = cv.list.map((v) => formatCVPrivate(v, space, depth)).join(spaceBefore);

  return `(list${spaceBefore}${items}${endSpace})`;
}

/**
 * formatTuple
 * @description format Tuple clarity values in clarity style strings
 * with the ability to prettify the result with line break end space indentation
 * @exemple
 * ```
 * formatTuple(Cl.tuple({ id: Cl.uint(1) }))
 * // { id: u1 }
 *
 * formatTuple(Cl.tuple({ id: Cl.uint(1) }, 2))
 * // {
 * //   id: u1
 * // }
 * ```
 */
function formatTuple(cv: TupleCV, space: number, depth = 1): string {
  if (Object.keys(cv.data).length === 0) return "{}";

  const items: Array<string> = [];
  for (const [key, value] of Object.entries(cv.data)) {
    items.push(`${key}: ${formatCVPrivate(value, space, depth)}`);
  }

  const spaceBefore = formatSpace(space, depth, false);
  const endSpace = formatSpace(space, depth, true);

  return `{${spaceBefore}${items.join(`,${spaceBefore}`)}${endSpace}}`;
}

function exhaustiveCheck(param: never): never {
  throw new Error(`invalid clarity value type: ${param}`);
}

// we don't want the exported function to have a `depth` argument
function formatCVPrivate(cv: ClarityValue, space = 0, depth: number): string {
  if (cv.type === ClarityType.BoolFalse) return "false";
  if (cv.type === ClarityType.BoolTrue) return "true";

  if (cv.type === ClarityType.Int) return cv.value.toString();
  if (cv.type === ClarityType.UInt) return `u${cv.value.toString()}`;

  if (cv.type === ClarityType.StringASCII) return `"${cv.data}"`;
  if (cv.type === ClarityType.StringUTF8) return `u"${cv.data}"`;

  if (cv.type === ClarityType.PrincipalContract) return `'${principalToString(cv)}`;
  if (cv.type === ClarityType.PrincipalStandard) return `'${principalToString(cv)}`;

  if (cv.type === ClarityType.Buffer) return formatBuffer(cv);

  if (cv.type === ClarityType.OptionalNone) return "none";
  if (cv.type === ClarityType.OptionalSome)
    return `(some ${formatCVPrivate(cv.value, space, depth)})`;

  if (cv.type === ClarityType.ResponseOk) return `(ok ${formatCVPrivate(cv.value, space, depth)})`;
  if (cv.type === ClarityType.ResponseErr)
    return `(err ${formatCVPrivate(cv.value, space, depth)})`;

  if (cv.type === ClarityType.List) {
    return formatList(cv, space, depth + 1);
  }
  if (cv.type === ClarityType.Tuple) {
    return formatTuple(cv, space, depth + 1);
  }

  // make sure that we exhausted all ClarityTypes
  exhaustiveCheck(cv);
}

export function formatCV(cv: ClarityValue, space = 0): string {
  return formatCVPrivate(cv, space, 0);
}

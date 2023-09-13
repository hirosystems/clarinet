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

function formatBuffer(cv: BufferCV) {
  const hex = Array.from(cv.buffer).map((n) => byteToHex[n]);
  return `0x${hex.join("")}`;
}

function formatList(cv: ListCV) {
  return `(list ${cv.list.map((v) => formatCV(v)).join(" ")})`;
}

function formatTuple(cv: TupleCV) {
  const items: Array<string> = [];
  for (const [key, value] of Object.entries(cv.data)) {
    items.push(`${key}: ${formatCV(value)}`);
  }
  return `{ ${items.join(", ")} }`;
}

function exhaustiveCheck(param: never): never {
  throw new Error(`invalid clarity value type: ${param}`);
}

export function formatCV(cv: ClarityValue): string {
  if (cv.type === ClarityType.BoolFalse) return "false";
  if (cv.type === ClarityType.BoolTrue) return "true";
  if (cv.type === ClarityType.Int) return cv.value.toString();
  if (cv.type === ClarityType.UInt) return `u${cv.value.toString()}`;
  if (cv.type === ClarityType.StringASCII) return `"${cv.data}"`;
  if (cv.type === ClarityType.StringUTF8) return `u"${cv.data}"`;
  if (cv.type === ClarityType.PrincipalContract) return `'${principalToString(cv)}`;
  if (cv.type === ClarityType.PrincipalStandard) return `'${principalToString(cv)}`;
  if (cv.type === ClarityType.OptionalNone) return "none";
  if (cv.type === ClarityType.OptionalSome) return `(some ${formatCV(cv.value)})`;
  if (cv.type === ClarityType.ResponseOk) return `(ok ${formatCV(cv.value)})`;
  if (cv.type === ClarityType.ResponseErr) return `(err ${formatCV(cv.value)})`;
  if (cv.type === ClarityType.Buffer) return formatBuffer(cv);
  if (cv.type === ClarityType.List) return formatList(cv);
  if (cv.type === ClarityType.Tuple) return formatTuple(cv);

  // make sure that we exhausted all ClarityTypes
  exhaustiveCheck(cv);
}

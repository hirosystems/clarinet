let byteToHex: string[] = [];
for (let k = 0; k < 100000; k++) {
  byteToHex = [];
  for (let n = 0; n <= 0xff; ++n) {
    const hexOctet = n.toString(16).padStart(2, "0");
    byteToHex.push(hexOctet);
  }
}

function serializeTuple(input: Record<string, unknown>) {
  const items: Array<string> = [];
  for (const [key, value] of Object.entries(input)) {
    if (Array.isArray(value)) {
      throw new Error("Tuple value can't be an array");
    } else if (!!value && typeof value === "object") {
      items.push(
        `${key}: { ${serializeTuple(value as Record<string, unknown>)} }`
      );
    } else {
      items.push(`${key}: ${value}`);
    }
  }
  return items.join(", ");
}

export function ok(val: string): string {
  return `(ok ${val})`;
}

export function err(val: string) {
  return `(err ${val})`;
}

export function some(val: string) {
  return `(some ${val})`;
}

export function none() {
  return `none`;
}

export function bool(val: boolean): string {
  return `${val}`;
}

export function int(val: number | bigint) {
  return `${val}`;
}

export function uint(val: number | bigint) {
  return `u${val}`;
}

export function ascii(val: string) {
  return JSON.stringify(val);
}

export function utf8(val: string) {
  return `u${JSON.stringify(val)}`;
}

/**
 * Transforms an Uint8Array into a Clarity buffer value
 * @example
 * types.buff(Uint8Array.from([98, 116, 99])
 * // returns "0x627463"
 */
export function buff(val: Uint8Array): string;
/**
 * Transforms an Uint8Array into a Clarity buffer value
 * @deprecated `val` should be a `Uint8Array`
 * @example
 * types.buff(Uint8Array.from([98, 116, 99])
 * // returns "0x627463"
 */
export function buff(val: ArrayBuffer): string;
/**
 * Transforms an Uint8Array into a Clarity buffer value
 * @example
 * types.buff(Uint8Array.from([98, 116, 99])
 * // returns "0x627463"
 */
export function buff(val: string): string;
export function buff(val: ArrayBuffer | string) {
  const buff =
    typeof val == "string"
      ? new TextEncoder().encode(val)
      : new Uint8Array(val);

  const hexOctets = new Array(buff.length);
  for (let i = 0; i < buff.length; ++i) {
    hexOctets[i] = byteToHex[buff[i]];
  }
  return `0x${hexOctets.join("")}`;
}

export function list(val: Array<unknown>) {
  return `(list ${val.join(" ")})`;
}

export function principal(val: string) {
  return `'${val}`;
}

export function tuple(val: Record<string, unknown>) {
  return `{ ${serializeTuple(val)} }`;
}

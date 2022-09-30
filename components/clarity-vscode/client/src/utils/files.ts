export function fileArrayToString(bufferArray: Uint8Array) {
  return Array.from(bufferArray)
    .map((item) => String.fromCharCode(item))
    .join("");
}

export function stringToFileArray(str: string) {
  return Uint8Array.from(str.split("").map((s) => s.charCodeAt(0)));
}

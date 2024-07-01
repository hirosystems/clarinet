export const defaultFileStore = new Map<string, string>();

function fileArrayToString(bufferArray: Uint8Array) {
  return Array.from(bufferArray)
    .map((item) => String.fromCharCode(item))
    .join("");
}

function isValidReadEvent(e: any): e is { path: string } {
  return typeof e?.path === "string";
}

function isValidReadManyEvent(e: any): e is { paths: string[] } {
  return Array.isArray(e?.paths) && e.paths.every((s: unknown) => typeof s === "string");
}

function isValidWriteEvent(e: any): e is { path: string; content: number[] } {
  return typeof e?.path === "string" && Array.isArray(e?.content);
}

async function exists(event: unknown) {
  if (!isValidReadEvent(event)) throw new Error("invalid read event");
  return defaultFileStore.has(event.path);
}

async function readFile(event: unknown) {
  if (!isValidReadEvent(event)) throw new Error("invalid read event");
  return defaultFileStore.get(event.path) ?? null;
}

async function readFiles(event: any) {
  if (!isValidReadManyEvent(event)) throw new Error("invalid read event");
  const files = event.paths.map((p) => {
    try {
      return defaultFileStore.get(p);
    } catch (err) {
      console.warn(err);
      return null;
    }
  });
  return Object.fromEntries(
    files.reduce(
      (acc, f, i) => {
        if (f === null || f === undefined) return acc;
        return acc.concat([[event.paths[i], f]]);
      },
      [] as [string, string][],
    ),
  );
}

async function writeFile(event: unknown) {
  if (!isValidWriteEvent(event)) throw new Error("invalid write event");
  return defaultFileStore.set(event.path, fileArrayToString(Uint8Array.from(event.content)));
}

export function defaultVfs(action: string, data: unknown) {
  if (action === "vfs/exists") return exists(data);
  if (action === "vfs/readFile") return readFile(data);
  if (action === "vfs/readFiles") return readFiles(data);
  if (action === "vfs/writeFile") return writeFile(data);
  throw new Error("invalid vfs action");
}

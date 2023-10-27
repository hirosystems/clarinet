import * as fs from "node:fs/promises";
import path from "node:path";

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
  try {
    await fs.stat(event.path);
    return true;
  } catch {
    return false;
  }
}

async function readFile(event: unknown) {
  if (!isValidReadEvent(event)) throw new Error("invalid read event");
  return fileArrayToString(await fs.readFile(event.path));
}

async function readFiles(event: any) {
  if (!isValidReadManyEvent(event)) throw new Error("invalid read event");
  const files = await Promise.all(
    event.paths.map(async (p) => {
      try {
        return fs.readFile(p);
      } catch (err) {
        console.warn(err);
        return null;
      }
    }),
  );
  return Object.fromEntries(
    files.reduce(
      (acc, f, i) => {
        if (f === null) return acc;
        return acc.concat([[event.paths[i], fileArrayToString(f)]]);
      },
      [] as [string, string][],
    ),
  );
}

async function writeFile(event: unknown) {
  if (!isValidWriteEvent(event)) throw new Error("invalid write event");
  const dir = path.dirname(event.path);
  if (dir !== ".") await fs.mkdir(dir, { recursive: true });
  return fs.writeFile(event.path, Uint8Array.from(event.content));
}

export function vfs(action: string, data: unknown) {
  if (action === "vfs/exists") return exists(data);
  if (action === "vfs/readFile") return readFile(data);
  if (action === "vfs/readFiles") return readFiles(data);
  if (action === "vfs/writeFile") return writeFile(data);
  throw new Error("invalid vfs action");
}

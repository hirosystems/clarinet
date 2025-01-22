#!/usr/bin/node

import { spawn } from "node:child_process";
import fs from "node:fs/promises";
import path from "node:path";

// directory of the current file
const rootDir = new URL(".", import.meta.url).pathname;

/**
 * build sdk js script
 */
async function build_wasm_js_scripts() {
  const dir = path.join(rootDir, "../clarity-repl/js");
  await execCommand("npm", ["install"], dir);
}

/**
 * build clarinet-sdk-wasm
 */
async function build_wasm_sdk() {
  console.log("Deleting pkg-node");
  await rmIfExists(path.join(rootDir, "pkg-node"));
  console.log("Deleting pkg-browser");
  await rmIfExists(path.join(rootDir, "pkg-browser"));

  await Promise.all([
    execCommand("wasm-pack", [
      "build",
      "--release",
      "--scope",
      "hirosystems",
      "--out-dir",
      "pkg-node",
      "--target",
      "nodejs",
    ]),
    execCommand("wasm-pack", [
      "build",
      "--release",
      "--scope",
      "hirosystems",
      "--out-dir",
      "pkg-browser",
      "--target",
      "web",
    ]),
  ]);

  await updatePackageName();
  await updatePackageJson("pkg-node/package.json");
  await updatePackageJson("pkg-browser/package.json");
}

/**
 * execCommand
 * @param {string} command
 * @param {string[]} args
 * @returns
 */
export const execCommand = async (command, args, cwd = rootDir) => {
  return new Promise((resolve, reject) => {
    const childProcess = spawn(command, args, {
      cwd,
    });
    childProcess.stdout.on("data", (data) => {
      process.stdout.write(data.toString());
    });
    childProcess.stderr.on("data", (data) => {
      process.stderr.write(data.toString());
    });
    childProcess.on("error", (error) => {
      reject(error);
    });
    childProcess.on("exit", (code) => {
      if (code === 0) {
        resolve();
      } else {
        reject(new Error(`❌ Command exited with code ${code}.`));
      }
    });
  });
};

/**
 * rmIfExists
 * @param {string} dirPath
 */
async function rmIfExists(dirPath) {
  try {
    await fs.rm(dirPath, { recursive: true, force: true });
  } catch (error) {
    if (error.code !== "ENOENT") {
      throw error;
    }
  }
}

/**
 * updatePackageName
 */
async function updatePackageName() {
  const filePath = path.join(rootDir, "pkg-browser/package.json");

  const fileData = await fs.readFile(filePath, "utf-8");
  const updatedData = fileData.replace(
    '"name": "@hirosystems/clarinet-sdk-wasm"',
    '"name": "@hirosystems/clarinet-sdk-wasm-browser"',
  );
  await fs.writeFile(filePath, updatedData, "utf-8");
  console.log("✅ pkg-browser/package.json name updated");
}

/**
 * updatePackagesIncludedFiles
 * Include snippets/ files and add the sync-request dependency
 * @param {string} path
 */
async function updatePackageJson(file) {
  const filePath = path.join(rootDir, file);

  const fileData = JSON.parse(await fs.readFile(filePath, "utf-8"));
  fileData.files.push("snippets/");

  fileData.dependencies = { "sync-request": "6.1.0" };
  await fs.writeFile(filePath, JSON.stringify(fileData, null, 2), "utf-8");
  console.log(`✅ ${file} updated`);
}

try {
  await build_wasm_js_scripts();
  await build_wasm_sdk();
  console.log("\n✅ Project successfully built.\n🚀 Ready to publish.");
  console.log("Run the following commands to publish");
  console.log("\n```");
  console.log("$ npm run publish:sdk-wasm");
  console.log("```\n");
} catch (error) {
  console.error("❌ Error building:", error);
  throw error;
}

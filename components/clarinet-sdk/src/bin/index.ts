#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { green, red, yellow } from "kolorist";
import prompts from "prompts";
import { exec } from "node:child_process";

try {
  main();
} catch (error: any) {
  console.error(red(`Failed to execute init script:\n${error}`));
}

async function main() {
  try {
    await checkIfProjectDirectoryIsValid();
  } catch (e: any) {
    console.warn(yellow(`Could not initialize Clarinet testing framework:\n${red(e.message)}`));
    process.exit(1);
  }

  const sdkBinDir = path.dirname(__filename);
  const projectName = path.basename(path.resolve());

  copyTemplateFiles(sdkBinDir);
  updateClarinetSDKVersion(sdkBinDir);
  updatePackageJSONProjectName(projectName);
  updateGitIgnore();
  updateVSCodeWorkspaceSetting();

  console.log("\n");
  console.log(green("Project successfully initialised"));

  const result = await prompts({
    type: "confirm",
    name: "run",
    message: "Do you want to run npm install now?",
    initial: true,
  });

  if (result.run) {
    const child = exec("npm install");
    child.stdout?.on("data", (data) => {
      console.log(`stdout: ${data}`);
    });
    child.stderr?.on("data", (data) => {
      console.error(`stderr: ${data}`);
    });
    child.on("close", (code) => {
      console.log(`child process exited with code ${code}`);
    });
    await new Promise((resolve, reject) => {
      child.on("exit", resolve);
      child.on("error", reject);
    });
  }

  console.log("\n");
  console.log(
    green("You are now ready to test your smart contracts with Vitest and the Clarinet SDK"),
  );
  console.log(green("Open ./tests/contract.test.ts to see an example"));
}

// check if Clarinet.toml exists and if the Node/NPM boilerplate doesn't
async function checkIfProjectDirectoryIsValid() {
  const isClarinetProject = fs.existsSync(path.join(process.cwd(), "Clarinet.toml"));
  if (!isClarinetProject) {
    throw new Error(
      "Clarinet.toml not found in the current directory. Please run this command in a Clarinet project.",
    );
  }

  const unexpectedFiles = [
    "package.json",
    "vitest.config.js",
    "vitest.config.ts",
    "tsconfig.json",
    "tests/contract.test.ts",
  ];
  for (const unexpectedFile of unexpectedFiles) {
    if (fs.existsSync(path.join(process.cwd(), unexpectedFile))) {
      const errorMsg = `A ${unexpectedFile} file already exists in this directory. It is possible that the testing framework has already been initialised.`;
      throw new Error(errorMsg);
    }
  }

  return true;
}

// copy the Node/NPM boilerplate
function copyTemplateFiles(sdkBinDir: string) {
  console.log("Copying package.json, tsconfig.json, vitest.config.js and sample test file");

  fs.cpSync(path.join(sdkBinDir, "../../../templates"), path.join(process.cwd(), "."), {
    recursive: true,
  });
}

// update to package.json name to  "<project-name>-tests"
function updatePackageJSONProjectName(projectName: string) {
  console.log("Updating package.json");
  const packageJSONPath = path.join(process.cwd(), "package.json");
  const packageJSON = JSON.parse(fs.readFileSync(packageJSONPath, "utf-8"));
  packageJSON.name = `${projectName}-tests`;
  fs.writeFileSync(packageJSONPath, JSON.stringify(packageJSON, null, 2));
}

// make sure we the current version of `@hirosystems/clarinet-sdk`
function updateClarinetSDKVersion(sdkBinDir: string) {
  const sdkPackageJSONPath = path.join(sdkBinDir, "../../../package.json");
  const sdkPackageJSON = JSON.parse(fs.readFileSync(sdkPackageJSONPath, "utf-8"));
  const version = sdkPackageJSON.version;

  const projectPackageJSONPath = path.join(process.cwd(), "package.json");
  const projectPackageJSON = JSON.parse(fs.readFileSync(projectPackageJSONPath, "utf-8"));
  projectPackageJSON.dependencies["@hirosystems/clarinet-sdk"] = `^${version}`;

  fs.writeFileSync(projectPackageJSONPath, JSON.stringify(projectPackageJSON, null, 2));
}

// add node and npm directories to the .gitignore
function updateGitIgnore() {
  console.log("Updating .gitignore");
  const newLines = [
    "logs",
    "*.log",
    "npm-debug.log*",
    "coverage",
    "*.info",
    "costs-reports.json",
    "node_modules",
  ].join("\n");

  const gitIgnorePath = path.join(process.cwd(), ".gitignore");
  if (fs.existsSync(gitIgnorePath)) {
    fs.appendFileSync(
      gitIgnorePath,
      "\n# Ignore Node and NPM files. Added by the clarinet-sdk migration.",
    );
    fs.appendFileSync(gitIgnorePath, `\n${newLines}`);
  } else {
    fs.writeFileSync(gitIgnorePath, newLines);
  }
}

// disable the deno extension if it's enabled
function updateVSCodeWorkspaceSetting() {
  const vscodeSettingsPath = path.join(process.cwd(), ".vscode", "settings.json");
  if (fs.existsSync(vscodeSettingsPath)) {
    const vscodeSettings = JSON.parse(fs.readFileSync(vscodeSettingsPath, "utf-8"));
    if (vscodeSettings["deno.enable"] === true) {
      green("Updating workspace settings");
      vscodeSettings["deno.enable"] = false;
      fs.writeFileSync(vscodeSettingsPath, JSON.stringify(vscodeSettings, null, 2));
    }
  }
}

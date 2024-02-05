import * as path from "path";

import { BrowserType, runTests } from "@vscode/test-web";

function getArgv(name) {
  const arg = process.argv.find((a) => a.startsWith(`--${name}=`));
  if (!arg) return undefined;
  return arg.split("=")[1];
}

function isValidBrowserType(browserType: unknown): browserType is BrowserType {
  return (
    !!browserType &&
    typeof browserType === "string" &&
    ["chromium", "firefox", "webkit"].includes(browserType)
  );
}

async function main() {
  console.log("__dirname", __dirname);
  const extensionDevelopmentPath = path.resolve(__dirname, "../../../");
  console.log("-".repeat(200));
  console.log("-".repeat(200));
  console.log("extensionDevelopmentPath", extensionDevelopmentPath);
  const extensionTestsPath = path.resolve(__dirname, "./suite/index");
  const folderPath = path.resolve(__dirname, "../../../test-data");

  try {
    const waitForDebugger = Number(getArgv("waitForDebugger"));
    const browserType = getArgv("browserType") || "chromium";
    if (!isValidBrowserType(browserType))
      throw new Error("invalid browserType");

    await runTests({
      extensionDevelopmentPath,
      extensionTestsPath,
      folderPath,
      browserType,
      waitForDebugger,
      devTools: false,
      headless: false,
    });
  } catch (err) {
    console.error(err);
    console.error("Failed to run tests");
    process.exit(1);
  }
}

main();

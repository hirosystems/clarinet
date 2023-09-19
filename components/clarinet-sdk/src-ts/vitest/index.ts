import yargs from "yargs";
import { hideBin } from "yargs/helpers";

export function getClarinetVitestsArgv() {
  const argv = hideBin(process.argv);
  const topLevel = yargs(argv).argv;

  // @ts-ignore
  return yargs(topLevel._)
    .option("coverage", {
      alias: "cov",
      type: "boolean",
      default: false,
    })
    .option("coverage-filename", {
      alias: "cov-file",
      type: "string",
      default: "lcov.info",
    }).argv;
}

export const vitestHelpersPath = "node_modules/@hirosystems/clarinet-sdk/vitest-helpers/src/";
export const vitestSetupFilePath = `${vitestHelpersPath}vitest.setup.ts`;

import yargs from "yargs";
import { hideBin } from "yargs/helpers";

export function getClarinetVitestsArgv() {
  const argv = hideBin(process.argv);
  const topLevel = yargs(argv).argv;

  // @ts-ignore
  return yargs(topLevel._).option("clarity-coverage", {
    alias: "cov",
    type: "boolean",
    default: false,
  }).argv;
}

export const vitestHelpersPath = "node_modules/obscurity-sdk/vitest-helpers/";
export const vitestSetupFilePath = `${vitestHelpersPath}vitest.setup.ts`;

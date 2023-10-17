import yargs from "yargs";
import { hideBin } from "yargs/helpers";

export function getClarinetVitestsArgv() {
  const argv = hideBin(process.argv);
  const topLevel = yargs(argv).argv;

  // @ts-ignore
  return yargs(topLevel._)
    .option("manifest-path", {
      alias: "manifest",
      type: "string",
      default: "./Clarinet.toml",
    })
    .option("init-before-each", {
      description: "Reinitialize the Clarinet state before each test",
      type: "boolean",
      default: true,
    })
    .option("coverage", {
      alias: "cov",
      type: "boolean",
      default: false,
    })
    .option("costs", {
      alias: "cost",
      type: "boolean",
      default: false,
    })
    .option("coverage-filename", {
      alias: "cov-file",
      type: "string",
      default: "lcov.info",
    })
    .option("costs-filename", {
      alias: "costs-file",
      type: "string",
      default: "costs-reports.json",
    }).argv;
}

export const vitestHelpersPath = "node_modules/@hirosystems/clarinet-sdk/vitest-helpers/src/";
export const vitestSetupFilePath = `${vitestHelpersPath}vitest.setup.ts`;

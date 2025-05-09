import type { Simnet } from "../../dist/esm/node/src";

declare global {
  var simnet: Simnet;
  var testEnvironment: string;
  var coverageReports: string[];
  var costsReports: string[];
  var options: {
    clarinet: {
      manifestPath: string;
      initBeforeEach: boolean;
      coverage: boolean;
      coverageFilename: string;
      costs: boolean;
      costsFilename: string;
      includeBootContracts: boolean;
      bootContractsPath: string;
    };
  };
}

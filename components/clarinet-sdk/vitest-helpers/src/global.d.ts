import type { ClarityVM } from "../../dist/esm";

declare global {
  var vm: ClarityVM;
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
    };
  };
}

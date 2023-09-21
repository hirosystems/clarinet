import type { ClarityVM } from "../../dist/esm";

declare global {
  var vm: ClarityVM;
  var testEnvironment: string;
  var coverageReports: string[];
  var options: {
    clarinet: {
      coverage: boolean;
      coverageFilename: string;
    };
  };
}

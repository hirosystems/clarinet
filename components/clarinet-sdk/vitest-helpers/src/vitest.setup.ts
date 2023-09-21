import { Task, afterAll, beforeAll, beforeEach } from "vitest";

import "./clarityValuesMatchers";

function getFullTestName(task: Task, names: string[]) {
  const fullNames = [task.name, ...names];
  if (task.suite?.name) {
    return getFullTestName(task.suite, fullNames);
  }
  return fullNames;
}

beforeAll(async () => {
  await vm.initSession(process.cwd(), "./Clarinet.toml");
});

beforeEach(async (ctx) => {
  if (global.options.clarinet.coverage) {
    const suiteTestNames = getFullTestName(ctx.task, []);
    const fullName = [ctx.task.file?.name || "", ...suiteTestNames].join("__");
    vm.setCurrentTestName(fullName);
  }
});

afterAll(() => {
  if (global.options.clarinet.coverage) {
    coverageReports.push(vm.getReport().coverage);
  }
});

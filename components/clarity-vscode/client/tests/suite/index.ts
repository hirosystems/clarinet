// eslint-disable-next-line @typescript-eslint/no-require-imports
require("mocha/mocha");

export function run(): Promise<void> {
  return new Promise((c, e) => {
    mocha.setup({
      ui: "bdd",
      reporter: undefined,
      timeout: 5000,
    });

    const importAll = (r: __WebpackModuleApi.RequireContext) =>
      r.keys().forEach(r);
    importAll(require.context(".", true, /\.test$/));

    try {
      mocha.run((failures) => {
        if (failures > 0) {
          e(new Error(`${failures} tests failed.`));
        } else {
          c();
        }
      });
    } catch (err) {
      console.error(err);
      e(err);
    }
  });
}

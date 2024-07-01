/*
  The scripts creates a package.json file in dist/esm with to make it an ESM package
*/

const fs = require("fs");
const path = require("path");

function createEsmModulePackageJson() {
  const packageJsonFile = path.join("./dist/esm", "/package.json");
  fs.writeFileSync(packageJsonFile, new Uint8Array(Buffer.from('{"type": "module"}')));
}

createEsmModulePackageJson();

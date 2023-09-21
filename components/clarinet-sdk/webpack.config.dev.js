const path = require("path");

/** @typedef {import('webpack').Configuration} WebpackConfig **/

const target = "node18";
const entry = {
  index: "./src-ts/index.ts",
  "vitest/index": "./src-ts/vitest/index.ts",
  "bin/index": "./src-ts/bin/index.ts",
};

// watch and build src-ts only
// run npm run build to re-build rust sources

/** @type WebpackConfig */
const configESM = {
  mode: "production",
  entry,
  resolve: { extensions: [".ts", ".js"] },
  optimization: {
    minimize: false,
  },
  target,
  watch: true,
  output: {
    filename: "index.mjs",
    path: path.resolve(__dirname, "dist/esm"),
    library: {
      type: "module",
    },
  },
  module: {
    rules: [
      {
        test: /\.ts$/,
        loader: "ts-loader",
        exclude: /node_modules/,
        options: { configFile: "tsconfig.json" },
      },
    ],
  },
  experiments: {
    asyncWebAssembly: true,
    outputModule: true,
  },
};

module.exports = [configESM];

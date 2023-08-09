const path = require("path");

// watch and build src-ts only
// run npm run build to re-build rust sources

/** @typedef {import('webpack').Configuration} WebpackConfig **/

/** @type WebpackConfig */
const configBase = {
  mode: "production",
  entry: "./src-ts/index.ts",
  resolve: { extensions: [".ts", ".js"] },
  optimization: {
    minimize: false,
  },
};

/** @type WebpackConfig */
const configESM = {
  ...configBase,
  target: "node20",
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
        options: { configFile: "tsconfig.esm.json" },
      },
    ],
  },
  experiments: {
    asyncWebAssembly: true,
    outputModule: true,
  },
};

module.exports = [configESM];

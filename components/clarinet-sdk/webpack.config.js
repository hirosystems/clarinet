const path = require("path");
const WasmPackPlugin = require("@wasm-tool/wasm-pack-plugin");
const CopyPlugin = require("copy-webpack-plugin");

/** @typedef {import('webpack').Configuration} WebpackConfig **/

const target = "node18";
const entry = {
  index: "./src-ts/index.ts",
  "vitest/index": "./src-ts/vitest/index.ts",
};

/** @type WebpackConfig */
const configBase = {
  mode: "production",
  entry,
  resolve: { extensions: [".ts", ".js"] },
  optimization: {
    minimize: false,
  },
};

/** @type WebpackConfig */
const configESM = {
  ...configBase,
  target,
  output: {
    filename: "[name].mjs",
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
  plugins: [
    new WasmPackPlugin({
      crateDirectory: path.resolve(__dirname, "./"),
      extraArgs: "--release --target=bundler",
      outDir: path.resolve(__dirname, "./src-ts/sdk"),
    }),
    new CopyPlugin({
      patterns: [{ from: "./src-ts/sdk/index.d.ts", to: "sdk" }],
    }),
  ],
};

/** @type WebpackConfig */
const configCJS = {
  ...configBase,
  target,
  output: {
    filename: "[name].js",
    path: path.resolve(__dirname, "dist/cjs"),
    library: {
      type: "commonjs",
    },
  },
  module: {
    rules: [
      {
        test: /\.ts$/,
        loader: "ts-loader",
        exclude: /node_modules/,
        options: { configFile: "tsconfig.cjs.json" },
      },
    ],
  },
  experiments: {
    asyncWebAssembly: true,
    outputModule: false,
  },
  plugins: [
    new WasmPackPlugin({
      crateDirectory: path.resolve(__dirname, "./"),
      extraArgs: "--release --target=bundler",
      outDir: path.resolve(__dirname, "./src-ts/sdk"),
    }),
    new CopyPlugin({
      patterns: [{ from: "./src-ts/sdk/index.d.ts", to: "sdk" }],
    }),
  ],
};

module.exports = [configESM, configCJS];

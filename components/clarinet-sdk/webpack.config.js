const path = require("path");
const WasmPackPlugin = require("@wasm-tool/wasm-pack-plugin");
const CopyPlugin = require("copy-webpack-plugin");
const webpack = require("webpack");

/** @typedef {import('webpack').Configuration} WebpackConfig **/

const target = "node18";
const entry = {
  index: "./src-ts/index.ts",
  "vitest/index": "./src-ts/vitest/index.ts",
};

/** @type WebpackConfig */
const configBase = {
  mode: "production",
  resolve: { extensions: [".ts", ".js"] },
  optimization: {
    minimize: false,
  },
};

/** @type WebpackConfig */
const configESM = {
  ...configBase,
  entry,
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
  entry: {
    ...entry,
    "bin/index": "./src-ts/bin/index.ts", // only for CJS
  },
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
    new CopyPlugin({
      patterns: [{ from: "./src-ts/bin/templates/", to: "bin/templates/" }],
    }),
    new webpack.BannerPlugin({
      banner: "#!/usr/bin/env node",
      raw: true,
      include: "bin/index",
    }),
  ],
};

module.exports = [configESM, configCJS];

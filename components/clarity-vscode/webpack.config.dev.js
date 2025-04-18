// @ts-check
"use-strict";

const path = require("path");
const webpack = require("webpack");
const WasmPackPlugin = require("@wasm-tool/wasm-pack-plugin");

const configs = require("./webpack.config");

const [clientBrowserConfig, serverBrowserConfig] = configs;

const extensionURL = "http://localhost:3000/static/devextensions/";

clientBrowserConfig.plugins = [
  new webpack.DefinePlugin({
    __DEV_MODE__: JSON.stringify(true),
  }),
];

serverBrowserConfig.plugins = [
  new webpack.DefinePlugin({
    __EXTENSION_URL__: JSON.stringify(extensionURL),
  }),
  new WasmPackPlugin({
    crateDirectory: path.resolve(__dirname, "../clarity-lsp"),
    extraArgs: "--release --target=web",
    outDir: path.resolve(__dirname, "server/src/clarity-lsp-browser"),
    outName: "lsp-browser",
  }),
];

module.exports = [clientBrowserConfig, serverBrowserConfig];

import { wasm } from "@rollup/plugin-wasm";
import typescript from "@rollup/plugin-typescript";
import copy from "rollup-plugin-copy";

export default {
  input: {
    index: "src-ts/index.ts",
  },
  plugins: [
    wasm(),
    typescript({ tsconfig: "./tsconfig.json" }),
    copy({ targets: [{ src: "./src-ts/sdk/clarinet_sdk.d.ts", dest: "dist/sdk" }] }),
  ],
  module: "esnext",
  output: {
    dir: "dist",
    format: "cjs",
  },
};

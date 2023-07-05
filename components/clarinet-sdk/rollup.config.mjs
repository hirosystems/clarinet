import { wasm } from "@rollup/plugin-wasm";
import typescript from "@rollup/plugin-typescript";

export default {
  input: "src-ts/index.ts",
  plugins: [typescript({ tsconfig: "./tsconfig.json" }), wasm()],
  module: "esnext",
  output: {
    dir: "dist",
    format: "cjs",
  },
};

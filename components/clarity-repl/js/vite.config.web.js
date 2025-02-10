import { defineConfig } from "vite";

export default defineConfig({
  build: {
    lib: {
      entry: "./index.mjs",
      formats: ["es"],
      fileName: "bundle.web",
    },
    target: "modules",
  },
});

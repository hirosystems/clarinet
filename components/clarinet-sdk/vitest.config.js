/// <reference types="vitest" />
import { defineConfig } from "vite";

export default defineConfig({
  test: {
    include: ["./tests/**/*.test.ts", "./vitest-helpers/tests/**/*.test.ts"],
  },
});

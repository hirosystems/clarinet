/// <reference types="vitest" />
import { defineConfig } from "vite";

export default defineConfig({
  test: {
    singleThread: true,
    include: ["./tests/**/*.test.ts", "./vitest-helpers/tests/**/*.test.ts"],
  },
});

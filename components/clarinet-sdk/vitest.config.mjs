/// <reference types="vitest" />
import { defineConfig } from "vite";

export default defineConfig({
  test: {
    poolOptions: {
      threads: {
        singleThread: true,
      },
    },
    include: ["./tests/**/*.test.ts", "./vitest-helpers/tests/**/*.test.ts"],
  },
});

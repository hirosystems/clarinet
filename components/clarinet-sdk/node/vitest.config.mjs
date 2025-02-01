/// <reference types="vitest" />
import { defineConfig } from "vite";

export default defineConfig({
  test: {
    hookTimeout: 500,
    testTimeout: 1000,
    // https://vitest.dev/guide/common-errors.html#failed-to-terminate-worker
    pool: "forks",
    poolOptions: {
      forks: { singleFork: true },
    },
    include: ["./tests/**/*.test.ts", "./vitest-helpers/tests/**/*.test.ts"],
  },
});

/// <reference types="vitest" />
import { defineConfig } from "vite";

export default defineConfig({
  test: {
    // https://vitest.dev/guide/common-errors.html#failed-to-terminate-worker
    pool: "forks",
    poolOptions: {
      threads: { singleThread: true },
      forks: { singleFork: true },
    },
    include: ["./tests/**/*.test.ts", "./vitest-helpers/tests/**/*.test.ts"],
  },
});

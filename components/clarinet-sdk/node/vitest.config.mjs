/// <reference types="vitest" />
import { defineConfig } from "vite";

export default defineConfig({
  test: {
    pool: "forks",
    poolOptions: {
      forks: { singleFork: true },
    },
    include: ["./tests/**/*.test.ts", "./vitest-helpers/tests/**/*.test.ts"],
  },
});

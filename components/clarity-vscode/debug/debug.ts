import { lookpath } from "lookpath";
import { spawn } from "child_process";

(async () => {
  const path = await lookpath("clarinet");
  if (!path) return Promise.reject("'clarinet' is not installed");

  const dap = spawn(path, ["dap"], {
    stdio: [process.stdin, process.stdout, process.stderr],
  });

  await new Promise((resolve) => {
    dap.on("exit", resolve);
  });
})();

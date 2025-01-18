import { initSimnet } from "@hirosystems/clarinet-sdk";
import { resolve } from "path";

const main = async () => {
  const manifestDir = process.argv[2];

  const manifestPath = resolve(manifestDir, "Clarinet.toml");

  console.log(manifestPath);
  await initSimnet(manifestPath);
};

main();
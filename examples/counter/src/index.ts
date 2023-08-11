import { initVM } from "obscurity-sdk";
import { generateTSLib } from "obscurity-sdk/ts-gen";

async function main() {
  const vm = await initVM();
  const ci = vm.getContractsInterfaces();
  generateTSLib(ci);
}

main();

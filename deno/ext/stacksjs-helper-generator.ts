// Clarinet extention #1
//
// This extension is introspecting the contracts of the project it's running from, 
// and produce a Typescript data structure autocomplete friendly, that developers
// can use in their frontend code.
//
// When running:
// $ clarinet run --allow-write scripts/stacksjs-helper-generator.rs
//
// This script will write a file at the path artifacts/contracts.ts:
//
// export namespace CounterContract {
//     export const address = "ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM";
//     export const name = "counter";
//     export namespace Functions {
//         export namespace Increment {
//             export const name = "increment";
//             export interface IncrementArgs {
//                 step: ClarityValue,
//             }
//             export function args(args: IncrementArgs): [ClarityValue] {
//                 return [
//                     args.step,
//                 ];
//             }
//         }
//         // read-counter
//         export namespace ReadCounter {
//             export const name = "read-counter";
//
//         }
//     }
// }
//
// By importing this file in their frontend code, developers can use constants, instead 
// of hard coding principals and strings:
// 
// await makeContractCall({
//     contractAddress: CounterContract.address,
//     contractName: CounterContract.name,
//     functionName: CounterContract.Functions.Increment.name,
//     functionArgs: CounterContract.Functions.Increment.args({ step: uintCV(10); }),
//     ...
// }

import { Clarinet, Contract, Account, StacksNode } from '../index';

Clarinet.run({
    async fn(accounts: Map<string, Account>, contracts: Map<string, Contract>, node: StacksNode) {
        let code = [];
        code.push([
            `// Code generated with the stacksjs-helper-generator extension`,
            `// Manual edits will be overwritten`,
            ``,
            `import { ClarityValue } from "@stacks/transactions"`,
            ``,
        ]);

        for (let [contractId, contract] of contracts) {
            let [address, name] = contractId.split(".");
            code.push([
                `export namespace ${kebabToCamel(capitalize(name))}Contract {`,
                `    export const address = "${address}";`,
                `    export const name = "${name}";`,
                ``,
            ]);

            let functions = [];

            for (let func of contract.contract_interface.functions) {
                if (func.access === "public") {
                    functions.push(func);
                } else if (func.access === "read_only") {
                    functions.push(func)
                }
            }

            if (functions.length > 0) {
                code.push([
                    `    // Functions`,
                    `    export namespace Functions {`,
                ]);
                for (let f of functions) {

                    code.push([
                        `        // ${f.name}`,
                        `        export namespace ${kebabToCamel(capitalize(f.name))} {`,
                        `            export const name = "${f.name}";`,
                        ``
                    ]);
                    
                    if (f.args.length > 0) {

                        // Generate code for interface
                        code.push([
                            `            export interface ${kebabToCamel(capitalize(f.name))}Args {`,
                        ]);
                        for (let arg of f.args) {
                            code.push([
                                `                ${kebabToCamel(arg.name)}: ClarityValue,`,
                            ]);
                        }
                        code.push([
                            `            }`,
                            ``
                        ]);

                        // Generate code for helper function
                        code.push([
                            `            export function args(args: ${kebabToCamel(capitalize(f.name))}Args): [ClarityValue] {`,
                            `                return [`
                        ]);
                        for (let arg of f.args) {
                            code.push([
                                `                    args.${kebabToCamel(arg.name)},`,
                            ]);
                        }
                        code.push([
                            `                ];`,
                            `            }`,
                            ``
                        ]);
                    }

                    code.push([
                        `        }`,
                        ``
                    ]);
                }

                code.push([
                    `    }`,
                ]);
            }

            code.push([
                `}`,
                ``
            ]);
        }

        const write = await Deno.writeTextFile("./artifacts/contracts.ts", code.flat().join("\n"));
    }
});

function capitalize(source: string): string {
    return source[0].toUpperCase() + source.slice(1);
}

function kebabToCamel(source: string): string {
    return source
        .replace(/[^\w\-\_]/g, "")
        .replace(/(-\w)/g, (x) => x[1].toUpperCase());
}

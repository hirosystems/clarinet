// Clarinet extention 101
//
// This extension is introspecting the contracts of the project it's running from, 
// and produce a Typescript data structure autocomplete friendly, that developers
// can use in their frontend code.
//
// When running:
// $ clarinet run --allow-write scripts/abi-generator.rs
//
// This script will write a file at the path artifacts/contracts.ts:
//
// export namespace CounterContract {
//     export const ADDRESS = "ST000000000000000000002AMW42H";
//     export const NAME = "counter";
//     export enum Public {
//         INCREMENT = "increment",
//         DECREMENT = "decrement",
//     }   
//     export enum ReadOnly {
//         READ = "read",
//     }
// }
//
// By importing this file in their frontend code, developers can use constants, instead 
// of hard coding principals and strings:
// 
// await makeContractCall({
//     contractAddress: CounterContract.ADDRESS,
//     contractName: CounterContract.NAME,
//     functionName: CounterContract.Public.INCREMENT,
//     functionArgs: [CounterContract.Args.INCREMENT(namespace)], // or so
//     ...

import { Clarinet, Contract, Account, StacksNode } from 'https://deno.land/x/clarinet@v0.13.0/index.ts';

Clarinet.run({
    async fn(accounts: Map<string, Account>, contracts: Map<string, Contract>, node: StacksNode) {
        let code = [];
        code.push([
            `// Code generated with the clarinet-abi-generator extension`,
            `// Manual edits will be overwritten`,
            ``,
        ]);

        for (let [contractId, contract] of contracts) {
            let [address, name] = contractId.split(".");
            code.push([
                `export namespace ${kebabToCamel(capitalize(name))}Contract {`,
                `    export const ADDRESS = "${address}";`,
                `    export const NAME = "${name}";`,
                ``,
            ]);

            let public_funcs = [];
            let readonly_funcs = [];

            for (let func of contract.contract_interface.functions) {
                if (func.access === "public") {
                    public_funcs.push(func);
                } else if (func.access === "read_only") {
                    readonly_funcs.push(func)
                }
            }

            if (public_funcs.length > 0) {
                code.push([
                    `    // Public functions`,
                    `    export enum Public {`,
                ]);
                for (let f of public_funcs) {
                    code.push([
                        `        ${kebabToCamel(capitalize(f.name))} = "${f.name}",`,
                    ]);
                }
                code.push([
                    `    }`,
                    ``
                ]);
            }

            if (readonly_funcs.length > 0) {
                code.push([
                    `    // Read only functions`,
                    `    export enum ReadOnly {`,
                ]);
                for (let f of readonly_funcs) {
                    code.push([
                        `        ${kebabToCamel(capitalize(f.name))} = "${f.name}",`,
                    ]);
                }
                code.push([
                    `    }`,
                    ``
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

function snakeToCamel(source: string): string {
    return source.replace(/(_\w)/g, (x) => x[1].toUpperCase());
}

function kebabToCamel(source: string): string {
    return source.replace(/(-\w)/g, (x) => x[1].toUpperCase());
}

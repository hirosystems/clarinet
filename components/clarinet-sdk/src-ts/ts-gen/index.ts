import { ContractInterface } from "../contractInterface.js";

// @todo: fix the type, the key is a contract identifier not a string
export function generateTSLib(contractInterfaces: Map<string, ContractInterface>) {
  contractInterfaces.forEach((contractInterface, identifier) => {
    // @ts-ignore
    const name: string = identifier.name;

    console.log(name, JSON.stringify(contractInterface, null, 2));
  });
}

// input: contractInterface

// output:
// name-of-the-project.ts

// export const countract1 = {
//   increment: () => {
//     // call increment function
//   },
//   decrement: () => {
//     // call decrement function
//   }
// }

// export const countract2 = {
//   increment: () => {
//     // call increment function
//   },
//   decrement: () => {
//     // call decrement function
//   }
// }

// import contracts from "/ts-gen/proxy"

// const {counter }= contracts;

// const {increment } = counter

// // const res = vm.callPublicFn(contract, "increment", [], sender);

// callReadOnlyFunction.bind(null, "ASDKLJASLDKJ", )

// contracts.counter.increment()

// const res:  = await counter.increment(arg1, arg2)

// usage
// counter.increment()

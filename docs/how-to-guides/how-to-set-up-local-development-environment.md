---
title: Set up local Development Environment
---

## Developing a Clarity smart contract

This article helps you with creating a new project and develop a clarity smart contract.

_Topics covered in this guide_:

- [Develop a clarity smart contract](#develop-a-smart-contract)
- [Test and interact with smart contracts](#testing-with-clarinet)

## Develop a smart contract

Once you have installed Clarinet, you can begin a new Clarinet project with the command:

```sh
clarinet new my-project && cd my-project
```

This command creates a new directory and populates it with boilerplate configuration and testing files. The `toml` files
in the `settings` directory control the Clarinet environment. For example, the `Devnet.toml` file contains
definitions for wallets in the local REPL environment and their starting balances (in STX).

```toml
...
[accounts.deployer]
mnemonic = "fetch outside black test wash cover just actual execute nice door want airport betray quantum stamp fish act pen trust portion fatigue scissors vague"
balance = 1_000_000

[accounts.wallet_1]
mnemonic = "spoil sock coyote include verify comic jacket gain beauty tank flush victory illness edge reveal shallow plug hobby usual juice harsh pact wreck eight"
balance = 1_000_000

[accounts.wallet_2]
mnemonic = "arrange scale orient half ugly kid bike twin magnet joke hurt fiber ethics super receive version wreck media fluid much abstract reward street alter"
balance = 1_000_000
...
```

You can create a new contract in the project with the command:

```sh
clarinet contract new my-contract
```

This command creates a new `my-contract.clar` file in the `contracts` directory and a `my-contract_test.ts` in the
`tests` directory. Additionally, it adds the contract to the `Clarinet.toml` configuration file.

```toml
[contracts.my-contract]
path = "contracts/my-contract.clar"
```

### Set clarity version of contract

You can specify the clarity version of your contract in the `Clarinet.toml` configuration file by updating it as shown below.

```toml
[contracts.cbtc-token]
path = "contracts/cbtc-token.clar"
clarity_version = 1
```

```toml
[contracts.cbtc-token]
path = "contracts/cbtc-token.clar"
clarity_version = 2
```

At this point, you can begin editing your smart contract in the `contracts` directory. At any point, while you are developing, you can use the command `clarinet check` to check the syntax of your smart contract.
If you are using VSCode, the [Clarity extension](https://marketplace.visualstudio.com/items?itemName=HiroSystems.clarity-lsp) does the check for you.

Review this comprehensive walkthrough video for a more in-depth overview of developing with Clarinet.

<br /><iframe width="560" height="315" src="https://www.youtube.com/embed/zERDftjl6k8" title="YouTube video player" frameborder="0" allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture" allowfullscreen></iframe>

## Testing with Clarinet

Clarinet provides several powerful methods to test and interact with your smart contracts. As mentioned in the previous
section, you can always check your Clarity syntax using the `clarinet check` command. This validates any smart contracts
you are currently developing in the active project.

You can use two tools in Clarinet to test smart contracts: the console, an interactive Clarity REPL, and the test harness, a testing framework written in Typescript.
When developing a new smart contract using local Clarity REPL, you can exercise a contract without waiting for block times in a live blockchain.

### Testing with the console

The Clarinet console is an interactive Clarity REPL that runs in memory. Any contracts configured in the current project are automatically loaded into memory. Additionally, wallets defined in the `settings/Devnet.toml` file are initialized with STX tokens for testing purposes. When the console runs, it provides a summary of the deployed contracts, their public functions, wallet addresses, and balances.

```
clarity-repl v0.11.0
Enter "::help" for usage hints.
Connected to a transient in-memory database.
Initialized contracts
+-------------------------------------------------------+-------------------------+
| Contract identifier                                   | Public functions        |
+-------------------------------------------------------+-------------------------+
| ST1HTBVD3JG9C05J7HBJTHGR0GGW7KXW28M5JS8QE.my-contract | (echo-number (val int)) |
|                                                       | (say-hi)                |
+-------------------------------------------------------+-------------------------+

Initialized balances
+------------------------------------------------------+---------+
| Address                                              | STX     |
+------------------------------------------------------+---------+
| ST1HTBVD3JG9C05J7HBJTHGR0GGW7KXW28M5JS8QE (deployer) | 1000000 |
+------------------------------------------------------+---------+
| ST1J4G6RR643BCG8G8SR6M2D9Z9KXT2NJDRK3FBTK (wallet_1) | 1000000 |
+------------------------------------------------------+---------+
...
```

You can use the `::help` command for valid console commands.

```
>> ::help
::help                                  Display help
::functions                             Display all the native functions available in clarity
::keywords                              Display all the native keywords available in clarity
::describe <function> | <keyword>       Display documentation for a given native function or keyword
::mint_stx <principal> <amount>         Mint STX balance for a given principal
::set_tx_sender <principal>             Set tx-sender variable to principal
::get_assets_maps                       Get assets maps for active accounts
::get_costs <expr>                      Display the cost analysis
::get_contracts                         Get contracts
::get_block_height                      Get current block height
::advance_chain_tip <count>             Simulate mining of <count> blocks
::set_epoch <2.0> | <2.05> | <2.1>      Update the current epoch
::get_epoch                             Get current epoch
::toggle_costs                          Display cost analysis after every expression
::debug <expr>                          Start an interactive debug session executing <expr>
::trace <expr>                          Generate an execution trace for <expr>
::reload                                Reload the existing contract(s) in the session
::read <filename>                       Read expressions from a file
::encode <expr>                         Encode an expression to a Clarity Value bytes representation
::decode <bytes>                        Decode a Clarity Value bytes representation
```

The console commands control the REPL chain's state, letting you get information about it and advance the chain
tip. Additionally, you can enter Clarity commands into the console and observe the result of the command. The
`::list_functions` console command prints a cheat sheet of Clarity commands. For example, in the example contract,
you could use the REPL to call the `echo-number` function in the contract with the following command:

```
>> (contract-call? .my-contract echo-number 42)
(ok 42)
```

Note that by default, commands are always executed as the `deployer` address, which means you can use the shorthand
`.my-contract` without specifying a full address to the contract. If you changed the transaction address with the
`::set_tx_sender` command, you would need to provide the full address to the contract in the contract call
(`ST1HTBVD3JG9C05J7HBJTHGR0GGW7KXW28M5JS8QE.my-contract`).

You can refer to the [Clarity language reference](https://docs.stacks.co/docs/clarity/language-functions) for a complete overview of all Clarity functions.

### Testing smart contracts

Smart contracts can best tested with Node.js and Vitest thanks to the clarinet-sdk. See the [testing guide](../feature-guides/test-contract-with-clarinet-sdk.md) to learn more.

> `clarinet test` is now depracated and the recommended way is to use the JS SDK.

## Additional reading

- [Clarinet README](https://github.com/hirosystems/clarinet#clarinet)
- [clarinet repository](https://github.com/hirosystems/clarinet)
- [Clarity language reference](https://docs.stacks.co/references/language-functions)
- [Clarinet SDK](https://www.npmjs.com/package/@hirosystems/clarinet-sdk)
- [Clarity VSCode extension](https://marketplace.visualstudio.com/items?itemName=HiroSystems.clarity-lsp)

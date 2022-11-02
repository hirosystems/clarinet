---
title: Set up local Development Environment
---

## Developing a Clarity smart contract

Once you have installed Clarinet, you can begin a new Clarinet project with the command:

```sh
clarinet new my-project && cd my-project
```

This command creates a new directory and populates it with boilerplate configuration and testing files. The `toml` files
located in the `settings` directory control the Clarinet environment. For example, the `Devnet.toml` file contains
definitions for wallets in the local REPL environment, and their starting balances (in STX).

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

This command creates a new `my-contract.clar` file in the `contracts` directory, and a `my-contract_test.ts` in the
`tests` directory. Additionally, it adds the contract to the `Clarinet.toml` configuration file.

```toml
[contracts.my-contract]
path = "contracts/my-contract.clar"
```

At this point, you can begin editing your smart contract in the `contracts` directory. At any point while you are developing, you can use the command `clarinet check` to check the syntax of your smart contract.

For a more in-depth overview of developing with Clarinet, review this comprehensive walkthrough video.

<br /><iframe width="560" height="315" src="https://www.youtube.com/embed/zERDftjl6k8" title="YouTube video player" frameborder="0" allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture" allowfullscreen></iframe>

## Testing with Clarinet

Clarinet provides several powerful methods to test and interact with your smart contracts. As mentioned in the previous
section, you can always check your Clarity syntax using the `clarinet check` command. This validates any smart contracts
you are currently developing in the active project.

There are two tools in Clarinet you can use to test smart contracts: the console, an interactive Clarity REPL, and the test harness, a testing framework written in Typescript.

### Testing with the console

The Clarinet console is an interactive Clarity REPL that runs in-memory. Any contracts configured in the current project are automatically loaded into memory. Additionally, wallets defined in the `settings/Devnet.toml` file are initialized with STX tokens for testing purposes. When the console runs, it provides a summary of the deployed contracts, their public functions, as well as wallet addresses and balances.

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
::help                            Display help
::list_functions                  Display all the native functions available in Clarity
::describe_function <function>    Display documentation for a given native function fn-name
::mint_stx <principal> <amount>   Mint STX balance for a given principal
::set_tx_sender <principal>       Set tx-sender variable to principal
::get_assets_maps                 Get assets maps for active accounts
::get_costs <expr>                Display the cost analysis
::get_contracts                   Get contracts
::get_block_height                Get current block height
::advance_chain_tip <count>       Simulate mining of <count> blocks
```

The console commands control the state of the REPL chain, and let you get information about it and advance the chain
tip. Additionally, you can enter Clarity commands into the console and observe the result of the command. The
`::list_functions` console command prints a cheat sheet of Clarity commands. For example, in the example contract,
you could use the REPL to call the `echo-number` function in the contract with the following command:

```
>> (contract-call? .my-contract echo-number 42)
(ok 42)
```

Note that by default commands are always executed as the `deployer` address, which means you can use the shorthand
`.my-contract` without specifying a full address to the contract. If you changed the transaction address with the
`::set_tx_sender` command, you would need to provide the full address to the contract in the contract call
(`ST1HTBVD3JG9C05J7HBJTHGR0GGW7KXW28M5JS8QE.my-contract`).

You can refer to the [Clarity language reference](https://docs.stacks.co/docs/write-smart-contracts/clarity-language/) for a complete overview of all Clarity functions.

### Testing with the test harness

The test harness is a Deno testing library that can simulate the blockchain, exercise functions of the contract, and
make testing assertions about the state of the contract or chain.

You can run any tests configured in the `tests` directory with the command:

```sh
clarinet test
```

When you create a new contract, a test suite is automatically created for it. You can populate the test suite with
unit tests as you develop the contract.

An example unit test for the `echo-number` function is provided below:

```ts
...
Clarinet.test({
  name: 'the echo-number function returns the input value ok',
  async fn(chain: Chain, accounts: Map<string, Account>) {
    const testNum = '42';
    let deployerWallet = accounts.get('deployer')!;
    let block = chain.mineBlock([
      Tx.contractCall(
        `${deployerWallet.address}.my-contract`,
        'echo-number',
        [testNum],
        deployerWallet.address,
      ),
    ]);
    assertEquals(block.receipts.length, 1); // assert that the block received a single tx
    assertEquals(block.receipts[0].result, `(ok ${testNum})`); // assert that the result of the tx was ok and the input number
    assertEquals(block.height, 2); // assert that only a single block was mined
  },
});
```

For more information on assertions, review [asserts](https://deno.land/std@0.90.0/testing/asserts.ts) in the Deno standard library. For more information on the available Clarity calls in Deno, review the [Deno Clarinet library](https://github.com/hirosystems/clarinet/blob/develop/components/clarinet-deno/index.ts).

## Additional reading

- [Clarinet README](https://github.com/hirosystems/clarinet#clarinet)
- [clarinet repository](https://github.com/hirosystems/clarinet)
- [Clarity language reference](https://docs.stacks.co/references/language-functions)
- [Deno standard library - asserts](https://deno.land/std@0.90.0/testing/asserts.ts)
- [Clarity visual studio code plugin](https://marketplace.visualstudio.com/items?itemName=HiroSystems.clarity-lsp)



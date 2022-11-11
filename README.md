# Clarinet

Clarinet is a Clarity runtime packaged as a command line tool, designed to facilitate smart contract understanding,
development, testing and deployment. Clarinet consists of a Clarity Read-Evaluate-Print-Loop (REPL) environment and a testing harness. When used together, Clarity and the REPL environment enables you to rapidly develop and test a Clarity smart contract, allowing you to deploy the contract to a:

- devnet - a local a standalone development environment that simulates Bitcoin, Stacks node and other helpful components, similar to a staging environment.
- [testnet](https://docs.stacks.co/docs/understand-stacks/testnet) - a testing environment not running in production.
- [mainnet](https://stacks.org/stacks2mainnet) - a production environment where you can deploy smart contracts.

Clarity is a **decidable** smart contract language that optimizes for predictability and security, meaning you can know with certainty, from the code itself, what the program will do. Smart contracts allow you to encode essential business logic on a blockchain.

![screenshot](docs/images/demo.gif)

### Clarinet 101

Hiro has created an introductory video tutorial series that guides you through some of the fundamentals of of Clarinet, and how it can help develop, test, and deploy Clarity smart contracts.

Please revew and watch the YouTube playlist on [Hiro's Youtube](https://www.youtube.com/c/HiroSystems):
[<img src="docs/images/clarinet101.png">](https://youtube.com/playlist?list=PL5Ujm489LoJaAz9kUJm8lYUWdGJ2AnQTb) channel for information on how to use Clarinet for smart contracts.

## Installation

### Install on macOS (Homebrew)

To install Clarinet on macOS, run the following command:

```bash
brew install clarinet
```

### Install on Windows

If you would like to install Clarinet on Windows, the easiest way to install Clarinet on Windows is to use the MSI installer. You can download the executable from the [releases page](https://github.com/hirosystems/clarinet/releases).

Clarinet is also available on Winget; the package manager that Microsoft began including in the latest Windows updates:

```powershell
winget install clarinet
```

### Install from a pre-built binary

To install Clarinet from pre-built binaries, download the latest release from the [releases page](https://github.com/hirosystems/clarinet/releases).

Unzip the binary, then copy it to a location that is already in your path, such as `/usr/local/bin`.

```sh
# note: you can change the v0.27.0 with version that are available in the releases page.
wget -nv https://github.com/hirosystems/clarinet/releases/download/v0.27.0/clarinet-linux-x64-glibc.tar.gz -O clarinet-linux-x64.tar.gz
tar -xf clarinet-linux-x64.tar.gz
chmod +x ./clarinet
mv ./clarinet /usr/local/bin
```

On MacOS, you may get security errors when trying to run the pre-compiled binary. You can resolve the security warning
with with command

```sh
xattr -d com.apple.quarantine /path/to/downloaded/clarinet/binary
```

### Install from source using Cargo

If you would like to install Clarinet from source using Cargo, there are some specific steps you will need to follow, which are described below.

#### Prerequisites

You must first [Install Rust](https://www.rust-lang.org/tools/install) to use the Rust package manager Cargo.

If you are using Debian and Ubuntu-based distributions, make sure to run the following command to install required packages before building Clarinet.

```bash
sudo apt install build-essential pkg-config libssl-dev
```

#### Build Clarinet

When you are ready to build Clarinet, you can build from source using Cargo with the following commands:

```bash
git clone https://github.com/hirosystems/clarinet.git
cd clarinet
cargo clarinet-install
```

By default, you will be in Hiro's development branch, `develop`, with code that has not been released yet. If you plan on submitting any changes to the code, then this is the right branch for you.
If you want the latest stable version, switch to the main branch by entering the following command:

```bash
git checkout main
```

## Getting started with Clarinet

The following sections describe how to create a new project in Clarinet and populate it with smart contracts. Clarinet
also provides tools for interacting with your contracts in a REPL, and performing automated testing of contracts.

### Setup shell completions

Clarinet has many different built-in commands, which are useful to enable tab-completion in your shell. You can use `clarinet` to generate the shell completion scripts for many common shells using the command:

```sh
clarinet completions (bash|elvish|fish|powershell|zsh)
```

After generating the file, please refer to the documentation for your shell to determine where this file should be moved and what other steps may be necessary to enable tab-completion for `clarinet`.

### Create a new project

Once you have installed Clarinet, you can create a new project by entering the following command:

```bash
clarinet new my-project && cd my-project
```

Clarinet will create a project directory with the following directory layout:

```bash
.
├── Clarinet.toml
├── contracts
├── settings
│   └── Devnet.toml
│   └── Testnet.toml
│   └── Mainnet.toml
└── tests
```

The `Clarinet.toml` file contains configuration for the smart contracts in your project. When you create contracts in
your project, Clarinet will add them to this file.

The `settings/Devnet.toml` file contains configuration for accounts in the Clarinet console, including the seed
phrases and initial balances. Initial balances are in microSTX.

For a detailed video description on how you can create a new project, please see the [Creating a New Project](https://www.youtube.com/watch?v=F_Sb0sNafEg&list=PL5Ujm489LoJaAz9kUJm8lYUWdGJ2AnQTb&index=4) YouTube video.

### Add a new contract

Clarinet can handle adding a new contract and its configuration to your project with the following command:

```bash
$ clarinet contract new bbtc
```

Clarinet will add 2 files to your project: the contract file in the `contracts` directory, and the contract test file
in the `tests` directory.

```bash
.
├── Clarinet.toml
├── contracts
│   └── bbtc.clar
├── settings
│   └── Devnet.toml
│   └── Mainnet.toml
│   └── Testnet.toml
└── tests
    └── bbtc_test.ts
```

Clarinet will also add configuration to the `Clarinet.toml` file for your contract.

```toml
[project.cache_location]
path = ".cache"
[contracts.bbtc]
path = "contracts/bbtc.clar"
```

You may add contracts to your project by adding the files manually; however, you must add the appropriate configuration
to `Clarinet.toml` in order for Clarinet to recognize the contracts.

### Check your contracts

Clarinet provides syntax and semantics checkers for Clarity, which enable you to check if the Clarity code in your project is valid by using the following command:

```bash
$ clarinet check
```

This command uses the `Clarinet.toml` file to locate and analyze all of the contracts in the project. If the Clarity code is valid, the command will indicate success with the following message:

```
✔ 2 contracts checked
```

The checker may also report warnings that indicate the code is valid; however, you should be aware of a specific condition that might arise. For example, the check-checker analysis discussed below will generate warnings. If there are errors in the code, the output of the command will indicate the kind and location of the errors.

You may also perform syntax-check on a single file by using the following command.

```bash
$ clarinet check <path/to/file.clar>
```

If there are no syntax errors, the output of the command will be a success message similar to the example below.

```
✔ Syntax of contract successfully checked
```

**Note** Any syntactical errors in the Clarity code will be reported, but type-checking and other semantic checks will not be performed because Clarinet will only look at this one contract, since it does not have the full context to perform a complete check.

### Static Analysis

#### Check-Checker

The check-checker is a static analysis pass you can use to help find potential vulnerabilities in your contracts. To enable this pass, add the following lines to your `Clarinet.toml` file:

```toml
[repl.analysis]
passes = ["check_checker"]
```

The check-checker pass analyzes your contract to identify places where untrusted inputs might be used in a potentially dangerous way. Since public functions can be called by anyone, any arguments passed to these public functions should be considered untrusted. This analysis pass takes the opinion that all untrusted data must be checked before being used to modify the state on the blockchain. Modifying the state includes any operations that affect wallet balances, or any data stored in your contracts.

- Actions on Stacks wallets:
  - stx-burn?
  - stx-transfer?
- Actions on fungible tokens:
  - ft-burn?
  - ft-mint?
  - ft-transfer?
- Actions on non-fungible tokens:
  - nft-burn?
  - nft-mint?
  - nft-transfer?
- Actions on persisted data:
  - Maps:
    - map-delete
    - map-insert
    - map-set
  - Variables:
    - var-set

In addition to those operations, the check-checker is also a bit opinionated and prefers that untrusted data be checked near the source, making the code more readable and maintainable. For this reason, the check-checker also requires that arguments passed into private functions and return values must be checked.

- Calls to private functions
- Return values

Finally, another opportunity for exploits appears when contracts call functions from traits. Those traits are untrusted, just like other parameters to public functions, so they are also required to be checked.

- Dynamic contract calls (through traits)

When an untrusted input is used in one of these ways, you will see a warning like this:

```
bank:27:37: warning: use of potentially unchecked data
        (as-contract (stx-transfer? (to-uint amount) tx-sender customer))
                                    ^~~~~~~~~~~~~~~~
bank:21:36: note: source of untrusted input here
(define-public (withdrawal-unsafe (amount int))
```

In the case where an operation affects only the sender's own wallet (e.g. calling `stx-transfer?` with the sender set to `tx-sender`), then there is no need to generate a warning, because the untrusted input is only affecting the sender, who is the source of that input. To put it another way, the sender should be able to safely specify parameters in an operation that affects only themselves. This sender is also potentially protected by post-conditions.

For a more detailed description on how to use the Check-Checker, please see the [Catch Smart Contract Vulnerabilities With Clarinet’s Check-Checker Feature](https://www.youtube.com/watch?v=v2qXFL2owC8&list=PL5Ujm489LoJaAz9kUJm8lYUWdGJ2AnQTb&index=14) YouTube video.

##### Options

The check-checker provides some options that can be specified in `Clarinet.toml` to handle common usage scenarios that may reduce false positives from the analysis:

```toml
[repl.analysis.check_checker]
strict = false
trusted_sender = true
trusted_caller = true
callee_filter = true
```

If `strict` is set to true, all other options are ignored and the analysis proceeds with the most strict interpretation of the rules.

The `trusted_sender` and `trusted_caller` options handle a common practice in smart contracts where there is a concept of a trusted transaction sender (or transaction caller), which is treated like an admin user. Once a check has been performed to validate the sender (or caller), then all inputs should be trusted.

In the example below, the `asserts!` on line 3 is verifying the `tx-sender`. Because of that check, all inputs are trusted (if the `trusted_sender` option is enabled):

```clarity
(define-public (take (amount int) (from principal))
    (let ((balance (- (default-to 0 (get amount (map-get? accounts {holder: from}))) amount)))
        (asserts! (is-eq tx-sender (var-get bank-owner)) err-unauthorized)
        (map-set accounts {holder: from} {amount: balance})
        (stx-transfer? (to-uint amount) (as-contract tx-sender) tx-sender)
    )
)
```

The `callee_filter` option loosens the restriction on passing untrusted data to private functions, and instead, allows checks in a called function to propagate up to the caller. This is helpful, because it allows you to define input checks in a function that can be reused.

In the example below, the private function `validate` checks its parameter. The public function `save` calls `validate`, and when the `callee_filter` option is enabled, that call to `validate` will count as a check for the untrusted input, `amount`, resulting in no warnings from the check-checker.

```clarity
(define-public (save (amount uint))
    (begin
        (try! (validate amount))
        (var-set saved amount)
        (ok amount)
    )
)

(define-private (validate (amount uint))
    (let ((current (var-get saved)))
        (asserts! (> amount current) err-too-low)
        (asserts! (<= amount (* current u2)) err-too-high)
        (ok amount)
    )
)
```

##### Annotations

Sometimes, there is code that the check-checker analysis is unable to determine is safe; however, you know the code is safe. You want to pass this information to the check-checker to disable warnings that you consider to be false positives. To handle these cases, the check-checker supports several annotations, implemented using "magic comments" in the contract code.

**`#[allow(unchecked_params)]`**

This annotation tells the check-checker that the associated private function is allowed to receive unchecked arguments. It will not generate a warning for calls to this function that pass unchecked inputs. Inside the private function, the parameters are considered unchecked and could generate warnings.

```clarity
;; #[allow(unchecked_params)]
(define-private (my-func (amount uint))
    ...
)
```

**`#[allow(unchecked_data)]`**

This annotation tells the check-checker that the following expression is allowed to use unchecked data without warnings. It should be used with care, since this will disable all warnings from the associated expression.

```clarity
(define-public (dangerous (amount uint))
    (let ((sender tx-sender))
        ;; #[allow(unchecked_data)]
        (as-contract (stx-transfer? amount tx-sender sender))
    )
)
```

**`#[filter(var1, var2)]`**

This annotation tells the check-checker to consider the specified variables to be checked by the following expression. This is useful for the case where your contract does some indirect check that validates that an input is safe, but there is no way for the analysis to recognize this. In place of the list of variable names in the annotation, an `*` may be used to filter all inputs.

**Note** The command below is the safest and preferred way to silence warnings that you consider false positives.

```clarity
(define-public (filter_one (amount uint))
    (let ((sender tx-sender))
        ;; #[filter(amount)]
        (asserts! (> block-height u1000) (err u400))
        (as-contract (stx-transfer? amount tx-sender sender))
    )
)
```

### Execute a test suite

Clarinet provides a testing harness based on Deno that can enable you to create automated unit tests or pseudo-integration tests using Typescript.

```bash
$ clarinet test
```

For more information on how to create unit tests using Typescript, see the [Writing Unit Tests Using Typescript](https://www.youtube.com/watch?v=Z4YEHUxHWuE&list=PL5Ujm489LoJaAz9kUJm8lYUWdGJ2AnQTb&index=7) YouTube video.

When you use the `clarinet contract new foo` command to create a new contract, Clarinet will automatically create a unit test file for this new contract, `tests/foo_test.ts`. Other files under the `tests/_ directory` following the Deno test naming convention will also be included:

- named test.{ts, tsx, mts, js, mjs, jsx, cjs, cts},
- or ending with .test.{ts, tsx, mts, js, mjs, jsx, cjs, cts},
- or ending with \_test.{ts, tsx, mts, js, mjs, jsx, cjs, cts}

Within these tests, you can simulate mining a block containing transactions using your contract, and then examine the results of those transactions as well as the events generated by them.

See the [billboard example](examples/billboard/tests/billboard_test.ts) for sample unit tests.

**Note:** If you see an error in VS Code on the imports in the generated test file(s), that says, "An import path cannot end with a '.ts' extension" (an example is shown below), installing the [Deno extension](https://marketplace.visualstudio.com/items?itemName=denoland.vscode-deno) will resolve this error.

![VS Code deno error](docs/images/deno-error.png)

#### Measure and increase code coverage

To help maximize a smart contract's test coverage, Clarinet can produce a `lcov` report, using the following option:

```bash
$ clarinet test --coverage
```

From there, you can use the `lcov` tooling suite to produce HTML reports:

```bash
$ brew install lcov
$ genhtml coverage.lcov
$ open index.html
```

![lcov](docs/images/lcov.png)

### Cost optimizations

Clarinet can also be used to optimize costs. When executing a test suite, Clarinet will keep track of all the costs being computed when executing the `contract-call`, and display the most expensive ones in a table:

```bash
$ clarinet test --cost
```

The `--cost` option can be used in conjunction with `--watch` and filters to maximize productivity, as illustrated here:

![costs](docs/images/costs.gif)

### Load contracts in a console

The Clarinet console is an interactive Clarity REPL environment that runs in-memory. Any contracts in the current project will be
automatically loaded into memory.

```bash
$ clarinet console
```

You can use the `::help` command in the console for a list of valid commands, which can control the state of the
REPL chain, and allow you advance the chain tip. Additionally, you can enter Clarity commands into the console and observe
the result of the command.

You can exit the console by pressing `Ctrl + C` twice.

Changes to contracts are not loaded into the console while it is running. If you make any changes to your contracts you
must exit the console and run it again.

### Spawn a local Devnet

You can use Clarinet to deploy your contracts to your own local offline environment for testing and
evaluation on a blockchain by using the following command:

```bash
$ clarinet integrate
```

**Note** Make sure you have a working installation of Docker running locally.

### Interacting with contracts deployed on Mainnet

Composition and interactions between protocols and contracts are one of the key innovations in blockchains. Clarinet was designed to handle these types of interactions.

Before referring to contracts deployed on Mainnet, these contracts should be explicitly be listed as a `requirement` in the manifest `Clarinet.toml`, either manually:

```toml
[project]
name = "my-project"
[[project.requirements]]
contract_id = "SP2KAF9RF86PVX3NEE27DFV1CQX0T4WGR41X3S45C.bitcoin-whales"

```

or with the command:

```bash
clarinet requirements add SP2KAF9RF86PVX3NEE27DFV1CQX0T4WGR41X3S45C.bitcoin-whales
```

Clarinet will be able to resolve the `contract-call?` statements invoking requirements present in your local contracts by downloading and caching a copy of these contracts and using them during the execution of your test suites. All of the different features are available in `clarinet`.

When deploying your protocol to Devnet / Testnet, for the contracts involving requirements, the setting `remap_requirements` in your deployment plans must be set.

As a step-by-step example, we use here the following contract, [**bitcoin-whales**](https://explorer.stacks.co/txid/SP2KAF9RF86PVX3NEE27DFV1CQX0T4WGR41X3S45C.bitcoin-whales?chain=mainnet)

If you examine this contract, you will see that there are 3 different dependencies: two from the **same**
project (included in the same Clarinet.toml file), and one referring to a contract deployed outside of the current project.

### Same Project

In the contract snippet shown below *(line:260-265)*, there are dependencies on the contracts conversion and conversion-v2 which are included in the same `Clarinet.toml` file.

```clarity
(define-read-only (get-token-uri (token-id uint))
  (if (< token-id u5001)
    (ok (some (concat (concat (var-get ipfs-root) (unwrap-panic (contract-call? .conversion lookup token-id))) ".json")))
    (ok (some (concat (concat (var-get ipfs-root) (unwrap-panic (contract-call? .conversion-v2 lookup (- token-id u5001)))) ".json")))
    )
)
```

### External Deployer 

In this code snippet, there is a dependency on the `nft-trait` *(line:001)* deployed by `'SP2PABAF9FTAJYNFZH93XENAJ8FVY99RRM50D2JG9`.

```clarity
(impl-trait 'SP2PABAF9FTAJYNFZH93XENAJ8FVY99RRM50D2JG9.nft-trait.nft-trait)
```

- Dependencies from **external** contracts should be set in `[[project.requirements]]`

- Dependencies from **internal** contracts no longer need to be set in `depends_on`; however, this is still present in many contracts, tutorials and documentations. 

```toml
[project]
name = "my-project"

[[project.requirements]]
contract_id = "SP2PABAF9FTAJYNFZH93XENAJ8FVY99RRM50D2JG9.nft-trait"

[project.cache_location]
path = ".cache"

[contracts.bitcoin-whales]
path = "contracts/bitcoin-whales.clar"
# depends_on = ["conversion","conversion-v2"] # no longer needed, ignored if provided

[contracts.conversion]
path = "contracts/conversion.clar"

[contracts.conversion-v2]
path = "contracts/conversion-v2.clar"

[repl.analysis]
passes = ["check_checker"]

[repl.analysis.check_checker]
strict = false
trusted_sender = false
trusted_caller = false
callee_filter = false
```

As a next step we can generate a deployment plan for this project. If you are running `$ clarinet integrate` for the first time, this file should be created by Clarinet. In addition, you can run `$ clarinet deployment generate --devnet` to create or overwrite the file.

```yaml
---
id: 0
name: Devnet deployment
network: devnet
stacks-node: "http://localhost:20443"
bitcoin-node: "http://devnet:devnet@localhost:18443"
plan:
  batches:
    - id: 0
      transactions:
        - requirement-publish:
            contract-id: SP2PABAF9FTAJYNFZH93XENAJ8FVY99RRM50D2JG9.nft-trait
            remap-sender: ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM
            remap-principals:
              SP2PABAF9FTAJYNFZH93XENAJ8FVY99RRM50D2JG9: ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM
            cost: 4680
            path: ".requirements\\SP2PABAF9FTAJYNFZH93XENAJ8FVY99RRM50D2JG9.nft-trait.clar"
        - contract-publish:
            contract-name: conversion
            expected-sender: ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM
            cost: 340250
            path: "contracts\\conversion.clar"
            anchor-block-only: true
        - contract-publish:
            contract-name: conversion-v2
            expected-sender: ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM
            cost: 351290
            path: "contracts\\conversion-v2.clar"
            anchor-block-only: true
        - contract-publish:
            contract-name: bitcoin-whales
            expected-sender: ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM
            cost: 87210
            path: "contracts\\bitcoin-whales.clar"
            anchor-block-only: true
```

As the example above shows, Clarinet will remap the external contract to Hiro's Devnet address. In addition, it will also create a copy of the contract in the folder `requirements`

### Deploy contracts to Devnet / Testnet / Mainnet

You can use Clarinet to publish your contracts to Devnet / Testnet / Mainnet environment for testing and evaluation on a blockchain.

The first step to deploy a contract is to generate a deployment plan, with the following command:

```bash
$ clarinet deployment generate --mainnet
```

After **cautiously** reviewing (and updating if needed) the generated plan, you can use the command to handle the deployments of youe contract, acording to your deployment plan:

```bash
$ clarinet deployment apply -p <path-to-plan.yaml>
```

### Use Clarinet in your CI workflow as a GitHub Action

Clarinet may also be used in GitHub Actions as a step of your CI workflows.

You may set-up a simple workflow by adding the following steps in a file `.github/workflows/github-actions-clarinet.yml`:

```yaml
name: CI
on: [push]
jobs:
  tests:
    name: "Test contracts with Clarinet"
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - name: "Execute unit tests"
        uses: docker://hirosystems/clarinet:latest
        with:
          args: test --coverage --manifest-path=./Clarinet.toml
      - name: "Export code coverage"
        uses: codecov/codecov-action@v1
        with:
          files: ./coverage.lcov
          verbose: true
```

You may also add the steps above in your existing workflows. The generated code coverage output can then be used as is with GitHub Apps like https://codecov.io.

For more information on how you can use GitHub Actions with Clarinet, please see the [A Simple CI With Clarinet and GitHub](https://www.youtube.com/watch?v=cEv6Mi4EcKQ&list=PL5Ujm489LoJaAz9kUJm8lYUWdGJ2AnQTb&index=8) YouTube video

### Extensions

Clarinet can easily be extended by community members: open source contributions to Clarinet are welcome, but you may also write your own Clarinet extensions if you want to integrate Clarity contracts with your own tooling and workflow.

| Name                      | wallet access | disk write | disk read | Deployment                                                            | Description                                                                                                                                       |
| ------------------------- | ------------- | ---------- | --------- | --------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| stacksjs-helper-generator | no            | yes        | no        | https://deno.land/x/clarinet@v1.0.2/ext/stacksjs-helper-generator.ts | Facilitates contract integration by generating some typescript constants that can be used with stacks.js. Never hard code a stacks address again! |
|                           |               |            |           |                                                                       |

#### How to use extensions

Extensions are run with the following syntax:

```
$ clarinet run --allow-write https://deno.land/x/clarinet@v0.29.0/ext/stacksjs-helper-generator.ts
```

An extension can be deployed as a standalone plugin on Deno, or may also be a local file if it includes sensitive / private setup informations.

As illustrated in the example above, permissions (wallet / disk read / disk write) are declared using command flags. If at runtime, the Clarinet extension is trying to write to disk, read disk, or access wallets without permission, the script will fail.

### Debug your contracts

#### VS Code Debugger

Clarinet supports the [Debug Adapter Protocol](https://microsoft.github.io/debug-adapter-protocol/) (DAP) which enables you to debug your smart contracts inside of VS Code, or any code editor that supports the DAP protocol.

To setup a debug session, you will first need to create a `launch.json` file to tell VS Code what you want to debug. The easiest way to do this is to let VS Code generate the template for you by opening the "Run and Debug" view and clicking "create a launch.json file".

![Run and Debug View](docs/images/run-and-debug.png)

This will create the file `.vscode/launch.json` with the default template:

```json
{
  // Use IntelliSense to learn about possible attributes.
  // Hover to view descriptions of existing attributes.
  // For more information, visit: https://go.microsoft.com/fwlink/?linkid=830387
  "version": "0.2.0",
  "configurations": [
    {
      "type": "clarinet",
      "request": "launch",
      "name": "Call .foo.bar",
      "manifest": "${workspaceFolder}/Clarinet.toml",
      "expression": "(contract-call? .foo bar 42)"
    }
  ]
}
```

Depending on your needs, you will want to set the `name` field to whatever makes sense for your project, then set the `expression` to the Clarity expression that you would like to debug. In the case of the default example shown in the template above, the debugger would start executing the `bar` function of the `foo` contract, passing the argument `42`. Once this file is configured, the debugger works as expected for any [VS Code debugging](https://code.visualstudio.com/docs/editor/debugging).

Execution begins paused at the first expression. The debug toolbar includes buttons to continue, step over, step into, step out, restart, and stop, in that order.

![debug toolbar](docs/images/debug-toolbar.png)

Breakpoints can be set by clicking in the left gutter next to the code or using the right-click menu at a specific code location.

![breakpoint](docs/images/breakpoint.png)

Data watchpoints may also be set, by clicking the + in the Watch section of the Debug side bar and typing the contract variable to watch in the format `<principal>.<contract>.<name>` or using the shortcut for a local contract, `.<contract>.<name>`. When a watchpoint is set on a contract variable, execution will pause when its value will change.

![watchpoint](docs/images/watchpoint.png)

During execution, the values of the current contract's variables, the current function's arguments, and any local variables (i.e. from a `let` expression) are shown in the side bar. The current watchpoints are also shown with their current values. In both cases, the contents of a map are not shown, but can be queried in the Debug Console. The call stack is also updated to show the call stack of the current execution.

![view of side bar, showing variables, watchpoints, and call stack](docs/images/sidebar.png)

At any point during execution, an expression can be evaluated in the current context via the Debug Console. Just type any valid Clarity expression and hit enter to evaluate it. Upon completion, the events emitted and the return value are printed to the debug console.

![debug console](docs/images/debug-console.png)

For more information on how you can use VS Code to debug smart contracts, please see the [How to Debug Smart Contracts in Clarinet’s VS Code Extension](https://www.youtube.com/watch?v=DsLCDQSijwk&list=PL5Ujm489LoJaAz9kUJm8lYUWdGJ2AnQTb&index=15) YouTube video.

#### Command Line Debugger

Inside of the console (`clarinet console`), there is a debugger for stepping through your contracts on the command line, including support for:

- Breakpoints
  - **Source**: Break at a specific line (and optional column) of a contract (`break` or `b` command)
    ```
    b SP466FNC0P7JWTNM2R9T199QRZN1MYEDTAR0KP27.miamicoin-token:28:4
    ```
  - **Function**: Break at a specific function (`break` or `b` command)
    ```
    b .crashpunks-v2.transfer
    ```
  - **Data**: Break on read/write to a variable or map (`watch` or `w` to break on write, `rwatch` or `rw` to break on read, and `awatch` or `aw` to break on read or write)
    ```
    w contracts/SP2KAF9RF86PVX3NEE27DFV1CQX0T4WGR41X3S45C.bitcoin-whales.payout
    ```
- Step execution
  - **Step-in**: Step into the sub-expressions (`step` or `s` command)
  - **Step-out**: Complete execution of the current expression and return the result back to the parent (`finish` or `f` command)
  - **Step-over**: Continue to completion of the current expression, stepping over sub-expressions (`next` or `n` command)
  - **Continue**: Continue execution until hitting a breakpoint or completing execution (`continue` or `c` command)
- Evaluate and print the result of any expression (`print` or `p` command)

To initiate a debug session, first enter the REPL console using the following command:

```
clarinet console
```

Then, at the REPL prompt, enter this command to debug any expression:

```
::debug (contract-call? .foo hello .bar))
```

At the debug prompt, use any of the commands described above, or use `help` to get the full help documentation.

For more information about how to debug a smart contract using the command line, please see the [Debug Your Smart Contracts With Clarinet’s New Inline Debugger](https://www.youtube.com/watch?v=nVDWeuMnkDs&list=PL5Ujm489LoJaAz9kUJm8lYUWdGJ2AnQTb&index=13) YouTube video.

#### Execution Trace

When you are in the console (`clarinet console`), the `::trace <expr>` command enables you to execute an expression and print a trace of the execution, which can be very helpful for identifying problems with the contract.

This trace shows all function calls, both internal calls to private functions, and contract calls to other contracts. For each call, the parameters and return value are shown in the trace. Any events that are emitted are also shown in the trace.

![execution trace](docs/images/trace.png)

### Deploy with Hyperchains on Devnet

Clarinet can be used for facilitating experimentations with [Hyperchains](https://www.youtube.com/watch?v=PFPwuVCGGuI).
To begin working with subnets, in your `Devnet.toml`, enable the following flag:

```toml
[devnet]
# ...
enable_subnet_node = true
```

This same file may also be used for customizing the subnet-node (miner, etc).

When running the command:

```bash
$ clarinet integrate
```

Clarinet will spin-up a subnet node. More documentation on how to use and interact with this incoming L2 can be found on the [Hyperchain repository](https://github.com/hirosystems/stacks-subnets).

## Contributing to Clarinet

Contribtions are welcome and appreciated. The following sections provide information on how you can contribute to Clarinet.

### Prerequisites

Before contributing to Clarinet, please ensure you meet the following requirements:

- rust (>=1.52.0)
- cargo (>=1.52.0)
- node (>=v14.16.0) - Used for git commit hook
- npm (>=7.18.0) - Used for git commit hook

### Guide

This repo follows the [Conventional Commit](https://www.conventionalcommits.org/en/v1.0.0/#summary) specification when writing commit messages.

**Note**It is important that any pull requests you submit have commit messages that follow this standard.

To start contributing:

1. Fork this repo and clone the fork locally.
2. Create a new branch
   ```bash
   git checkout -b <my-branch>
   ```
3. Run `npm i` in the local repo to install and initialize `husky` and `commitlint`.

   ```bash
   npm i
   ```

4. These tools will be used in a `git commit` hook to lint and validate your commit message. If the message is invalid, `commitlint` will alert you to try again and fix it.

      Here is an example of a bad message response:

      ```bash
      $ git commit -m "bad message"
      $ ⧗   input: bad message
      $ ✖   subject may not be empty [subject-empty]
      $ ✖   type may not be empty [type-empty]
      $
      $ ✖   found 2 problems, 0 warnings
      $ ⓘ   Get help: https://github.com/conventional-changelog/commitlint/#what-is-commitlint
      $
      $ husky - commit-msg hook exited with code 1 (error)
      ```

      Here is an example of a good message response:

      ```bash
      $ git commit -m "fix: added missing dependency"
      $ [my-branch 4c028af] fix: added missing dependency
      $ 1 file changed, 50 insertions(+)
      ```

5. After making your changes, ensure the following:
   -  `cargo build` runs successfully.
   -  `cargo test` runs successfully.
   -  You have formatted your code with `cargo fmt --all --`
   -  All functional tests in the `examples` directory pass.
      ```bash
      for testdir in $(ls examples); do
          pushd examples/${testdir}
              ../../target/debug/clarinet test .
          popd
      done
      ```
6. Submit a pull request against the `develop` branch for review.

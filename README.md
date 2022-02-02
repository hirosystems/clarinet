# Clarinet

Clarinet is a Clarity runtime packaged as a command line tool, designed to facilitate smart contract understanding,
development, testing and deployment. Clarinet consists of a Clarity REPL and a testing harness, which, when used
together allow you to rapidly develop and test a Clarity smart contract, with the need to deploy the contract to a local
devnet or testnet.

Clarity is a **decidable** smart contract language that optimizes for predictability and security, designed for
the Stacks blockchain. Smart contracts allow developers to encode essential business logic on a blockchain.

![screenshot](docs/images/demo.gif)

### Clarinet 101

An introductory video tutorial series, from Hiro Engineer [Ludo Galabru](https://twitter.com/ludovic?lang=en), that guides developers through some of the fundamentals of of Clarinet, and how it can help develop, test, and deploy Clarity smart contracts. 


Check out the playlist on [Hiro's Youtube](https://www.youtube.com/c/HiroSystems):
[<img src="clarinet101.png">](https://youtube.com/playlist?list=PL5Ujm489LoJaAz9kUJm8lYUWdGJ2AnQTb)

## Installation


### Install on macOS (Homebrew)

```bash
brew install clarinet
```

### Install on Windows

The easiest way to install Clarinet on Windows is to use the MSI installer, that can be downloaded from the [releases page](https://github.com/hirosystems/clarinet/releases).

Clarinet is also available on Winget, the package manager that Microsoft started including in the latest Windows updates:

```powershell
winget install clarinet
```

### Install from a pre-built binary

To install Clarinet from pre-built binaries, download the latest release from the [releases page](https://github.com/hirosystems/clarinet/releases).
Unzip the binary, then copy it to a location that is already in your path, such as `/usr/local/bin`.

```sh
unzip clarinet-linux-x64.zip -d .
chmod +x ./clarinet
mv ./clarinet /usr/local/bin
```

On MacOS, you may get security errors when trying to run the pre-compiled binary. You can resolve the security warning
with with command

```sh
xattr -d com.apple.quarantine /path/to/downloaded/clarinet/binary
```

### Install from source using Cargo

#### Prerequisites

[Install Rust](https://www.rust-lang.org/tools/install) for access to `cargo`, the Rust package manager.

On Debian and Ubuntu-based distributions, please install the following packages before building Clarinet.

```bash
sudo apt install build-essential pkg-config libssl-dev
```

#### Build Clarinet

You can build Clarinet from source using Cargo with the following commands:

```bash
git clone https://github.com/hirosystems/clarinet.git --recursive
cd clarinet
cargo install --path . --locked
```

By default, you will be in our development branch, `develop`, with code that has not been released yet. If you plan to submit any changes to the code, then this is the right branch for you. If you just want the latest stable version, switch to the main branch:

```bash
git checkout main
```

If you have previously checked out the source, ensure you have the latest code (including submodules) before building using:
```
git pull
git submodule update --recursive
```

## Getting started with Clarinet

The following sections describe how to create a new project in Clarinet and populate it with smart contracts. Clarinet
also provides tools for interacting with your contracts in a REPL, and performing automated testing of contracts.

### Create a new project

Once installed, you can use clarinet to create a new project:

```bash
clarinet new my-project && cd my-project
```

Clarinet will create a project directory with the following directory layout:

```bash
.
├── Clarinet.toml
├── README.md
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

### Add a new contract

Clarinet can handle adding a new contract and its configuration to your project with the following command:

```bash
$ clarinet contract new bbtc
```

Clarinet will add 2 files to your project, the contract file in the `contracts` directory, and the contract test file
in the `tests` directory.

```bash
.
├── Clarinet.toml
├── README.md
├── contracts
│   └── bbtc.clar
├── settings
│   └── Devnet.toml
│   └── Mainnet.toml
│   └── Testnet.toml
└── tests
    └── bbtc_test.ts
```

Clarinet will also add configuration to the `Clarinet.toml` file for your contract. You add entries to the `depends_on`
field for each contract to indicate any contract dependencies a particular contract may have. This can be useful for
contracts that implement standard traits such as for fungible tokens.

```toml
[project]
name = "my-project"
requirements = []

[contracts.bbtc]
path = "contracts/bbtc.clar"
depends_on = []
```

You can add contracts to your project by adding the files manually, however you must add the appropriate configuration
to `Clarinet.toml` in order for Clarinet to recognize the contracts.

### Check the syntax of your contracts

Clarinet provides a syntax checker for Clarity. You can check if your Clarity code is valid with the command:

```bash
$ clarinet check
```

If the Clarity code is valid, the command will return no output. If there are errors in the code, the output of the
command will indicate where the errors are present.

### Execute a test suite

Clarinet provides a testing harness based on Deno that can allow you to create automated unit tests or
pseudo-integration tests using Typescript. Create tests in the appropriate test file, then execute them with the
command:

```bash
$ clarinet test
```

#### Measure and increase cost coverage

To help developers maximizing their test coverage, Clarinet can produce a `lcov` report, using the following option:
```bash
$ clarinet test --coverage
```

From there, developers can use the `lcov` tooling suite to produce HTML reports:

```bash
$ brew install lcov
$ genhtml coverage.lcov
$ open index.html
```

![lcov](docs/images/lcov.png)


### Cost optimizations

Clarinet can also be use for optimizing costs. When executing a test suite, Clarinet will keep track of all the costs being computed when executing the `contract-call`, and display the most expensive ones in a table:

```bash
$ clarinet test --cost
```

The `--cost` option can be used in conjunction with `--watch` and filters to maximize productivity, as illustrated here:

![costs](docs/images/costs.gif)


### Load contracts in a console

The Clarinet console is an interactive Clarity REPL that runs in-memory. Any contracts in the current project are
automatically loaded into memory.

```bash
$ clarinet console
```

You can use the `::help` command in the console for a list of valid commands, which can control the state of the
REPL chain, and let you advance the chain tip. Additionally, you can enter Clarity commands into the console and observe
the result of the command.

You can exit the console by pressing `Ctrl + C` twice.

Changes to contracts are not loaded into the console while it is running. If you make any changes to your contracts you
must exit the console and run it again.

### Spawn a local Devnet

You can use Clarinet to deploy your contracts to your own local offline environment for testing and
evaluation on a blockchain. Use the following command:

```bash
$ clarinet integrate
```

Make sure that you have a working installation of Docker running locally.


### Deploy contracts to Testnet

You can use Clarinet to deploy your contracts to the public Testnet environment for testing and
evaluation on a blockchain. Use the following command:

```bash
$ clarinet deploy --testnet
```

### Use Clarinet in your CI workflow as a GitHub Action

Clarinet can be used in GitHub Actions as a step of your CI workflows. 
You can set-up a simple workflow by adding the following steps in a file `.github/workflows/github-actions-clarinet.yml`:

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

Or add the steps above in your existing workflows.
The generated code coverage output can then be used as is with GitHub Apps like https://codecov.io.

### Extensions

Clarinet can easily be extended by community members: open source contributions to clarinet are welcome, but developers can also write their own clarinet extensions if they want to integrate clarity contracts with their own tooling and workflow.


| Name                          | wallet access | disk write | disk read | Deployment | Description |
| --- | --- | --- | --- | --- | --- |
| stacksjs-helper-generator | no | yes | no | https://deno.land/x/clarinet@v0.16.0/ext/stacksjs-helper-generator.ts | Facilitates contract integration by generating some typescript constants that can be used with stacks.js. Never hard code a stacks address again! | 
|                               |                       |                    |                   | |

#### How to use extensions

Extensions are ran with the following syntax:

```
$ clarinet run --allow-write https://deno.land/x/clarinet@v0.15.4/ext/stacksjs-helper-generator.ts
```

An extension can be deployed as a standalone plugin on Deno, or can also just be a local file if it includes sensitive / private setup informations.
As illustrated in the example above, permissions (wallet / disk read / disk write) are declared using command flags. If at runtime, the clarinet extension is trying to write to disk, read disk, or access wallets without permission, the script will end up failing. 

## Contributing

We welcome contributions to Clarinet! The following sections provide information on how to contribute.

### Prerequisites

- rust (>=1.52.0)
- cargo (>=1.52.0)
- node (>=v14.16.0) - Used for git commit hook
- npm (>=7.18.0) - Used for git commit hook

### Guide

This repo follows the [Conventional Commit](https://www.conventionalcommits.org/en/v1.0.0/#summary) spec when writing commit messages.
It's important any pull requests submitted have commit messages which follow this standard.

To start contributing:

1. Fork this repo and clone the fork locally.
1. Create a new branch
   ```bash
   git checkout -b <my-branch>
   ```
1. Run `npm i` in the local repo to install and initialize `husky` and `commitlint`.

   ```bash
   npm i
   ```

   1. These tools will be used in a git commit hook to lint and validate your commit message. If the message is invalid, `commitlint` will alert you to try again and fix it.

      Bad message:

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

      Good message:

      ```bash
      $ git commit -m "fix: added missing dependency"
      $ [my-branch 4c028af] fix: added missing dependency
      $ 1 file changed, 50 insertions(+)
      ```

1. After making your changes, ensure the following:
   1. `cargo build` runs successfully
   1. `cargo test` runs successfully
   1. You've formatted your code with `cargo fmt --all --`
   1. All functional tests in the `examples` directory pass.
      ```bash
      for testdir in $(ls examples); do
          pushd examples/${testdir}
              ../../target/debug/clarinet test .
          popd
      done
      ```
1. Submit a pull request against the `develop` branch for review.

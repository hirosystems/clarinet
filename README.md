# Clarinet

Clarinet is a Clarity runtime packaged as a command line tool, designed to facilitate smart contract understanding,
development, testing and deployment. Clarinet consists of a Clarity REPL and a testing harness, which, when used
together allow you to rapidly develop and test a Clarity smart contract, with the need to deploy the contract to a local
mocknet or testnet.

Clarity is a **decidable** smart contract language that optimizes for predictability and security, designed by
Blockstack. Smart contracts allow developers to encode essential business logic on a blockchain.

![screenshot](docs/images/demo.gif)

## Installation

The recommended way to install Clarinet is through the Rust package manager, Cargo. Other installation methods are
provided in case Cargo is not available to you.

### Install from source using Cargo

You can build Clarinet from source using Cargo with the following commands:

```bash
git clone git@github.com:hirosystems/clarinet.git
cd clarinet
cargo install --path . --locked
```

### Install from Homebrew (MacOS)

The version of Clarinet installed by Homebrew is behind the official version installed by Cargo. It is not currently
recommended to install Clarinet using Homebrew.

```bash
brew install hirosystems/clarinet/clarinet
```

Feel free to ⭐️ this repo! With 50+ stars, this package becomes eligible to `homebrew-core`, at which point it will be
possible automate the repo to keep the version of Clarinet installed by Homebrew up to date.

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
│   └── Development.toml
│   └── Mocknet.toml
└── tests
```

The `Clarinet.toml` file contains configuration for the smart contracts in your project. When you create contracts in
your project, Clarinet will add them to this file.

The `settings/Development.toml` file contains configuration for accounts in the Clarinet console, including the seed
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
│   └── bbtc.clar
├── settings
│   └── Development.toml
│   └── Mocknet.toml
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

You can review the available testing commands in the [Deno Clarity library](https://deno.land/x/clarinet@v0.13.0/index.ts).

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

### Deploy contracts to mocknet

If you are running a local mocknet, you can use Clarinet to deploy your contracts to that environment for testing and
evaluation on a blockchain. Use the following command:

```bash
$ clarinet deploy --mocknet
```

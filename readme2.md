# Clarinet: Simplifying Clarity Smart Contract Development

Clarinet is a powerful tool designed to streamline the process of developing, testing, and deploying Clarity smart contracts. With a built-in Clarity Read-Evaluate-Print-Loop (REPL) environment and testing harness, Clarinet empowers developers to create robust contracts and efficiently interact with different blockchain networks.

## Supported Networks

Clarinet supports the following networks for deploying your Clarity smart contracts:

- **devnet**: A local standalone development environment that simulates Bitcoin, Stacks node, and other components, providing a staging environment for testing.
- [**testnet**](https://docs.stacks.co/docs/understand-stacks/testnet): A non-production testing environment.
- [**mainnet**](https://stacks.org/stacks2mainnet): The production environment for deploying finalized smart contracts.

## Installation Options

Choose one of the following methods to install Clarinet on your machine.

<div class="tab-container">
  <button class="tab-button" onclick="showTab('tab1')">Tab 1</button>
  <button class="tab-button" onclick="showTab('tab2')">Tab 2</button>
  <button class="tab-button" onclick="showTab('tab3')">Tab 3</button>
</div>

<div id="tab1" class="tab-content">
  This is the content of Tab 1.
</div>

<div id="tab2" class="tab-content" style="display: none;">
  This is the content of Tab 2.
</div>

<div id="tab3" class="tab-content" style="display: none;">
  This is the content of Tab 3.
</div>

<script>
  function showTab(tabId) {
    const tabs = document.querySelectorAll('.tab-content');
    tabs.forEach(tab => {
      if (tab.id === tabId) {
        tab.style.display = 'block';
      } else {
        tab.style.display = 'none';
      }
    });
  }
</script>

<style>
  .tab-container {
    display: flex;
  }

  .tab-button {
    padding: 8px 16px;
    background-color: #ddd;
    border: none;
    cursor: pointer;
  }

  .tab-content {
    margin-top: 16px;
    border: 1px solid #ddd;
    padding: 16px;
  }
</style>


### Install on macOS (Homebrew)

```bash
brew install clarinet
```

### Install on Windows

Install Clarinet using the MSI installer available on the releases page. Alternatively, you can use the Windows package manager Winget:

```powershell
winget install clarinet
```

### Install from Pre-built Binary

1. Download the latest release from here.
2. Unzip the binary and copy it to a location in your path, like `/usr/local/bin`.

Example:

```sh
wget -nv https://github.com/hirosystems/clarinet/releases/download/v0.27.0/clarinet-linux-x64-glibc.tar.gz -O clarinet-linux-x64.tar.gz
tar -xf clarinet-linux-x64.tar.gz
chmod +x ./clarinet
mv ./clarinet /usr/local/bin
```

### Install from Source using Cargo

To install Clarinet from source using Cargo, follow these steps:

First [Install Rust](https://www.rust-lang.org/tools/install) to use the Rust package manager Cargo.

If you are using Debian and Ubuntu-based distributions, make sure to run the following command to install the required packages before building Clarinet.

```bash
sudo apt install build-essential pkg-config libssl-dev curl
```

## Build Clarinet

```bash
git clone https://github.com/hirosystems/clarinet.git
cd clarinet
cargo clarinet-install
```

## Getting started

Learn how to create a new project, work with smart contracts, and interact with the Clarinet console.

### Setup Shell Completions

Enable tab completion for Clarinet commands in your shell:

```sh
clarinet completions (bash|elvish|fish|powershell|zsh)
```

After generating the file, please refer to the documentation for your shell to determine where this file should be moved and what other steps may be necessary to enable tab completion for `clarinet`.

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

The `Clarinet.toml` file contains the configuration for the smart contracts in your project. When you create contracts in
your project, Clarinet will add them to this file.

The `settings/Devnet.toml` file contains configuration for accounts in the Clarinet console, including the seed phrases and initial balances. Initial balances are in microSTX.

For a detailed video description of how you can create a new project, please see the [Creating a New Project](https://www.youtube.com/watch?v=F_Sb0sNafEg&list=PL5Ujm489LoJaAz9kUJm8lYUWdGJ2AnQTb&index=4) YouTube video.

### Add a new contract

Clarinet can handle adding a new contract and its configuration to your project with the following command:

```bash
clarinet contract new my-contract
```

This command creates a new `my-contract.clar` file in the `contracts` directory and a `my-contract_test.ts` in the
`tests` directory. Additionally, it adds the contract to the `Clarinet.toml` configuration file.

```toml
[contracts.my-contract]
path = "contracts/my-contract.clar"
```

### Load contracts in a console

The Clarinet console is an interactive Clarity REPL environment that runs in memory. Any contracts in the current project will be automatically loaded into memory.

```bash
clarinet console
```

You can use the `::help` command in the console for a list of valid commands, which can control the state of the REPL chain, and allow you to advance the chain tip. Additionally, you can enter Clarity commands into the console and observe the result of the command.

Changes to contracts are not loaded into the console while it is running. If you make any changes to your contracts you must exit the console and run it again.

### Spawn a local Devnet

You can use Clarinet to deploy your contracts to your own local offline environment for testing and evaluation on a blockchain by using the following command:

> **_Note_**
>
> Make sure you have a working installation of Docker running locally.

```bash
clarinet integrate
```

### Deploy contracts to Devnet / Testnet / Mainnet

You can use Clarinet to publish your contracts to Devnet / Testnet / Mainnet environment for testing and evaluation on a blockchain.

The first step to deploying a contract is to generate a deployment plan with the following command:

```bash
clarinet deployment generate --mainnet
```

After **cautiously** reviewing (and updating if needed) the generated plan, you can use the command to handle the deployments of your contract according to your deployment plan:

```bash
clarinet deployment apply -p <path-to-plan.yaml>
```

### Use Clarinet in your CI workflow as a GitHub Action

Clarinet may also be used in GitHub Actions as a step of your CI workflows.

You may set up a simple workflow by adding the following steps in a file `.github/workflows/github-actions-clarinet.yml`:

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

You may also add the steps above to your existing workflows. The generated code coverage output can then be used as is with GitHub Apps like https://codecov.io.

For more information on how you can use GitHub Actions with Clarinet, please see the [A Simple CI With Clarinet and GitHub](https://www.youtube.com/watch?v=cEv6Mi4EcKQ&list=PL5Ujm489LoJaAz9kUJm8lYUWdGJ2AnQTb&index=8) YouTube video

### Deploy with Subnets on Devnet

Clarinet can facilitate experimentations with [Subnets](https://www.youtube.com/watch?v=PFPwuVCGGuI).
To begin working with subnets, in your `Devnet.toml`, enable the following flag:

```toml
[devnet]
# ...
enable_subnet_node = true
```

This same file may also be used for customizing the subnet-node (miner, etc).

When running the command:

```bash
clarinet integrate
```

Clarinet will spin up a subnet node. More documentation on using and interacting with this incoming L2 can be found in the [Subnets repository](https://github.com/hirosystems/stacks-subnets).

---
title: Customize Deployment
---

Deployment Plans are reproducible deployment steps that publish a collection of on-chain transactions and one or more contracts to a network, whether a local developer network, the public testnet, or into production on mainnet. Deployment plans minimize the inherent complexity of deployments, such as smart contract dependencies and interactions, transaction chaining limits, deployment costs, and more, while ensuring reproducible deployments critical for testing purposes.

*Topics covered in this guide*:

* [Understand deployment plans](#deployment-plans)
* [View sample deployment plan](#view-your-deployment-plan)
* [Set deployment plan specifications](#deployment-plans-specifications-files)
* [Add project contract requirements](#adding-project-contract-requirements)
* [Customize your deployment plan](#edit-your-deployment-plan-directly)

![deployment-plans](../images/deployment-plans.png)

## Deployment plans

Furthermore, many details of your deployment are configurable in a deployment plan's specifications. However, every Clarinet project comes with a default deployment plan, so customization is not required. For a video walkthrough of how to customize deployment plans, [watch this video](https://www.youtube.com/watch?v=YcIg5VCO98s).


| Network | Features | Limitations |
| ------- | -------- | ----------- |
| `simnet` | - Widely configurable Stacks simnet <br/> - No private keys needed (for contacts, transactions) <br/> - Stored entirely in memory | - No Bitcoin emulation |
| `devnet` | - Widely configurable Stacks and Bitcoin devnets <br/> - Publish contracts, transactions across multiple Stacks and/or Bitcoin blocks <br/> - Control chainstate for tests | - Accounts need private keys (see `Devnet.toml`) |
| `testnet`/`mainnet` | - Public networks with other users and contracts <br/> | - Time-consuming and expensive environment for testing |

## View your deployment plan

Clarinet uses a deployment plan for every deployment-whether for a basic simnet (for use in Clarinet's `console`, `test`, and `check` features), a devnet (for use with Clarinet's `integrate` command), or the public testnet and mainnet.

You can view the current deployment plan saved to disk at any time with the following command (specifying the network `simnet`, `devnet`, `testnet`, `mainnet`)

```bash
clarinet deployments generate --<network>
```

Upon this command, your project's default deployment plan, a `.yaml` file (for example, `default.devnet-plan.yaml`), is generated from your project specifications and contracts. Here, for example, is the deployment plan for [this `counter` smart contract application](https://github.com/hirosystems/clarinet/blob/develop/components/clarinet-cli/examples/counter/deployments/default.devnet-plan.yaml):

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
        - contract-publish:
            contract-name: counter
            expected-sender: ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM
            cost: 5960
            path: contracts/counter.clar
            anchor-block-only: true
            clarity-version: 1
```

## Deployment plan's specifications files 

A project's deployment plan specifications are set on two files within a Clarinet project—the `Clarinet.toml` file as well as the network’s corresponding `.toml` file (for example, `Devnet.toml`) in the `settings/` folder.

Specifications in the `Clarinet.toml` file you can change include
- The Clarity version or blockchain epoch that Clarinet emulates for your smart contracts
- Project requirements in the form of contract dependencies (see next section)

Specifications in the `Devnet.toml` file you can change include
- Accounts, balances, and names
- Blockchain environment features, such as block time or API ports, of your devnet
- [Stacking](https://docs.stacks.co/docs/understand-stacks/stacking) orders

## Adding project contract requirements

Your project can reference a smart contract that already exists on the blockchain. For example, [Stacks Improvement Proposal 009](https://github.com/stacksgov/sips/blob/main/sips/sip-010/sip-010-fungible-token-standard.md) specifies a trait reference for all non-fungible tokens. Instead of redeploying this trait as a separate contract each time a new NFT collection is published, you can reference the contract directly within deployment plans with the following Clarinet command, including the contract principal reference:

```bash
clarinet requirements add SP2PABAF9FTAJYNFZH93XENAJ8FVY99RRM50D2JG9.nft-trait
```

This command updates a Clarinet project's deployment plan for use in development. For instance, `clarinet integrate` would now download a copy of the contract principal via the Stacks API node that is booted alongside each devnet deployment, and remaps the principal that publishes this contract to one of the accounts specified in your `Devnet.toml` file. This permits a developer to maintain the minimum set of contracts their Clarinet project will deploy to mainnet (with a single use of an [`impl-trait`](https://docs.stacks.co/docs/clarity/language-functions#impl-trait) statement in their Clarity contract).

## Edit your deployment plan directly

Recall that a default deployment plan is used for every deployment, comprising your projects specifications as set in your `Clarinet.toml` and `<network>.toml` files.

The default plan is generated upon each `clarinet deployments generate --<network>` shell command to your `deployments/` folder.

However, you can also manually edit this default deployment `.yaml` file directly—programmatically orchestrating your contract publication and user interaction beyond your plan's defaults. 

:::note
When deploying, Clarinet prompts you to overwrite any manually added or edited changes that differ from the specifications laid out in your project's `.toml` files. Type `no` to proceed with your manually configured deployment plan instead of the default.
:::

Here are some of the relevant transactions and configurations available to developers manually configuring their deployment plans:
- `emulated-contract-publish`: Deploy a contract in an in-memory simulated chain (`simnet` only).
- `emulated-contract-call`: Call a contract that has been deployed in an in-memory simulated chain (simnet only).
- `requirement-publish`: Deploy an external contract (that is, one not published by the same set of private keys as the user wallet currently deploying) on another `testnet`/`devnet` using another wallet + search, and replace all the references to this contract in the local contracts to deploy (`devnet`/`testnet` only).
- `contract-publish`: Deploy a contract (`devnet`/`testnet`/`mainnet`).
- `contract-call`: Call a contract (`devnet`/`testnet`/`mainnet`).
- `stx-transfer`: Transfer STX between specified accounts.
- `btc-transfer`: Simple bitcoin transfer from a p2pkh address to a p2pkh address (experimental, `regtest`/`testnet`/`mainnet`).
- Waiting between blocks to make transactions is done by specifying transaction directives in a different batch in the deployment plan (see the `batches` field)

### `emulated-contract-publish`

This default transaction is included in every generated deployment for a `simnet` deployment. The institutional lending protocol [Zest has a code audit](https://github.com/Trust-Machines/Zest-Audit/blob/1c35ea49fff7d990aa58d045f1e3bcac5e65406c/deployments/default.simnet-plan.yaml) that includes a simnet environment

```yaml
        - emulated-contract-publish:
            contract-name: extension-trait
            emulated-sender: ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM
            path: contracts/dao/traits/extension-trait.clar
            clarity-version: 1
```

### `emulated-contract-call`

This is a transaction type for a `simnet` deployment where the developer wants to emulate a contract call.

::note: Read-only functions and functions that take no arguments are passed an empty list like so: `parameters: []`
```yaml
        - emulated-contract-call:
            contract-id: ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.counter
            emulated-sender: ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM
            method: increment
            parameters:
                - u1
```

### `requirement-publish`

This cBTC smart contract application has a [devnet deployment plan](https://github.com/hirosystems/clarinet/blob/develop/components/clarinet-cli/examples/cbtc/deployments/default.devnet-plan.yaml) that publishes the contract required by its core smart contract.

```yaml
        - requirement-publish:
            contract-id: SP3FBR2AGK5H9QBDH3EEN6DF8EK8JY7RX8QJ5SVTE.sip-010-trait-ft-standard
            remap-sender: ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM
            remap-principals:
                SP3FBR2AGK5H9QBDH3EEN6DF8EK8JY7RX8QJ5SVTE: ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM
            cost: 8400
            path: ".cache/requirements/SP3FBR2AGK5H9QBDH3EEN6DF8EK8JY7RX8QJ5SVTE.sip-010-trait-ft-standard.clar"
```

### `contract-publish`

This default transaction is included in every generated deployment plan for every contract your Clarinet project has. This "counter" application has a [devnet deployment plan](https://github.com/hirosystems/clarinet/blob/develop/components/clarinet-cli/examples/counter/deployments/default.devnet-plan.yaml) that publishes its core smart contract.

```yaml
        - contract-publish:
            contract-name: counter
            expected-sender: ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM
            cost: 5960
            path: contracts/counter.clar
            anchor-block-only: true
            clarity-version: 1
```

### `contract-call`

The developer can call a function and pass arguments of any valid Clarity type.

::note: Read-only functions and functions that take no arguments are passed an empty list like so: `parameters: []`
```yaml
        - contract-call:
            contract-id: ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.bitcoin-dao
            expected-sender: ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM
            method: add-new-member
            parameters:
                - u1
                - "0x01"
                - "{ id: u4, username: "Satoshi", address: 'SP1HTBVD3JG9C05J7HBJTHGR0GGW7KXW28M5JS8QE }"
                - "'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM"
            cost: 5960
```

### `stx-transfer`

The developer can specify the originating account of an STX transfer, the quantity denominated in microstacks, and the recipient, as well as an optional memo field (Note: the memo field can only contain a Clarity [buffer type](https://book.clarity-lang.org/ch02-02-sequence-types.html) up to 34 bytes in size, pre-fixed with `0x` (for example, `0x68656c6c6f21`), not ASCII text or strings)

```yaml
        - stx-transfer:
            expected-sender: ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM
            recipient: ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.my-contract
            mstx-amount: 1000
            memo: "0x01"
            cost: 10000
```

### `btc-transfer`

This cBTC smart contract application has a [devnet deployment plan](https://github.com/hirosystems/clarinet/blob/develop/components/clarinet-cli/examples/cbtc/deployments/wrap-btc.devnet-plan.yaml) that includes broadcasting a BTC transfer transaction to the emulated bitcoin chain with these parameters.

```yaml
        - btc-transfer:
            expected-sender: mjSrB3wS4xab3kYqFktwBzfTdPg367ZJ2d
            recipient: bcrt1qnxknq3wqtphv7sfwy07m7e4sr6ut9yt6ed99jg
            sats-amount: 100000000
            sats-per-byte: 10
```


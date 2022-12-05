---
title: Clarinet Deployment Plans
---

## Overview

Deployment plans allow teams to work together when deploying smart contracts. Hiro has created a set of primitives that enables teams collaborate more effectively, ensuring teams can create smart contracts faster and easier.

By using Deployment Plans, you can simplify the smart contract deployment process to stacks or bitcoin environment.

## Design

Deployment Plans are reproducible deployment steps that publish a “protocol”; a collection of on-chain transactions and contracts—to a network, whether it is on a local developer network, the public testnet, or into production on mainnet.

Using a Deployment Plan can minimize the inherent complexity with deployments, thereby making it much easier to deploy a contract without errors. Some of these complexities can include dependencies, chaining limits, the process to initialize a smart contract, and underlying deployment costs.

A deployment plan is made up of:

- smart contracts
- accounts with their token balances, and
- content and sequence of transactions (across a single or multiple Stacks or Bitcoin blocks)

A Deployment Plan’s specifications exist on two files within a Clarinet project: the `Clarinet.toml` and network’s `.toml` file (for example, `devnet.toml`) under the “deployments” folder.

You can commit, audit, and test contracts without including any secrets in the Deployment Plan, and share these contracts without exposing any sensitive information.

## Deployment plan primitives

Deployment plans consist of the following primitives:

- deploy contracts
- call contracts
- send bitcoin transactions
- wait for block
- send stacks to an address or contract

With these four individual primitives, you can then:

- Deploy a contract in an in-memory simulated chain (simnet only). 
- Call a contract that has been deployed in an in-memory simulated chain (simnet only).
- Deploy an external contract on another testnet / devnet network using another wallet + search, and replace all references to this contract in the local contracts to deploy (devnet / testnet only).
- Deploy a contract (devnet / testnet / mainnet).
- Call a contract (devnet / testnet / mainnet).
- Perform a simple bitcoin transfer from a p2pkh address to a p2pkh address (experimental, regtest / testnet / mainnet).

## References

For a more detailed discussion of how to use Deployment Plans, please see the following resources:

- [Meet 4 New Features in Clarinet](https://www.hiro.so/blog/meet-4-new-features-in-clarinet) blog post.

- [Technical Deep Dive On Clarinet](https://www.youtube.com/watch?v=ciHxOGBBS18) YouTube video.

---
# The default id is the same as the one being defined below. so not needed
title: Overview
---

## What is Clarinet?

[Clarinet](https://www.hiro.so/clarinet) provides a CLI package with a [clarity](https://clarity-lang.org/) runtime, an REPL, and a testing harness. Clarinet includes a Javascript library, testing environment, and a browser-based Sandbox. With Clarinet, you can rigorously iterate on your smart contracts locally before moving into production.

Clarinet consists of two components:

- Clarity REPL (Read, Evaluate, Print, Loop)
- Testing harness.

When the above components are used together, you can rapidly develop and test a Clarity smart contract, with the need to deploy the contract to a local devnet or testnet environments.

![screenshot](images/demo.gif)

To better understand Clarinet and how to develop with Clarinet, Hiro has created an introductory video tutorial series, from Hiro Engineer [Ludo Galabru](https://twitter.com/ludovic?lang=en), that will guide you through some of the basics and fundamentals of using Clarinet. The video also includes how you can use Clarinet to develop, test, and deploy smart contracts.

To view these video tutorials, please see [Hiro's Youtube channel](https://www.youtube.com/c/HiroSystems).
[![Clarinet101](images/clarinet101.png)](https://youtube.com/playlist?list=PL5Ujm489LoJaAz9kUJm8lYUWdGJ2AnQTb) 

For more latest information on Clarinet product, refer to [blog posts on Clarinet](https://www.hiro.so/search?query=Clarinet).

The Clarinet tool is used for developing smart contracts using a larger development strategy that involves:

- Building and testing the contract locally.
- Deploying the final draft contract to a testnet environment.
- Testing on a live blockchain.
- Deploying the final contract to the mainnet.

When developing smart contracts, you can also use the [Clarity Visual Studio Code plugin](https://marketplace.visualstudio.com/items?itemName=HiroSystems.clarity-lsp).

- When developing a new smart contract using local Clarity REPL, you can exercise a contract without the need to wait for block times in a live blockchain.

- Clarinet allows you to instantly initialize wallets and populate them with tokens, which helps to interactively or programmatically test the behavior of the smart contract. Blocks are mined instantly, so you can control the number of blocks that are mined between testing transactions.


---
title: Clarinet Deployment Plans
---

## Overview

Deployment Plans are reproducible deployment steps that publish a collection of on-chain transactions and one or more contracts to a network, whether a local developer network, the public testnet, or into production on mainnet. Deployment plans minimize the inherent complexity of deployments, such as smart-contract dependencies and interactions, transaction chaining limits, deployment costs, and more, while ensuring reproducible deployments critical for testing and automation purposes.

*Topics covered in this guide*:

* [Deployment plan design](#design)
* [Plan primitives](#deployment-plan-primitives)
* [References](#references)

## Design

The default deployment plan of every Clarinet project is contained within specifications set inside certain files. In addition to this default deployment plan, the user can manually configure each plan, adding additional transactions or contract calls across multiple Stacks or Bitcoin blocks.

You can commit, audit, and test contracts without including any secrets in the Deployment Plan and share these contracts without exposing any sensitive information.

## Deployment plan primitives

| Transaction primitive | Typical usage |
|---|---|
| publish contracts | - deploy a contract to an in-memory simulated Stacks chain or an integrate Stacks-Bitcoin environment <br /> - deploy to a public testnet or mainnet <br /> - deploy an external contract to your local network for testing |
| call contract functions | - call a contract deployed to any of your local devnets or public networks |
| send BTC | - Perform a simple bitcoin transfer from a p2pkh address to a p2pkh address (devnet/testnet/mainnet)  |
| wait for block | - Test or automate contract deployment across multiple Stacks or Bitcoin blocks  |
| send STX | - send stacks to an address or contract |

## References

For a more detailed discussion on how to use Deployment Plans, please see the following resources:

- [How To Guide for deployment plans](../how-to-guides/how-to-use-deployment-plans.md).

- [How to Set Up Custom Deployment Plans](https://www.youtube.com/watch?v=YcIg5VCO98s) YouTube video.

- [Meet 4 New Features in Clarinet](https://www.hiro.so/blog/meet-4-new-features-in-clarinet) blog post.

- [Technical Deep Dive On Clarinet](https://www.youtube.com/watch?v=ciHxOGBBS18) YouTube video.

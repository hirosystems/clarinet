---
title: Deploy Clarinet with Subnets
---

Clarinet may facilitate experimentation with [subnets](https://www.youtube.com/watch?v=PFPwuVCGGuI). You can follow this article to enable subnet node on your devnet environment.

*Topics covered in this guide*:

* [Enable subnet node](#subnet)
* [Spin up your subnet node](#spin-subnet-node)

## Subnet

To get started with subnets, enable the flag in your `Devnet.toml` by adding the configuration below.

```toml
[devnet]
# ...
enable_subnet_node = true
```

## Spin subnet node
You can use the `Devnet.toml` to customize the subnet-node (miner, etc). 
Spin up your subnet node with the command below.

```bash
clarinet integrate
```
You can find more information on using and interacting with Subnets in the [subnets repository](https://github.com/hirosystems/stacks-subnets).


---
title: Deploy Clarinet with Subnets
---

Clarinet may be used for facilitating experimentations with [subnets](https://www.youtube.com/watch?v=PFPwuVCGGuI).
To get started with subnets, in your `Devnet.toml`, enable the flag

```toml
[devnet]
# ...
enable_subnet_node = true
```

This same file can be used for customizing the subnet-node (miner, etc).

When running the command below, Clarinet will spin up a subnet node.

```bash
$ clarinet integrate
```

For more information on how to use and interact with this incoming L2 
can be found on the [subnet repository](https://github.com/hirosystems/stacks-subnets).

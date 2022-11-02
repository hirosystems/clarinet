---
title: Deploy Contracts
---

You can use Clarinet to publish your contracts to Devnet / Testnet / Mainnet environment for  testing and evaluation on a blockchain.

The first step is to generate a deployment plan, with the command below.

```bash
$ clarinet deployment generate --mainnet
```

After **cautiously** reviewing (and updating if needed) the generated plan, you can use the command below.

```bash
$ clarinet deployment apply -p <path-to-plan.yaml>
```

which will handle the deployments of your contracts, according to the plan.



---
title: Deploy Contracts
---

You can use Clarinet to publish your contracts to the public testnet or mainnet for testing or production.

*Topics covered in this guide*:

* [Generate deployment plan](#generate-deployment-plan)
* [Deploy your contract](#deploy)

# Generate Deployment plan

The first step is to generate a deployment plan with the command below (note: replace `--mainnet` with `--testnet` to deploy to the latter). Please specify a cost strategy to incentivize miners to carry your transaction (either `--low-cost`, `--medium-cost`, `--high-cost`, or `--manual-cost`). The final command might look like:

```bash
clarinet deployment generate --mainnet --medium-cost
```

# Deploy

After **carefully** reviewing (and updating if needed) the generated deployment plan, you can use the command below to handle the deployments of your contracts.

```bash
clarinet deployment apply --mainnet
```

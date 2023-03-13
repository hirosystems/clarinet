---
title: Create new Project
---

Once you have installed Clarinet, you may then use Clarinet to create a new project. 

*Topic covered in this guide*:

* [Create new project](#new-project)

## New project

To create a new project, enter the command shown below.

```bash
clarinet new my-project && cd my-project
``` 

Clarinet creates a project directory with the following directory layout: 

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


The `Clarinet.toml` file contains configuration files for the smart contracts in your project. Clarinet will add contracts to this file when you create contracts in your project.

The `settings/Devnet.toml` file contains configuration for accounts in the Clarinet console, including the seed phrases and initial balances. Initial balances are in microstacks (uSTX).

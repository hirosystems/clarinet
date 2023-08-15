---
title: Add new Contract
---

Clarinet can handle adding a new contract and its configuration to your project.

*Topics covered in this guide*:

* [Add a new contract](#new-contract)
* [Verify contract configuration](#contract-configuration)

## New Contract

You can use the command below to add a new contract.

```bash
clarinet contract new bbtc
```

Clarinet will add two files to your project:
- the contract file in the `contracts` directory
- the contract test file in the `tests` directory

```bash
.
├── Clarinet.toml
├── contracts
│   └── bbtc.clar
├── settings
│   └── Devnet.toml
│   └── Mainnet.toml
│   └── Testnet.toml
└── tests
    └── bbtc_test.ts
```

## Contract Configuration

Clarinet will also add your contract configuration in the `Clarinet.toml`.

```toml
[contracts.my-contract]
path = "contracts/my-contract.clar"
clarity_version = 2
epoch = 2.4
```

You can add contracts to your project by adding the files manually; however, you must make sure to add the appropriate configuration
to `Clarinet.toml` for Clarinet to recognize the contracts.

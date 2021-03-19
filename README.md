# clarinet

Clarinet is a clarity runtime packaged as a command line tool, designed to facilitate smart contract understanding, development, testing and deployment. 

Clarity is a **decidable** smart contract language that optimizes for predictability and security, designed by Blockstack. Smart contracts allow developers to encode essential business logic on a blockchain. 

![screenshot](docs/images/demo.gif)

## Installation

### Install from brew

```bash
$ brew tap lgalabru/clarinet
$ brew install clarinet
```

Assuming you have a working installation of Rust, Clarinet can be installed from Cargo as a crate, or from source.

### Install from cargo

```bash
$ cargo install clarinet
```

### Install from source

```bash
$ git clone git@github.com:lgalabru/clarinet.git
$ cd clarinet
$ cargo install --path .
```


## Getting started with clarinet

### Create a new project

Once installed, you can use clarinet to create a new project:

```bash
$ clarinet new my-project
$ cd my-project
```

Clarinet will be maintaining a working directory with the following directory layout:

```bash
$ tree .
.
├── Clarinet.toml
├── README.md
├── contracts
│   └── bbtc.clar
├── settings
│   └── Development.toml
└── tests
    └── bbtc_test.ts
```

### Add a new contract

New contracts can be added manually, or with the following command:

```bash
$ clarinet generate contract bbtc
```

### Execute test suite

```bash
$ clarinet test
```

### Load contracts in a console

```bash
$ clarinet console
```
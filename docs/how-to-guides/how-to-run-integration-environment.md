---
title: "Run local Integration Environment"
---

Once you have reached a point where your Clarity smart contract is functional, you can develop a web frontend against your contract. This can be challenging, as the contract must be deployed to a live blockchain to interact with it from a web app fully. Clarinet provides an easy method to deploy your contract to a blockchain that is configurable and controllable locally on your machine. This integration feature is called DevNet.

DevNet allows you to perform frontend development and integration testing without the need to deploy your contract to public testnet. This is valuable if you are in the early stages of developing a product, contract, or app in stealth. DevNet uses Docker to launch local instances of Bitcoin, Stacks, Stacks API, Explorer, and Bitcoin Explorer and provides total configuration control over all those instances. Once running, DevNet automatically deploys your contracts and creates Stacks accounts with pre-defined balances.

The services launched by DevNet represent a full instance of the Stacks blockchain with the Proof of Transfer consensus mechanism running against a locally running Bitcoin testnet. DevNet allows you to control block times, PoX transactions, and contract deployments. Because DevNet is running locally, it can be reset or reconfigured anytime. This allows for rapid frontend development without interacting with the public blockchain.


## Prerequisites

To run DevNet, you must have [Clarinet installed](../getting-started.md), and you also should have Docker installed locally. Refer to the [Docker documentation](https://docs.docker.com/get-docker/) for instructions on installing Docker on your development machine.

## Launching DevNet

Clarinet provides a sensible default configuration for DevNet. If you wish to use the default configuration, you can launch DevNet from the root of your Clarinet project with the command:

```sh
clarinet integrate
```

Clarinet fetches the appropriate Docker images for the Bitcoin node, Stacks node, Stacks API node, and the Bitcoin and Explorers. This can take several minutes on the first launch. Once the images are launched, the DevNet interface is displayed in your terminal window. The contracts in your project are deployed to the DevNet blockchain in the second block of the chain, so you may need to wait for the third block before launching your frontend development environment.

Review the following sections for information about the DevNet interface and configuration options for DevNet.

## DevNet interface

![DevNet interface](/img/devnet-interface.png)

The DevNet interface is displayed as a terminal GUI and consists of four primary panels: the system log, service status, mempool summary, and a minimal block explorer.

The system log provides a log of events happening throughout the DevNet stack. You can use this log to monitor the health of the local blockchain and review any events that occur. For services that provide a web interface, the URL for the local service is displayed next to the container name. You can connect to these URLs using a web browser to access the service.

The service status provides a status summary for the Docker containers that make up the DevNet stack. A green icon next to the container indicates that it is in a healthy state, a yellow icon indicates that the container is booting, and a red icon indicates a problem with the service.

The mempool summary displays a list of transactions in the mempool. These include historical transactions from the beginning of the blockchain.

The block explorer has two sub-panels: the block summary and the block transactions. You can use the `Arrow` keys to select a block within the chain (shown at the top of the block explorer), and the block summary and block transactions panels display information about that block. The block summary displays the Stacks block height, the Stacks block hash, the Bitcoin block height of the anchor block, and the PoX cycle number of the block. The block transactions panel displays all Stacks transactions that were included in the block.

You can access the locally running Explorer and Bitcoin Explorer from the URLs in the service status window for more detailed information about the blocks.

You can press `0` in the interface to reset the DevNet. Press `Ctrl` + `C` to stop the DevNet and shut down the
containers.

## Configuring DevNet

By default, DevNet launches a local Stacks 2.0 testnet with a fixed block time of 30 seconds. It runs Docker images that host a Bitcoin node, a Stacks Node, the Stacks API, the Explorer, and the Bitcoin Explorer. The default settings should be adequate for most developers, but you can change many settings to customize your development environment.

DevNet settings are located in the `settings/Devnet.toml` file. The file defines the wallets that are created in the
DevNet blockchain, the Stacks miner configuration, Proof of Transfer activity, and many other options.

### Accounts configuration

By default, Clarinet generates 10 wallets in the DevNet configuration file, a deployer wallet and 9 other accounts.
The accounts are seeded with a configurable balance of STX. Each wallet is defined under the heading
`[accounts.wallet_name]` in the TOML configuration file. Each heading has the following options:

- `mnemonic`: the 24-word keyphrase used to generate the wallet address
- `balance`: the balance in micro-STX of the account when the blockchain starts

The private key (`secret_key`), Stacks address, and BTC address are provided as comments under each wallet. These are useful for configuring stacking orders on DevNet.

### Blockchain configuration

DevNet provides a sensible default configuration for the local blockchain, with a fixed block time of 30 seconds and
the latest development images for each of the Stacks and Bitcoin nodes. These parameters are defined under the
`[devnet]` heading. You can customize these defaults by setting any of the following parameters.

>  **_NOTE:_**
> 
> The default value is used if any of the parameters are not supplied in the configuration file.


- `pox_stacking_orders`: defined by stacking orders headings later in the file
- `orchestrator_port`: the port number for the Bitcoin orchestrator service
- `bitcoin_node_p2p_port`: the port number for Bitcoin P2P network traffic
- `bitcoin_node_rpc_port`: the port number for Bitcoin RPC network traffic
- `bitcoin_node_username`: the username for the Bitcoin node container
- `bitcoin_node_password`: the password for the Bitcoin node container
- `bitcoin_controller_port`: the port number for the Bitcoin controller network traffic
- `bitcoin_controller_block_time`: the fixed block time for the testnet in milliseconds
- `stacks_node_rpc_port`: the port number for Stacks RPC network traffic
- `stacks_node_p2p_port`: the port number for Stacks P2P network traffic
- `stacks_node_events_observers`: a whitelist of addresses for observing Stacks node events
- `stacks_api_port`: the port number for Stacks API network traffic
- `stacks_api_events_port`: the port number for Stacks API events network traffic
- `bitcoin_explorer_port`: the port number for Bitcoin Explorer HTTP traffic
- `stacks_explorer_port`: the port number for Explorer HTTP traffic
- `miner_mnemonic`: the 24-word keyphrase for the STX miner wallet
- `miner_derivation_path`: the derivation path for the STX miner
- `working_dir`: the local working directory for filesystem storage for the testnet
- `postgres_port`: the port number for the Postgres DB (for running the Stacks API)
- `postgres_username`: the username for the Postgres DB
- `postgres_password`: the password for the Postgres DB
- `postgres_database`: the database name of the Postgres DB
- `bitcoin_node_image_url`: a Docker image path for the Bitcoin node container
- `stacks_node_image_url`: a Docker image path for the Stacks node container
- `stacks_api_image_url`: a Docker image path for the Stacks API node container
- `stacks_explorer_image_url`: a Docker image path for the Explorer node container
- `bitcoin_explorer_image_url`: a Docker image path for the Bitcoin Explorer node container
- `postgres_image_url`: a Docker image path for the Postgres DB container
- `disable_bitcoin_explorer`: Boolean to set if the Bitcoin Explorer container runs in the DevNet stack
- `disable_stacks_explorer`: Boolean to set if the Explorer container runs in the DevNet stack
- `disable_stacks_api`: Boolean to set if the Stacks API container runs in the DevNet stack

### Stacking orders

You can configure any of the wallets in the DevNet to participate in stacking to exercise the PoX contract within DevNet. This can be useful if you are developing a contract that interacts with the PoX contract and you need to set specific test conditions.

Each stacking order is defined under the heading `[devnet.pox_stacking_orders]`. This heading is repeated for as many stacking orders that are necessary for your configuration.

- `start_at_cycle`: the stacking cycle that the wallet should start participating in. The wallet's stacking order occurs at the block preceding the beginning of that cycle.
- `duration`: the stacking duration for the stacking cycle
- `wallet`: the alias of the wallet participating
- `slots`: the number of stacking slots that the wallet will participate in
- `btc_address`: the BTC address that stacking rewards should be sent to

For more information, you can refer to the following links:

- [clarinet installed](../getting-started.md)
- [docker documentation](https://docs.docker.com/get-docker/)


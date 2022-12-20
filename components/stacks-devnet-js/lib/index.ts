"use strict";

const {
  stacksDevnetNew,
  stacksDevnetStart,
  stacksDevnetTerminate,
  stacksDevnetWaitForStacksBlock,
  stacksDevnetWaitForBitcoinBlock,
  stacksDevnetGetStacksNodeUrl,
  stacksDevnetGetBitcoinNodeUrl,
  stacksDevnetGetStacksApiUrl,
  stacksDevnetGetStacksExplorerUrl,
  stacksDevnetGetBitcoinExplorerUrl,
} = require("../native/index.node");
import {
  BitcoinChainUpdate,
  StacksBlockMetadata,
  StacksChainUpdate,
} from "@hirosystems/chainhook-types";
export * from "@hirosystems/chainhook-types";

/**
 * Account to include in the genesis accounts
 * @export
 * @interface Account
 */
export interface Account {
  /**
   * An ID to use to identify the account
   * @type {string}
   * @memberof Account
   */
  label: string;
  /**
   * The mnemonic to use for generating the keypair
   * @type {string}
   * @memberof Account
   */
  mnemonic: string;
  /**
   * The amount of µSTX to seed
   * @type {number}
   * @memberof Account
   */
  balance: number;
  /**
   * The derivation path to use
   * @type {number}
   * @memberof Account
   */
  derivation?: string;
  /**
   * Should a mainnet/testnet address be constructed
   * @type {number}
   * @memberof Account
   */
  is_mainnet?: boolean;
}

/**
 * Transaction helper to ease scheduling of Stacking operations that can be performed by the genesis accounts
 * @export
 * @interface PoxStackingOrder
 */
export interface PoxStackingOrder {
  /**
   * The ID of the reward cycle targeted
   * @type {number}
   * @memberof PoxStackingOrder
   */
  start_at_cycle: number;
  /**
   * The number of reward cycles (max 12)
   * @type {number}
   * @memberof PoxStackingOrder
   */
  duration: number;
  /**
   * The ID of the wallet performing the stacking operation
   * @type {string}
   * @memberof PoxStackingOrder
   */
  wallet: string;
  /**
   * The number of reward slots targeted, used for inferring the amount of STX to lock
   * @type {number}
   * @memberof PoxStackingOrder
   */
  slots: number;
  /**
   * The Bitcoin address where the rewards should be sent
   * @type {number}
   * @memberof PoxStackingOrder
   */
  btc_address: string;
}

/**
 * Set of values that can be used for overriding values coming from the default project settings
 * @export
 * @interface DevnetConfig
 */
export interface DevnetConfig {
  /**
   * Optional network id
   * @type {number}
   * @memberof DevnetConfig
   */
  network_id?: number;
  /**
   * The port that should be used by the orchestrator
   * @type {number}
   * @memberof DevnetConfig
   */
  orchestrator_port?: number;
  /**
   * The port that should be used by the orchestrator's control plan
   * @type {number}
   * @memberof DevnetConfig
   */
  orchestrator_control_port?: number;
  /**
   * The port that should be used by bitcoind's data plane
   * @type {number}
   * @memberof DevnetConfig
   */
  bitcoin_node_p2p_port?: number;
  /**
   * The port that should be used by bitcoind's control plane
   * @type {number}
   * @memberof DevnetConfig
   */
  bitcoin_node_rpc_port?: number;
  /**
   * The port that should be used by stacks's data plane
   * @type {number}
   * @memberof DevnetConfig
   */
  stacks_node_p2p_port?: number;
  /**
   * The port that should be used by stacks's control plane
   * @type {number}
   * @memberof DevnetConfig
   */
  stacks_node_rpc_port?: number;
  /**
   * An array of stacks events observers (["localhost:300", etc])
   * @type {number}
   * @memberof DevnetConfig
   */
  stacks_node_events_observers?: string[];
  /**
   * The port that should be used by the stacks-blockchain-api
   * @type {number}
   * @memberof DevnetConfig
   */
  stacks_api_port?: number;
  /**
   * The port that should be used by the event listener of the stacks-blockchain-api
   * @type {number}
   * @memberof DevnetConfig
   */
  stacks_api_events_port?: number;
  /**
   * The port that should be used by the http interface of the bitcoin explorer
   * @type {number}
   * @memberof DevnetConfig
   */
  bitcoin_explorer_port?: number;
  /**
   * The port that should be used by the http interface of the stacks explorer
   * @type {number}
   * @memberof DevnetConfig
   */
  stacks_explorer_port?: number;
  /**
   * Bind bitcoind and stacks-node data volumes (false by default)
   * @type {number}
   * @memberof DevnetConfig
   */
  bind_containers_volumes?: boolean;
  /**
   * Disable Bitcoin automining
   * @type {number}
   * @memberof DevnetConfig
   */
  bitcoin_controller_automining_disabled?: boolean;
  /**
   * The username to use for authenticating bitcoind RPC calls
   * @type {number}
   * @memberof DevnetConfig
   */
  bitcoin_node_username?: string;
  /**
   * The password to use for authenticating bitcoind RPC calls
   * @type {number}
   * @memberof DevnetConfig
   */
  bitcoin_node_password?: string;
  /**
   * The mnemonic to use for the bitcoin miner
   * @type {number}
   * @memberof DevnetConfig
   */
  miner_mnemonic?: string;
  /**
   * The derivation path to use for the bitcoin miner
   * @type {number}
   * @memberof DevnetConfig
   */
  miner_derivation_path?: string;
  /**
   * The delay between bitcoin blocks
   * @type {number}
   * @memberof DevnetConfig
   */
  bitcoin_controller_block_time?: number;
  /**
   * The path where the chainstates (bitcoin, stacks, logs) will be persisted
   * @type {number}
   * @memberof DevnetConfig
   */
  working_dir?: string;
  /**
   * The port that should be used by the postgres server used by the stacks-blockchain-api
   * @type {number}
   * @memberof DevnetConfig
   */
  postgres_port?: number;
  /**
   * The username to use for authenticating postgres connections, used by the stacks-blockchain-api
   * @type {number}
   * @memberof DevnetConfig
   */
  postgres_username?: string;
  /**
   * The password to use for authenticating postgres connections, used by the stacks-blockchain-api
   * @type {number}
   * @memberof DevnetConfig
   */
  postgres_password?: string;
  /**
   * The name of the postgres database used by the stacks-blockchain-api
   * @type {number}
   * @memberof DevnetConfig
   */
  postgres_database?: string;
  /**
   * An array of PoX stacking orders
   * @type {PoxStackingOrder[]}
   * @memberof DevnetConfig
   */
  pox_stacking_orders?: PoxStackingOrder[];
  /**
   * The port that should be used by the bitcoin-node
   * @type {string}
   * @memberof DevnetConfig
   */
  bitcoin_node_image_url?: string;
  /**
   * The port that should be used by the bitcoin-explorer
   * @type {string}
   * @memberof DevnetConfig
   */
  bitcoin_explorer_image_url?: string;
  /**
   * The port that should be used by the stacks-node
   * @type {string}
   * @memberof DevnetConfig
   */
  stacks_node_image_url?: string;
  /**
   * The container image to use for stacks-blockchain-api
   * @type {string}
   * @memberof DevnetConfig
   */
  stacks_api_image_url?: string;
  /**
   * The container image to use for stacks-explorer
   * @type {string}
   * @memberof DevnetConfig
   */
  stacks_explorer_image_url?: string;
  /**
   * The container image to use for postgres
   * @type {string}
   * @memberof DevnetConfig
   */
  postgres_image_url?: string;
  /**
   * Disable bitcoin explorer (true by default)
   * @type {boolean}
   * @memberof DevnetConfig
   */
  disable_bitcoin_explorer?: boolean;
  /**
   * Disable stacks explorer (true by default)
   * @type {boolean}
   * @memberof DevnetConfig
   */
  disable_stacks_explorer?: boolean;
  /**
   * Disable stacks API (true by default)
   * @type {boolean}
   * @memberof DevnetConfig
   */
  disable_stacks_api?: boolean;
  /**
   * Enable support for Stacks 2.1 (false by default)
   * @type {boolean}
   * @memberof DevnetConfig
   */
  enable_next_features?: boolean;
  /**
   * Bitcoin block height starting the epoch 2.0
   * @type {number}
   * @memberof DevnetConfig
   */
  epoch_2_0?: number;
  /**
   * Bitcoin block height starting the epoch 2.05
   * @type {number}
   * @memberof DevnetConfig
   */
  epoch_2_05?: number;
  /**
   * Bitcoin block height starting the epoch 2.1
   * @type {number}
   * @memberof DevnetConfig
   */
  epoch_2_1?: number;
  /**
   * Bitcoin block height activating switch to POX 2.0
   * @type {number}
   * @memberof DevnetConfig
   */
  pox_2_activation?: number;
}

/**
 * Settings to use for the Devnet network to spawn. Load a given manifest file, that can be overriden.
 * bitcoin-explorer, stacks-explorer and stacks-blockchain-api disabled by default.
 * @export
 * @interface NetworkConfig
 */
export interface NetworkConfig {
  /**
   * The path on disk of the Clarinet manifest file.
   * @type {string}
   * @memberof NetworkConfig
   */
  clarinetManifestPath?: string;
  /**
   * Display logs in the console
   * @type {boolean}
   * @memberof NetworkConfig
   */
  logs?: boolean;
  /**
   * Accounts to include in the genesis file
   * @type {Account[]}
   * @memberof NetworkConfig
   */
  accounts?: Account[];
  /**
   * Devnet config values that will be overriding any values present in the Devnet.toml file.
   * @type {DevnetConfig}
   * @memberof NetworkConfig
   */
  devnet?: DevnetConfig;
}

export class DevnetNetworkFactory {
  private static instance: DevnetNetworkFactory | undefined = undefined;
  private nextNetworkId: number = 0;

  private constructor() { }

  static sharedInstance(): DevnetNetworkFactory {
    if (!DevnetNetworkFactory.instance) {
      DevnetNetworkFactory.instance = new DevnetNetworkFactory();
    }
    return DevnetNetworkFactory.instance;
  }

  buildNetwork(manifest: NetworkConfig): DevnetNetworkOrchestrator {
    let network = new DevnetNetworkOrchestrator(getIsolatedNetworkConfigUsingNetworkId(this.nextNetworkId, manifest));
    this.nextNetworkId += 1;
    return network;
  }
}

export function getIsolatedNetworkConfigUsingNetworkId(networkId: number, networkConfig: NetworkConfig, interval = 10000) {
  const manifestPath = networkConfig.clarinetManifestPath || "./Clarinet.toml";
  const logs = networkConfig.logs || false;
  const accounts = networkConfig.accounts || [];
  // Devnet settings
  var devnetDefaults = {
    network_id: networkId,
    bitcoin_controller_automining_disabled: false,
    bitcoin_node_p2p_port: interval + networkId * 20 + 1,
    bitcoin_node_rpc_port: interval + networkId * 20 + 2,
    stacks_node_p2p_port: interval + networkId * 20 + 3,
    stacks_node_rpc_port: interval + networkId * 20 + 4,
    orchestrator_port: interval + networkId * 20 + 5,
    orchestrator_control_port: interval + networkId * 20 + 6,
    stacks_api_port: interval + networkId * 20 + 7,
    stacks_api_events_port: interval + networkId * 20 + 8,
    postgres_port: interval + networkId * 20 + 9,
    stacks_explorer_port: interval + networkId * 20 + 10,
    bitcoin_explorer_port: interval + networkId * 20 + 11,
    subnet_node_p2p_port: interval + networkId * 20 + 12,
    subnet_node_rpc_port: interval + networkId * 20 + 13,
    subnet_api_port: interval + networkId * 20 + 14,
    subnet_api_events_port: interval + networkId * 20 + 15,
  };
  var devnet = Object.assign(devnetDefaults, networkConfig.devnet);
  return {
    clarinetManifestPath: manifestPath,
    logs,
    accounts,
    devnet: devnet,
  };
}

export class DevnetNetworkOrchestrator {
  handle: any;
  lastCooldownEndedAt: Date;
  defaultCooldown: number;

  /**
   * @summary Construct a new DevnetNetworkOrchestrator
   * @param {NetworkConfig} manifest
   * @memberof DevnetNetworkOrchestrator
   */
  constructor(config: NetworkConfig, defaultCooldown = 3000) {
    let manifestPath = config.clarinetManifestPath!;
    var logs = config.logs;
    logs ||= false;
    var accounts = config.accounts;
    accounts ||= [];
    var devnet = config.devnet;
    devnet ||= {};
    this.handle = stacksDevnetNew(manifestPath, logs, accounts, devnet);
    this.lastCooldownEndedAt = new Date();
    this.defaultCooldown = defaultCooldown;
  }

  /**
   * @summary Start orchestrating containers
   * @memberof DevnetNetworkOrchestrator
   */
  start(timeout: number = 600, emptyBuffer: boolean = true) {
    return stacksDevnetStart.call(this.handle, timeout, emptyBuffer);
  }

  /**
   * @summary Returns the URL of the stacks-node container
   * @memberof DevnetNetworkOrchestrator
   */
  getStacksNodeUrl() {
    return stacksDevnetGetStacksNodeUrl.call(this.handle);
  }

  /**
   * @summary Returns the URL of the bitcoin-node container
   * @memberof DevnetNetworkOrchestrator
   */
  getBitcoinNodeUrl() {
    return stacksDevnetGetBitcoinNodeUrl.call(this.handle);
  }

  /**
   * @summary Returns the URL of the stacks-api container
   * @memberof DevnetNetworkOrchestrator
   */
  getStacksApiUrl() {
    return stacksDevnetGetStacksApiUrl.call(this.handle);
  }

  /**
   * @summary Returns the URL of the stacks-explorer container
   * @memberof DevnetNetworkOrchestrator
   */
  getStacksExplorerUrl() {
    return stacksDevnetGetStacksExplorerUrl.call(this.handle);
  }

  /**
   * @summary Returns the URL of the bitcoin-explorer container
   * @memberof DevnetNetworkOrchestrator
   */
  getBitcoinExplorerUrl() {
    return stacksDevnetGetBitcoinExplorerUrl.call(this.handle);
  }

  /**
   * @summary Wait for the next Stacks block
   * @memberof DevnetNetworkOrchestrator
   */
  async waitForNextStacksBlock(): Promise<StacksChainUpdate> {
    let now = new Date();
    let ms_elapsed = (now.getTime() - this.lastCooldownEndedAt.getTime());
    let cooldown = Math.max(0, this.defaultCooldown - ms_elapsed);
    let wait = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));
    this.lastCooldownEndedAt = now
    return wait(cooldown)
      .then(() => {
        this.lastCooldownEndedAt = new Date();
        return stacksDevnetWaitForStacksBlock.call(this.handle)
      })
      .catch(e => {
        this.lastCooldownEndedAt = new Date();
        throw e
      });
  }

  /**
   * @summary Wait for the next Stacks block
   * @memberof DevnetNetworkOrchestrator
   */
  async waitForStacksBlockOfHeight(targetBlockHeight: number, maxErrors = 5): Promise<StacksChainUpdate> {
    let errorCount = 0;
    while (true) {
      try {
        let chainUpdate = await this.waitForNextStacksBlock();
        if (chainUpdate === undefined) {
          errorCount += 1;
          if (errorCount >= maxErrors) {
            throw 'waitForNextStacksBlock maxErrors reached'
          }
          continue;
        }
        let currentBlockHeight = chainUpdate.new_blocks[0].block.block_identifier.index;
        errorCount = 0;
        if (currentBlockHeight >= targetBlockHeight) {
          return chainUpdate;
        }
      } catch (error) {
        errorCount += 1;
        if (errorCount >= maxErrors) {
          throw error;
        }
      }
    }
  }

  /**
   * @summary Wait for the next Stacks block
   * @memberof DevnetNetworkOrchestrator
   */
  async waitForStacksBlockAnchoredOnBitcoinBlockOfHeight(minBitcoinBlockHeight: number, maxErrors = 5): Promise<StacksChainUpdate> {
    let errorCount = 0;
    while (true) {
      try {
        let chainUpdate = await this.waitForNextStacksBlock();
        if (chainUpdate === undefined) {
          errorCount += 1;
          if (errorCount >= maxErrors) {
            throw 'waitForNextStacksBlock maxErrors reached'
          }
          continue;
        }
        let metadata = chainUpdate.new_blocks[0].block.metadata! as StacksBlockMetadata;
        let currentBitcoinBlockHeight = metadata.bitcoin_anchor_block_identifier.index;
        errorCount = 0;
        if (currentBitcoinBlockHeight >= minBitcoinBlockHeight) {
          return chainUpdate;
        }
      } catch (error) {
        errorCount += 1;
        if (errorCount >= maxErrors) {
          throw error;
        }
      }
    }
  }
  
  /**
   * @summary Wait for the next Bitcoin block
   * @memberof DevnetNetworkOrchestrator
   */
  async waitForNextBitcoinBlock(): Promise<BitcoinChainUpdate> {
    let now = new Date();
    let ms_elapsed = (now.getTime() - this.lastCooldownEndedAt.getTime());
    let cooldown = Math.max(0, this.defaultCooldown - ms_elapsed);
    let wait = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));
    return wait(cooldown)
      .then(() => {
        this.lastCooldownEndedAt = new Date();
        return stacksDevnetWaitForBitcoinBlock.call(this.handle)
      });
  }

  /**
   * @summary Terminates the containers
   * @memberof DevnetNetworkOrchestrator
   */
  terminate(): boolean {
    return stacksDevnetTerminate.call(this.handle);
  }
}

import { Block } from "./types";
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
    id: string;
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
     * The port that should be used by the orchestrator
     * @type {number}
     * @memberof DevnetConfig
     */
    orchestrator_port?: number;
    /**
     * The port that should be used by bitcoind's data plan
     * @type {number}
     * @memberof DevnetConfig
     */
    bitcoin_node_p2p_port?: number;
    /**
     * The port that should be used by bitcoind's control plan
     * @type {number}
     * @memberof DevnetConfig
     */
    bitcoin_node_rpc_port?: number;
    /**
     * The port that should be used by stacks's data plan
     * @type {number}
     * @memberof DevnetConfig
     */
    stacks_node_p2p_port?: number;
    /**
     * The port that should be used by stacks's control plan
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
     * The port that should be used by the bitcoin controller/puppeteer port
     * @type {number}
     * @memberof DevnetConfig
     */
    bitcoin_controller_port?: number;
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
}
/**
 * Settings to use for the Devnet network to spawn. Load a given manifest file, that can be overriden.
 * bitcoin-explorer, stacks-explorer and stacks-blockchain-api disabled by default.
 * @export
 * @interface ClarinetManifest
 */
export interface ClarinetManifest {
    /**
     * The path on disk of the Clarinet manifest file.
     * @type {string}
     * @memberof ClarinetManifest
     */
    path: string;
    /**
     * Display logs in the console
     * @type {boolean}
     * @memberof ClarinetManifest
     */
    logs?: boolean;
    /**
     * Accounts to include in the genesis file
     * @type {Account[]}
     * @memberof ClarinetManifest
     */
    accounts?: Account[];
    /**
     * Blockchains that utilize a username model (where the address is not a derivative of a cryptographic public key) should specify the public key(s) owned by the address in metadata.
     * @type {DevnetConfig}
     * @memberof ClarinetManifest
     */
    devnet?: DevnetConfig;
}
export declare class StacksDevnetOrchestrator {
    handle: any;
    /**
     * @summary Construct a new StacksDevnetOrchestrator
     * @param {ClarinetManifest} manifest
     * @memberof StacksDevnetOrchestrator
     */
    constructor(manifest: ClarinetManifest);
    /**
     * @summary Start orchestrating containers
     * @memberof StacksDevnetOrchestrator
     */
    start(): any;
    /**
     * @summary Returns the URL of the stacks-node container
     * @memberof StacksDevnetOrchestrator
     */
    getStacksNodeUrl(): any;
    /**
     * @summary Wait for the next Stacks block
     * @memberof StacksDevnetOrchestrator
     */
    waitForStacksBlock(): Block;
    /**
     * @summary Wait for the next Bitcoin block
     * @memberof StacksDevnetOrchestrator
     */
    waitForBitcoinBlock(): Block;
    /**
     * @summary Terminates the containers
     * @memberof StacksDevnetOrchestrator
     */
    stop(): void;
}
//# sourceMappingURL=index.d.ts.map
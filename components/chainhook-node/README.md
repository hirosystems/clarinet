# chainhook-node

## Usage

To get started, [build `clarinet` from source](https://github.com/hirosystems/clarinet#install-from-source-using-cargo), and then `cd components/chainhook-node` and run `cargo install --path .` to install `chainhook-node`.

Before running `chainhook-node`, you need to [install redis](https://redis.io/docs/getting-started/installation/) and run a redis server locally.

 ### Start a Testnet node 

```bash
$ chainhook-node start --testnet
```

### Start a Mainnet node 

```bash
$ chainhook-node start --mainnet
```

### Start a Devnet node 

```bash
$ chainhook-node start --devnet
```

## Predicates available

### Bitcoin

```yaml
# Get any transaction matching a given txid
# `txid` mandatory argument admits:
#  - 32 bytes hex encoded type. example: "0xfaaac1833dc4883e7ec28f61e35b41f896c395f8d288b1a177155de2abd6052f" 
predicate:
    txid: 0xfaaac1833dc4883e7ec28f61e35b41f896c395f8d288b1a177155de2abd6052f

# Get any transaction including an OP_RETURN output starting with a set of characters.
# `starts-with` mandatory argument admits:
#  - ASCII string type. example: `X2[`
#  - hex encoded bytes. example: `0x589403`
predicate:
    scope: outputs
    op-return:
        starts-with: X2[

# Get any transaction including an OP_RETURN output matching the sequence of bytes specified 
# `equals` mandatory argument admits:
#  - hex encoded bytes. example: `0x589403`
predicate:
    scope: outputs
    op-return:
        equals: 0x69bd04208265aca9424d0337dac7d9e84371a2c91ece1891d67d3554bd9fdbe60afc6924d4b0773d90000006700010000006600012

# Get any transaction including an OP_RETURN output ending with a set of characters 
# `ends-with` mandatory argument admits:
#  - ASCII string type. example: `X2[`
#  - hex encoded bytes. example: `0x589403`
predicate:
    scope: outputs
    op-return:
        ends-with: 0x76a914000000000000000000000000000000000000000088ac

# Get any transaction including a Stacks Proof of Burn commitment 
predicate:
    scope: outputs
    stacks-op:
        type: pob-commit

# Get any transaction including a Stacks Proof of Transfer commitment
# `recipients` mandatory argument admits:
#  - string "*"
#  - array of strings type. example: ["mr1iPkD9N3RJZZxXRk7xF9d36gffa6exNC", "muYdXKmX9bByAueDe6KFfHd5Ff1gdN9ErG"]
#  - array of hex encoded bytes type. example: ["76a914000000000000000000000000000000000000000088ac", "0x76a914ee9369fb719c0ba43ddf4d94638a970b84775f4788ac"]
predicate:
    scope: outputs
    stacks-op:
        type: pox-commit
        recipients: *

# Get any transaction including a key registration operation 
predicate:
    scope: outputs
    stacks-op:
        type: key-registration

# Get any transaction including a STX transfer operation 
# `recipient` optional argument admits:
#  - string encoding a valid STX address. example: "ST2CY5V39NHDPWSXMW9QDT3HC3GD6Q6XX4CFRK9AG"
# `sender` optional argument admits:
#  - string type. example: "mr1iPkD9N3RJZZxXRk7xF9d36gffa6exNC"
#  - hex encoded bytes type. example: "0x76a914ee9369fb719c0ba43ddf4d94638a970b84775f4788ac" 
predicate:
    scope: outputs
    stacks-op:
        type: stx-transfer

# Get any transaction including a STX lock operation
# `sender` optional argument admits:
#  - string type. example: "mr1iPkD9N3RJZZxXRk7xF9d36gffa6exNC"
#  - hex encoded bytes type. example: "0x76a914ee9369fb719c0ba43ddf4d94638a970b84775f4788ac" 
predicate:
    scope: outputs
    stacks-op:
        type: stx-lock

# Get any transaction including a p2pkh output paying a given recipient
# `p2pkh` construct admits:
#  - string type. example: "mr1iPkD9N3RJZZxXRk7xF9d36gffa6exNC"
#  - hex encoded bytes type. example: "0x76a914ee9369fb719c0ba43ddf4d94638a970b84775f4788ac" 
predicate:
    scope: outputs
    p2pkh: mr1iPkD9N3RJZZxXRk7xF9d36gffa6exNC

# Get any transaction including a p2sh output paying a given recipient
# `p2sh` construct admits:
#  - string type. example: "2MxDJ723HBJtEMa2a9vcsns4qztxBuC8Zb2"
#  - hex encoded bytes type. example: "0x76a914ee9369fb719c0ba43ddf4d94638a970b84775f4788ac" 
predicate:
    scope: outputs
    p2sh: 2MxDJ723HBJtEMa2a9vcsns4qztxBuC8Zb2

# Get any transaction including a p2wpkh output paying a given recipient
# `p2wpkh` construct admits:
#  - string type. example: "bcrt1qnxknq3wqtphv7sfwy07m7e4sr6ut9yt6ed99jg"
predicate:
    scope: outputs
    p2wpkh: bcrt1qnxknq3wqtphv7sfwy07m7e4sr6ut9yt6ed99jg

# Get any transaction including a p2wsh output paying a given recipient
# `p2wsh` construct admits:
#  - string type. example: "bc1qklpmx03a8qkv263gy8te36w0z9yafxplc5kwzc"
predicate:
    scope: outputs
    p2wsh: bc1qklpmx03a8qkv263gy8te36w0z9yafxplc5kwzc

# Additional predicates including support for taproot coming soon
```

### Stacks

```yaml
# Get any transaction matching a given txid
# `txid` mandatory argument admits:
#  - 32 bytes hex encoded type. example: "0xfaaac1833dc4883e7ec28f61e35b41f896c395f8d288b1a177155de2abd6052f" 
predicate:
    txid: 0xfaaac1833dc4883e7ec28f61e35b41f896c395f8d288b1a177155de2abd6052f

# Get any transaction related to a given fungible token asset identifier
# `asset-identifier` mandatory argument admits:
#  - string type, fully qualifying the asset identifier to observe. example: `ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.cbtc-sip10::cbtc`
# `actions` mandatory argument admits:
#  - array of string type constrained to `mint`, `transfer` and `burn` values. example: ["mint", "burn"]
predicate:
    ft-event:
        asset-identifier: 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.cbtc-sip10::cbtc'
        actions:
            - mint
            - burn

# Get any transaction related to a given non fungible token asset identifier
# `asset-identifier` mandatory argument admits:
#  - string type, fully qualifying the asset identifier to observe. example: `ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.monkey-sip09::monkeys`
# `actions` mandatory argument admits:
#  - array of string type constrained to `mint`, `transfer` and `burn` values. example: ["mint", "burn"]
predicate:
    nft-event:
        asset-identifier: 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.monkey-sip09::monkeys'
        actions:
            - transfer
            - burn

# Get any transaction moving STX tokens
# `actions` mandatory argument admits:
#  - array of string type constrained to `mint`, `transfer` and `lock` values. example: ["mint", "lock"]
predicate:
    stx-event:
        actions:
            - mint
            - lock

# Get any transaction emitting given print events predicate
# `contract-identifier` mandatory argument admits:
#  - string type, fully qualifying the contract to observe. example: `ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.monkey-sip09`
# `contains` mandatory argument admits:
#  - string type, used for matching event
predicate:
    print-event:
        contract-identifier: 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.monkey-sip09'
        contains: "vault"

# Get any transaction including a contract deployment
# `deployer` mandatory argument admits:
#  - string "*"
#  - string encoding a valid STX address. example: "ST2CY5V39NHDPWSXMW9QDT3HC3GD6Q6XX4CFRK9AG"
predicate:
    contract-deploy:
        deployer: "ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM"

# Get any transaction including a contract deployment implementing a given trait (coming soon)
# `impl-trait` mandatory argument admits:
#  - string type, fully qualifying the trait's shape to observe. example: `ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.sip09-protocol`
predicate:
    contract-deploy:
        impl-trait: "ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.sip09-protocol"
```


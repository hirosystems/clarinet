# cBTC example

In this example we are exploring a naive and centralized but, yet functional approach for wrapping / unwrapping BTC to SIP10 tokens.

By sending BTC to the `authority` address, a party would see an equivalent amount of cBTC being minted on the Stacks Blockchain.

When burning cBTC, a token owner will see some Bitcoin being transferred to his Bitcoin address.

This protocol was meant to illustrate possible interactions between Bitcoin and Stacks using a mechanism called `chainhooks`. The design of this protocol is limited (proofs not being checked, central trustee, etc) and should not be used in production. 

## How to use

Start a local Devnet with the command:

```bash
clarinet integrate
```

In another console, change the directory to `./serverless/`. After running

```bash
cd serverless
yarn global add serverless    # Install serverless globally
yarn add --dev serverless-plugin-typescript@latest
yarn                          # Install dependencies
```

and making sure that the command `serverless` is available in your `$PATH`, the lambda functions can be started locally with the following command:

```bash
serverless offline --verbose
```

Once the message `Protocol deployed` appears on the screen, transfers tokens back and forth between the Bitcoin Blockchain and the Stacks Blockchain can be performed
thanks to the deployment plans:

- `deployments/wrap-btc.devnet-plan.yaml`: a BTC transaction is being performed, using the following parameters:

```yaml
- btc-transfer:
    expected-sender: mjSrB3wS4xab3kYqFktwBzfTdPg367ZJ2d
    recipient: mr1iPkD9N3RJZZxXRk7xF9d36gffa6exNC
    sats-amount: 100000000
    sats-per-byte: 10
```

A chainhook predicate, specified in `chainhooks/wrap-btc.json` is observing BTC transfers being performed to the address `mr1iPkD9N3RJZZxXRk7xF9d36gffa6exNC` thanks to the following configuration:

```json
"if_this": {
    "scope": "outputs",
    "p2pkh": {
        "equals": "mr1iPkD9N3RJZZxXRk7xF9d36gffa6exNC"
    }
},
"then_that": {
    "http_post": {
        "url": "http://localhost:3000/api/v1/wrapBtc",
        "authorization_header": "Bearer cn389ncoiwuencr"
    }
}
```

In this protocol, this transaction assumes usage of p2pkh addresses, and sends the change back to the sender, using the same address. When minting the `cBTC` tokens, the authority is converting 
the 2nd output of the transaction to a Stacks address, and sending the minted tokens to this address.  

- `deployments/unwrap-btc.devnet-plan.yaml`: a contract call is being issued, using the following settings:

```yaml
- contract-call:
    contract-id: ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.cbtc-token
    expected-sender: STNHKEPYEPJ8ET55ZZ0M5A34J0R3N5FM2CMMMAZ6
    method: burn
    parameters:
      - u100000
    cost: 5960
```

Another chainhook predicate, specified in `chainhooks/unwrap-btc.json` is observing cBTC burn events occuring on the Stacks blockchain, thanks to the following configuration:

```json
"if_this": {
    "scope": "ft_event",
    "asset_identifier": "ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.cbtc-token::cbtc",
    "actions": [
        "burn"
    ]
},
"then_that": {
    "http_post": {
        "url": "http://localhost:3000/api/v1/unwrapBtc",
        "authorization_header": "Bearer cn389ncoiwuencr"
    }
}
```

When the authority process this chainhook occurences, it sends BTC from its reserve to `cBTC` burner, by assuming that a p2pkh is being used.

The wrap / unwrap deployment plans can both be respectively performed with the commands:

```bash
clarinet deployment apply -p deployments/wrap-btc.devnet-plan.yaml
```

and

```bash
clarinet deployment apply -p deployments/unwrap-btc.devnet-plan.yaml
```

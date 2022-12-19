
In a console, launch `redis-server` with the following command

```bash
$ redis-server
```

In another console, we will launch `vault-monitor`. `vault-monitor` is a program that will be processing the events triggered by `chainhook-db`. Ruby on Rails (ruby 2.7+, rails 7+) was used to demonstrate that Chainhooks is a language agnostic layer. 

```bash
# Navigate to vault monitor directory
$ cd vault-monitor

# Install dependencies
$ bundle install

# Create database and run db migrations (will use sqlite in development mode)
$ rails db:migrate

# Run program
$ rails server
```

`vault-monitor` exposes an admin readonly user interface at this address `http://localhost:3000/admin`.

In another console, launch `chainhook-node`, using the command:

```bash
$ chainhook-node replay --testnet
```

Finally, make `vault-monitor` register a chainhook, using the following command:

```bash
curl -X "POST" "http://0.0.0.0:20446/v1/chainhooks/" \
     -H 'Content-Type: application/json' \
     -d $'{
  "stacks": {
    "predicate": {
      "type": "print_event",
      "rule": {
        "contains": "vault",
        "contract_identifier": "SP2C2YFP12AJZB4MABJBAJ55XECVS7E4PMMZ89YZR.arkadiko-freddie-v1-1"
      }
    },
    "action": {
      "http": {
        "url": "http://localhost:3000/chainhooks/v1/vaults",
        "method": "POST",
        "authorization_header": "Bearer cn389ncoiwuencr"
      }
    },
    "uuid": "1",
    "decode_clarity_values": true,
    "version": 1,
    "name": "Vault events observer",
    "network": "mainnet"
  }
}'
```


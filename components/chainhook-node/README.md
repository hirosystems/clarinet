
### Usage

```bash
$ chainhook-node start --config=./Mainnet.toml
```

Mainnet.toml

```toml
[storage]
driver = "redis"
redis_uri = ""

[event_relaying]
enabled = false

[[event_source]]
type = "stacks-node" # other: "chainhook-node", "tsv-file", "tsv-url"
stacks_node_url = ""
chainhook_node_url = ""
polling_delay = 1000
tsv_file_path = ""
tsv_file_url = ""

[chainhooks]
stacks_max_registrations = 10
bitcoin_max_registrations = 10
```
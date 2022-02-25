complete -c clarinet -n "__fish_use_subcommand" -l help -d 'Print help information'
complete -c clarinet -n "__fish_use_subcommand" -l version -d 'Print version information'
complete -c clarinet -n "__fish_use_subcommand" -f -a "new" -d 'Create and scaffold a new project'
complete -c clarinet -n "__fish_use_subcommand" -f -a "contract" -d 'Subcommands for working with contracts'
complete -c clarinet -n "__fish_use_subcommand" -f -a "console" -d 'Load contracts in a REPL for an interactive session'
complete -c clarinet -n "__fish_use_subcommand" -f -a "test" -d 'Execute test suite'
complete -c clarinet -n "__fish_use_subcommand" -f -a "check" -d 'Check syntax of your contracts'
complete -c clarinet -n "__fish_use_subcommand" -f -a "publish" -d 'Publish contracts on chain'
complete -c clarinet -n "__fish_use_subcommand" -f -a "run" -d 'Execute Clarinet extension'
complete -c clarinet -n "__fish_use_subcommand" -f -a "integrate" -d 'Start devnet environment for integration testing'
complete -c clarinet -n "__fish_use_subcommand" -f -a "lsp" -d 'Start an LSP server (for integration with editors)'
complete -c clarinet -n "__fish_use_subcommand" -f -a "completions" -d 'Generate shell completions scripts'
complete -c clarinet -n "__fish_seen_subcommand_from new" -l help -d 'Print help information'
complete -c clarinet -n "__fish_seen_subcommand_from new" -l version -d 'Print version information'
complete -c clarinet -n "__fish_seen_subcommand_from new" -l disable-telemetry -d 'Do not provide developer usage telemetry for this project'
complete -c clarinet -n "__fish_seen_subcommand_from contract; and not __fish_seen_subcommand_from new; and not __fish_seen_subcommand_from requirement; and not __fish_seen_subcommand_from fork" -l help -d 'Print help information'
complete -c clarinet -n "__fish_seen_subcommand_from contract; and not __fish_seen_subcommand_from new; and not __fish_seen_subcommand_from requirement; and not __fish_seen_subcommand_from fork" -l version -d 'Print version information'
complete -c clarinet -n "__fish_seen_subcommand_from contract; and not __fish_seen_subcommand_from new; and not __fish_seen_subcommand_from requirement; and not __fish_seen_subcommand_from fork" -f -a "new" -d 'Generate files and settings for a new contract'
complete -c clarinet -n "__fish_seen_subcommand_from contract; and not __fish_seen_subcommand_from new; and not __fish_seen_subcommand_from requirement; and not __fish_seen_subcommand_from fork" -f -a "requirement" -d 'Add third-party requirements to this project'
complete -c clarinet -n "__fish_seen_subcommand_from contract; and not __fish_seen_subcommand_from new; and not __fish_seen_subcommand_from requirement; and not __fish_seen_subcommand_from fork" -f -a "fork" -d 'Replicate a third-party contract into this project'
complete -c clarinet -n "__fish_seen_subcommand_from contract; and __fish_seen_subcommand_from new" -l manifest-path -d 'Path to Clarinet.toml' -r
complete -c clarinet -n "__fish_seen_subcommand_from contract; and __fish_seen_subcommand_from new" -l help -d 'Print help information'
complete -c clarinet -n "__fish_seen_subcommand_from contract; and __fish_seen_subcommand_from new" -l version -d 'Print version information'
complete -c clarinet -n "__fish_seen_subcommand_from contract; and __fish_seen_subcommand_from requirement" -l manifest-path -d 'Path to Clarinet.toml' -r
complete -c clarinet -n "__fish_seen_subcommand_from contract; and __fish_seen_subcommand_from requirement" -l help -d 'Print help information'
complete -c clarinet -n "__fish_seen_subcommand_from contract; and __fish_seen_subcommand_from requirement" -l version -d 'Print version information'
complete -c clarinet -n "__fish_seen_subcommand_from contract; and __fish_seen_subcommand_from fork" -l manifest-path -d 'Path to Clarinet.toml' -r
complete -c clarinet -n "__fish_seen_subcommand_from contract; and __fish_seen_subcommand_from fork" -l help -d 'Print help information'
complete -c clarinet -n "__fish_seen_subcommand_from contract; and __fish_seen_subcommand_from fork" -l version -d 'Print version information'
complete -c clarinet -n "__fish_seen_subcommand_from console" -l manifest-path -d 'Path to Clarinet.toml' -r
complete -c clarinet -n "__fish_seen_subcommand_from console" -l help -d 'Print help information'
complete -c clarinet -n "__fish_seen_subcommand_from console" -l version -d 'Print version information'
complete -c clarinet -n "__fish_seen_subcommand_from test" -l manifest-path -d 'Path to Clarinet.toml' -r
complete -c clarinet -n "__fish_seen_subcommand_from test" -l help -d 'Print help information'
complete -c clarinet -n "__fish_seen_subcommand_from test" -l version -d 'Print version information'
complete -c clarinet -n "__fish_seen_subcommand_from test" -l coverage -d 'Generate coverage file (coverage.lcov)'
complete -c clarinet -n "__fish_seen_subcommand_from test" -l costs -d 'Generate costs report'
complete -c clarinet -n "__fish_seen_subcommand_from test" -l watch -d 'Relaunch tests upon updates to contracts'
complete -c clarinet -n "__fish_seen_subcommand_from check" -l manifest-path -d 'Path to Clarinet.toml' -r
complete -c clarinet -n "__fish_seen_subcommand_from check" -l help -d 'Print help information'
complete -c clarinet -n "__fish_seen_subcommand_from check" -l version -d 'Print version information'
complete -c clarinet -n "__fish_seen_subcommand_from publish" -l manifest-path -d 'Path to Clarinet.toml' -r
complete -c clarinet -n "__fish_seen_subcommand_from publish" -l help -d 'Print help information'
complete -c clarinet -n "__fish_seen_subcommand_from publish" -l version -d 'Print version information'
complete -c clarinet -n "__fish_seen_subcommand_from publish" -l devnet -d 'Deploy contracts on devnet, using settings/Devnet.toml'
complete -c clarinet -n "__fish_seen_subcommand_from publish" -l testnet -d 'Deploy contracts on testnet, using settings/Testnet.toml'
complete -c clarinet -n "__fish_seen_subcommand_from publish" -l mainnet -d 'Deploy contracts on mainnet, using settings/Mainnet.toml'
complete -c clarinet -n "__fish_seen_subcommand_from run" -l manifest-path -d 'Path to Clarinet.toml' -r
complete -c clarinet -n "__fish_seen_subcommand_from run" -l help -d 'Print help information'
complete -c clarinet -n "__fish_seen_subcommand_from run" -l version -d 'Print version information'
complete -c clarinet -n "__fish_seen_subcommand_from run" -l allow-wallets -d 'Allow access to wallets'
complete -c clarinet -n "__fish_seen_subcommand_from run" -l allow-write -d 'Allow write access to disk'
complete -c clarinet -n "__fish_seen_subcommand_from run" -l allow-read -d 'Allow read access to disk'
complete -c clarinet -n "__fish_seen_subcommand_from integrate" -l manifest-path -d 'Path to Clarinet.toml' -r
complete -c clarinet -n "__fish_seen_subcommand_from integrate" -l help -d 'Print help information'
complete -c clarinet -n "__fish_seen_subcommand_from integrate" -l version -d 'Print version information'
complete -c clarinet -n "__fish_seen_subcommand_from integrate" -l no-dashboard -d 'Display streams of logs instead of terminal UI dashboard'
complete -c clarinet -n "__fish_seen_subcommand_from lsp" -l help -d 'Print help information'
complete -c clarinet -n "__fish_seen_subcommand_from lsp" -l version -d 'Print version information'
complete -c clarinet -n "__fish_seen_subcommand_from completions" -l help -d 'Print help information'
complete -c clarinet -n "__fish_seen_subcommand_from completions" -l version -d 'Print version information'

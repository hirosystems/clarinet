# Custom Boot Contracts Example

This example demonstrates how to use override boot contracts in Clarinet with custom implementations.

## Overview

Clarinet embeds a copy of the boot contracts (like `pox-4`, `costs`, etc.) that are used by default. For Stacks core developers, it's useful to be able to load custom code instead of the embedded versions, or add new boot contracts entirely.

## Configuration

To override boot contracts, add an `[override_boot_contracts_source]` section to your `Clarinet.toml`:

```toml
[project]
name = "my-project"
# ... other project settings

[project.override_boot_contracts_source]
pox-4 = "./custom-boot-contracts/pox-4.clar"
pox-5 = "./custom-boot-contracts/pox-5.clar"
```

## Supported Boot Contract Overrides

You can override any of the following boot contracts:
- `genesis`
- `lockup`
- `bns`
- `cost-voting`
- `costs`
- `pox`
- `costs-2`
- `pox-2`
- `costs-3`
- `pox-3`
- `pox-4`
- `signers`
- `signers-voting`

## How It Works

1. When Clarinet loads boot contracts, it first checks if there are any overrides specified in the `Clarinet.toml`
2. If an override is found for a specific boot contract, it loads the custom source from the specified file path
3. If it's not overriding an existing contract, it will additionally load it alongside other boot contracts
3. The custom source is used instead of the embedded version
4. If the custom file cannot be loaded, a warning is printed and the embedded version is used as fallback

## Example Usage

In this example:

1. **Custom PoX-4 Contract** (`custom-boot-contracts/pox-4.clar`): Overrides the default PoX-4 contract with custom logic
3. **Example Contract** (`contracts/example-contract.clar`): Demonstrates how to interact with the custom boot contracts

## Testing

To test this example:

```bash
cd components/clarinet-cli/examples/custom-boot-contracts
clarinet console
```

In the console, you can test the custom boot contracts:

```clarity
;; Test the custom pox-4 contract
(contract-call? 'SP000000000000000000002Q6VF78.pox-4 get-pox-info)

;; Deploy and test the example contract
(contract-call? 'ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5.example-contract test-custom-pox-info)
```

## Use Cases

This feature is particularly useful for:
- Stacks core developers testing changes to boot contracts
- Research and experimentation with different boot contract implementations
- Custom blockchain configurations for development and testing
- Educational purposes to understand how boot contracts work

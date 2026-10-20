# Clarinet SDK Workspace

This workspace regroups
`@stacks/clarinet-sdk` for node.js and `@stacks/clarinet-sdk-browser` for web browsers.  
They respectively rely on `@stacks/clarinet-sdk-wasm` and `@stacks/clarinet-sdk-wasm-browser`.

The Wasm packages are built separately for Node.js and browsers. Both SDKs share
`./common/src/sdkProxy.ts`, with small adapters in `./node/src/sdkProxy.ts` and
`./browser/src/sdkProxy.ts`.

## Transaction behavior

Simnet public calls, deployments, and native STX transfers use the shared
stacks-core Clarity transaction frame. Included execution failures and
post-condition aborts roll back payload writes but consume the sender's nonce;
rejected transactions consume no nonce. Existing convenience calls do not charge
transaction fees.

Deployments use stacks-node's initializer. A successful `deployContract` returns
`Cl.ok(Cl.bool(true))`, regardless of the initializer's final expression, and
stores the contract even if it defines no functions. Duplicate contract names
are rejected before analysis without consuming a nonce. Use read-only calls,
`getDataVar`, and events to check initialization effects. Use `execute` when you
want to evaluate a snippet and receive its expression value. Deployment costs
include contract storage and subsequent calls account for the stored contract's
actual data size, so cost assertions may need updating.

`transferSTX` represents a native transfer: self-transfers, zero-amount transfers,
and transfers exceeding the available unlocked balance throw without consuming a
nonce. Inside `mineBlock`, the same transfers instead return the `stx-transfer?`
error response (`(err u1)`, `(err u2)` or `(err u3)`), consume a nonce, and the
rest of the block is still processed. This differs
from calling the Clarity `stx-transfer?` function inside a contract, which retains
its ordinary response semantics. Private calls remain a simnet convenience and
use the same transaction frame.

## Signed transactions

`simnet.broadcastTransaction(rawHex)` accepts a complete signed Stacks transaction
serialized by Stacks.js. It verifies signatures, network/epoch rules, and origin
and sponsor nonces; charges the fee from unlocked STX; and executes native STX
transfers, public contract calls, or deployments. Ordinary simnet uses testnet
transactions. Supply an explicit fee and nonce when constructing a transaction
so Stacks.js does not fetch them from a remote node.

```ts
const transaction = await makeSTXTokenTransfer({
  network: "testnet",
  senderKey,
  recipient,
  amount: 100n,
  fee: 10n,
  nonce: simnet.getAccountNonce(sender),
});
const outcome = simnet.broadcastTransaction(transaction.serialize());
if (outcome.status === "success") {
  console.log(outcome.transaction.txid, outcome.execution.result);
}
```

The result is a discriminated union:

- `success`: includes `transaction` metadata and the usual parsed `execution`.
- `abort_by_response`, `abort_by_post_condition`, `runtime_error`, or
  `analysis_error`: included failures retain the fee and nonce charges. Results
  include transaction metadata, an error message, an optional Clarity `result`,
  attempted events, and costs. Aborted payload writes and asset-cache updates
  are rolled back; attempted events do not imply committed asset movement.
  With performance tracking enabled, `abort_by_response` also includes
  `performance` data; other failure statuses do not.
- `rejected`: nothing is charged or executed permanently. `transaction` is
  present if decoding succeeded, otherwise `null`.

`transaction` contains `txid` (64 lowercase hex characters, without `0x`), `fee`
and `nonce` (both `bigint`). The ID is calculated from the complete signed
transaction and matches Stacks.js. Replaying an included transaction is rejected
by nonce validation. Included submissions execute in the next block and advance
the chain by one block; rejections do not advance it.

Raw payloads execute exactly as signed: contract source, boot addresses,
post-conditions, and transfer memo bytes are not rewritten. Private calls and
coinbase, tenure-change, and poison-microblock payloads are unsupported. Existing
convenience calls remain unsigned and fee-free. `mineBlock` behavior is unchanged;
raw batch entries and batch alignment are separate work. This method does not
submit to a network or emulate a mempool.

## Contributing

The clarinet-sdk requires a few steps to be built and tested locally.

Clone the clarinet repo and `cd` into it:

```sh
git clone git@github.com:stx-labs/clarinet.git
cd clarinet
```

Open the SDK workspace in VSCode, it's especially useful to get rust-analyzer
to consider the right files with the right cargo features.

```sh
code components/clarinet-sdk/clarinet-sdk.code-workspace
```

The SDK mainly relies on two components:

- the Rust component: `components/clarinet-sdk-wasm`
- the TS component: `components/clarinet-sdk`

To work with these two packages locally, the first one needs to be built with
wasm-pack (install [wasm-pack](https://wasm-bindgen.github.io/wasm-pack/installer)).

```sh
# install dependencies without running prepare scripts
# (the SDK packages' `prepare` hook depends on the Wasm build,
# which doesn't exist yet on a fresh clone)
pnpm install --ignore-scripts
# build the Wasm package
pnpm run build:sdk-wasm
# install dependencies and build the node package
pnpm install
# make sure the installation works
pnpm test
```

### Release

The Node.js and browser versions can be published with this single command.
Check both package versions first.

```sh
# the Wasm package must be published first
# $ pnpm run publish:sdk-wasm
pnpm run publish:sdk
```

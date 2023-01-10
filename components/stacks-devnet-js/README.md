# stacks-devnet-js

`stacks-devnet-js` is a node library, designed to let developers write integration tests for decentralized protocols built on top of the Stacks blockchain.
It is implemented as a dynamic library that can be loaded by Node, and will let you orchestrate a Stacks Devnet network, locally, using Docker.

### Installation

```bash
# Yarn
yarn add dev @hirosystems/stacks-devnet-js

# NPM
npm install --save-dev @hirosystems/stacks-devnet-js
```

If any error occurs during the installation of this package, feel free to open an issue on this repository.


### Usage

```typescript
import {
  makeSTXTokenTransfer,
  broadcastTransaction,
  AnchorMode,
} from '@stacks/transactions';
import { StacksTestnet }from '@stacks/network';
import { DevnetNetworkOrchestrator } from "@hirosystems/stacks-devnet-js";
import { describe, expect, it, beforeAll, afterAll } from 'vitest'
import BigNum from 'bn.js';

describe('Full end to end integration tests made simple', () => {
    let orchestrator: DevnetNetworkOrchestrator;

    beforeAll(async (ctx) => {
        orchestrator = buildDevnetNetworkOrchestrator(getNetworkIdFromCtx(ctx.id));
        orchestrator.start()
    });

    afterAll(() => {
        orchestrator.terminate();
    });

    it('submitting stacks-stx through pox-1 contract during epoch 2.0 should succeed', async () => {
      // Let's wait for our Genesis block
      var block = orchestrator.waitForNextStacksBlock();

      // Build a transaction
      const txOptions = {
        recipient: 'ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5',
        amount: new BigNum(12345),
        senderKey: '753b7cc01a1a2e86221266a154af739463fce51219d97e4f856cd7200c3bd2a601',
        network,
        memo: 'test memo',
        nonce: new BigNum(0), // set a nonce manually if you don't want builder to fetch from a Stacks node
        fee: new BigNum(200), // set a tx fee if you don't want the builder to estimate
        anchorMode: AnchorMode.OnChainOnly
      };
      const transaction = await makeSTXTokenTransfer(txOptions);

      // Broadcast transaction to our Devnet stacks node
      await broadcastTransaction(transaction, network);

      // Wait for the next block
      block = orchestrator.waitForNextStacksBlock();

      // Ensure that the transaction was included in the block
      console.log(`Next Block: ${JSON.stringify(block)}`);
    }
})
```

### Case Study

The repo [stacks-2-1-testing](http://github.com/hirosystems/stacks-2-1-testing) is using this library for testing, end to end, the Stacks 2.1 Proof of Transfer contract along with all the new Clarity 2 functions.

### Screencasts

A series of short tutorials is available as a playlist of screencasts on Youtube, covering the following subjects:

- [Introduction to smart contract integration with Clarinet](https://youtu.be/pucJ_tOC3pk)
- [Setup a React project interacting with Clarinet](https://youtu.be/b7iipqzTUH8)
- [Setup an integration test environment with stacks-devnet-js](https://youtu.be/BqeL17m1dZk)

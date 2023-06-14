import {
  Clarinet,
  Tx,
  Chain,
  Account,
  types,
} from "https://deno.land/x/clarinet@v1.5.4/index.ts";

Clarinet.test({
  name: "Ensure that nft can be transferred form one account to another",
  fn(chain: Chain, accounts: Map<string, Account>) {
    const deployer = accounts.get("deployer")!;
    const wallet_1 = accounts.get("wallet_1")!;
    const wallet_2 = accounts.get("wallet_2")!;

    let block = chain.mineBlock([
      Tx.contractCall(
        "simple-nft",
        "test-mint",
        [types.principal(wallet_1.address)],
        wallet_1.address
      ),
    ]);

    block.receipts[0].result.expectOk().expectBool(true);

    block = chain.mineBlock([
      Tx.contractCall(
        "simple-nft",
        "transfer",
        [
          types.uint(1),
          types.principal(wallet_1.address),
          types.principal(wallet_2.address),
        ],
        wallet_1.address
      ),
    ]);

    block.receipts[0].result.expectOk().expectBool(true);

    block.receipts[0].events.expectNonFungibleTokenTransferEvent(
      types.uint(1),
      wallet_1.address,
      wallet_2.address,
      `${deployer.address}.simple-nft`,
      "nft"
    );
  },
});

Clarinet.test({
  name: "Ensure that nft can be minted",
  fn(chain: Chain, accounts: Map<string, Account>) {
    const deployer = accounts.get("deployer")!;
    const wallet_1 = accounts.get("wallet_1")!;

    const block = chain.mineBlock([
      Tx.contractCall(
        "simple-nft",
        "test-mint",
        [types.principal(wallet_1.address)],
        wallet_1.address
      ),
    ]);

    block.receipts[0].result.expectOk().expectBool(true);

    block.receipts[0].events.expectNonFungibleTokenMintEvent(
      types.uint(1),
      wallet_1.address,
      `${deployer.address}.simple-nft`,
      "nft"
    );
  },
});

Clarinet.test({
  name: "Ensure that nft can be burned",
  fn(chain: Chain, accounts: Map<string, Account>) {
    const deployer = accounts.get("deployer")!;
    const wallet_1 = accounts.get("wallet_1")!;

    let block = chain.mineBlock([
      Tx.contractCall(
        "simple-nft",
        "test-mint",
        [types.principal(wallet_1.address)],
        wallet_1.address
      ),
    ]);

    block.receipts[0].result.expectOk().expectBool(true);

    block = chain.mineBlock([
      Tx.contractCall(
        "simple-nft",
        "test-burn",
        [types.uint(1), types.principal(wallet_1.address)],
        wallet_1.address
      ),
    ]);
    block.receipts[0].result.expectOk().expectBool(true);

    block.receipts[0].events.expectNonFungibleTokenBurnEvent(
      types.uint(1),
      wallet_1.address,
      `${deployer.address}.simple-nft`,
      "nft"
    );
  },
});

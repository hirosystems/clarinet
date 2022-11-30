export type ExpectSTXTransferEvent = {
  amount: bigint;
  sender: string;
  recipient: string;
};

export type ExpectSTXBurnEvent = {
  amount: bigint;
  sender: string;
};

export type ExpectFungibleTokenTransferEvent = {
  amount: bigint;
  sender: string;
  recipient: string;
  assetId: string;
};

export type ExpectFungibleTokenMintEvent = {
  amount: bigint;
  recipient: string;
  assetId: string;
};

export type ExpectFungibleTokenBurnEvent = {
  amount: bigint;
  sender: string;
  assetId: string;
};

export type ExpectPrintEvent = {
  contract_identifier: string;
  value: string;
};

export type ExpectNonFungibleTokenTransferEvent = {
  tokenId: string;
  sender: string;
  recipient: string;
  assetId: string;
};

export type ExpectNonFungibleTokenMintEvent = {
  tokenId: string;
  recipient: string;
  assetId: string;
};

export type ExpectNonFungibleTokenBurnEvent = {
  tokenId: string;
  sender: string;
  assetId: string;
};

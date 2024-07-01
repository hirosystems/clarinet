import { Cl, ClarityValue } from "@stacks/transactions";

export type ClarityEvent = {
  event: string;
  data: { raw_value?: string; value?: ClarityValue; [key: string]: any };
};

export type ParsedTransactionResult = {
  result: ClarityValue;
  events: ClarityEvent[];
};

export type CallFn = (
  contract: string,
  method: string,
  args: ClarityValue[],
  sender: string,
) => ParsedTransactionResult;

export type DeployContractOptions = {
  clarityVersion: 1 | 2 | 3;
};
export type DeployContract = (
  name: string,
  content: string,
  options: DeployContractOptions | null,
  sender: string,
) => ParsedTransactionResult;

export type TransferSTX = (
  amount: number | bigint,
  recipient: string,
  sender: string,
) => ParsedTransactionResult;

export type Tx =
  | {
      callPublicFn: {
        contract: string;
        method: string;
        args: ClarityValue[];
        sender: string;
      };
      callPrivateFn?: never;
      deployContract?: never;
      transferSTX?: never;
    }
  | {
      callPublicFn?: never;
      callPrivateFn: {
        contract: string;
        method: string;
        args: ClarityValue[];
        sender: string;
      };
      deployContract?: never;
      transferSTX?: never;
    }
  | {
      callPublicFn?: never;
      callPrivateFn?: never;
      deployContract: {
        name: string;
        content: string;
        options: DeployContractOptions | null;
        sender: string;
      };
      transferSTX?: never;
    }
  | {
      callPublicFn?: never;
      callPrivateFn?: never;
      deployContradct?: never;
      transferSTX: { amount: number; recipient: string; sender: string };
    };

export const tx = {
  callPublicFn: (contract: string, method: string, args: ClarityValue[], sender: string): Tx => ({
    callPublicFn: { contract, method, args, sender },
  }),
  callPrivateFn: (contract: string, method: string, args: ClarityValue[], sender: string): Tx => ({
    callPrivateFn: { contract, method, args, sender },
  }),
  deployContract: (
    name: string,
    content: string,
    options: DeployContractOptions | null,
    sender: string,
  ): Tx => ({
    deployContract: { name, content, options, sender },
  }),
  transferSTX: (amount: number, recipient: string, sender: string): Tx => ({
    transferSTX: { amount, recipient, sender },
  }),
};

export function parseEvents(events: string): ClarityEvent[] {
  try {
    // @todo: improve type safety
    return JSON.parse(events).map((e: string) => {
      const { event, data } = JSON.parse(e);
      if ("raw_value" in data) {
        data.value = Cl.deserialize(data.raw_value);
      }
      return {
        event: event,
        data: data,
      };
    });
  } catch (e) {
    console.error(`Fail to parse events: ${e}`);
    return [];
  }
}

export type MineBlock = (txs: Array<Tx>) => ParsedTransactionResult[];
export type Execute = (snippet: string) => ParsedTransactionResult;
export type GetDataVar = (contract: string, dataVar: string) => ClarityValue;
export type GetMapEntry = (contract: string, mapName: string, mapKey: ClarityValue) => ClarityValue;

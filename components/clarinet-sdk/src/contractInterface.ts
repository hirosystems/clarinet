export type ContractInterfaceFunctionAccess = "private" | "public" | "read_only";

export type ContractInterfaceTupleEntryType = { name: string; type: ContractInterfaceAtomType };

export type ContractInterfaceAtomType =
  | "none"
  | "int128"
  | "uint128"
  | "bool"
  | "principal"
  | { buffer: { length: number } }
  | { "string-utf8": { length: number } }
  | { "string-ascii": { length: number } }
  | { tuple: ContractInterfaceTupleEntryType[] }
  | { optional: ContractInterfaceAtomType }
  | { response: { ok: ContractInterfaceAtomType; error: ContractInterfaceAtomType } }
  | { list: { type: ContractInterfaceAtomType; length: number } }
  | "trait_reference";

export type ContractInterfaceFunctionArg = { name: string; type: ContractInterfaceAtomType };

export type ContractInterfaceFunctionOutput = { type: ContractInterfaceAtomType };

export type ContractInterfaceFunction = {
  name: string;
  access: ContractInterfaceFunctionAccess;
  args: ContractInterfaceFunctionArg[];
  outputs: ContractInterfaceFunctionOutput;
};

export type ContractInterfaceVariableAccess = "constant" | "variable";

export type ContractInterfaceVariable = {
  name: string;
  type: ContractInterfaceAtomType;
  access: ContractInterfaceVariableAccess;
};

export type ContractInterfaceMap = {
  name: string;
  key: ContractInterfaceAtomType;
  value: ContractInterfaceAtomType;
};

export type ContractInterfaceFungibleTokens = { name: string };

export type ContractInterfaceNonFungibleTokens = { name: string; type: ContractInterfaceAtomType };

export type StacksEpochId =
  | "Epoch10"
  | "Epoch20"
  | "Epoch2_05"
  | "Epoch21"
  | "Epoch22"
  | "Epoch23"
  | "Epoch24";

export type ClarityVersion = "Clarity1" | "Clarity2";

export type ContractInterface = {
  functions: ContractInterfaceFunction[];
  variables: ContractInterfaceVariable[];
  maps: ContractInterfaceMap[];
  fungible_tokens: ContractInterfaceFungibleTokens[];
  non_fungible_tokens: ContractInterfaceNonFungibleTokens[];
  epoch: StacksEpochId;
  clarity_version: ClarityVersion;
};

type Atom = {
  Atom: String;
};

type AtomValue = {
  AtomValue: any;
};

type List = {
  List: Expression[];
};

type LiteralValue = {
  LiteralValue: any;
};

type Field = {
  Field: any;
};

type TraitReference = {
  TraitReference: any;
};

type ExpressionType = Atom | AtomValue | List | LiteralValue | Field | TraitReference;

type Span = {
  start_line: number;
  start_column: number;
  end_line: number;
  end_column: number;
};

type Expression = {
  expr: ExpressionType;
  id: number;
  span: Span;
};

/** ContractAST basic type. To be improved */
export type ContractAST = {
  contract_identifier: any;
  pre_expressions: any[];
  expressions: Expression[];
  top_level_expression_sorting: number[];
  referenced_traits: Map<any, any>;
  implemented_traits: any[];
};

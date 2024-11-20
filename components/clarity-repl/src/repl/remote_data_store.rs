use clarity::vm::errors::{Error, InterpreterError, InterpreterResult as Result};
use clarity::vm::representations::{ClarityName, Span, TraitDefinition};
use clarity::vm::types::{TraitIdentifier, Value};

use serde_json::Value as JsonValue;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct RawSymbolicExpression {
    pub expr: RawSymbolicExpressionType,
    pub id: u64,

    #[serde(default)]
    pub span: Span,
    #[serde(default)]
    pub pre_comments: Vec<(String, Span)>,
    #[serde(default)]
    pub post_comments: Vec<(String, Span)>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
enum RawSymbolicExpressionType {
    AtomValue(Value),
    Atom(ClarityName),
    List(Vec<RawSymbolicExpression>),
    LiteralValue(Value),
    Field(TraitIdentifier),
    TraitReference(ClarityName, TraitDefinition),
}

// fn add_symbolic_expr_properties(mut exprs: Vec<RawSymbolicExpression>) {
//     for mut expr in exprs.into_iter() {
//         expr.span = Some(Span::zero());
//         expr.pre_comments = Some(vec![]);
//         expr.post_comments = Some(vec![]);
//         if let RawSymbolicExpressionType::List(mut list) = expr.expr {
//             add_symbolic_expr_properties(list);
//         }
//     }
// }

pub fn modify_function_bodies(json_str: &str) -> Result<String> {
    let mut value: JsonValue = serde_json::from_str(json_str)
        .map_err(|e| Error::Interpreter(InterpreterError::Expect(e.to_string())))?;

    // Navigate to the functions
    if let Some(contract) = value.get_mut("contract_context") {
        if let Some(functions) = contract.get_mut("functions") {
            if let Some(obj) = functions.as_object_mut() {
                // Iterate through all functions
                for (_name, function) in obj {
                    if let Some(body) = function.get_mut("body") {
                        // Deserialize the body into our known type
                        let expr: RawSymbolicExpression = serde_json::from_value(body.clone())
                            .map_err(|e| {
                                Error::Interpreter(InterpreterError::Expect(e.to_string()))
                            })?;

                        // add_symbolic_expr_properties(vec![expr.clone()]);

                        // Replace the old body with the modified version
                        *body = serde_json::to_value(expr).map_err(|e| {
                            Error::Interpreter(InterpreterError::Expect(e.to_string()))
                        })?;
                    }
                }
            }
        }
    }

    serde_json::to_string(&value)
        .map_err(|e| Error::Interpreter(InterpreterError::Expect(e.to_string())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_modify_function_bodies_with_function() {
        let raw_json = r#"{
          "contract_context": {
            "variables": {
              "ERR_BLOCK_NOT_FOUND": {
                "Response": { "committed": false, "data": { "UInt": 1003 } }
              }
            },
            "functions": {
              "get-count": {
                "identifier": {
                  "identifier": "ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5.counter:get-count"
                },
                "name": "get-count",
                "arg_types": [],
                "define_type": "ReadOnly",
                "arguments": [],
                "body": {
                  "expr": {
                    "List": [
                      { "expr": { "Atom": "var-get" }, "id": 2 },
                      { "expr": { "Atom": "count" }, "id": 3 }
                    ]
                  },
                  "id": 1
                }
              }
            },
            "persisted_names": ["cost"]
          }
        }"#;

        let result = modify_function_bodies(raw_json).unwrap();
        let expected = json!({
          "contract_context": {
            "variables": {
              "ERR_BLOCK_NOT_FOUND": {
                "Response": { "committed": false, "data": { "UInt": 1003 } }
              }
            },
            "functions": {
              "get-count": {
                "identifier": {
                  "identifier": "ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5.counter:get-count"
                },
                "name": "get-count",
                "arg_types": [],
                "define_type": "ReadOnly",
                "arguments": [],
                "body": {
                  "expr": {
                    "List": [
                      {
                        "expr": { "Atom": "var-get" },
                        "id": 2,
                        "span": Span::zero(),
                        "pre_comments": [],
                        "post_comments": []
                      },
                      {
                        "expr": { "Atom": "count" },
                        "id": 3,
                        "span": Span::zero(),
                        "pre_comments": [],
                        "post_comments": []
                      }
                    ]
                  },
                  "id": 1,
                  "span": Span::zero(),
                  "pre_comments": [],
                  "post_comments": []
                }
              }
            },
            "persisted_names": ["cost"]
          }
        })
        .to_string();

        assert_eq!(result, expected);
    }
}

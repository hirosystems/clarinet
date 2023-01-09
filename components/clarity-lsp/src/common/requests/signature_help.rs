use clarity_repl::clarity::{docs::FunctionAPI, ClarityName, SymbolicExpression};
use lsp_types::{ParameterInformation, ParameterLabel, Position, SignatureInformation};

use crate::state::ActiveContractData;

use super::{api_ref::API_REF, helpers::is_position_within_span};

fn get_function_at_position(
    position: &Position,
    expressions: &Vec<SymbolicExpression>,
) -> Option<(ClarityName, Option<u32>)> {
    for expr in expressions {
        if is_position_within_span(position, &expr.span, 0) {
            if let Some(expressions) = expr.match_list() {
                return get_function_at_position(position, &expressions.to_vec());
            }
        }
    }

    let mut position_in_parameters: i32 = -1;
    for parameter in expressions {
        if position.line == parameter.span.end_line {
            if position.character > parameter.span.end_column + 1 {
                position_in_parameters += 1
            }
        } else if position.line > parameter.span.end_line {
            position_in_parameters += 1
        }
    }

    let (function_name, _) = expressions.split_first()?;

    return Some((
        function_name.match_atom()?.to_owned(),
        position_in_parameters.try_into().ok(),
    ));
}

pub fn get_signatures(
    contract: &ActiveContractData,
    position: &Position,
) -> Option<Vec<SignatureInformation>> {
    let (function_name, mut active_parameter) =
        get_function_at_position(position, contract.expressions.as_ref()?)?;

    if ["let", "begin"].contains(&function_name.as_str()) {
        // showing signature help for let and begin adds to much noise
        return None;
    }

    let (version, _, reference) = API_REF.get(&function_name.to_string())?;
    let FunctionAPI {
        signature,
        output_type,
        ..
    } = (*reference).as_ref()?;

    if version > &contract.clarity_version {
        return None;
    }

    let signatures = signature
        .split(" |")
        .map(|mut signature| {
            signature = signature.trim();
            let mut signature_without_parenthesis = signature.chars();
            signature_without_parenthesis.next();
            signature_without_parenthesis.next_back();
            let signature_without_parenthesis = signature_without_parenthesis.as_str();
            let parameters = signature_without_parenthesis
                .split(" ")
                .collect::<Vec<&str>>();
            let (_, parameters) = parameters.split_first().expect("invalid signature format");

            if active_parameter.unwrap_or_default() >= parameters.len().try_into().unwrap() {
                if let Some(variadic_index) = parameters.iter().position(|p| p.contains("...")) {
                    active_parameter = Some(variadic_index.try_into().unwrap());
                }
            }
            SignatureInformation {
                active_parameter,
                documentation: None,
                label: format!("{} -> {}", &signature, &output_type),
                parameters: Some(
                    parameters
                        .iter()
                        .map(|param| ParameterInformation {
                            documentation: None,
                            label: ParameterLabel::Simple(param.to_string()),
                        })
                        .collect::<Vec<ParameterInformation>>(),
                ),
            }
        })
        .collect::<Vec<SignatureInformation>>();

    Some(signatures)
}

#[cfg(test)]
mod definitions_visitor_tests {
    use clarity_repl::clarity::ClarityVersion::Clarity2;
    use clarity_repl::clarity::{
        functions::NativeFunctions, stacks_common::types::StacksEpochId::Epoch21,
    };
    use lsp_types::{ParameterInformation, ParameterLabel::Simple, Position, SignatureInformation};

    use crate::state::ActiveContractData;

    use super::get_signatures;

    fn get_source_signature(
        source: &str,
        position: &Position,
    ) -> Option<Vec<lsp_types::SignatureInformation>> {
        let contract = &ActiveContractData::new(Clarity2, Epoch21, None, source);
        get_signatures(&contract, position)
    }

    #[test]
    fn get_simple_signature() {
        let signatures = get_source_signature(
            "(var-set counter )",
            &Position {
                line: 1,
                character: 18,
            },
        );

        assert!(signatures.is_some());
        let signatures = signatures.unwrap();
        assert_eq!(signatures.len(), 1);
        assert_eq!(
            signatures.get(0).unwrap(),
            &SignatureInformation {
                label: "(var-set var-name expr1) -> bool".to_string(),
                documentation: None,
                parameters: Some(
                    [
                        ParameterInformation {
                            label: Simple("var-name".to_string()),
                            documentation: None,
                        },
                        ParameterInformation {
                            label: Simple("expr1".to_string()),
                            documentation: None,
                        },
                    ]
                    .to_vec(),
                ),
                active_parameter: Some(1),
            }
        );
    }

    #[test]
    fn ensure_all_native_function_have_valid_signature() {
        for method in NativeFunctions::ALL_NAMES {
            if ["let", "begin"].contains(&method) {
                continue;
            }

            let src = format!("({} )", &method);
            let signatures = get_source_signature(
                src.as_str(),
                &Position {
                    line: 1,
                    character: 2,
                },
            );
            assert!(signatures.is_some());
            match *method {
                "match" => {
                    assert_eq!(signatures.unwrap().len(), 2)
                }
                _ => {
                    assert_eq!(signatures.unwrap().len(), 1)
                }
            }
        }
    }
}

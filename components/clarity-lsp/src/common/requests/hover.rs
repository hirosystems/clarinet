use clarity_repl::clarity::{ClarityVersion, SymbolicExpression};
use lsp_types::Position;

use super::{api_ref::API_REF, helpers::get_expression_name_at_position};

pub fn get_expression_documentation(
    position: &Position,
    clarity_version: ClarityVersion,
    expressions: &Vec<SymbolicExpression>,
) -> Option<String> {
    let expression_name = get_expression_name_at_position(position, expressions)?;

    match API_REF.get(&expression_name.to_string()) {
        Some((version, documentation, _)) => {
            if version <= &clarity_version {
                return Some(documentation.to_owned());
            }
            None
        }
        None => None,
    }
}

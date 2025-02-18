use clarity::vm::representations::PreSymbolicExpression;

/// trim but leaves newlines preserved
pub fn t(input: &str) -> &str {
    let start = input
        .find(|c: char| !c.is_whitespace() || c == '\n')
        .unwrap_or(0);

    let end = input
        .rfind(|c: char| !c.is_whitespace() || c == '\n')
        .map(|pos| pos + 1)
        .unwrap_or(0);

    &input[start..end]
}
/// REMOVE: just grabs the 1st and rest from a PSE
pub fn name_and_args(
    exprs: &[PreSymbolicExpression],
) -> Option<(&PreSymbolicExpression, &[PreSymbolicExpression])> {
    if exprs.len() >= 2 {
        Some((&exprs[1], &exprs[2..]))
    } else {
        None // Return None if there aren't enough items
    }
}

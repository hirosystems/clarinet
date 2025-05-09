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

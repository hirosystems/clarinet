use std::str::Chars;

pub fn check_if_should_wrap(chars: &mut Chars, column: usize) -> bool {
    let mut index = column;
    while index > 0 {
        index -= 1;

        match chars.nth(index) {
            Some('(') => return false,
            Some(char) => return char.is_whitespace(),
            None => return true,
        }
    }
    true
}

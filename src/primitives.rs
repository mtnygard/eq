/// Primitive formatting utilities for EDN values

/// Escape special characters in strings
pub fn escape_string(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '"' => "\\\"".to_string(),
            '\\' => "\\\\".to_string(),
            '\n' => "\\n".to_string(),
            '\r' => "\\r".to_string(),
            '\t' => "\\t".to_string(),
            c => c.to_string(),
        })
        .collect()
}

/// Format character literals properly
pub fn format_character(c: char) -> String {
    match c {
        '\n' => "\\newline".to_string(),
        '\t' => "\\tab".to_string(),
        '\r' => "\\return".to_string(),
        ' ' => "\\space".to_string(),
        c => format!("\\{}", c),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_strings() {
        assert_eq!(escape_string("hello\nworld"), "hello\\nworld");
        assert_eq!(escape_string("quote\"test"), "quote\\\"test");
        assert_eq!(escape_string("backslash\\test"), "backslash\\\\test");
    }

    #[test]
    fn test_character_formatting() {
        assert_eq!(format_character('a'), "\\a");
        assert_eq!(format_character('\n'), "\\newline");
        assert_eq!(format_character('\t'), "\\tab");
        assert_eq!(format_character(' '), "\\space");
    }
}
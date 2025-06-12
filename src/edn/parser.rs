use crate::edn::EdnValue;
use crate::error::{EqError, EqResult};
use indexmap::IndexMap;
use std::collections::HashSet;

#[derive(Debug)]
pub struct Parser {
    input: Vec<char>,
    position: usize,
    line: usize,
    column: usize,
}

impl Parser {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            position: 0,
            line: 1,
            column: 1,
        }
    }

    pub fn parse(&mut self) -> EqResult<EdnValue> {
        self.skip_whitespace_and_comments();
        if self.is_at_end() {
            return Ok(EdnValue::Nil);
        }
        self.parse_value()
    }

    fn parse_value(&mut self) -> EqResult<EdnValue> {
        self.skip_whitespace_and_comments();
        
        if self.is_at_end() {
            return Err(EqError::parse_error(self.line, self.column, "Unexpected end of input"));
        }

        let ch = self.peek();
        match ch {
            'n' => self.parse_nil(),
            't' | 'f' => self.parse_boolean(),
            '"' => self.parse_string(),
            ':' => self.parse_keyword(),
            '\\' => self.parse_character(),
            '[' => self.parse_vector(),
            '(' => self.parse_list(),
            '{' => self.parse_map(),
            '#' => self.parse_dispatch(),
            '^' => self.parse_metadata(),
            '0'..='9' => self.parse_number(),
            '-' => {
                // Look ahead to see if this is a negative number or a symbol
                if self.position + 1 < self.input.len() && self.input[self.position + 1].is_ascii_digit() {
                    self.parse_number()
                } else {
                    self.parse_symbol()
                }
            }
            _ if ch.is_alphabetic() || "+-*/_!?$%&=<>.-".contains(ch) => self.parse_symbol(),
            _ => Err(EqError::parse_error(
                self.line,
                self.column,
                format!("Unexpected character: '{}'", ch),
            )),
        }
    }

    fn parse_nil(&mut self) -> EqResult<EdnValue> {
        if self.consume_literal("nil") {
            Ok(EdnValue::Nil)
        } else {
            self.parse_symbol()
        }
    }

    fn parse_boolean(&mut self) -> EqResult<EdnValue> {
        if self.consume_literal("true") {
            Ok(EdnValue::Bool(true))
        } else if self.consume_literal("false") {
            Ok(EdnValue::Bool(false))
        } else {
            self.parse_symbol()
        }
    }

    fn parse_string(&mut self) -> EqResult<EdnValue> {
        self.advance(); // consume opening quote
        let mut value = String::new();
        
        while !self.is_at_end() && self.peek() != '"' {
            if self.peek() == '\\' {
                self.advance(); // consume backslash
                if self.is_at_end() {
                    return Err(EqError::parse_error(self.line, self.column, "Unterminated string escape"));
                }
                match self.peek() {
                    '"' => value.push('"'),
                    '\\' => value.push('\\'),
                    'n' => value.push('\n'),
                    'r' => value.push('\r'),
                    't' => value.push('\t'),
                    'u' => {
                        // Handle unicode escape in string
                        self.advance(); // consume 'u'
                        let unicode_char = self.parse_unicode_escape_in_string()?;
                        value.push(unicode_char);
                        continue; // Skip the advance() at the end of the loop
                    }
                    c => {
                        return Err(EqError::parse_error(
                            self.line,
                            self.column,
                            format!("Invalid escape sequence: \\{}", c),
                        ))
                    }
                }
                self.advance();
            } else {
                value.push(self.peek());
                self.advance();
            }
        }
        
        if self.is_at_end() {
            return Err(EqError::parse_error(self.line, self.column, "Unterminated string"));
        }
        
        self.advance(); // consume closing quote
        Ok(EdnValue::String(value))
    }

    fn parse_keyword(&mut self) -> EqResult<EdnValue> {
        self.advance(); // consume ':'
        let name = self.read_symbol_name();
        if name.is_empty() {
            return Err(EqError::parse_error(self.line, self.column, "Empty keyword"));
        }
        Ok(EdnValue::Keyword(name))
    }

    fn parse_character(&mut self) -> EqResult<EdnValue> {
        self.advance(); // consume '\'
        if self.is_at_end() {
            return Err(EqError::parse_error(self.line, self.column, "Incomplete character literal"));
        }
        
        // Check for unicode escape
        if self.peek() == 'u' {
            return self.parse_unicode_character();
        }
        
        // Read the character name
        let start_pos = self.position;
        while !self.is_at_end() && self.is_symbol_char(self.peek()) {
            self.advance();
        }
        
        let char_name: String = self.input[start_pos..self.position].iter().collect();
        
        let character = match char_name.as_str() {
            "newline" => '\n',
            "tab" => '\t',
            "return" => '\r',
            "space" => ' ',
            "formfeed" => '\x0C',
            "backspace" => '\x08',
            single_char if single_char.len() == 1 => single_char.chars().next().unwrap(),
            _ => return Err(EqError::parse_error(
                self.line,
                self.column,
                format!("Invalid character literal: \\{}", char_name)
            )),
        };
        
        Ok(EdnValue::Character(character))
    }

    fn parse_vector(&mut self) -> EqResult<EdnValue> {
        self.advance(); // consume '['
        let mut elements = Vec::new();
        
        self.skip_whitespace_and_comments();
        while !self.is_at_end() && self.peek() != ']' {
            if self.peek() == '#' && self.peek_ahead(1) == Some('_') {
                // Handle discard macro
                self.advance(); // consume '#'
                self.advance(); // consume '_'
                self.skip_whitespace_and_comments();
                // Parse and discard the next value
                let _discarded = self.parse_value()?;
            } else {
                elements.push(self.parse_value()?);
            }
            self.skip_whitespace_and_comments();
        }
        
        if self.is_at_end() {
            return Err(EqError::parse_error(self.line, self.column, "Unterminated vector"));
        }
        
        self.advance(); // consume ']'
        Ok(EdnValue::Vector(elements))
    }

    fn parse_list(&mut self) -> EqResult<EdnValue> {
        self.advance(); // consume '('
        let mut elements = Vec::new();
        
        self.skip_whitespace_and_comments();
        while !self.is_at_end() && self.peek() != ')' {
            if self.peek() == '#' && self.peek_ahead(1) == Some('_') {
                // Handle discard macro
                self.advance(); // consume '#'
                self.advance(); // consume '_'
                self.skip_whitespace_and_comments();
                // Parse and discard the next value
                let _discarded = self.parse_value()?;
            } else {
                elements.push(self.parse_value()?);
            }
            self.skip_whitespace_and_comments();
        }
        
        if self.is_at_end() {
            return Err(EqError::parse_error(self.line, self.column, "Unterminated list"));
        }
        
        self.advance(); // consume ')'
        Ok(EdnValue::List(elements))
    }

    fn parse_map(&mut self) -> EqResult<EdnValue> {
        self.advance(); // consume '{'
        let mut map = IndexMap::new();
        
        self.skip_whitespace_and_comments();
        while !self.is_at_end() && self.peek() != '}' {
            // Parse key (handling discard)
            let key = if self.peek() == '#' && self.peek_ahead(1) == Some('_') {
                // Discard the key, parse and discard next value, then continue to next iteration
                self.advance(); // consume '#'
                self.advance(); // consume '_'
                self.skip_whitespace_and_comments();
                let _discarded_key = self.parse_value()?;
                self.skip_whitespace_and_comments();
                
                // Also need to discard the corresponding value
                if self.is_at_end() || self.peek() == '}' {
                    return Err(EqError::parse_error(
                        self.line,
                        self.column,
                        "Map literal must contain an even number of forms"
                    ));
                }
                
                if self.peek() == '#' && self.peek_ahead(1) == Some('_') {
                    self.advance(); // consume '#'
                    self.advance(); // consume '_'
                    self.skip_whitespace_and_comments();
                }
                let _discarded_value = self.parse_value()?;
                self.skip_whitespace_and_comments();
                continue;
            } else {
                self.parse_value()?
            };
            
            self.skip_whitespace_and_comments();
            
            if self.is_at_end() || self.peek() == '}' {
                return Err(EqError::parse_error(
                    self.line,
                    self.column,
                    "Map literal must contain an even number of forms"
                ));
            }
            
            // Parse value (handling discard)
            let value = if self.peek() == '#' && self.peek_ahead(1) == Some('_') {
                // Discard the value, parse and discard next value, then continue
                self.advance(); // consume '#'
                self.advance(); // consume '_'
                self.skip_whitespace_and_comments();
                let _discarded_value = self.parse_value()?;
                self.skip_whitespace_and_comments();
                continue;
            } else {
                self.parse_value()?
            };
            
            map.insert(key, value);
            self.skip_whitespace_and_comments();
        }
        
        if self.is_at_end() {
            return Err(EqError::parse_error(self.line, self.column, "Unterminated map"));
        }
        
        self.advance(); // consume '}'
        Ok(EdnValue::Map(map))
    }

    fn parse_dispatch(&mut self) -> EqResult<EdnValue> {
        self.advance(); // consume '#'
        if self.is_at_end() {
            return Err(EqError::parse_error(self.line, self.column, "Incomplete dispatch"));
        }
        
        match self.peek() {
            '{' => self.parse_set(),
            '_' => self.parse_discard(),
            _ => self.parse_tagged_literal(),
        }
    }

    fn parse_set(&mut self) -> EqResult<EdnValue> {
        self.advance(); // consume '{'
        let mut set = HashSet::new();
        
        self.skip_whitespace_and_comments();
        while !self.is_at_end() && self.peek() != '}' {
            if self.peek() == '#' && self.peek_ahead(1) == Some('_') {
                // Handle discard macro
                self.advance(); // consume '#'
                self.advance(); // consume '_'
                self.skip_whitespace_and_comments();
                // Parse and discard the next value
                let _discarded = self.parse_value()?;
            } else {
                let element = self.parse_value()?;
                if !set.insert(element.clone()) {
                    return Err(EqError::parse_error(
                        self.line,
                        self.column,
                        "Duplicate element in set"
                    ));
                }
            }
            self.skip_whitespace_and_comments();
        }
        
        if self.is_at_end() {
            return Err(EqError::parse_error(self.line, self.column, "Unterminated set"));
        }
        
        self.advance(); // consume '}'
        Ok(EdnValue::Set(set))
    }

    fn parse_tagged_literal(&mut self) -> EqResult<EdnValue> {
        let tag = self.read_symbol_name();
        if tag.is_empty() {
            return Err(EqError::parse_error(self.line, self.column, "Empty tag"));
        }
        
        self.skip_whitespace_and_comments();
        let value = self.parse_value()?;
        
        // Handle built-in tagged literals
        match tag.as_str() {
            "inst" => {
                if let EdnValue::String(s) = value {
                    // Validate ISO 8601 format (basic validation)
                    if self.is_valid_instant_string(&s) {
                        Ok(EdnValue::Instant(s))
                    } else {
                        Err(EqError::parse_error(
                            self.line,
                            self.column,
                            format!("Invalid instant format: {}", s)
                        ))
                    }
                } else {
                    Err(EqError::parse_error(
                        self.line,
                        self.column,
                        "#inst requires a string value"
                    ))
                }
            }
            "uuid" => {
                if let EdnValue::String(s) = value {
                    // Validate UUID format (basic validation)
                    if self.is_valid_uuid_string(&s) {
                        Ok(EdnValue::Uuid(s))
                    } else {
                        Err(EqError::parse_error(
                            self.line,
                            self.column,
                            format!("Invalid UUID format: {}", s)
                        ))
                    }
                } else {
                    Err(EqError::parse_error(
                        self.line,
                        self.column,
                        "#uuid requires a string value"
                    ))
                }
            }
            _ => {
                // Generic tagged literal
                Ok(EdnValue::Tagged {
                    tag,
                    value: Box::new(value),
                })
            }
        }
    }

    fn parse_metadata(&mut self) -> EqResult<EdnValue> {
        self.advance(); // consume '^'
        let metadata = self.parse_value()?;
        self.skip_whitespace_and_comments();
        let value = self.parse_value()?;
        
        Ok(EdnValue::WithMetadata {
            metadata: Box::new(metadata),
            value: Box::new(value),
        })
    }

    fn parse_discard(&mut self) -> EqResult<EdnValue> {
        self.advance(); // consume '_'
        self.skip_whitespace_and_comments();
        
        // Parse and discard the next value
        let _discarded = self.parse_value()?;
        
        // The discard macro itself doesn't produce a value, so this should only be called
        // in contexts where we can handle the absence of a value (like collections).
        // For standalone discard at top level, we need to parse the next value after discarding.
        self.skip_whitespace_and_comments();
        if self.is_at_end() {
            // If we're at end after discarding, return nil
            Ok(EdnValue::Nil)
        } else {
            // Parse the next value
            self.parse_value()
        }
    }

    fn parse_number(&mut self) -> EqResult<EdnValue> {
        let start_pos = self.position;
        let mut has_dot = false;
        
        if self.peek() == '-' {
            self.advance();
        }
        
        while !self.is_at_end() && (self.peek().is_ascii_digit() || self.peek() == '.') {
            if self.peek() == '.' {
                if has_dot {
                    break; // Second dot, not part of this number
                }
                has_dot = true;
            }
            self.advance();
        }
        
        let number_str: String = self.input[start_pos..self.position].iter().collect();
        
        if has_dot {
            number_str.parse::<f64>()
                .map(EdnValue::Float)
                .map_err(|_| EqError::parse_error(
                    self.line,
                    self.column,
                    format!("Invalid float: {}", number_str)
                ))
        } else {
            number_str.parse::<i64>()
                .map(EdnValue::Integer)
                .map_err(|_| EqError::parse_error(
                    self.line,
                    self.column,
                    format!("Invalid integer: {}", number_str)
                ))
        }
    }

    fn parse_symbol(&mut self) -> EqResult<EdnValue> {
        let name = self.read_symbol_name();
        if name.is_empty() {
            return Err(EqError::parse_error(self.line, self.column, "Empty symbol"));
        }
        Ok(EdnValue::Symbol(name))
    }

    fn read_symbol_name(&mut self) -> String {
        let start_pos = self.position;
        
        while !self.is_at_end() && self.is_symbol_char(self.peek()) {
            self.advance();
        }
        
        self.input[start_pos..self.position].iter().collect()
    }

    fn is_symbol_char(&self, ch: char) -> bool {
        ch.is_alphanumeric() || "+-*/_!?$%&=<>.-".contains(ch)
    }

    fn consume_literal(&mut self, literal: &str) -> bool {
        let start_pos = self.position;
        
        for expected_char in literal.chars() {
            if self.is_at_end() || self.peek() != expected_char {
                self.position = start_pos; // backtrack
                return false;
            }
            self.advance();
        }
        
        // Make sure we're at a word boundary
        if !self.is_at_end() && self.is_symbol_char(self.peek()) {
            self.position = start_pos; // backtrack
            return false;
        }
        
        true
    }


    fn skip_whitespace_and_comments(&mut self) {
        while !self.is_at_end() {
            let ch = self.peek();
            if ch.is_whitespace() {
                if ch == '\n' {
                    self.line += 1;
                    self.column = 1;
                } else {
                    self.column += 1;
                }
                self.advance();
            } else if ch == ';' {
                // Skip comment until end of line
                while !self.is_at_end() && self.peek() != '\n' {
                    self.advance();
                }
            } else {
                break;
            }
        }
    }

    fn peek(&self) -> char {
        if self.is_at_end() {
            '\0'
        } else {
            self.input[self.position]
        }
    }

    fn peek_ahead(&self, offset: usize) -> Option<char> {
        let pos = self.position + offset;
        if pos < self.input.len() {
            Some(self.input[pos])
        } else {
            None
        }
    }

    fn advance(&mut self) {
        if !self.is_at_end() {
            self.position += 1;
            self.column += 1;
        }
    }

    fn is_at_end(&self) -> bool {
        self.position >= self.input.len()
    }

    fn is_valid_instant_string(&self, s: &str) -> bool {
        // Basic ISO 8601 validation - just check for common patterns
        // Full validation would require a proper datetime parser
        
        // RFC 3339 / ISO 8601 patterns:
        // 2023-01-01T00:00:00.000Z
        // 2023-01-01T12:30:45.123-05:00
        // 2023-01-01T12:30:45Z
        
        if s.len() < 19 {
            return false; // Minimum length for YYYY-MM-DDTHH:MM:SS
        }
        
        let chars: Vec<char> = s.chars().collect();
        
        // Check basic structure: YYYY-MM-DDTHH:MM:SS
        if chars.len() >= 19 {
            chars[4] == '-' &&
            chars[7] == '-' &&
            chars[10] == 'T' &&
            chars[13] == ':' &&
            chars[16] == ':' &&
            chars[0..4].iter().all(|c| c.is_ascii_digit()) &&
            chars[5..7].iter().all(|c| c.is_ascii_digit()) &&
            chars[8..10].iter().all(|c| c.is_ascii_digit()) &&
            chars[11..13].iter().all(|c| c.is_ascii_digit()) &&
            chars[14..16].iter().all(|c| c.is_ascii_digit()) &&
            chars[17..19].iter().all(|c| c.is_ascii_digit())
        } else {
            false
        }
    }

    fn is_valid_uuid_string(&self, s: &str) -> bool {
        // Basic UUID validation
        // Standard UUID format: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
        // 8-4-4-4-12 hex digits separated by hyphens
        
        if s.len() != 36 {
            return false;
        }
        
        let chars: Vec<char> = s.chars().collect();
        
        // Check hyphen positions
        if chars[8] != '-' || chars[13] != '-' || chars[18] != '-' || chars[23] != '-' {
            return false;
        }
        
        // Check hex digits in each segment
        let segments = [
            &chars[0..8],   // 8 chars
            &chars[9..13],  // 4 chars  
            &chars[14..18], // 4 chars
            &chars[19..23], // 4 chars
            &chars[24..36], // 12 chars
        ];
        
        segments.iter().all(|segment| {
            segment.iter().all(|c| c.is_ascii_hexdigit())
        })
    }

    fn parse_unicode_character(&mut self) -> EqResult<EdnValue> {
        self.advance(); // consume 'u'
        
        // Read exactly 4 hex digits
        let mut hex_digits = String::new();
        for _ in 0..4 {
            if self.is_at_end() || !self.peek().is_ascii_hexdigit() {
                return Err(EqError::parse_error(
                    self.line,
                    self.column,
                    "Unicode escape requires exactly 4 hex digits"
                ));
            }
            hex_digits.push(self.peek());
            self.advance();
        }
        
        // Parse hex value
        if let Ok(code_point) = u32::from_str_radix(&hex_digits, 16) {
            if let Some(character) = char::from_u32(code_point) {
                Ok(EdnValue::Character(character))
            } else {
                Err(EqError::parse_error(
                    self.line,
                    self.column,
                    format!("Invalid Unicode code point: U+{}", hex_digits)
                ))
            }
        } else {
            Err(EqError::parse_error(
                self.line,
                self.column,
                format!("Invalid hex digits in Unicode escape: {}", hex_digits)
            ))
        }
    }

    fn parse_unicode_escape_in_string(&mut self) -> EqResult<char> {
        // Read exactly 4 hex digits
        let mut hex_digits = String::new();
        for _ in 0..4 {
            if self.is_at_end() || !self.peek().is_ascii_hexdigit() {
                return Err(EqError::parse_error(
                    self.line,
                    self.column,
                    "Unicode escape in string requires exactly 4 hex digits"
                ));
            }
            hex_digits.push(self.peek());
            self.advance();
        }
        
        // Parse hex value
        if let Ok(code_point) = u32::from_str_radix(&hex_digits, 16) {
            if let Some(character) = char::from_u32(code_point) {
                Ok(character)
            } else {
                Err(EqError::parse_error(
                    self.line,
                    self.column,
                    format!("Invalid Unicode code point in string: U+{}", hex_digits)
                ))
            }
        } else {
            Err(EqError::parse_error(
                self.line,
                self.column,
                format!("Invalid hex digits in Unicode escape: {}", hex_digits)
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_parse_nil() {
        let mut parser = Parser::new("nil");
        assert_eq!(parser.parse().unwrap(), EdnValue::Nil);
    }

    #[test]
    fn test_parse_boolean() {
        let mut parser = Parser::new("true");
        assert_eq!(parser.parse().unwrap(), EdnValue::Bool(true));
        
        let mut parser = Parser::new("false");
        assert_eq!(parser.parse().unwrap(), EdnValue::Bool(false));
    }

    #[test]
    fn test_parse_string() {
        let mut parser = Parser::new("\"hello world\"");
        assert_eq!(parser.parse().unwrap(), EdnValue::String("hello world".to_string()));
        
        let mut parser = Parser::new("\"hello\\nworld\"");
        assert_eq!(parser.parse().unwrap(), EdnValue::String("hello\nworld".to_string()));
    }

    #[test]
    fn test_parse_keyword() {
        let mut parser = Parser::new(":key");
        assert_eq!(parser.parse().unwrap(), EdnValue::Keyword("key".to_string()));
        
        let mut parser = Parser::new(":ns/key");
        assert_eq!(parser.parse().unwrap(), EdnValue::Keyword("ns/key".to_string()));
    }

    #[test]
    fn test_parse_character() {
        let mut parser = Parser::new("\\a");
        assert_eq!(parser.parse().unwrap(), EdnValue::Character('a'));
        
        let mut parser = Parser::new("\\newline");
        assert_eq!(parser.parse().unwrap(), EdnValue::Character('\n'));
        
        let mut parser = Parser::new("\\tab");
        assert_eq!(parser.parse().unwrap(), EdnValue::Character('\t'));
        
        let mut parser = Parser::new("\\formfeed");
        assert_eq!(parser.parse().unwrap(), EdnValue::Character('\x0C'));
        
        let mut parser = Parser::new("\\backspace");
        assert_eq!(parser.parse().unwrap(), EdnValue::Character('\x08'));
    }

    #[test]
    fn test_parse_unicode_character() {
        // Test Omega symbol (Ω)
        let mut parser = Parser::new("\\u03A9");
        assert_eq!(parser.parse().unwrap(), EdnValue::Character('Ω'));
        
        // Test Latin A
        let mut parser = Parser::new("\\u0041");
        assert_eq!(parser.parse().unwrap(), EdnValue::Character('A'));
        
        // Test null character
        let mut parser = Parser::new("\\u0000");
        assert_eq!(parser.parse().unwrap(), EdnValue::Character('\0'));
    }

    #[test]
    fn test_parse_unicode_in_string() {
        // Test string with unicode escape
        let mut parser = Parser::new("\"Hello \\u03A9 World\"");
        assert_eq!(parser.parse().unwrap(), EdnValue::String("Hello Ω World".to_string()));
        
        // Test multiple unicode escapes
        let mut parser = Parser::new("\"\\u0041\\u0042\\u0043\"");
        assert_eq!(parser.parse().unwrap(), EdnValue::String("ABC".to_string()));
    }

    #[test]
    fn test_invalid_unicode_escapes() {
        // Invalid character unicode (too few digits)
        let mut parser = Parser::new("\\u03A");
        assert!(parser.parse().is_err());
        
        // Invalid character unicode (non-hex)
        let mut parser = Parser::new("\\u03GH");
        assert!(parser.parse().is_err());
        
        // Invalid string unicode (too few digits)
        let mut parser = Parser::new("\"\\u03A\"");
        assert!(parser.parse().is_err());
    }

    #[test]
    fn test_parse_numbers() {
        let mut parser = Parser::new("42");
        assert_eq!(parser.parse().unwrap(), EdnValue::Integer(42));
        
        let mut parser = Parser::new("-17");
        assert_eq!(parser.parse().unwrap(), EdnValue::Integer(-17));
        
        let mut parser = Parser::new("3.14");
        assert_eq!(parser.parse().unwrap(), EdnValue::Float(3.14));
        
        let mut parser = Parser::new("-2.5");
        assert_eq!(parser.parse().unwrap(), EdnValue::Float(-2.5));
    }

    #[test]
    fn test_parse_symbol() {
        let mut parser = Parser::new("symbol");
        assert_eq!(parser.parse().unwrap(), EdnValue::Symbol("symbol".to_string()));
        
        let mut parser = Parser::new("ns/symbol");
        assert_eq!(parser.parse().unwrap(), EdnValue::Symbol("ns/symbol".to_string()));
        
        let mut parser = Parser::new("+");
        assert_eq!(parser.parse().unwrap(), EdnValue::Symbol("+".to_string()));
    }

    #[test]
    fn test_parse_vector() {
        let mut parser = Parser::new("[1 2 3]");
        let result = parser.parse().unwrap();
        
        if let EdnValue::Vector(v) = result {
            assert_eq!(v.len(), 3);
            assert_eq!(v[0], EdnValue::Integer(1));
            assert_eq!(v[1], EdnValue::Integer(2));
            assert_eq!(v[2], EdnValue::Integer(3));
        } else {
            panic!("Expected vector");
        }
    }

    #[test]
    fn test_parse_list() {
        let mut parser = Parser::new("(+ 1 2)");
        let result = parser.parse().unwrap();
        
        if let EdnValue::List(l) = result {
            assert_eq!(l.len(), 3);
            assert_eq!(l[0], EdnValue::Symbol("+".to_string()));
            assert_eq!(l[1], EdnValue::Integer(1));
            assert_eq!(l[2], EdnValue::Integer(2));
        } else {
            panic!("Expected list");
        }
    }

    #[test]
    fn test_parse_map() {
        let mut parser = Parser::new("{:name \"Alice\" :age 30}");
        let result = parser.parse().unwrap();
        
        if let EdnValue::Map(m) = result {
            assert_eq!(m.len(), 2);
            assert_eq!(
                m.get(&EdnValue::Keyword("name".to_string())),
                Some(&EdnValue::String("Alice".to_string()))
            );
            assert_eq!(
                m.get(&EdnValue::Keyword("age".to_string())),
                Some(&EdnValue::Integer(30))
            );
        } else {
            panic!("Expected map");
        }
    }

    #[test]
    fn test_parse_set() {
        let mut parser = Parser::new("#{1 2 3}");
        let result = parser.parse().unwrap();
        
        if let EdnValue::Set(s) = result {
            assert_eq!(s.len(), 3);
            assert!(s.contains(&EdnValue::Integer(1)));
            assert!(s.contains(&EdnValue::Integer(2)));
            assert!(s.contains(&EdnValue::Integer(3)));
        } else {
            panic!("Expected set");
        }
    }

    #[test]
    fn test_parse_tagged_literal() {
        // Generic tagged literal
        let mut parser = Parser::new("#custom \"value\"");
        let result = parser.parse().unwrap();
        
        if let EdnValue::Tagged { tag, value } = result {
            assert_eq!(tag, "custom");
            assert_eq!(*value, EdnValue::String("value".to_string()));
        } else {
            panic!("Expected tagged literal");
        }
    }

    #[test]
    fn test_parse_instant() {
        // Valid instant
        let mut parser = Parser::new("#inst \"2023-01-01T12:30:45Z\"");
        let result = parser.parse().unwrap();
        
        if let EdnValue::Instant(s) = result {
            assert_eq!(s, "2023-01-01T12:30:45Z");
        } else {
            panic!("Expected instant");
        }
        
        // Valid instant with timezone
        let mut parser = Parser::new("#inst \"2023-01-01T12:30:45.123-05:00\"");
        let result = parser.parse().unwrap();
        
        if let EdnValue::Instant(s) = result {
            assert_eq!(s, "2023-01-01T12:30:45.123-05:00");
        } else {
            panic!("Expected instant");
        }
    }

    #[test]
    fn test_parse_uuid() {
        // Valid UUID
        let mut parser = Parser::new("#uuid \"f81d4fae-7dec-11d0-a765-00a0c91e6bf6\"");
        let result = parser.parse().unwrap();
        
        if let EdnValue::Uuid(s) = result {
            assert_eq!(s, "f81d4fae-7dec-11d0-a765-00a0c91e6bf6");
        } else {
            panic!("Expected UUID");
        }
    }

    #[test]
    fn test_invalid_instant() {
        // Invalid instant format
        let mut parser = Parser::new("#inst \"not-a-date\"");
        assert!(parser.parse().is_err());
        
        // Non-string value
        let mut parser = Parser::new("#inst 123");
        assert!(parser.parse().is_err());
    }

    #[test]
    fn test_invalid_uuid() {
        // Invalid UUID format
        let mut parser = Parser::new("#uuid \"not-a-uuid\"");
        assert!(parser.parse().is_err());
        
        // Non-string value
        let mut parser = Parser::new("#uuid 123");
        assert!(parser.parse().is_err());
    }

    #[test]
    fn test_parse_metadata() {
        // Test simple keyword metadata
        let mut parser = Parser::new("^:tag {:key \"value\"}");
        let result = parser.parse().unwrap();
        
        if let EdnValue::WithMetadata { metadata, value } = result {
            assert_eq!(*metadata, EdnValue::Keyword("tag".to_string()));
            assert!(matches!(value.as_ref(), EdnValue::Map(_)));
        } else {
            panic!("Expected metadata");
        }
        
        // Test map metadata
        let mut parser = Parser::new("^{:replace true} #{:a :b}");
        let result = parser.parse().unwrap();
        
        if let EdnValue::WithMetadata { metadata, value } = result {
            assert!(matches!(metadata.as_ref(), EdnValue::Map(_)));
            assert!(matches!(value.as_ref(), EdnValue::Set(_)));
        } else {
            panic!("Expected metadata");
        }
    }

    #[test]
    fn test_parse_discard() {
        // Test discard in vector
        let mut parser = Parser::new("[1 2 #_ 3 4]");
        let result = parser.parse().unwrap();
        
        if let EdnValue::Vector(v) = result {
            assert_eq!(v.len(), 3);
            assert_eq!(v[0], EdnValue::Integer(1));
            assert_eq!(v[1], EdnValue::Integer(2));
            assert_eq!(v[2], EdnValue::Integer(4));
        } else {
            panic!("Expected vector");
        }
        
        // Test discard in map
        let mut parser = Parser::new("{:a 1 #_ :b #_ 2 :c 3}");
        let result = parser.parse().unwrap();
        
        if let EdnValue::Map(m) = result {
            assert_eq!(m.len(), 2);
            assert_eq!(m.get(&EdnValue::Keyword("a".to_string())), Some(&EdnValue::Integer(1)));
            assert_eq!(m.get(&EdnValue::Keyword("c".to_string())), Some(&EdnValue::Integer(3)));
            assert_eq!(m.get(&EdnValue::Keyword("b".to_string())), None);
        } else {
            panic!("Expected map");
        }
        
        // Test discard in set
        let mut parser = Parser::new("#{1 #_ 2 3}");
        let result = parser.parse().unwrap();
        
        if let EdnValue::Set(s) = result {
            assert_eq!(s.len(), 2);
            assert!(s.contains(&EdnValue::Integer(1)));
            assert!(s.contains(&EdnValue::Integer(3)));
            assert!(!s.contains(&EdnValue::Integer(2)));
        } else {
            panic!("Expected set");
        }

        // Test standalone discard followed by value
        let mut parser = Parser::new("#_ :discarded :kept");
        let result = parser.parse().unwrap();
        assert_eq!(result, EdnValue::Keyword("kept".to_string()));
    }

    #[test]
    fn test_parse_nested_structures() {
        let mut parser = Parser::new("{:users [{:name \"Alice\" :tags #{:admin :user}} {:name \"Bob\"}]}");
        let result = parser.parse().unwrap();
        
        // Just verify it parses without error - full structure validation would be verbose
        assert!(matches!(result, EdnValue::Map(_)));
    }

    #[test]
    fn test_parse_with_comments() {
        let mut parser = Parser::new(r#"
            ; This is a comment
            {:name "Alice" ; inline comment
             :age 30}
            "#);
        let result = parser.parse().unwrap();
        assert!(matches!(result, EdnValue::Map(_)));
    }

    #[test]
    fn test_parse_errors() {
        // Unterminated string
        let mut parser = Parser::new("\"unterminated");
        assert!(parser.parse().is_err());
        
        // Unterminated vector
        let mut parser = Parser::new("[1 2");
        assert!(parser.parse().is_err());
        
        // Invalid map (odd number of elements)
        let mut parser = Parser::new("{:key}");
        assert!(parser.parse().is_err());
        
        // Duplicate set elements
        let mut parser = Parser::new("#{1 1}");
        assert!(parser.parse().is_err());
    }

    #[test]
    fn test_whitespace_handling() {
        let inputs = vec![
            "  42  ",
            "\n\t42\r\n",
            "42",
        ];
        
        for input in inputs {
            let mut parser = Parser::new(input);
            assert_eq!(parser.parse().unwrap(), EdnValue::Integer(42));
        }
    }
}
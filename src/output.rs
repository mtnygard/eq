use crate::edn::EdnValue;

/// Configuration for output formatting
#[derive(Debug, Clone)]
pub struct OutputConfig {
    pub compact: bool,
    pub raw_strings: bool,
    pub use_tabs: bool,
    pub indent_size: usize,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            compact: false,
            raw_strings: false,
            use_tabs: false,
            indent_size: 2,
        }
    }
}

/// Format an EDN value for output
pub fn format_output(value: &EdnValue, config: &OutputConfig) -> String {
    if config.compact {
        format_compact(value, config)
    } else {
        format_pretty(value, config, 0)
    }
}

/// Format with compact output (no whitespace)
fn format_compact(value: &EdnValue, config: &OutputConfig) -> String {
    match value {
        EdnValue::Nil => "nil".to_string(),
        EdnValue::Bool(b) => b.to_string(),
        EdnValue::String(s) => {
            if config.raw_strings {
                s.clone()
            } else {
                format!("\"{}\"", escape_string(s))
            }
        }
        EdnValue::Keyword(k) => format!(":{}", k),
        EdnValue::Symbol(s) => s.clone(),
        EdnValue::Character(c) => format_character(*c),
        EdnValue::Integer(i) => i.to_string(),
        EdnValue::Float(f) => f.to_string(),
        EdnValue::Vector(v) => {
            let items: Vec<String> = v.iter().map(|item| format_compact(item, config)).collect();
            format!("[{}]", items.join(" "))
        }
        EdnValue::List(l) => {
            let items: Vec<String> = l.iter().map(|item| format_compact(item, config)).collect();
            format!("({})", items.join(" "))
        }
        EdnValue::Map(m) => {
            let items: Vec<String> = m.iter()
                .map(|(k, v)| format!("{} {}", format_compact(k, config), format_compact(v, config)))
                .collect();
            format!("{{{}}}", items.join(" "))
        }
        EdnValue::Set(s) => {
            let mut items: Vec<String> = s.iter().map(|item| format_compact(item, config)).collect();
            items.sort(); // Ensure deterministic output
            format!("#{{{}}}", items.join(" "))
        }
        EdnValue::Tagged { tag, value } => {
            format!("#{} {}", tag, format_compact(value, config))
        }
        EdnValue::WithMetadata { metadata, value } => {
            format!("^{} {}", format_compact(metadata, config), format_compact(value, config))
        }
        EdnValue::Instant(s) => format!("#inst \"{}\"", s),
        EdnValue::Uuid(s) => format!("#uuid \"{}\"", s),
    }
}

/// Format with pretty printing (indented, multi-line)
fn format_pretty(value: &EdnValue, config: &OutputConfig, depth: usize) -> String {
    match value {
        EdnValue::Nil => "nil".to_string(),
        EdnValue::Bool(b) => b.to_string(),
        EdnValue::String(s) => {
            if config.raw_strings {
                s.clone()
            } else {
                format!("\"{}\"", escape_string(s))
            }
        }
        EdnValue::Keyword(k) => format!(":{}", k),
        EdnValue::Symbol(s) => s.clone(),
        EdnValue::Character(c) => format_character(*c),
        EdnValue::Integer(i) => i.to_string(),
        EdnValue::Float(f) => f.to_string(),
        EdnValue::Vector(v) => format_pretty_collection('[', ']', v, config, depth),
        EdnValue::List(l) => format_pretty_collection('(', ')', l, config, depth),
        EdnValue::Map(m) => format_pretty_map(m, config, depth),
        EdnValue::Set(s) => {
            let mut items: Vec<&EdnValue> = s.iter().collect();
            items.sort_by_key(|v| format!("{:?}", v)); // Deterministic ordering
            format_pretty_collection_with_prefix("#{", '}', &items, config, depth)
        }
        EdnValue::Tagged { tag, value } => {
            format!("#{} {}", tag, format_pretty(value, config, depth))
        }
        EdnValue::WithMetadata { metadata, value } => {
            format!("^{} {}", format_pretty(metadata, config, depth), format_pretty(value, config, depth))
        }
        EdnValue::Instant(s) => format!("#inst \"{}\"", s),
        EdnValue::Uuid(s) => format!("#uuid \"{}\"", s),
    }
}

/// Format a collection with pretty printing
fn format_pretty_collection(
    open: char,
    close: char,
    items: &[EdnValue],
    config: &OutputConfig,
    depth: usize,
) -> String {
    if items.is_empty() {
        return format!("{}{}", open, close);
    }
    
    // Check if collection is simple enough to fit on one line
    if should_format_inline(items, config) {
        let formatted_items: Vec<String> = items.iter()
            .map(|item| format_compact(item, config))
            .collect();
        return format!("{}{}{}", open, formatted_items.join(" "), close);
    }
    
    // Multi-line format
    let mut result = String::new();
    result.push(open);
    
    for (i, item) in items.iter().enumerate() {
        if i == 0 {
            result.push(' ');
        } else {
            result.push('\n');
            result.push_str(&make_indent(config, depth + 1));
        }
        result.push_str(&format_pretty(item, config, depth + 1));
    }
    
    result.push(close);
    result
}

/// Format a collection with a prefix (like sets)
fn format_pretty_collection_with_prefix(
    prefix: &str,
    close: char,
    items: &[&EdnValue],
    config: &OutputConfig,
    depth: usize,
) -> String {
    if items.is_empty() {
        return format!("{}{}", prefix, close);
    }
    
    // Check if collection is simple enough to fit on one line
    if should_format_inline_refs(items, config) {
        let formatted_items: Vec<String> = items.iter()
            .map(|item| format_compact(item, config))
            .collect();
        return format!("{}{}{}", prefix, formatted_items.join(" "), close);
    }
    
    // Multi-line format
    let mut result = String::new();
    result.push_str(prefix);
    
    for (i, item) in items.iter().enumerate() {
        if i == 0 {
            result.push(' ');
        } else {
            result.push('\n');
            result.push_str(&make_indent(config, depth + 1));
        }
        result.push_str(&format_pretty(item, config, depth + 1));
    }
    
    result.push(close);
    result
}

/// Format a map with pretty printing
fn format_pretty_map(
    map: &indexmap::IndexMap<EdnValue, EdnValue>,
    config: &OutputConfig,
    depth: usize,
) -> String {
    if map.is_empty() {
        return "{}".to_string();
    }
    
    // Check if map is simple enough to fit on one line
    if should_format_map_inline(map, config) {
        let items: Vec<String> = map.iter()
            .map(|(k, v)| format!("{} {}", format_compact(k, config), format_compact(v, config)))
            .collect();
        return format!("{{{}}}", items.join(" "));
    }
    
    // Multi-line format
    let mut result = String::new();
    result.push('{');
    
    for (i, (key, value)) in map.iter().enumerate() {
        if i == 0 {
            result.push(' ');
        } else {
            result.push('\n');
            result.push_str(&make_indent(config, depth + 1));
        }
        
        result.push_str(&format_pretty(key, config, depth + 1));
        result.push(' ');
        
        // If value is a collection, put it on the next line
        if is_collection(value) && !is_simple_collection(value) {
            result.push('\n');
            result.push_str(&make_indent(config, depth + 1));
        }
        
        result.push_str(&format_pretty(value, config, depth + 1));
    }
    
    result.push('}');
    result
}

/// Create indentation string
fn make_indent(config: &OutputConfig, depth: usize) -> String {
    if config.use_tabs {
        "\t".repeat(depth)
    } else {
        " ".repeat(depth * config.indent_size)
    }
}

/// Check if a collection should be formatted inline
fn should_format_inline(items: &[EdnValue], config: &OutputConfig) -> bool {
    if items.len() > 4 {
        return false;
    }
    
    // Estimate line length
    let estimated_length: usize = items.iter()
        .map(|item| format_compact(item, config).len())
        .sum::<usize>() + items.len(); // +1 for each space
    
    estimated_length < 60
}

/// Check if a collection of references should be formatted inline
fn should_format_inline_refs(items: &[&EdnValue], config: &OutputConfig) -> bool {
    if items.len() > 4 {
        return false;
    }
    
    let estimated_length: usize = items.iter()
        .map(|item| format_compact(item, config).len())
        .sum::<usize>() + items.len();
    
    estimated_length < 60
}

/// Check if a map should be formatted inline
fn should_format_map_inline(
    map: &indexmap::IndexMap<EdnValue, EdnValue>,
    config: &OutputConfig,
) -> bool {
    if map.len() > 2 {
        return false;
    }
    
    let estimated_length: usize = map.iter()
        .map(|(k, v)| format_compact(k, config).len() + format_compact(v, config).len() + 1)
        .sum::<usize>() + map.len() * 2; // +2 for spacing
    
    estimated_length < 50
}

/// Check if a value is a collection
fn is_collection(value: &EdnValue) -> bool {
    match value {
        EdnValue::Vector(_) | EdnValue::List(_) | EdnValue::Map(_) | EdnValue::Set(_) => true,
        EdnValue::WithMetadata { value, .. } => is_collection(value),
        _ => false,
    }
}

/// Check if a collection is simple (contains only primitive values)
fn is_simple_collection(value: &EdnValue) -> bool {
    match value {
        EdnValue::Vector(v) => v.iter().all(|item| !is_collection(item)),
        EdnValue::List(l) => l.iter().all(|item| !is_collection(item)),
        EdnValue::Map(m) => m.iter().all(|(k, v)| !is_collection(k) && !is_collection(v)),
        EdnValue::Set(s) => s.iter().all(|item| !is_collection(item)),
        EdnValue::WithMetadata { value, .. } => is_simple_collection(value),
        _ => true,
    }
}

/// Escape special characters in strings
fn escape_string(s: &str) -> String {
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
fn format_character(c: char) -> String {
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
    use indexmap::IndexMap;
    use std::collections::HashSet;

    #[test]
    fn test_format_primitives() {
        let config = OutputConfig::default();
        
        assert_eq!(format_output(&EdnValue::Nil, &config), "nil");
        assert_eq!(format_output(&EdnValue::Bool(true), &config), "true");
        assert_eq!(format_output(&EdnValue::Integer(42), &config), "42");
        assert_eq!(format_output(&EdnValue::Float(3.14), &config), "3.14");
        assert_eq!(format_output(&EdnValue::String("hello".to_string()), &config), "\"hello\"");
        assert_eq!(format_output(&EdnValue::Keyword("key".to_string()), &config), ":key");
        assert_eq!(format_output(&EdnValue::Character('a'), &config), "\\a");
        assert_eq!(format_output(&EdnValue::Character('\n'), &config), "\\newline");
    }

    #[test]
    fn test_format_collections() {
        let config = OutputConfig::default();
        
        // Simple vector
        let vec = EdnValue::Vector(vec![
            EdnValue::Integer(1),
            EdnValue::Integer(2),
            EdnValue::Integer(3),
        ]);
        assert_eq!(format_output(&vec, &config), "[1 2 3]");
        
        // Simple map
        let mut map = IndexMap::new();
        map.insert(EdnValue::Keyword("name".to_string()), EdnValue::String("Alice".to_string()));
        let map_val = EdnValue::Map(map);
        assert_eq!(format_output(&map_val, &config), "{:name \"Alice\"}");
    }

    #[test]
    fn test_compact_format() {
        let mut config = OutputConfig::default();
        config.compact = true;
        
        let nested = EdnValue::Vector(vec![
            EdnValue::Map({
                let mut m = IndexMap::new();
                m.insert(EdnValue::Keyword("a".to_string()), EdnValue::Integer(1));
                m.insert(EdnValue::Keyword("b".to_string()), EdnValue::Vector(vec![
                    EdnValue::Integer(2),
                    EdnValue::Integer(3),
                ]));
                m
            })
        ]);
        
        let result = format_output(&nested, &config);
        assert!(!result.contains('\n')); // Should be all on one line
        assert!(result.contains("{:a 1 :b [2 3]}"));
    }

    #[test]
    fn test_pretty_format() {
        let config = OutputConfig::default();
        
        // Large enough collection to trigger multi-line
        let large_vec = EdnValue::Vector(vec![
            EdnValue::String("item1".to_string()),
            EdnValue::String("item2".to_string()),
            EdnValue::String("item3".to_string()),
            EdnValue::String("item4".to_string()),
            EdnValue::String("item5".to_string()),
        ]);
        
        let result = format_output(&large_vec, &config);
        assert!(result.contains('\n')); // Should be multi-line
    }

    #[test]
    fn test_raw_strings() {
        let mut config = OutputConfig::default();
        config.raw_strings = true;
        
        let string_val = EdnValue::String("hello world".to_string());
        assert_eq!(format_output(&string_val, &config), "hello world");
        
        config.raw_strings = false;
        assert_eq!(format_output(&string_val, &config), "\"hello world\"");
    }

    #[test]
    fn test_indentation_config() {
        let mut config = OutputConfig::default();
        config.indent_size = 4;
        
        let nested = EdnValue::Vector(vec![
            EdnValue::Vector(vec![EdnValue::Integer(1)]),
            EdnValue::Vector(vec![EdnValue::Integer(2)]),
            EdnValue::Vector(vec![EdnValue::Integer(3)]),
            EdnValue::Vector(vec![EdnValue::Integer(4)]),
            EdnValue::Vector(vec![EdnValue::Integer(5)]),
        ]);
        
        let result = format_output(&nested, &config);
        // Should contain 4-space indentation
        assert!(result.lines().any(|line| line.starts_with("    ")));
    }

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

    #[test]
    fn test_set_formatting() {
        let mut set = HashSet::new();
        set.insert(EdnValue::Integer(3));
        set.insert(EdnValue::Integer(1));
        set.insert(EdnValue::Integer(2));
        
        let set_val = EdnValue::Set(set);
        let config = OutputConfig::default();
        let result = format_output(&set_val, &config);
        
        // Should be deterministically ordered
        assert!(result.starts_with("#{"));
        assert!(result.ends_with("}"));
        assert!(result.contains("1"));
        assert!(result.contains("2"));
        assert!(result.contains("3"));
    }

    #[test]
    fn test_tagged_values() {
        let config = OutputConfig::default();
        let tagged = EdnValue::Tagged {
            tag: "inst".to_string(),
            value: Box::new(EdnValue::String("2023-01-01".to_string())),
        };
        
        assert_eq!(format_output(&tagged, &config), "#inst \"2023-01-01\"");
    }
}
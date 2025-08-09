use crate::edn::EdnValue;
use crate::formatter::{Formatter, CompactFormatter, PrettyFormatter};

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
        let formatter = CompactFormatter;
        formatter.format(value, config, 0)
    } else {
        let formatter = PrettyFormatter;
        formatter.format(value, config, 0)
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
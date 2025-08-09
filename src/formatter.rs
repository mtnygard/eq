use crate::edn::EdnValue;
use crate::primitives::{escape_string, format_character};
use crate::output::OutputConfig;
use crate::collection_formatter::CollectionFormatter;
use indexmap::IndexMap;

/// Trait for formatting EDN values
pub trait Formatter {
    fn format(&self, value: &EdnValue, config: &OutputConfig, depth: usize) -> String;
    fn format_collection(&self, open: char, close: char, items: &[EdnValue], config: &OutputConfig, depth: usize) -> String;
    fn format_map(&self, map: &IndexMap<EdnValue, EdnValue>, config: &OutputConfig, depth: usize) -> String;
    fn format_set(&self, items: &mut Vec<&EdnValue>, prefix: &str, close: char, config: &OutputConfig, depth: usize) -> String;
}

/// Compact formatter - no whitespace
pub struct CompactFormatter;

impl Formatter for CompactFormatter {
    fn format(&self, value: &EdnValue, config: &OutputConfig, _depth: usize) -> String {
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
            EdnValue::Vector(v) => self.format_collection('[', ']', v, config, 0),
            EdnValue::List(l) => self.format_collection('(', ')', l, config, 0),
            EdnValue::Map(m) => self.format_map(m, config, 0),
            EdnValue::Set(s) => {
                let mut items: Vec<&EdnValue> = s.iter().collect();
                self.format_set(&mut items, "#{", '}', config, 0)
            }
            EdnValue::Tagged { tag, value } => {
                format!("#{} {}", tag, self.format(value, config, 0))
            }
            EdnValue::WithMetadata { metadata, value } => {
                format!("^{} {}", self.format(metadata, config, 0), self.format(value, config, 0))
            }
            EdnValue::Lambda(lambda) => {
                let params = lambda.params.join(" ");
                format!("(fn [{}] {})", params, self.format(&lambda.body, config, 0))
            }
            EdnValue::Instant(s) => format!("#inst \"{}\"", s),
            EdnValue::Uuid(s) => format!("#uuid \"{}\"", s),
        }
    }

    fn format_collection(&self, open: char, close: char, items: &[EdnValue], config: &OutputConfig, depth: usize) -> String {
        let cf = CollectionFormatter::new(self, config);
        let formatted = items.iter().map(|item| self.format(item, config, depth));
        cf.format(&open.to_string(), &close.to_string(), formatted, depth, true)
    }

    fn format_map(&self, map: &IndexMap<EdnValue, EdnValue>, config: &OutputConfig, depth: usize) -> String {
        let cf = CollectionFormatter::new(self, config);
        let pairs = map.iter().map(|(k, v)| (self.format(k, config, depth), self.format(v, config, depth)));
        cf.format_pairs("{", "}", pairs, depth, true)
    }

    fn format_set(&self, items: &mut Vec<&EdnValue>, prefix: &str, close: char, config: &OutputConfig, depth: usize) -> String {
        items.sort_by_key(|v| format!("{:?}", v)); // Ensure deterministic output
        let cf = CollectionFormatter::new(self, config);
        let formatted = items.iter().map(|item| self.format(item, config, depth));
        cf.format(prefix, &close.to_string(), formatted, depth, true)
    }
}

/// Pretty formatter - indented, multi-line
pub struct PrettyFormatter;

impl PrettyFormatter {
    fn make_indent(&self, config: &OutputConfig, depth: usize) -> String {
        if config.use_tabs {
            "\t".repeat(depth)
        } else {
            " ".repeat(depth * config.indent_size)
        }
    }


    fn is_collection(&self, value: &EdnValue) -> bool {
        match value {
            EdnValue::Vector(_) | EdnValue::List(_) | EdnValue::Map(_) | EdnValue::Set(_) => true,
            EdnValue::WithMetadata { value, .. } => self.is_collection(value),
            _ => false,
        }
    }

    fn is_simple_collection(&self, value: &EdnValue) -> bool {
        match value {
            EdnValue::Vector(v) => v.iter().all(|item| !self.is_collection(item)),
            EdnValue::List(l) => l.iter().all(|item| !self.is_collection(item)),
            EdnValue::Map(m) => m.iter().all(|(k, v)| !self.is_collection(k) && !self.is_collection(v)),
            EdnValue::Set(s) => s.iter().all(|item| !self.is_collection(item)),
            EdnValue::WithMetadata { value, .. } => self.is_simple_collection(value),
            _ => true,
        }
    }
}

impl Formatter for PrettyFormatter {
    fn format(&self, value: &EdnValue, config: &OutputConfig, depth: usize) -> String {
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
            EdnValue::Vector(v) => self.format_collection('[', ']', v, config, depth),
            EdnValue::List(l) => self.format_collection('(', ')', l, config, depth),
            EdnValue::Map(m) => self.format_map(m, config, depth),
            EdnValue::Set(s) => {
                let mut items: Vec<&EdnValue> = s.iter().collect();
                self.format_set(&mut items, "#{", '}', config, depth)
            }
            EdnValue::Tagged { tag, value } => {
                format!("#{} {}", tag, self.format(value, config, depth))
            }
            EdnValue::WithMetadata { metadata, value } => {
                format!("^{} {}", self.format(metadata, config, depth), self.format(value, config, depth))
            }
            EdnValue::Lambda(lambda) => {
                let params = lambda.params.join(" ");
                format!("(fn [{}] {})", params, self.format(&lambda.body, config, depth))
            }
            EdnValue::Instant(s) => format!("#inst \"{}\"", s),
            EdnValue::Uuid(s) => format!("#uuid \"{}\"", s),
        }
    }

    fn format_collection(&self, open: char, close: char, items: &[EdnValue], config: &OutputConfig, depth: usize) -> String {
        let cf = CollectionFormatter::new(self, config);
        let should_inline = cf.should_inline(items);
        
        if should_inline {
            let compact = CompactFormatter;
            let formatted = items.iter().map(|item| compact.format(item, config, 0));
            cf.format(&open.to_string(), &close.to_string(), formatted, depth, true)
        } else {
            let formatted = items.iter().map(|item| self.format(item, config, depth + 1));
            cf.format(&open.to_string(), &close.to_string(), formatted, depth, false)
        }
    }

    fn format_map(&self, map: &IndexMap<EdnValue, EdnValue>, config: &OutputConfig, depth: usize) -> String {
        let cf = CollectionFormatter::new(self, config);
        let compact = CompactFormatter;
        
        // Calculate estimated length
        let estimated_length: usize = map.iter()
            .map(|(k, v)| compact.format(k, config, 0).len() + compact.format(v, config, 0).len() + 1)
            .sum::<usize>() + map.len() * 2;
        
        let should_inline = cf.should_inline_map(map.len(), estimated_length);
        
        if should_inline {
            let pairs = map.iter().map(|(k, v)| (compact.format(k, config, 0), compact.format(v, config, 0)));
            cf.format_pairs("{", "}", pairs, depth, true)
        } else {
            // Multi-line with special handling for nested collections
            let mut result = String::new();
            result.push('{');
            
            for (i, (key, value)) in map.iter().enumerate() {
                if i == 0 {
                    result.push(' ');
                } else {
                    result.push('\n');
                    result.push_str(&self.make_indent(config, depth + 1));
                }
                
                result.push_str(&self.format(key, config, depth + 1));
                result.push(' ');
                
                // If value is a collection, put it on the next line
                if self.is_collection(value) && !self.is_simple_collection(value) {
                    result.push('\n');
                    result.push_str(&self.make_indent(config, depth + 1));
                }
                
                result.push_str(&self.format(value, config, depth + 1));
            }
            
            result.push('}');
            result
        }
    }

    fn format_set(&self, items: &mut Vec<&EdnValue>, prefix: &str, close: char, config: &OutputConfig, depth: usize) -> String {
        items.sort_by_key(|v| format!("{:?}", v)); // Deterministic ordering
        
        let cf = CollectionFormatter::new(self, config);
        let items_slice: Vec<EdnValue> = items.iter().map(|&v| v.clone()).collect();
        let should_inline = cf.should_inline(&items_slice);
        
        if should_inline {
            let compact = CompactFormatter;
            let formatted = items.iter().map(|item| compact.format(item, config, 0));
            cf.format(prefix, &close.to_string(), formatted, depth, true)
        } else {
            let formatted = items.iter().map(|item| self.format(item, config, depth + 1));
            cf.format(prefix, &close.to_string(), formatted, depth, false)
        }
    }
}
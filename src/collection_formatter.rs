use crate::edn::EdnValue;
use crate::formatter::{Formatter, CompactFormatter};
use crate::output::OutputConfig;

/// Unified collection formatter that handles all collection types
pub struct CollectionFormatter<'a> {
    config: &'a OutputConfig,
}

impl<'a> CollectionFormatter<'a> {
    pub fn new(_formatter: &'a dyn Formatter, config: &'a OutputConfig) -> Self {
        CollectionFormatter { config }
    }

    /// Format any collection with customizable delimiters
    pub fn format(
        &self,
        prefix: &str,
        suffix: &str,
        items: impl Iterator<Item = String>,
        depth: usize,
        should_inline: bool,
    ) -> String {
        let items: Vec<String> = items.collect();
        
        if items.is_empty() {
            return format!("{}{}", prefix, suffix);
        }

        if should_inline {
            format!("{}{}{}", prefix, items.join(" "), suffix)
        } else {
            self.format_multiline(prefix, suffix, items, depth)
        }
    }

    /// Format collection with key-value pairs (for maps)
    pub fn format_pairs(
        &self,
        prefix: &str,
        suffix: &str,
        pairs: impl Iterator<Item = (String, String)>,
        depth: usize,
        should_inline: bool,
    ) -> String {
        let items: Vec<String> = pairs.map(|(k, v)| format!("{} {}", k, v)).collect();
        
        if items.is_empty() {
            return format!("{}{}", prefix, suffix);
        }

        if should_inline {
            format!("{}{}{}", prefix, items.join(" "), suffix)
        } else {
            self.format_multiline(prefix, suffix, items, depth)
        }
    }

    /// Helper for multi-line formatting
    fn format_multiline(
        &self,
        prefix: &str,
        suffix: &str,
        items: Vec<String>,
        depth: usize,
    ) -> String {
        let mut result = String::new();
        result.push_str(prefix);
        
        for (i, item) in items.iter().enumerate() {
            if i == 0 && prefix.chars().last() != Some('{') {
                result.push(' ');
            } else if i == 0 {
                result.push(' ');
            } else {
                result.push('\n');
                result.push_str(&self.make_indent(depth + 1));
            }
            result.push_str(item);
        }
        
        result.push_str(suffix);
        result
    }

    fn make_indent(&self, depth: usize) -> String {
        if self.config.use_tabs {
            "\t".repeat(depth)
        } else {
            " ".repeat(depth * self.config.indent_size)
        }
    }

    /// Check if items should be formatted inline based on size heuristics
    pub fn should_inline(&self, items: &[EdnValue]) -> bool {
        if items.len() > 4 {
            return false;
        }
        
        let compact = CompactFormatter;
        let estimated_length: usize = items.iter()
            .map(|item| compact.format(item, self.config, 0).len())
            .sum::<usize>() + items.len();
        
        estimated_length < 60
    }

    /// Check if map should be formatted inline
    pub fn should_inline_map(&self, size: usize, estimated_length: usize) -> bool {
        size <= 2 && estimated_length < 50
    }
}
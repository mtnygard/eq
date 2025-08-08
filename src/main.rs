use clap::Parser;
use std::io::{self, Read};
use std::fs;
use std::path::PathBuf;
use walkdir::WalkDir;
use glob::Pattern;

mod cli;
mod edn;
mod error;
mod query;
mod analyzer;
mod evaluator;
mod output;
mod builtins;

use cli::Args;
use error::EqResult;
use edn::{EdnValue, Parser as EdnParser};
use query::QueryParser;
use analyzer::analyze;
use evaluator::evaluate;
use output::{OutputConfig, format_output};

fn find_files_recursive(paths: &[PathBuf], pattern: &str, recursive: bool) -> EqResult<Vec<PathBuf>> {
    let glob_pattern = Pattern::new(pattern)?;
    let mut files = Vec::new();
    
    for path in paths {
        if path.is_file() {
            // If it's a file, just add it directly
            files.push(path.clone());
        } else if path.is_dir() {
            if recursive {
                // Walk the directory tree
                for entry in WalkDir::new(path).follow_links(true) {
                    let entry = entry?;
                    if entry.file_type().is_file() {
                        if let Some(file_name) = entry.path().file_name() {
                            if let Some(file_name_str) = file_name.to_str() {
                                if glob_pattern.matches(file_name_str) {
                                    files.push(entry.path().to_path_buf());
                                }
                            }
                        }
                    }
                }
            } else {
                // Just look at immediate children
                for entry in fs::read_dir(path)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(file_name) = path.file_name() {
                            if let Some(file_name_str) = file_name.to_str() {
                                if glob_pattern.matches(file_name_str) {
                                    files.push(path);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    Ok(files)
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> EqResult<()> {
    let args = Args::parse();
    
    // Set up output configuration
    let mut output_config = OutputConfig::default();
    output_config.compact = args.compact;
    output_config.raw_strings = args.raw_output;
    output_config.use_tabs = args.tab;
    output_config.indent_size = args.indent;
    
    // Get the filter expression
    let filter = if let Some(filter_file) = &args.from_file {
        fs::read_to_string(filter_file)?
    } else {
        args.filter.clone()
    };
    
    // Parse and analyze the query
    let query_ast = QueryParser::parse(&filter)?;
    let analyzed_query = analyze(query_ast)?;
    
    // Process inputs
    if args.null_input {
        // No input, just run filter on nil
        let result = evaluate(&analyzed_query, &EdnValue::Nil)?;
        print_result(&result, &output_config, &args, None);
    } else if args.files.is_empty() && !args.recursive {
        // Read from stdin
        process_input(&analyzed_query, &output_config, &args, io::stdin(), None)?;
    } else {
        // Check if we need to do recursive file finding
        let files_to_process = if args.files.iter().any(|p| p.is_dir()) || args.recursive {
            // If recursive flag is set but no files specified, search current directory
            let search_paths = if args.files.is_empty() && args.recursive {
                vec![PathBuf::from(".")]
            } else {
                args.files.clone()
            };
            find_files_recursive(&search_paths, &args.glob_pattern, args.recursive)?
        } else {
            args.files.clone()
        };
        
        // Process each file
        for file_path in &files_to_process {
            let file = fs::File::open(file_path)?;
            let filename = file_path.to_string_lossy();
            process_input(&analyzed_query, &output_config, &args, file, Some(&filename))?;
        }
    }
    
    Ok(())
}

fn print_result(result: &EdnValue, output_config: &OutputConfig, args: &Args, filename: Option<&str>) {
    // Skip output for nil values if suppress_nil flag is set
    if args.suppress_nil && matches!(result, EdnValue::Nil) {
        return;
    }
    
    let output = format_output(result, output_config);
    if args.with_filename {
        if let Some(fname) = filename {
            println!("{}:{}", fname, output);
        } else {
            println!("(stdin):{}", output);
        }
    } else {
        println!("{}", output);
    }
}

fn process_input<R: Read>(
    query: &query::ast::Expr,
    output_config: &OutputConfig,
    args: &Args,
    mut reader: R,
    filename: Option<&str>,
) -> EqResult<()> {
    let mut input_string = String::new();
    reader.read_to_string(&mut input_string)?;
    
    if args.raw_input {
        // Treat each line as a string
        for line in input_string.lines() {
            let input_value = EdnValue::String(line.to_string());
            let result = evaluate(query, &input_value)?;
            print_result(&result, output_config, args, filename);
        }
    } else if args.slurp {
        // Parse all values and put them in a vector
        let mut values = Vec::new();
        let mut parser = EdnParser::new_with_filename(&input_string, filename.map(|s| s.to_string()));
        
        // Keep parsing until we reach the end
        while let Ok(value) = parser.parse() {
            if matches!(value, EdnValue::Nil) {
                // Check if we're actually at the end or if nil was parsed
                break;
            }
            values.push(value);
        }
        
        let input_array = EdnValue::Vector(values);
        let result = evaluate(query, &input_array)?;
        print_result(&result, output_config, args, filename);
    } else {
        // Parse and process each top-level EDN value
        let mut parser = EdnParser::new_with_filename(&input_string, filename.map(|s| s.to_string()));
        
        loop {
            let value = parser.parse()?;
            
            // Check if we've reached the end of input
            if matches!(value, EdnValue::Nil) && parser.remaining_input().trim().is_empty() {
                break;
            }
            
            // Process the parsed value
            let result = evaluate(query, &value)?;
            print_result(&result, output_config, args, filename);
            
            // Check if there's more to parse
            if parser.remaining_input().trim().is_empty() {
                break;
            }
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_identity_query() {
        let query_ast = QueryParser::parse(".").unwrap();
        let analyzed_query = analyze(query_ast).unwrap();
        let config = OutputConfig::default();
        
        let input = EdnValue::Integer(42);
        let result = evaluate(&analyzed_query, &input).unwrap();
        
        assert_eq!(format_output(&result, &config), "42");
    }

    #[test]
    fn test_keyword_access() {
        let query_ast = QueryParser::parse("(:name .)").unwrap();
        let analyzed_query = analyze(query_ast).unwrap();
        let config = OutputConfig::default();
        
        let mut map = indexmap::IndexMap::new();
        map.insert(EdnValue::Keyword("name".to_string()), EdnValue::String("Alice".to_string()));
        let input = EdnValue::Map(map);
        
        let result = evaluate(&analyzed_query, &input).unwrap();
        assert_eq!(format_output(&result, &config), "\"Alice\"");
    }

    #[test]
    fn test_collection_operations() {
        let query_ast = QueryParser::parse("(first .)").unwrap();
        let analyzed_query = analyze(query_ast).unwrap();
        let config = OutputConfig::default();
        
        let input = EdnValue::Vector(vec![
            EdnValue::String("first".to_string()),
            EdnValue::String("second".to_string()),
        ]);
        
        let result = evaluate(&analyzed_query, &input).unwrap();
        assert_eq!(format_output(&result, &config), "\"first\"");
    }

    #[test]
    fn test_raw_input_processing() {
        let args = Args {
            filter: ".".to_string(),
            files: vec![],
            compact: false,
            raw_output: false,
            raw_input: true,
            slurp: false,
            null_input: false,
            exit_status: false,
            from_file: None,
            tab: false,
            indent: 2,
            debug: false,
            verbose: false,
            with_filename: false,
            recursive: false,
            glob_pattern: "*.edn".to_string(),
            suppress_nil: false,
        };
        
        let query_ast = QueryParser::parse(".").unwrap();
        let analyzed_query = analyze(query_ast).unwrap();
        let config = OutputConfig::default();
        
        let input_data = "hello\nworld\n";
        let cursor = Cursor::new(input_data);
        
        // This would normally print, but we can't easily test that
        // In a real implementation, we'd refactor to return results
        process_input(&analyzed_query, &config, &args, cursor, Some("test_input")).unwrap();
    }

    #[test]
    fn test_complex_query() {
        let query_ast = QueryParser::parse("(-> . (first) (:name))").unwrap();
        let analyzed_query = analyze(query_ast).unwrap();
        let config = OutputConfig::default();
        
        let mut person1 = indexmap::IndexMap::new();
        person1.insert(EdnValue::Keyword("name".to_string()), EdnValue::String("Alice".to_string()));
        
        let mut person2 = indexmap::IndexMap::new();
        person2.insert(EdnValue::Keyword("name".to_string()), EdnValue::String("Bob".to_string()));
        
        let input = EdnValue::Vector(vec![
            EdnValue::Map(person1),
            EdnValue::Map(person2),
        ]);
        
        let result = evaluate(&analyzed_query, &input).unwrap();
        assert_eq!(format_output(&result, &config), "\"Alice\"");
    }
    
    #[test]
    fn test_find_files_recursive() {
        use std::fs;
        use std::env;
        
        // Create a temporary directory structure for testing
        let temp_dir = env::temp_dir().join("eq_test_recursive");
        let _ = fs::remove_dir_all(&temp_dir); // Clean up if exists
        fs::create_dir_all(&temp_dir).unwrap();
        
        // Create test files
        fs::write(temp_dir.join("test1.edn"), "{}").unwrap();
        fs::write(temp_dir.join("test2.edn"), "[]").unwrap();
        fs::write(temp_dir.join("other.json"), "{}").unwrap();
        
        // Create subdirectory with more files
        let sub_dir = temp_dir.join("subdir");
        fs::create_dir_all(&sub_dir).unwrap();
        fs::write(sub_dir.join("test3.edn"), "nil").unwrap();
        fs::write(sub_dir.join("test4.json"), "{}").unwrap();
        
        // Test non-recursive with *.edn pattern
        let files = find_files_recursive(&vec![temp_dir.clone()], "*.edn", false).unwrap();
        assert_eq!(files.len(), 2); // Should find test1.edn and test2.edn
        
        // Test recursive with *.edn pattern
        let files = find_files_recursive(&vec![temp_dir.clone()], "*.edn", true).unwrap();
        assert_eq!(files.len(), 3); // Should find test1.edn, test2.edn, and test3.edn
        
        // Test recursive with *.json pattern
        let files = find_files_recursive(&vec![temp_dir.clone()], "*.json", true).unwrap();
        assert_eq!(files.len(), 2); // Should find other.json and test4.json
        
        // Test with direct file path
        let direct_file = temp_dir.join("test1.edn");
        let files = find_files_recursive(&vec![direct_file], "*.edn", false).unwrap();
        assert_eq!(files.len(), 1); // Should return the file itself
        
        // Clean up
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
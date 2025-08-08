use std::process::Command;
use std::fs;

#[test]
fn test_basic_operations() {
    // Create test data
    fs::write("test_basic.edn", r#"{:name "Alice" :age 30}"#).unwrap();
    
    // Test identity
    let output = Command::new("./target/release/eq")
        .args(&[".", "test_basic.edn"])
        .output()
        .expect("Failed to execute eq");
    
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Alice"));
    assert!(stdout.contains("30"));
    
    // Test keyword access
    let output = Command::new("./target/release/eq")
        .args(&["(:name)", "test_basic.edn"])
        .output()
        .expect("Failed to execute eq");
    
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout.trim(), "\"Alice\"");
    
    // Cleanup
    fs::remove_file("test_basic.edn").unwrap();
}

#[test]
fn test_collection_operations() {
    // Create test data
    fs::write("test_array.edn", r#"[1 2 3 4 5]"#).unwrap();
    
    // Test first
    let output = Command::new("./target/release/eq")
        .args(&["(first)", "test_array.edn"])
        .output()
        .expect("Failed to execute eq");
    
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout.trim(), "1");
    
    // Test count
    let output = Command::new("./target/release/eq")
        .args(&["(count)", "test_array.edn"])
        .output()
        .expect("Failed to execute eq");
    
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout.trim(), "5");
    
    // Cleanup
    fs::remove_file("test_array.edn").unwrap();
}

#[test]
fn test_compact_output() {
    // Create test data
    fs::write("test_compact.edn", r#"{:a {:b {:c 42}}}"#).unwrap();
    
    // Test compact output
    let output = Command::new("./target/release/eq")
        .args(&["-c", ".", "test_compact.edn"])
        .output()
        .expect("Failed to execute eq");
    
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.trim().contains('\n')); // Should be on one line (ignoring final newline)
    assert!(stdout.contains("{:a {:b {:c 42}}}"));
    
    // Cleanup
    fs::remove_file("test_compact.edn").unwrap();
}

#[test]
fn test_raw_output() {
    // Create test data
    fs::write("test_raw.edn", r#"{:message "Hello World"}"#).unwrap();
    
    // Test raw string output
    let output = Command::new("./target/release/eq")
        .args(&["--raw-output", "(:message)", "test_raw.edn"])
        .output()
        .expect("Failed to execute eq");
    
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout.trim(), "Hello World"); // No quotes
    
    // Cleanup
    fs::remove_file("test_raw.edn").unwrap();
}

#[test]
fn test_error_handling() {
    // Test invalid query
    let output = Command::new("./target/release/eq")
        .args(&["(invalid-function)", "-n"]) // null input to avoid file issues
        .output()
        .expect("Failed to execute eq");
    
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Unknown function"));
}

#[test]
fn test_null_input() {
    // Test null input mode - just test that nil input works
    let output = Command::new("./target/release/eq")
        .args(&["-n", "(nil?)"])
        .output()
        .expect("Failed to execute eq");
    
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout.trim(), "true");
}

#[test]
fn test_broken_edn_files() {
    // Test unterminated string
    fs::write("test_broken1.edn", r#"{"unterminated string}"#).unwrap();
    let output = Command::new("./target/release/eq")
        .args(&[".", "test_broken1.edn"])
        .output()
        .expect("Failed to execute eq");
    
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Parse error") || stderr.contains("ParseError") || stderr.contains("Unterminated"));
    fs::remove_file("test_broken1.edn").unwrap();
    
    // Test unterminated vector
    fs::write("test_broken2.edn", r#"[1 2 3"#).unwrap();
    let output = Command::new("./target/release/eq")
        .args(&[".", "test_broken2.edn"])
        .output()
        .expect("Failed to execute eq");
    
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Parse error") || stderr.contains("ParseError") || stderr.contains("Unterminated"));
    fs::remove_file("test_broken2.edn").unwrap();
    
    // Test invalid map (odd number of elements)
    fs::write("test_broken3.edn", r#"{:key}"#).unwrap();
    let output = Command::new("./target/release/eq")
        .args(&[".", "test_broken3.edn"])
        .output()
        .expect("Failed to execute eq");
    
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Parse error") || stderr.contains("ParseError") || stderr.contains("Expected"));
    fs::remove_file("test_broken3.edn").unwrap();
    
    // Test duplicate set elements  
    fs::write("test_broken4.edn", r#"#{1 1}"#).unwrap();
    let output = Command::new("./target/release/eq")
        .args(&[".", "test_broken4.edn"])
        .output()
        .expect("Failed to execute eq");
    
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("ParseError") || stderr.contains("Duplicate"));
    fs::remove_file("test_broken4.edn").unwrap();
}

#[test]
fn test_broken_queries() {
    // Test empty parentheses
    let output = Command::new("./target/release/eq")
        .args(&["()", "-n"])
        .output()
        .expect("Failed to execute eq");
    
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("QueryError") || stderr.contains("Empty"));
    
    // Test unterminated parentheses
    let output = Command::new("./target/release/eq")
        .args(&["(first", "-n"])
        .output()
        .expect("Failed to execute eq");
    
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Parse error") || stderr.contains("ParseError") || stderr.contains("Unterminated"));
    
    // Test invalid function arguments
    let output = Command::new("./target/release/eq")
        .args(&["(get)", "-n"])
        .output()
        .expect("Failed to execute eq");
    
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("QueryError") || stderr.contains("takes exactly"));
    
    // Test too many arguments
    let output = Command::new("./target/release/eq")
        .args(&["(get :a :b :c)", "-n"])
        .output()
        .expect("Failed to execute eq");
    
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("QueryError") || stderr.contains("takes exactly"));
}

#[test]
fn test_metadata_parsing() {
    // Test simple keyword metadata
    fs::write("test_metadata1.edn", r#"^:tag {:key "value"}"#).unwrap();
    let output = Command::new("./target/release/eq")
        .args(&[".", "test_metadata1.edn"])
        .output()
        .expect("Failed to execute eq");
    
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("^:tag"));
    assert!(stdout.contains("{:key \"value\"}"));
    fs::remove_file("test_metadata1.edn").unwrap();
    
    // Test map metadata with the user's example
    fs::write("test_metadata2.edn", r#"{:features ^{:replace true} #{:datomic :datomic-init}}"#).unwrap();
    let output = Command::new("./target/release/eq")
        .args(&[".", "test_metadata2.edn"])
        .output()
        .expect("Failed to execute eq");
    
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("^{:replace true}"));
    assert!(stdout.contains("#{:datomic :datomic-init}"));
    fs::remove_file("test_metadata2.edn").unwrap();
    
    // Test accessing the value through metadata
    fs::write("test_metadata3.edn", r#"^{:doc "A set"} #{:a :b :c}"#).unwrap();
    let output = Command::new("./target/release/eq")
        .args(&["(count)", "test_metadata3.edn"])
        .output()
        .expect("Failed to execute eq");
    
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout.trim(), "3"); // Should count the set elements, not the metadata
    fs::remove_file("test_metadata3.edn").unwrap();
}

#[test]
fn test_discard_macro() {
    // Test discard in vector
    fs::write("test_discard1.edn", r#"[1 2 #_ 3 4]"#).unwrap();
    let output = Command::new("./target/release/eq")
        .args(&[".", "test_discard1.edn"])
        .output()
        .expect("Failed to execute eq");
    
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("[1 2 4]"));
    fs::remove_file("test_discard1.edn").unwrap();
    
    // Test discard in map
    fs::write("test_discard2.edn", r#"{:a 1 #_ :b #_ 2 :c 3}"#).unwrap();
    let output = Command::new("./target/release/eq")
        .args(&[".", "test_discard2.edn"])
        .output()
        .expect("Failed to execute eq");
    
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(":a 1"));
    assert!(stdout.contains(":c 3"));
    assert!(!stdout.contains(":b"));
    fs::remove_file("test_discard2.edn").unwrap();
    
    // Test discard in set
    fs::write("test_discard3.edn", r#"#{1 #_ 2 3}"#).unwrap();
    let output = Command::new("./target/release/eq")
        .args(&["(count)", "test_discard3.edn"])
        .output()
        .expect("Failed to execute eq");
    
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout.trim(), "2"); // Should be 2 elements after discarding 2
    fs::remove_file("test_discard3.edn").unwrap();
}

#[test]
fn test_builtin_tagged_literals() {
    // Test #inst
    fs::write("test_inst.edn", r#"#inst "2023-01-01T12:30:45Z""#).unwrap();
    let output = Command::new("./target/release/eq")
        .args(&[".", "test_inst.edn"])
        .output()
        .expect("Failed to execute eq");
    
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("#inst \"2023-01-01T12:30:45Z\""));
    fs::remove_file("test_inst.edn").unwrap();
    
    // Test #uuid
    fs::write("test_uuid.edn", r#"#uuid "f81d4fae-7dec-11d0-a765-00a0c91e6bf6""#).unwrap();
    let output = Command::new("./target/release/eq")
        .args(&[".", "test_uuid.edn"])
        .output()
        .expect("Failed to execute eq");
    
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("#uuid \"f81d4fae-7dec-11d0-a765-00a0c91e6bf6\""));
    fs::remove_file("test_uuid.edn").unwrap();
    
    // Test invalid formats
    fs::write("test_bad_inst.edn", r#"#inst "not-a-date""#).unwrap();
    let output = Command::new("./target/release/eq")
        .args(&[".", "test_bad_inst.edn"])
        .output()
        .expect("Failed to execute eq");
    
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Invalid instant format"));
    fs::remove_file("test_bad_inst.edn").unwrap();
}

#[test]
fn test_unicode_escapes() {
    // Test unicode character literal
    fs::write("test_unicode_char.edn", r#"\u03A9"#).unwrap();
    let output = Command::new("./target/release/eq")
        .args(&[".", "test_unicode_char.edn"])
        .output()
        .expect("Failed to execute eq");
    
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\\Ω") || stdout.contains("Ω")); // Output format may vary
    fs::remove_file("test_unicode_char.edn").unwrap();
    
    // Test unicode in string
    fs::write("test_unicode_string.edn", r#""Hello \u03A9 World""#).unwrap();
    let output = Command::new("./target/release/eq")
        .args(&[".", "test_unicode_string.edn"])
        .output()
        .expect("Failed to execute eq");
    
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Hello Ω World"));
    fs::remove_file("test_unicode_string.edn").unwrap();
}

#[test]
fn test_new_character_literals() {
    // Test formfeed character
    fs::write("test_formfeed.edn", r#"\formfeed"#).unwrap();
    let output = Command::new("./target/release/eq")
        .args(&[".", "test_formfeed.edn"])
        .output()
        .expect("Failed to execute eq");
    
    assert!(output.status.success());
    fs::remove_file("test_formfeed.edn").unwrap();
    
    // Test backspace character
    fs::write("test_backspace.edn", r#"\backspace"#).unwrap();
    let output = Command::new("./target/release/eq")
        .args(&[".", "test_backspace.edn"])
        .output()
        .expect("Failed to execute eq");
    
    assert!(output.status.success());
    fs::remove_file("test_backspace.edn").unwrap();
}

#[test]
fn test_comma_as_whitespace() {
    // Test commas in various collections
    fs::write("test_commas.edn", r#"[1, 2, 3]"#).unwrap();
    let output = Command::new("./target/release/eq")
        .args(&[".", "test_commas.edn"])
        .output()
        .expect("Failed to execute eq");
    
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("[1 2 3]"));
    fs::remove_file("test_commas.edn").unwrap();
    
    // Test commas in map
    fs::write("test_commas_map.edn", r#"{:a 1, :b 2, :c 3}"#).unwrap();
    let output = Command::new("./target/release/eq")
        .args(&[".", "test_commas_map.edn"])
        .output()
        .expect("Failed to execute eq");
    
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(":a 1"));
    assert!(stdout.contains(":b 2"));
    assert!(stdout.contains(":c 3"));
    fs::remove_file("test_commas_map.edn").unwrap();
    
    // Test trailing commas
    fs::write("test_trailing_commas.edn", r#"[1, 2, 3,]"#).unwrap();
    let output = Command::new("./target/release/eq")
        .args(&["(count)", "test_trailing_commas.edn"])
        .output()
        .expect("Failed to execute eq");
    
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout.trim(), "3");
    fs::remove_file("test_trailing_commas.edn").unwrap();
    
    // Test multiple consecutive commas
    fs::write("test_multiple_commas.edn", r#"[1,, 2,,, 3]"#).unwrap();
    let output = Command::new("./target/release/eq")
        .args(&["(count)", "test_multiple_commas.edn"])
        .output()
        .expect("Failed to execute eq");
    
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout.trim(), "3");
    fs::remove_file("test_multiple_commas.edn").unwrap();
}

#[test]
fn test_file_errors() {
    // Test non-existent file
    let output = Command::new("./target/release/eq")
        .args(&[".", "nonexistent.edn"])
        .output()
        .expect("Failed to execute eq");
    
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("No such file") || stderr.contains("not found"));
    
    // Test directory instead of file - should now succeed but produce no output
    let _ = fs::remove_dir("test_dir"); // Clean up if exists from previous run
    fs::create_dir("test_dir").unwrap();
    let output = Command::new("./target/release/eq")
        .args(&[".", "test_dir"])
        .output()
        .expect("Failed to execute eq");
    
    // Empty directory should succeed but produce no output
    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    fs::remove_dir("test_dir").unwrap();
}
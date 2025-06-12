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
        .args(&[":name", "test_basic.edn"])
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
        .args(&["-r", ":message", "test_raw.edn"])
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
    assert!(stderr.contains("ParseError") || stderr.contains("Unterminated"));
    fs::remove_file("test_broken1.edn").unwrap();
    
    // Test unterminated vector
    fs::write("test_broken2.edn", r#"[1 2 3"#).unwrap();
    let output = Command::new("./target/release/eq")
        .args(&[".", "test_broken2.edn"])
        .output()
        .expect("Failed to execute eq");
    
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("ParseError") || stderr.contains("Unterminated"));
    fs::remove_file("test_broken2.edn").unwrap();
    
    // Test invalid map (odd number of elements)
    fs::write("test_broken3.edn", r#"{:key}"#).unwrap();
    let output = Command::new("./target/release/eq")
        .args(&[".", "test_broken3.edn"])
        .output()
        .expect("Failed to execute eq");
    
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("ParseError") || stderr.contains("Expected"));
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
    assert!(stderr.contains("ParseError") || stderr.contains("Unterminated"));
    
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
fn test_file_errors() {
    // Test non-existent file
    let output = Command::new("./target/release/eq")
        .args(&[".", "nonexistent.edn"])
        .output()
        .expect("Failed to execute eq");
    
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("No such file") || stderr.contains("not found"));
    
    // Test directory instead of file
    fs::create_dir("test_dir").unwrap();
    let output = Command::new("./target/release/eq")
        .args(&[".", "test_dir"])
        .output()
        .expect("Failed to execute eq");
    
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("directory") || stderr.contains("Invalid"));
    fs::remove_dir("test_dir").unwrap();
}
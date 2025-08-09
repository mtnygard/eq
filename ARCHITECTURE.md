# eq - Architecture Document

## Design Goals

### Primary Requirements
- **Fast startup time** - Sub-100ms cold start for typical queries
- **Portability** - Single binary deployment across platforms
- **Memory efficiency** - Handle large EDN files without excessive memory usage
- **Streaming capability** - Process data incrementally where possible

### Secondary Requirements
- Maintainable codebase with clear separation of concerns
- Extensible architecture for future enhancements
- Robust error handling and reporting

## Technology Stack

### Core Language: Rust
- **Rationale**: Fast compilation to native code, zero-cost abstractions, excellent memory safety
- **Startup time**: Native binaries start in ~1-5ms vs JVM's 100-500ms overhead
- **Portability**: Cross-compile to all major platforms from single codebase
- **Memory**: Precise control over allocations, no GC pauses

### Alternative Considerations
- **Go**: Good performance but GC can cause unpredictable pauses
- **Zig/C**: Maximum performance but higher development complexity
- **Native Clojure**: Too slow for CLI tool (JVM startup overhead)
- **GraalVM Native**: Better than JVM but still slower than Rust, larger binaries

## Architecture Overview

```
┌─────────────────┐    ┌──────────────┐    ┌─────────────────┐
│   CLI Parser    │────│    Core      │────│   Output        │
│   (clap-based)  │    │   Engine     │    │  Formatter      │
└─────────────────┘    └──────────────┘    └─────────────────┘
                              │
                    ┌─────────┼─────────┐
                    │         │         │
            ┌───────▼────┐ ┌──▼───┐ ┌───▼──────┐
            │ EDN Parser │ │Query │ │File      │
            │            │ │Engine│ │Discovery │
            └────────────┘ └──────┘ └──────────┘
                                          │
                                    ┌─────▼─────┐
                                    │   Glob    │
                                    │  Pattern  │
                                    │ Matching  │
                                    └───────────┘
```

## Component Design

### 1. CLI Parser (`cli/`)
**Responsibility**: Command-line argument parsing and validation

**Implementation**:
- Use `clap` crate for argument parsing (compile-time validation)
- Minimal allocation during parsing
- Early validation to fail fast on invalid arguments

**Key Structures**:
```rust
pub struct Args {
    pub filter: String,
    pub files: Vec<PathBuf>,
    pub compact: bool,
    pub raw_output: bool,
    pub raw_input: bool,
    pub slurp: bool,
    pub null_input: bool,
    pub exit_status: bool,
    pub from_file: Option<PathBuf>,
    pub tab: bool,
    pub indent: usize,
    pub debug: bool,
    pub verbose: bool,
    pub with_filename: bool,
    pub recursive: bool,
    pub glob_pattern: String,
    pub suppress_nil: bool,
}
```

### 2. EDN Parser (`edn/`)
**Responsibility**: Parse EDN text into internal data structures

**Implementation**:
- Custom recursive descent parser (not regex-based)
- Zero-copy parsing where possible using string slices
- Streaming parser for large files
- Preserve source location information for error reporting

**Key Structures**:
```rust
pub enum EdnValue {
    Nil,
    Bool(bool),
    String(Cow<'a, str>),  // Zero-copy when possible
    Keyword(Cow<'a, str>),
    Integer(i64),
    Float(f64),
    Vector(Vec<EdnValue>),
    Map(IndexMap<EdnValue, EdnValue>),
    Set(HashSet<EdnValue>),
    List(Vec<EdnValue>),
    Tagged { tag: String, value: Box<EdnValue> },
}
```

**Design Decisions**:
- `Cow<str>` for zero-copy string handling
- `IndexMap` to preserve insertion order in maps
- Lazy parsing for streaming scenarios

### 3. Query Engine (`query/`, `evaluator.rs`)
**Responsibility**: Parse and evaluate filter expressions

**Current Implementation**:
- Direct AST evaluation (no bytecode compilation yet)
- Recursive descent parser for Clojure-like syntax
- Built-in function registry with extensible design

```rust
// Current AST structure (simplified)
pub enum QueryNode {
    Identity,
    Get(String),
    GetIn(Vec<EdnValue>),
    FunctionCall { name: String, args: Vec<QueryNode> },
    // ... other operations
}
```

**Evaluation Pipeline**:
1. **Parser**: Parse filter string into AST
2. **Evaluator**: Direct evaluation of AST nodes against input data
3. **Built-ins**: Function registry for core operations

*Note: Bytecode compilation and VM are architectural future goals*

### 4. Function Registry (`evaluator.rs`)
**Responsibility**: Manage built-in and user-defined functions

**Design**:
- Global function registry initialized at startup
- Type-safe function signatures
- Extensible for future plugin system

```rust
pub type BuiltinFunction = fn(&[EdnValue]) -> Result<EdnValue, EvaluationError>;

pub struct FunctionRegistry {
    functions: HashMap<String, BuiltinFunction>,
}
```

### 5. Current File Processing
**Responsibility**: Process individual files and handle I/O

**Current Implementation**:
- File-by-file processing (streaming is a future goal)
- Memory-mapped files for large inputs
- Error handling with context preservation

```rust
// Current approach - processes entire file into memory
pub fn process_file(path: &Path, query: &str) -> Result<EdnValue, EqError> {
    let content = std::fs::read_to_string(path)?;
    let parsed = parse_edn(&content)?;
    evaluate_query(query, parsed)
}
```

*Note: True streaming implementation is planned for future versions*

### 6. File Discovery (`main.rs`)
**Responsibility**: Find and filter files based on glob patterns and recursion settings

**Implementation**:
- Uses `glob` crate for pattern matching
- `walkdir` for recursive directory traversal
- Supports both file arguments and directory scanning
- Integrates with CLI flags for recursive search and pattern filtering

```rust
fn find_files_recursive(paths: &[PathBuf], pattern: &str, recursive: bool) -> EqResult<Vec<PathBuf>> {
    let glob_pattern = Pattern::new(pattern)?;
    // Implementation handles recursive traversal and pattern matching
}
```

### 7. Output Formatter (`output/`)
**Responsibility**: Serialize results back to EDN format

**Features**:
- Pretty printing with configurable indentation
- Compact output mode
- Raw string output mode
- Optional filename prefixing (like grep -H)
- Nil suppression option

## Memory Management Strategy

### Stack Allocation Priority
- Use stack allocation for small, fixed-size data
- `SmallVec` for collections that are usually small
- String interning for repeated keywords/symbols

### Streaming for Large Data
- Process arrays element-by-element when possible
- Lazy evaluation of filter chains
- Bounded memory usage regardless of input size

### Zero-Copy Optimizations
- String slices instead of owned strings where possible
- Avoid unnecessary clones during processing
- Reference counting for shared immutable data

## Performance Optimizations

### Compilation Optimizations
- Compile with `--release` and `lto = true`
- Profile-guided optimization (PGO) for hot paths
- Target-specific optimizations

### Runtime Optimizations
- JIT compilation for frequently used queries (future enhancement)
- Memoization of compiled query bytecode
- SIMD operations for numeric array processing

### Startup Time Optimizations
- Minimize global static initialization
- Lazy loading of non-essential components
- Avoid dynamic library dependencies

## Error Handling Strategy

### Error Types
```rust
pub enum EqError {
    ParseError { line: usize, column: usize, message: String },
    QueryError { message: String },
    IoError(std::io::Error),
    RuntimeError { context: String, source: Box<dyn Error> },
}
```

### Error Recovery
- Graceful degradation where possible
- Clear, actionable error messages
- Source location information for parse errors

## Build and Distribution

### Build Configuration
```toml
[profile.release]
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

### Distribution Strategy
- GitHub Releases with pre-built binaries
- Cargo for Rust ecosystem distribution
- Package managers (Homebrew, apt/yum) - future consideration
- Container images for CI/CD usage - future consideration

### Cross-Compilation Targets
- `x86_64-unknown-linux-gnu`
- `x86_64-pc-windows-msvc` 
- `x86_64-apple-darwin`
- `aarch64-apple-darwin` (Apple Silicon)
- `aarch64-unknown-linux-gnu` (ARM64 Linux)

## Testing Strategy

### Unit Tests
- Each component thoroughly unit tested
- Property-based testing for parser and VM
- Benchmark tests for performance regression detection

### Integration Tests
- End-to-end CLI testing
- Compatibility tests against reference EDN implementations
- Performance benchmarks against `jq` and other tools

### Fuzzing
- Fuzz testing for parser robustness
- Query compiler fuzzing
- Memory safety validation

## Future Architecture Considerations

### Plugin System
- Dynamic loading of user-defined functions
- WebAssembly plugin interface for sandboxing

### Language Server Protocol
- LSP implementation for filter expression editing
- IDE integration with syntax highlighting and completion

### Distributed Processing
- Cluster mode for processing very large datasets
- Integration with stream processing frameworks
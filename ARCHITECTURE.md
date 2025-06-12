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
│                 │    │   Engine     │    │  Formatter      │
└─────────────────┘    └──────────────┘    └─────────────────┘
                              │
                    ┌─────────┼─────────┐
                    │         │         │
            ┌───────▼────┐ ┌──▼───┐ ┌───▼──────┐
            │ EDN Parser │ │Query │ │Streaming │
            │            │ │ VM   │ │ Engine   │
            └────────────┘ └──────┘ └──────────┘
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
pub struct Config {
    pub filter: String,
    pub files: Vec<PathBuf>,
    pub output_format: OutputFormat,
    pub compact: bool,
    pub raw_output: bool,
    // ... other options
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

### 3. Query Compiler (`query/`)
**Responsibility**: Parse and compile filter expressions to bytecode

**Architecture**:
```rust
// Abstract Syntax Tree
pub enum Expr {
    Identity,
    Get(GetExpr),
    GetIn(Vec<EdnValue>),
    Filter(Box<Expr>),
    Map(Box<Expr>),
    // ... other operations
}

// Compiled bytecode for execution
pub enum OpCode {
    Push(EdnValue),
    Get,
    GetIn(u8), // operand count
    Filter,
    Map,
    // ... corresponding ops
}

pub struct CompiledQuery {
    bytecode: Vec<OpCode>,
    constants: Vec<EdnValue>,
}
```

**Compilation Pipeline**:
1. **Lexer**: Tokenize filter string
2. **Parser**: Build AST from tokens  
3. **Compiler**: Generate bytecode from AST
4. **Optimizer**: Optimize bytecode (constant folding, dead code elimination)

### 4. Query VM (`vm/`)
**Responsibility**: Execute compiled queries efficiently

**Design**:
- Stack-based virtual machine
- Minimal allocation during execution
- Built-in streaming support for large collections

```rust
pub struct QueryVM {
    stack: Vec<EdnValue>,
    constants: Vec<EdnValue>,
    pc: usize, // program counter
}

impl QueryVM {
    pub fn execute(&mut self, bytecode: &[OpCode], input: EdnValue) -> Result<EdnValue> {
        // Execute bytecode instructions
    }
    
    pub fn execute_streaming<R: Read>(&mut self, bytecode: &[OpCode], input: R) -> impl Stream<Item = EdnValue> {
        // Streaming execution for large inputs
    }
}
```

### 5. Streaming Engine (`stream/`)
**Responsibility**: Handle large files without loading entirely into memory

**Implementation**:
- Async iterators over EDN values
- Backpressure handling
- Chunked processing for arrays/vectors

```rust
pub trait EdnStream {
    type Item = Result<EdnValue, EdnError>;
    
    fn next(&mut self) -> Option<Self::Item>;
}

pub struct FileEdnStream {
    reader: BufReader<File>,
    parser: StreamingEdnParser,
}
```

### 6. Output Formatter (`output/`)
**Responsibility**: Serialize results back to EDN format

**Features**:
- Pretty printing with configurable indentation
- Compact output mode
- Raw string output mode
- Preserve comments when possible

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
- Package managers (Homebrew, Cargo, apt/yum)
- Container images for CI/CD usage
- WebAssembly build for browser usage

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
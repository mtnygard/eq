# eq - EDN Query Tool Specification

## Overview

`eq` is a command-line tool for parsing, filtering, mapping, and transforming EDN (Extensible Data Notation) data. It provides a Clojure-inspired query language for processing EDN files, similar to how `jq` processes JSON.

## Basic Usage

```bash
eq [OPTIONS] '<filter>' [file...]
```

If no file is specified, `eq` reads from stdin.

## Filter Language

The filter language is based on Clojure syntax and functions:

**Important**: All functions require explicit arguments. Use `.` to represent the current input value being processed.

### Basic Selectors

- `.` - Identity (returns the input unchanged)
- `(get :key)` - Get value by key from map
- `(get 0)` - Get value by index from vector
- `(get-in [:a :b])` - Navigate nested structures
- `(:key data)` - Keyword as function (shorthand for `(get :key data)`)

### Collection Operations

- `(first coll)` - Get first element of collection
- `(last coll)` - Get last element of collection
- `(rest coll)` - Get all but first element of collection
- `(take n coll)` - Take first n elements of collection
- `(drop n coll)` - Drop first n elements of collection
- `(nth n coll)` - Get nth element of collection (0-indexed)
- `(count coll)` - Get count of collection
- `(keys map)` - Get keys of map
- `(vals map)` - Get values of map

### Filtering and Mapping

- `(filter pred)` - Filter collection by predicate
- `(map f)` - Map function over collection
- `(remove pred)` - Remove elements matching predicate
- `(select-keys [:k1 :k2])` - Select only specified keys from map

### Predicates

- `(nil? value)` - Test if value is nil
- `(empty? coll)` - Test if collection is empty
- `(contains? key map)` - Test if map contains key
- `(number?)`, `(string?)`, `(keyword?)`, `(boolean?)` - Type predicates
- `(=)`, `(<)`, `(>)`, `(<=)`, `(>=)` - Comparison operators

### Composition

- `(->)` - Thread-first macro for chaining operations
- `(->>)` - Thread-last macro for chaining operations
- `(comp f g)` - Function composition

### Conditionals

- `(if test then else)` - Conditional expression
- `(when test expr)` - Conditional with implicit nil else
- `(cond)` - Multi-branch conditional

### Aggregation

- `(reduce f init)` - Reduce collection with function
- `(apply f)` - Apply function to collection as arguments
- `(group-by f)` - Group collection by function result
- `(frequencies)` - Count frequencies of elements

## Command Line Options

### Input/Output Format
- `-c, --compact-output` - Compact instead of pretty-printed output
- `-r, --raw-output` - Output raw strings, not EDN strings
- `-R, --raw-input` - Each line of input is a string, not parsed as EDN
- `-s, --slurp` - Read entire input stream into array
- `-n, --null-input` - Don't read input; filter gets nil input

### Error Handling
- `-e, --exit-status` - Set exit status based on output
- `-f, --from-file file` - Read filter from file
- `--tab` - Use tabs for indentation
- `--indent n` - Use n spaces for indentation (default: 2)

### Debugging
- `--debug` - Show debug information
- `--verbose` - Verbose output

## Examples

### Basic Selection
```bash
# Get value by key
echo '{:name "Alice" :age 30}' | eq '(:name)'
# Output: "Alice"

# Navigate nested structure
echo '{:user {:profile {:name "Bob"}}}' | eq '(get-in [:user :profile :name])'
# Output: "Bob"
```

### Array Operations
```bash
# Get first element
echo '[1 2 3 4 5]' | eq '(first)'
# Output: 1

# Filter even numbers
echo '[1 2 3 4 5 6]' | eq '(filter even?)'
# Output: [2 4 6]

# Map increment over array
echo '[1 2 3]' | eq '(map inc)'
# Output: [2 3 4]
```

### Complex Queries
```bash
# Chain operations with thread-first
echo '[{:name "Alice" :age 25} {:name "Bob" :age 30}]' | eq '(->> (filter #(> (:age %) 26)) (map :name))'
# Output: ["Bob"]

# Group by property
echo '[{:type :cat :name "Fluffy"} {:type :dog :name "Rex"} {:type :cat :name "Whiskers"}]' | eq '(group-by :type)'
# Output: {:cat [{:type :cat :name "Fluffy"} {:type :cat :name "Whiskers"}] :dog [{:type :dog :name "Rex"}]}
```

### File Processing
```bash
# Process multiple files
eq '(map :id)' data1.edn data2.edn

# Read filter from file
eq -f query.eq data.edn
```

## Error Handling

- Invalid EDN input results in parse error with line/column information
- Invalid filter expressions result in compilation errors
- Runtime errors (e.g., accessing non-existent keys) can be handled gracefully or cause failure based on options
- Exit codes: 0 for success, 1 for error, 5 for null output (with -e flag)

## Implementation Notes

### Core Features
- Parse EDN using a robust EDN parser
- Compile filter expressions to executable functions
- Support streaming for large files where possible
- Preserve EDN formatting and comments when feasible

### Data Types
Support all EDN data types:
- nil, booleans, strings, characters
- integers, floats, decimals
- keywords, symbols
- lists, vectors, maps, sets
- tagged literals
- comments (preserved in output when possible)

### Performance
- Lazy evaluation where possible
- Streaming for large datasets
- Memory-efficient processing

## Future Extensions

- Custom tagged literal handlers
- Plugin system for user-defined functions
- SQL-like query syntax as alternative to Clojure syntax
- Integration with Clojure namespaces and libraries
- REPL mode for interactive exploration
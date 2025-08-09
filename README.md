# eq - EDN Query Tool

`eq` is a command-line tool for processing EDN (Extensible Data Notation) files, similar to how `jq` processes JSON. It uses Clojure-inspired syntax to query, filter, and transform EDN data.

## Usage

```
Command-line EDN processor

Usage: eq [OPTIONS] <FILTER> [FILES]...

Arguments:
  <FILTER>    Filter expression to apply
  [FILES]...  Input files (reads from stdin if none provided)

Options:
  -c, --compact                 Compact instead of pretty-printed output
      --raw-output              Output raw strings, not EDN strings
  -R, --raw-input               Each line of input is a string, not parsed as EDN
  -s, --slurp                   Read entire input stream into array
  -n, --null-input              Don't read input; filter gets nil input
  -e, --exit-status             Set exit status based on output
  -f, --from-file <FILE>        Read filter from file
      --tab                     Use tabs for indentation
      --indent <N>              Use n spaces for indentation [default: 2]
      --debug                   Show debug information
  -v, --verbose                 Verbose output
  -H, --with-filename           Print filename for each output line (like grep -H)
  -r, --recursive               Recursively search directories for files
  -p, --pattern <GLOB_PATTERN>  Glob pattern for file matching (default: "*.edn")
  -h, --help                    Print help
  -V, --version                 Print version
```

## Installation

### From Source
```bash
git clone <repository-url>
cd eq
cargo build --release
```

The binary will be available at `target/release/eq`.

### Quick Test
```bash
echo '{:name "Alice" :age 30}' | ./target/release/eq '(:name .)'
# Output: "Alice"
```

## Basic Usage

```bash
eq '<filter>' [file...]
```

If no file is provided, `eq` reads from stdin.

## Important: Function Syntax

All functions in `eq` require explicit arguments. For example:
- Use `(first .)` instead of `(first)`  
- Use `(:name .)` instead of `:name`
- Use `(count .)` instead of `(count)`

The dot (`.`) represents the current input value being processed. This design enables consistent behavior with threading macros like `->` and `->>`.

## Getting Started

### Simple Data Extraction

**Extract a field from a map:**
```bash
# Input: {:name "Alice" :age 30 :city "New York"}
eq '(:name .)' data.edn
# Output: "Alice"

eq '(:age .)' data.edn  
# Output: 30
```

**Get the whole structure (identity):**
```bash
eq '.' data.edn
# Output: {:name "Alice" :age 30 :city "New York"}
```

### Working with Collections

**Arrays/vectors:**
```bash
# Input: [1 2 3 4 5]
eq '(first .)' numbers.edn
# Output: 1

eq '(last .)' numbers.edn
# Output: 5

eq '(count .)' numbers.edn
# Output: 5
```

**Take and drop elements:**
```bash
# Input: [1 2 3 4 5]
eq '(take 3 .)' numbers.edn
# Output: [1 2 3]

eq '(drop 2 .)' numbers.edn
# Output: [3 4 5]
```

### Nested Data Navigation

**Access nested fields:**
```bash
# Input: {:user {:profile {:name "Alice" :email "alice@example.com"}}}
eq '(get-in . [:user :profile :name])' user.edn
# Output: "Alice"
```

**Chain operations with threading:**
```bash
# Input: [{:name "Alice" :scores [85 92 78]} {:name "Bob" :scores [91 87 93]}]
eq '(-> . (first) :name)' students.edn
# Output: "Alice"

eq '(-> . (first) :scores (first))' students.edn  
# Output: 85
```

## Real-World Examples

### Example 1: Processing User Data

**Input file (users.edn):**
```clojure
[{:name "Alice" :age 30 :department "Engineering" :skills [:clojure :rust :python]}
 {:name "Bob" :age 25 :department "Design" :skills [:figma :sketch]}
 {:name "Charlie" :age 35 :department "Engineering" :skills [:java :clojure :go]}]
```

**Get the first user:**
```bash
eq '(first .)' users.edn
# Output: {:name "Alice" :age 30 :department "Engineering" :skills [:clojure :rust :python]}
```

**Get first user's name:**
```bash
eq '(-> . (first) (:name))' users.edn
# Output: "Alice"
```

**Count total users:**
```bash
eq '(count .)' users.edn
# Output: 3
```

### Example 2: Configuration File Processing

**Input file (config.edn):**
```clojure
{:database {:host "localhost" :port 5432 :name "myapp"}
 :server {:port 8080 :threads 10}
 :logging {:level :info :file "app.log"}}
```

**Get database configuration:**
```bash
eq '(:database .)' config.edn
# Output: {:host "localhost" :port 5432 :name "myapp"}
```

**Get just the database port:**
```bash
eq '(get-in . [:database :port])' config.edn
# Output: 5432
```

**Get all top-level keys:**
```bash
eq '(keys .)' config.edn
# Output: [:database :server :logging]
```

### Example 3: Data Analysis

**Input file (sales.edn):**
```clojure
[{:product "Widget A" :quantity 10 :price 25.99}
 {:product "Widget B" :quantity 5 :price 45.50}
 {:product "Widget C" :quantity 8 :price 15.75}]
```

**Get first product's details:**
```bash
eq '(-> . (first) (:product))' sales.edn
# Output: "Widget A"
```

**Check data structure:**
```bash
eq '(-> . (first) (keys))' sales.edn
# Output: [:product :quantity :price]
```

## Command Line Options

### Output Formatting

**Compact output (no pretty printing):**
```bash
eq -c '.' config.edn
# Output: {:database {:host "localhost" :port 5432} :server {:port 8080}}
```

**Raw string output (remove quotes):**
```bash
eq --raw-output '(:name .)' user.edn
# Output: Alice (instead of "Alice")
```

**Custom indentation:**
```bash
eq --indent 4 '.' config.edn  # Use 4 spaces
eq --tab '.' config.edn       # Use tabs
```

### Directory and File Processing

**Process all EDN files in a directory:**
```bash
eq '(:name .)' directory/
# Processes all *.edn files in directory/
```

**Recursively process files in subdirectories:**
```bash
eq -r '(:name .)' project/
# Recursively finds and processes all *.edn files
```

**Use custom file patterns:**
```bash
eq -p '*.json' '.' data/
# Process all JSON files instead of EDN files

eq -r -p '*.config.edn' '.' project/
# Recursively find all files matching *.config.edn
```

**Show filenames with output:**
```bash
eq -H '(:name .)' *.edn
# Output: file1.edn:"Alice"
#         file2.edn:"Bob"

eq -r -H '(:status .)' logs/
# Shows filename with each result when processing multiple files
```

### Input Modes

**Process each line as a string:**
```bash
echo -e "hello\nworld" | eq -R '.'
# Output: 
# "hello"
# "world"
```

**Slurp all input into an array:**
```bash
# Input: multiple EDN values on separate lines
# {:name "Alice"}
# {:name "Bob"}
eq -s '(count .)' multi-users.edn
# Output: 2
```

**Null input (useful for generating data):**
```bash
eq -n '(if (nil? .) :is-null :not-null)'
# Output: :is-null
```

### Reading Filters from Files

**Save complex queries in files:**
```bash
echo '(-> . (first) (:name))' > get-first-name.eq
eq -f get-first-name.eq users.edn
# Output: "Alice"
```

## Lambda Functions and Higher-Order Operations

`eq` supports lambda functions (anonymous functions) for powerful data transformation and filtering operations, similar to Clojure and functional programming languages.

### Lambda Syntax

**Explicit lambda syntax:**
```bash
# (fn [parameters] body)
(fn [x] (> x 10))      # Function that checks if x is greater than 10
(fn [x y] (+ x y))     # Function that adds two numbers
```

**Anonymous function syntax (shorthand):**
```bash
# #(body-with-% parameters)  
#(> % 10)              # Same as (fn [%] (> % 10))
#(< 3 %)               # Same as (fn [%] (< 3 %))
```

The `%` symbol represents the implicit parameter in anonymous functions.

### Higher-Order Functions

**map - Transform each element in a collection:**
```bash
# Input: [1 2 3 4 5]
eq '(map #(> % 3) .)' numbers.edn
# Output: [false false false true true]

# Transform numbers to strings
eq '(map #(str %) .)' numbers.edn  # Note: str function would need to be implemented
```

**select - Filter elements that satisfy a predicate (keep matching):**
```bash
# Input: [1 2 3 4 5]  
eq '(select #(> % 3) .)' numbers.edn
# Output: [4 5]

# Input: ["hello" "world" 42 true "test"]
eq '(select #(string? %) .)' mixed.edn  
# Output: ["hello" "world" "test"]
```

**remove - Filter out elements that satisfy a predicate (remove matching):**
```bash
# Input: [1 2 3 4 5]
eq '(remove #(> % 3) .)' numbers.edn  
# Output: [1 2 3]

# Remove nil values
# Input: [1 nil 3 nil 5]
eq '(remove #(nil? %) .)' sparse.edn
# Output: [1 3 5]
```

### Real-World Lambda Examples

**Example 1: Data Validation**

**Input file (users.edn):**
```clojure  
[{:name "Alice" :age 30 :active true}
 {:name "Bob" :age 17 :active true}  
 {:name "Charlie" :age 25 :active false}]
```

**Find active adult users:**
```bash
eq '(select #(and (:active %) (>= (:age %) 18)) .)' users.edn
# Output: [{:name "Alice" :age 30 :active true}]
```

**Find users who need activation:**  
```bash
eq '(select #(not (:active %)) .)' users.edn
# Output: [{:name "Charlie" :age 25 :active false}]
```

**Example 2: Number Processing**

**Input file (scores.edn):**
```clojure
[85 92 78 96 73 88 91]
```

**Find passing scores (>= 80):**
```bash
eq '(select #(>= % 80) .)' scores.edn  
# Output: [85 92 96 88 91]
```

**Count failing scores:**
```bash
eq '(-> . (remove #(>= % 80)) (count))' scores.edn
# Output: 2
```

**Convert to pass/fail:**
```bash  
eq '(map #(if (>= % 80) :pass :fail) .)' scores.edn
# Output: [:pass :pass :fail :pass :fail :pass :pass]
```

**Example 3: Configuration Filtering**

**Input file (services.edn):**
```clojure
[{:name "web" :port 8080 :enabled true}
 {:name "db" :port 5432 :enabled false}  
 {:name "cache" :port 6379 :enabled true}]
```

**Get enabled services:**
```bash
eq '(select #(:enabled %) .)' services.edn
# Output: [{:name "web" :port 8080 :enabled true} 
#          {:name "cache" :port 6379 :enabled true}]
```

**Get service names for enabled services:**
```bash
eq '(-> . (select #(:enabled %)) (map #(:name %)))' services.edn  
# Output: ["web" "cache"]
```

### Combining Lambda Functions

**Chain multiple operations:**
```bash
# Input: [1 2 3 4 5 6 7 8 9 10]
# Get even numbers, then square them, then filter > 20
eq '(-> . 
        (select #(= (mod % 2) 0))  
        (map #(* % %))
        (select #(> % 20)))' numbers.edn
# Output: [36 64 100]  
```

**Using with other functions:**
```bash
# Input: [1 2 3 2 1 4 2]  
eq '(-> . (remove #(= % 2)) (frequencies))' numbers.edn
# Output: {1 2, 3 1, 4 1}
```

### Supported Predicates in Lambdas

Lambda functions can use any of the built-in predicates and comparison operators:

**Type predicates:**
- `nil?` - Check if value is nil
- `number?` - Check if value is a number  
- `string?` - Check if value is a string
- `boolean?` - Check if value is a boolean
- `keyword?` - Check if value is a keyword
- `empty?` - Check if collection is empty

**Comparison operators:**
- `=` - Equal to
- `<` - Less than  
- `>` - Greater than
- `<=` - Less than or equal
- `>=` - Greater than or equal

**Example using multiple predicates:**
```bash
# Input: [1 "hello" nil 42 "" true]
eq '(select #(and (number? %) (> % 10)) .)' mixed.edn
# Output: [42]
```

### Tips for Using Lambdas

1. **Use explicit syntax for complex functions:**
   ```bash
   # Complex logic is clearer with explicit parameters
   (fn [user] (and (:active user) (>= (:age user) 21)))
   ```

2. **Use anonymous syntax for simple predicates:**
   ```bash
   # Simple checks work well with shorthand  
   #(> % 100)
   #(string? %)
   ```

3. **Combine with threading macros:**
   ```bash
   # Chain operations for readability
   (-> . (select #(active? %)) (map #(:name %)) (take 5))
   ```

4. **Remember parameter binding:**
   - In `#(...)` syntax, `%` is the implicit parameter
   - In `(fn [...] ...)` syntax, use explicit parameter names

## Advanced Query Language

### Type Checking
```bash
# Check if value is nil
eq '(nil? .)' data.edn

# Check if value is a number  
eq '(number? .)' data.edn

# Check if collection is empty
eq '(empty? .)' data.edn
```

### Comparisons
```bash
# Input: {:age 30}
eq '(-> . :age (> 25))' user.edn
# Output: true

eq '(-> . :age (= 30))' user.edn  
# Output: true
```

### Conditional Logic
```bash
# Input: {:status "active"}
eq '(if (= :status "active") :online :offline)' user.edn
# Output: :online
```

### Data Frequency Analysis
```bash
# Input: [:red :blue :red :green :blue :red]
eq '(frequencies .)' colors.edn
# Output: {:red 3 :blue 2 :green 1}
```

## Working with Different Input Sources

### From Files
```bash
eq '(:name .)' user.edn
eq '(count .)' collection.edn
```

### From Stdin (Pipes)
```bash
curl -s api.example.com/users.edn | eq '(-> . (first) (:name))'
cat data.edn | eq '(take 5 .)'
```

### Multiple Files
```bash
eq '(:timestamp .)' log1.edn log2.edn log3.edn
```

## Error Handling

`eq` provides helpful error messages:

```bash
# Invalid query syntax
eq '(invalid-function)' data.edn
# Error: Unknown function: invalid-function

# File not found
eq '.' nonexistent.edn  
# Error: No such file or directory

# Invalid EDN syntax in input
echo '{:invalid edn' | eq '.'
# Error: Parse error at line 1, column 13: Unterminated map
```

## Performance Tips

1. **Use compact output (-c) for large datasets** when you don't need pretty printing
2. **Prefer specific field access** (`:field`) over generic get operations when possible
3. **Use threading macros** (`->`, `->>`) for readable query chains
4. **Store complex queries in files** (-f) for reuse

## Common Patterns

### Data Validation
```bash
# Check if all required fields exist
eq '(-> . (contains? :name))' user.edn
eq '(-> . :email (nil?) not)' user.edn
```

### Data Extraction Pipelines
```bash
# Extract and transform data
eq '(-> . :users (first) :profile :settings (keys))' app-state.edn
```

### Configuration Management
```bash
# Extract environment-specific config
eq '(get-in . [:environments :production :database])' config.edn
```

### Log Analysis
```bash
# Get error entries (assuming log structure)
eq '(-> . :entries (filter #(= (:level %) :error)))' log.edn
```

## Comparison with jq

| Operation | jq | eq |
|-----------|----|----|
| Identity | `.` | `.` |
| Field access | `.name` | `(:name .)` |
| Array first | `.[0]` or `first` | `(first .)` |
| Array length | `length` | `(count .)` |
| Map keys | `keys` | `(keys .)` |
| Nested access | `.user.profile.name` | `(get-in . [:user :profile :name])` |
| Chaining | `.user \| .name` | `(-> . (:user) (:name))` |
| Map/transform | `map(. + 1)` | `(map #(+ % 1) .)` |
| Filter/select | `map(select(. > 3))` | `(select #(> % 3) .)` |
| Remove/reject | `map(select(. <= 3))` | `(remove #(> % 3) .)` |

## Contributing

Found a bug or want to add a feature? Contributions welcome!

## License

Proprietary software. All rights reserved. See LICENSE file for details.
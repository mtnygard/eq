use indexmap::IndexMap;
use std::collections::HashSet;
use std::fmt;
use std::hash::{Hash, Hasher};
// Remove dependency on CompiledQuery since we're not using it anymore

/// Trait for sequential collection operations like first, last, rest, take, drop
pub trait EdnSequential {
    /// Get the first element of a sequential collection
    fn first(&self) -> Option<&EdnValue>;
    
    /// Get the last element of a sequential collection  
    fn last(&self) -> Option<&EdnValue>;
    
    /// Get all elements except the first (returns Vector for consistency)
    fn rest(&self) -> EdnValue;
    
    /// Take the first n elements (returns Vector for consistency)
    fn take(&self, n: usize) -> EdnValue;
    
    /// Drop the first n elements (returns Vector for consistency) 
    fn drop(&self, n: usize) -> EdnValue;
    
    /// Helper to get collection as slice for implementations
    fn as_slice(&self) -> &[EdnValue];
}

/// Trait for iterating over collection values
pub trait EdnIterable {
    /// Get an iterator over the values in this collection
    fn iter_values(&self) -> Box<dyn Iterator<Item = &EdnValue> + '_>;
}

/// Trait for associative collections that support key/index-based access
pub trait EdnAssociative {
    /// Get value by key (for maps) or index (for sequences)
    fn get(&self, key: &EdnValue) -> Option<&EdnValue>;
    
    /// Check if key/index exists (default implementation uses get)
    fn contains_key(&self, key: &EdnValue) -> bool {
        self.get(key).is_some()
    }
}

/// EDN value types with zero-copy string optimization
#[derive(Debug, Clone, PartialEq)]
pub enum EdnValue {
    Nil,
    Bool(bool),
    String(String),
    Keyword(String),
    Symbol(String),
    Character(char),
    Integer(i64),
    Float(f64),
    Vector(Vec<EdnValue>),
    List(Vec<EdnValue>),
    Map(IndexMap<EdnValue, EdnValue>),
    Set(HashSet<EdnValue>),
    Tagged {
        tag: String,
        value: Box<EdnValue>,
    },
    WithMetadata {
        metadata: Box<EdnValue>,
        value: Box<EdnValue>,
    },
    Instant(String), // ISO 8601 timestamp string
    Uuid(String),    // UUID string
}

impl EdnValue {
    /// Returns the type name of this value
    pub fn type_name(&self) -> &'static str {
        match self {
            EdnValue::Nil => "nil",
            EdnValue::Bool(_) => "boolean",
            EdnValue::String(_) => "string",
            EdnValue::Keyword(_) => "keyword",
            EdnValue::Symbol(_) => "symbol",
            EdnValue::Character(_) => "character",
            EdnValue::Integer(_) => "integer",
            EdnValue::Float(_) => "float",
            EdnValue::Vector(_) => "vector",
            EdnValue::List(_) => "list",
            EdnValue::Map(_) => "map",
            EdnValue::Set(_) => "set",
            EdnValue::Tagged { .. } => "tagged",
            EdnValue::WithMetadata { .. } => "with-metadata",
            EdnValue::Instant(_) => "instant",
            EdnValue::Uuid(_) => "uuid",
        }
    }
    
    /// Check if this value is truthy (everything except nil and false)
    pub fn is_truthy(&self) -> bool {
        !matches!(self, EdnValue::Nil | EdnValue::Bool(false))
    }
    
    /// Get the count of elements in a collection
    pub fn count(&self) -> Option<usize> {
        match self {
            EdnValue::Vector(v) => Some(v.len()),
            EdnValue::List(l) => Some(l.len()),
            EdnValue::Map(m) => Some(m.len()),
            EdnValue::Set(s) => Some(s.len()),
            EdnValue::String(s) => Some(s.chars().count()),
            EdnValue::WithMetadata { value, .. } => value.count(),
            _ => None,
        }
    }
    
    /// Get nested value using a path of keys
    pub fn get_in<I>(&self, path: I) -> Option<&EdnValue>
    where
        I: IntoIterator<Item = EdnValue>,
    {
        let mut current = Some(self);
        for key in path {
            current = current.and_then(|v| v.get(&key));
        }
        current
    }
}

impl EdnSequential for EdnValue {
    fn first(&self) -> Option<&EdnValue> {
        match self {
            EdnValue::Vector(v) => v.first(),
            EdnValue::List(l) => l.first(),
            EdnValue::WithMetadata { value, .. } => value.first(),
            _ => None,
        }
    }
    
    fn last(&self) -> Option<&EdnValue> {
        match self {
            EdnValue::Vector(v) => v.last(),
            EdnValue::List(l) => l.last(),
            EdnValue::WithMetadata { value, .. } => value.last(),
            _ => None,
        }
    }
    
    fn rest(&self) -> EdnValue {
        let slice = self.as_slice();
        if slice.is_empty() {
            EdnValue::Vector(Vec::new())
        } else {
            EdnValue::Vector(slice[1..].to_vec())
        }
    }
    
    fn take(&self, n: usize) -> EdnValue {
        let slice = self.as_slice();
        EdnValue::Vector(slice.iter().take(n).cloned().collect())
    }
    
    fn drop(&self, n: usize) -> EdnValue {
        let slice = self.as_slice();
        EdnValue::Vector(slice.iter().skip(n).cloned().collect())
    }
    
    fn as_slice(&self) -> &[EdnValue] {
        match self {
            EdnValue::Vector(v) => v,
            EdnValue::List(l) => l,
            EdnValue::WithMetadata { value, .. } => value.as_slice(),
            _ => &[],
        }
    }
}

impl EdnIterable for EdnValue {
    fn iter_values(&self) -> Box<dyn Iterator<Item = &EdnValue> + '_> {
        match self {
            EdnValue::Vector(v) => Box::new(v.iter()),
            EdnValue::List(l) => Box::new(l.iter()),
            EdnValue::Map(m) => Box::new(m.values()),
            EdnValue::Set(s) => Box::new(s.iter()),
            EdnValue::WithMetadata { value, .. } => value.iter_values(),
            _ => Box::new(std::iter::empty()),
        }
    }
}

impl EdnAssociative for EdnValue {
    fn get(&self, key: &EdnValue) -> Option<&EdnValue> {
        match (self, key) {
            (EdnValue::Map(m), k) => m.get(k),
            (EdnValue::Vector(v), EdnValue::Integer(i)) => {
                if *i >= 0 {
                    v.get(*i as usize)
                } else {
                    // Negative indexing from end
                    let len = v.len() as i64;
                    v.get((len + i) as usize)
                }
            }
            (EdnValue::List(l), EdnValue::Integer(i)) => {
                if *i >= 0 {
                    l.get(*i as usize)
                } else {
                    let len = l.len() as i64;
                    l.get((len + i) as usize)
                }
            }
            (EdnValue::WithMetadata { value, .. }, k) => value.get(k),
            _ => None,
        }
    }
}

// Implement Eq for EdnValue (required for HashMap keys)
impl Eq for EdnValue {}

// Custom Hash implementation to handle floating point values
impl Hash for EdnValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            EdnValue::Nil => 0.hash(state),
            EdnValue::Bool(b) => b.hash(state),
            EdnValue::String(s) => s.hash(state),
            EdnValue::Keyword(k) => k.hash(state),
            EdnValue::Symbol(s) => s.hash(state),
            EdnValue::Character(c) => c.hash(state),
            EdnValue::Integer(i) => i.hash(state),
            EdnValue::Float(f) => {
                // Handle NaN and convert to bits for consistent hashing
                if f.is_nan() {
                    "NaN".hash(state);
                } else {
                    f.to_bits().hash(state);
                }
            }
            EdnValue::Vector(v) => v.hash(state),
            EdnValue::List(l) => l.hash(state),
            EdnValue::Map(m) => {
                for (k, v) in m {
                    k.hash(state);
                    v.hash(state);
                }
            }
            EdnValue::Set(s) => {
                let mut items: Vec<_> = s.iter().collect();
                items.sort_by_key(|v| format!("{:?}", v)); // Deterministic ordering
                items.hash(state);
            }
            EdnValue::Tagged { tag, value } => {
                tag.hash(state);
                value.hash(state);
            }
            EdnValue::WithMetadata { metadata, value } => {
                metadata.hash(state);
                value.hash(state);
            }
            EdnValue::Instant(s) => s.hash(state),
            EdnValue::Uuid(s) => s.hash(state),
        }
    }
}

impl fmt::Display for EdnValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EdnValue::Nil => write!(f, "nil"),
            EdnValue::Bool(b) => write!(f, "{}", b),
            EdnValue::String(s) => write!(f, "\"{}\"", escape_string(s)),
            EdnValue::Keyword(k) => write!(f, ":{}", k),
            EdnValue::Symbol(s) => write!(f, "{}", s),
            EdnValue::Character(c) => write!(f, "\\{}", c),
            EdnValue::Integer(i) => write!(f, "{}", i),
            EdnValue::Float(fl) => write!(f, "{}", fl),
            EdnValue::Vector(v) => {
                write!(f, "[")?;
                for (i, item) in v.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            }
            EdnValue::List(l) => {
                write!(f, "(")?;
                for (i, item) in l.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, ")")
            }
            EdnValue::Map(m) => {
                write!(f, "{{")?;
                for (i, (k, v)) in m.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{} {}", k, v)?;
                }
                write!(f, "}}")
            }
            EdnValue::Set(s) => {
                write!(f, "#{{")?;
                let mut items: Vec<_> = s.iter().collect();
                items.sort_by_key(|v| format!("{}", v));
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, "}}")
            }
            EdnValue::Tagged { tag, value } => write!(f, "#{} {}", tag, value),
            EdnValue::WithMetadata { metadata, value } => write!(f, "^{} {}", metadata, value),
            EdnValue::Instant(s) => write!(f, "#inst \"{}\"", s),
            EdnValue::Uuid(s) => write!(f, "#uuid \"{}\"", s),
        }
    }
}

fn escape_string(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '"' => "\\\"".to_string(),
            '\\' => "\\\\".to_string(),
            '\n' => "\\n".to_string(),
            '\r' => "\\r".to_string(),
            '\t' => "\\t".to_string(),
            c => c.to_string(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_names() {
        assert_eq!(EdnValue::Nil.type_name(), "nil");
        assert_eq!(EdnValue::Bool(true).type_name(), "boolean");
        assert_eq!(EdnValue::String("test".to_string()).type_name(), "string");
        assert_eq!(EdnValue::Keyword("key".to_string()).type_name(), "keyword");
        assert_eq!(EdnValue::Integer(42).type_name(), "integer");
    }

    #[test]
    fn test_truthy() {
        assert!(!EdnValue::Nil.is_truthy());
        assert!(!EdnValue::Bool(false).is_truthy());
        assert!(EdnValue::Bool(true).is_truthy());
        assert!(EdnValue::Integer(0).is_truthy());
        assert!(EdnValue::String("".to_string()).is_truthy());
    }

    #[test]
    fn test_count() {
        let vec = EdnValue::Vector(vec![
            EdnValue::Integer(1),
            EdnValue::Integer(2),
            EdnValue::Integer(3)
        ]);
        assert_eq!(vec.count(), Some(3));

        let mut map = IndexMap::new();
        map.insert(EdnValue::Keyword("a".to_string()), EdnValue::Integer(1));
        let map_val = EdnValue::Map(map);
        assert_eq!(map_val.count(), Some(1));

        let string_val = EdnValue::String("hello".to_string());
        assert_eq!(string_val.count(), Some(5));

        assert_eq!(EdnValue::Integer(42).count(), None);
    }

    #[test]
    fn test_get() {
        // Vector access
        let vec = EdnValue::Vector(vec![
            EdnValue::Integer(10),
            EdnValue::Integer(20),
            EdnValue::Integer(30)
        ]);
        assert_eq!(vec.get(&EdnValue::Integer(0)), Some(&EdnValue::Integer(10)));
        assert_eq!(vec.get(&EdnValue::Integer(2)), Some(&EdnValue::Integer(30)));
        assert_eq!(vec.get(&EdnValue::Integer(-1)), Some(&EdnValue::Integer(30)));
        assert_eq!(vec.get(&EdnValue::Integer(5)), None);

        // Map access
        let mut map = IndexMap::new();
        map.insert(EdnValue::Keyword("name".to_string()), EdnValue::String("Alice".to_string()));
        let map_val = EdnValue::Map(map);
        assert_eq!(
            map_val.get(&EdnValue::Keyword("name".to_string())),
            Some(&EdnValue::String("Alice".to_string()))
        );
        assert_eq!(map_val.get(&EdnValue::Keyword("age".to_string())), None);
    }

    #[test]
    fn test_get_in() {
        let mut inner_map = IndexMap::new();
        inner_map.insert(EdnValue::Keyword("name".to_string()), EdnValue::String("Bob".to_string()));
        
        let mut outer_map = IndexMap::new();
        outer_map.insert(EdnValue::Keyword("user".to_string()), EdnValue::Map(inner_map));
        
        let root = EdnValue::Map(outer_map);
        
        let path = vec![
            EdnValue::Keyword("user".to_string()),
            EdnValue::Keyword("name".to_string())
        ];
        
        assert_eq!(root.get_in(path), Some(&EdnValue::String("Bob".to_string())));
    }

    #[test]
    fn test_get_in_mixed_keys() {
        // Create a structure with mixed keyword and integer access:
        // {:skills [:clojure :rust :javascript] :profile {:scores [85 92 78]}}
        let skills_vector = EdnValue::Vector(vec![
            EdnValue::Keyword("clojure".to_string()),
            EdnValue::Keyword("rust".to_string()),
            EdnValue::Keyword("javascript".to_string()),
        ]);
        
        let scores_vector = EdnValue::Vector(vec![
            EdnValue::Integer(85),
            EdnValue::Integer(92),
            EdnValue::Integer(78),
        ]);
        
        let mut profile_map = IndexMap::new();
        profile_map.insert(EdnValue::Keyword("scores".to_string()), scores_vector);
        
        let mut root_map = IndexMap::new();
        root_map.insert(EdnValue::Keyword("skills".to_string()), skills_vector);
        root_map.insert(EdnValue::Keyword("profile".to_string()), EdnValue::Map(profile_map));
        
        let root = EdnValue::Map(root_map);
        
        // Test mixed path: keyword -> integer (accessing second skill)
        let path1 = vec![
            EdnValue::Keyword("skills".to_string()),
            EdnValue::Integer(1)
        ];
        assert_eq!(root.get_in(path1), Some(&EdnValue::Keyword("rust".to_string())));
        
        // Test mixed path: keyword -> keyword -> integer (accessing first score)
        let path2 = vec![
            EdnValue::Keyword("profile".to_string()),
            EdnValue::Keyword("scores".to_string()),
            EdnValue::Integer(0)
        ];
        assert_eq!(root.get_in(path2), Some(&EdnValue::Integer(85)));
        
        // Test mixed path: keyword -> keyword -> integer (accessing last score with negative index)
        let path3 = vec![
            EdnValue::Keyword("profile".to_string()),
            EdnValue::Keyword("scores".to_string()),
            EdnValue::Integer(-1)
        ];
        assert_eq!(root.get_in(path3), Some(&EdnValue::Integer(78)));
        
        // Test invalid path
        let invalid_path = vec![
            EdnValue::Keyword("skills".to_string()),
            EdnValue::Integer(10)
        ];
        assert_eq!(root.get_in(invalid_path), None);
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", EdnValue::Nil), "nil");
        assert_eq!(format!("{}", EdnValue::Bool(true)), "true");
        assert_eq!(format!("{}", EdnValue::String("hello".to_string())), "\"hello\"");
        assert_eq!(format!("{}", EdnValue::Keyword("key".to_string())), ":key");
        assert_eq!(format!("{}", EdnValue::Integer(42)), "42");
        
        let vec = EdnValue::Vector(vec![
            EdnValue::Integer(1),
            EdnValue::Integer(2)
        ]);
        assert_eq!(format!("{}", vec), "[1 2]");
    }

    #[test]
    fn test_hash_consistency() {
        use std::collections::HashMap;
        
        let mut map = HashMap::new();
        let key = EdnValue::String("test".to_string());
        map.insert(key.clone(), "value");
        
        assert_eq!(map.get(&key), Some(&"value"));
        
        // Test that NaN values can be used as keys (but equality is tricky with NaN)
        let nan_key1 = EdnValue::Float(f64::NAN);
        let nan_key2 = EdnValue::Float(f64::NAN);
        map.insert(nan_key1.clone(), "nan_value");
        
        // Both NaN keys should hash the same but they won't be equal due to NaN != NaN
        // This test verifies that NaN values can be used as keys without panicking
        let _ = map.get(&nan_key1);
        let _ = map.get(&nan_key2);
        
        // Test normal float values work
        let float_key = EdnValue::Float(3.14);
        map.insert(float_key.clone(), "pi");
        assert_eq!(map.get(&float_key), Some(&"pi"));
    }
}
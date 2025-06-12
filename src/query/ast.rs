use crate::edn::EdnValue;

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    // Basic selectors
    Identity,                               // .
    Get(EdnValue),                         // (get :key) or (get 0)
    GetIn(Vec<EdnValue>),                  // (get-in [:a :b])
    KeywordAccess(String),                 // :key (shorthand for get)

    // Collection operations
    First,                                 // (first)
    Last,                                  // (last)
    Rest,                                  // (rest)
    Take(Box<Expr>),                      // (take n)
    Drop(Box<Expr>),                      // (drop n)
    Nth(Box<Expr>),                       // (nth n)
    Count,                                // (count)
    Keys,                                 // (keys)
    Vals,                                 // (vals)

    // Filtering and mapping
    Filter(Box<Expr>),                    // (filter pred)
    Map(Box<Expr>),                       // (map f)
    Remove(Box<Expr>),                    // (remove pred)
    SelectKeys(Vec<EdnValue>),            // (select-keys [:k1 :k2])

    // Predicates
    IsNil,                                // (nil?)
    IsEmpty,                              // (empty?)
    Contains(Box<Expr>),                  // (contains? k)
    IsNumber,                             // (number?)
    IsString,                             // (string?)
    IsKeyword,                            // (keyword?)
    IsBoolean,                            // (boolean?)

    // Comparison
    Equal(Box<Expr>),                     // (= x)
    LessThan(Box<Expr>),                  // (< x)
    GreaterThan(Box<Expr>),               // (> x)
    LessEqual(Box<Expr>),                 // (<= x)
    GreaterEqual(Box<Expr>),              // (>= x)

    // Composition
    ThreadFirst(Vec<Expr>),               // (-> x f g h)
    ThreadLast(Vec<Expr>),                // (->> x f g h)
    Comp(Vec<Expr>),                      // (comp f g)

    // Conditionals
    If {                                  // (if test then else)
        test: Box<Expr>,
        then_expr: Box<Expr>,
        else_expr: Option<Box<Expr>>,
    },
    When {                                // (when test expr)
        test: Box<Expr>,
        expr: Box<Expr>,
    },

    // Aggregation
    Reduce {                              // (reduce f init)
        func: Box<Expr>,
        init: Option<Box<Expr>>,
    },
    Apply(Box<Expr>),                     // (apply f)
    GroupBy(Box<Expr>),                   // (group-by f)
    Frequencies,                          // (frequencies)

    // Literals
    Literal(EdnValue),                    // literal values
    
    // Function composition
    Lambda(Vec<String>, Box<Expr>),       // #(+ %1 %2) - anonymous functions
}

impl Expr {
    /// Create a literal expression
    pub fn literal(value: EdnValue) -> Self {
        Expr::Literal(value)
    }

    /// Create a get expression from a key
    pub fn get(key: EdnValue) -> Self {
        Expr::Get(key)
    }

    /// Create a get-in expression from a path
    pub fn get_in(path: Vec<EdnValue>) -> Self {
        Expr::GetIn(path)
    }

    /// Create a keyword access expression
    pub fn keyword(name: impl Into<String>) -> Self {
        Expr::KeywordAccess(name.into())
    }

    /// Create a filter expression
    pub fn filter(predicate: Expr) -> Self {
        Expr::Filter(Box::new(predicate))
    }

    /// Create a map expression
    pub fn map(func: Expr) -> Self {
        Expr::Map(Box::new(func))
    }

    /// Create a thread-first expression
    pub fn thread_first(exprs: Vec<Expr>) -> Self {
        Expr::ThreadFirst(exprs)
    }

    /// Create a thread-last expression
    pub fn thread_last(exprs: Vec<Expr>) -> Self {
        Expr::ThreadLast(exprs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expr_creation() {
        let identity = Expr::Identity;
        assert_eq!(identity, Expr::Identity);

        let get_expr = Expr::get(EdnValue::Keyword("name".to_string()));
        assert_eq!(get_expr, Expr::Get(EdnValue::Keyword("name".to_string())));

        let keyword_expr = Expr::keyword("age");
        assert_eq!(keyword_expr, Expr::KeywordAccess("age".to_string()));
    }

    #[test]
    fn test_complex_expressions() {
        let filter_expr = Expr::filter(Expr::IsNumber);
        assert_eq!(filter_expr, Expr::Filter(Box::new(Expr::IsNumber)));

        let map_expr = Expr::map(Expr::keyword("name"));
        assert_eq!(map_expr, Expr::Map(Box::new(Expr::KeywordAccess("name".to_string()))));
    }

    #[test]
    fn test_threading_expressions() {
        let thread_first = Expr::thread_first(vec![
            Expr::Identity,
            Expr::First,
            Expr::keyword("name")
        ]);
        
        match thread_first {
            Expr::ThreadFirst(exprs) => {
                assert_eq!(exprs.len(), 3);
                assert_eq!(exprs[0], Expr::Identity);
                assert_eq!(exprs[1], Expr::First);
                assert_eq!(exprs[2], Expr::KeywordAccess("name".to_string()));
            }
            _ => panic!("Expected ThreadFirst"),
        }
    }
}
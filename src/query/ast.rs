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
    Select(Box<Expr>),                    // (select pred) - returns input if pred is truthy, nil otherwise

    // Predicates
    IsNil,                                // (nil?)
    IsEmpty,                              // (empty?)
    Contains(Box<Expr>),                  // (contains? k)
    IsNumber,                             // (number?)
    IsString,                             // (string?)
    IsKeyword,                            // (keyword?)
    IsBoolean,                            // (boolean?)

    // Comparison
    Equal(Box<Expr>, Box<Expr>),          // (= left right)
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
    
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expr_creation() {
        let identity = Expr::Identity;
        assert_eq!(identity, Expr::Identity);

        let get_expr = Expr::Get(EdnValue::Keyword("name".to_string()));
        assert_eq!(get_expr, Expr::Get(EdnValue::Keyword("name".to_string())));

        let keyword_expr = Expr::KeywordAccess("age".to_string());
        assert_eq!(keyword_expr, Expr::KeywordAccess("age".to_string()));
    }

    #[test]
    fn test_complex_expressions() {
        let filter_expr = Expr::Filter(Box::new(Expr::IsNumber));
        assert_eq!(filter_expr, Expr::Filter(Box::new(Expr::IsNumber)));

        let map_expr = Expr::Map(Box::new(Expr::KeywordAccess("name".to_string())));
        assert_eq!(map_expr, Expr::Map(Box::new(Expr::KeywordAccess("name".to_string()))));
    }

    #[test]
    fn test_threading_expressions() {
        let thread_first = Expr::ThreadFirst(vec![
            Expr::Identity,
            Expr::First,
            Expr::KeywordAccess("name".to_string())
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
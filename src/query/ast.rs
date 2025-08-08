use crate::edn::EdnValue;
use std::collections::HashMap;
use std::sync::Arc;

/// Type alias for builtin function implementations
pub type BuiltinFn = Arc<dyn Fn(&[EdnValue], &EdnValue) -> crate::error::EqResult<EdnValue> + Send + Sync>;

/// Environment for symbol bindings during evaluation
#[derive(Debug, Clone)]
pub struct Environment {
    bindings: HashMap<String, EdnValue>,
}

impl Environment {
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }

    pub fn with_context(context: EdnValue) -> Self {
        let mut env = Self::new();
        env.bind(".".to_string(), context);
        env
    }

    pub fn bind(&mut self, name: String, value: EdnValue) {
        self.bindings.insert(name, value);
    }

    pub fn lookup(&self, name: &str) -> Option<&EdnValue> {
        self.bindings.get(name)
    }
}

/// Registry for builtin functions
#[derive(Clone)]
pub struct FunctionRegistry {
    functions: HashMap<String, BuiltinFn>,
}

impl std::fmt::Debug for FunctionRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FunctionRegistry")
            .field("functions", &self.functions.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl FunctionRegistry {
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
        }
    }

    pub fn register<F>(&mut self, name: String, func: F)
    where
        F: Fn(&[EdnValue], &EdnValue) -> crate::error::EqResult<EdnValue> + Send + Sync + 'static,
    {
        self.functions.insert(name, Arc::new(func));
    }

    pub fn get(&self, name: &str) -> Option<&BuiltinFn> {
        self.functions.get(name)
    }
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)] // Some variants are used by analyzer but not detected by compiler
pub enum Expr {
    // Basic selectors
    Symbol(String),                        // symbol lookup in environment
    Get(EdnValue),                         // (get :key) or (get 0)
    GetIn(Box<Expr>, Vec<EdnValue>),       // (get-in input [:a :b])
    KeywordAccess(String),                 // :key (shorthand for get)
    KeywordGet(String, Box<Expr>),         // (:key expr) - get key from result of expr
    KeywordGetWithDefault(String, Box<Expr>, Box<Expr>), // (:key expr default) - get key with default

    // General function call
    Function {
        name: String,
        args: Vec<Expr>,
    },

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

    // Raw parsed forms (before analysis)
    List(Vec<EdnValue>),                 // raw list from parser, needs analysis
    
    // Literals
    Literal(EdnValue),                    // literal values
    
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expr_creation() {
        let identity = Expr::Symbol(".".to_string());
        assert_eq!(identity, Expr::Symbol(".".to_string()));

        let get_expr = Expr::Get(EdnValue::Keyword("name".to_string()));
        assert_eq!(get_expr, Expr::Get(EdnValue::Keyword("name".to_string())));

        let keyword_expr = Expr::KeywordAccess("age".to_string());
        assert_eq!(keyword_expr, Expr::KeywordAccess("age".to_string()));
    }

    #[test]
    fn test_complex_expressions() {
        let _function_expr = Expr::Function {
            name: "select".to_string(),
            args: vec![Expr::Function {
                name: "number?".to_string(),
                args: vec![],
            }],
        };
        
        let keyword_expr = Expr::KeywordAccess("name".to_string());
        assert_eq!(keyword_expr, Expr::KeywordAccess("name".to_string()));
    }

    #[test]
    fn test_threading_expressions() {
        let thread_first = Expr::ThreadFirst(vec![
            Expr::Symbol(".".to_string()),
            Expr::Function {
                name: "first".to_string(),
                args: vec![],
            },
            Expr::KeywordAccess("name".to_string())
        ]);
        
        match thread_first {
            Expr::ThreadFirst(exprs) => {
                assert_eq!(exprs.len(), 3);
                assert_eq!(exprs[0], Expr::Symbol(".".to_string()));
                assert!(matches!(exprs[1], Expr::Function { .. }));
                assert_eq!(exprs[2], Expr::KeywordAccess("name".to_string()));
            }
            _ => panic!("Expected ThreadFirst"),
        }
    }
}
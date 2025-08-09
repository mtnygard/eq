use crate::edn::EdnValue;
use std::collections::HashMap;
use std::sync::Arc;

/// Type alias for builtin function implementations
pub type BuiltinFn = Arc<dyn Fn(&[EdnValue]) -> crate::error::EqResult<EdnValue> + Send + Sync>;

/// Type alias for special form implementations (take unevaluated expressions)
pub type SpecialFormFn = Arc<dyn Fn(&[Expr], &EdnValue, &Environment) -> crate::error::EqResult<EdnValue> + Send + Sync>;

/// Type alias for macro implementations (take unevaluated expressions, return new expression)
pub type MacroFn = Arc<dyn Fn(&[Expr]) -> crate::error::EqResult<Expr> + Send + Sync>;

/// Represents either a regular function, special form, or macro
#[derive(Clone)]
pub enum FunctionType {
    Regular(BuiltinFn),
    SpecialForm(SpecialFormFn),
    Macro(MacroFn),
}

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

/// Registry for builtin functions and special forms
#[derive(Clone)]
pub struct FunctionRegistry {
    functions: HashMap<String, FunctionType>,
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
        F: Fn(&[EdnValue]) -> crate::error::EqResult<EdnValue> + Send + Sync + 'static,
    {
        self.functions.insert(name, FunctionType::Regular(Arc::new(func)));
    }

    pub fn register_special_form<F>(&mut self, name: String, func: F)
    where
        F: Fn(&[Expr], &EdnValue, &Environment) -> crate::error::EqResult<EdnValue> + Send + Sync + 'static,
    {
        self.functions.insert(name, FunctionType::SpecialForm(Arc::new(func)));
    }

    pub fn register_macro<F>(&mut self, name: String, func: F)
    where
        F: Fn(&[Expr]) -> crate::error::EqResult<Expr> + Send + Sync + 'static,
    {
        self.functions.insert(name, FunctionType::Macro(Arc::new(func)));
    }

    pub fn get(&self, name: &str) -> Option<&FunctionType> {
        self.functions.get(name)
    }
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)] // Some variants are used by analyzer but not detected by compiler
pub enum Expr {
    // Basic selectors
    Symbol(String),                        // symbol lookup in environment
    KeywordAccess(String),                 // :key (shorthand for get)
    KeywordGet(String, Box<Expr>),         // (:key expr) - get key from result of expr
    KeywordGetWithDefault(String, Box<Expr>, Box<Expr>), // (:key expr default) - get key with default

    // General function call
    Function {
        name: String,
        args: Vec<Expr>,
    },

    // Lambda function call  
    LambdaCall {
        func: Box<Expr>,  // Expression that evaluates to a lambda
        args: Vec<Expr>,
    },

    // Composition
    Comp(Vec<Expr>),                      // (comp f g)

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

        let _get_expr = Expr::Function {
            name: "get".to_string(),
            args: vec![Expr::Literal(EdnValue::Keyword("name".to_string()))],
        };
        
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
    fn test_composition_expressions() {
        let comp_expr = Expr::Comp(vec![
            Expr::Symbol(".".to_string()),
            Expr::Function {
                name: "first".to_string(),
                args: vec![],
            },
            Expr::KeywordAccess("name".to_string())
        ]);
        
        match comp_expr {
            Expr::Comp(exprs) => {
                assert_eq!(exprs.len(), 3);
                assert_eq!(exprs[0], Expr::Symbol(".".to_string()));
                assert!(matches!(exprs[1], Expr::Function { .. }));
                assert_eq!(exprs[2], Expr::KeywordAccess("name".to_string()));
            }
            _ => panic!("Expected Comp"),
        }
    }
}
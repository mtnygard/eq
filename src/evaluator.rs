use crate::edn::EdnValue;
use crate::error::{EqError, EqResult};
use crate::query::ast::{Expr, FunctionRegistry, Environment};
use crate::builtins::create_builtin_registry;

use std::sync::OnceLock;

/// Global function registry - initialized once
static FUNCTION_REGISTRY: OnceLock<FunctionRegistry> = OnceLock::new();

/// Initialize the global function registry
fn get_function_registry() -> &'static FunctionRegistry {
    FUNCTION_REGISTRY.get_or_init(|| create_builtin_registry())
}

/// Direct AST evaluator that treats expressions as functions
/// Each expression takes a context (current data) and returns a value
pub fn evaluate(expr: &Expr, context: &EdnValue) -> EqResult<EdnValue> {
    let env = Environment::with_context(context.clone());
    evaluate_with_env(expr, context, &env)
}

/// Evaluate an expression with a given environment
pub fn evaluate_with_env(expr: &Expr, context: &EdnValue, env: &Environment) -> EqResult<EdnValue> {
    match expr {
        Expr::Symbol(name) => {
            env.lookup(name)
                .cloned()
                .ok_or_else(|| EqError::query_error(format!("Undefined symbol: {}", name)))
        }
        
        Expr::Get(key) => {
            Ok(context.get(key).cloned().unwrap_or(EdnValue::Nil))
        }
        
        Expr::GetIn(input_expr, path) => {
            let input_value = evaluate_with_env(input_expr, context, env)?;
            Ok(input_value.get_in(path.clone()).cloned().unwrap_or(EdnValue::Nil))
        }
        
        Expr::KeywordAccess(name) => {
            let key = EdnValue::Keyword(name.clone());
            Ok(context.get(&key).cloned().unwrap_or(EdnValue::Nil))
        }
        
        Expr::KeywordGet(name, expr) => {
            let target = evaluate_with_env(expr, context, env)?;
            let key = EdnValue::Keyword(name.clone());
            Ok(target.get(&key).cloned().unwrap_or(EdnValue::Nil))
        }
        
        Expr::KeywordGetWithDefault(name, expr, default_expr) => {
            let target = evaluate_with_env(expr, context, env)?;
            let key = EdnValue::Keyword(name.clone());
            match target.get(&key) {
                Some(value) => Ok(value.clone()),
                None => evaluate_with_env(default_expr, context, env),
            }
        }
        
        // Collection operations
        Expr::Function { name, args } => {
            let registry = get_function_registry();
            if let Some(func) = registry.get(name) {
                // Evaluate all arguments
                let mut eval_args = Vec::new();
                for arg in args {
                    eval_args.push(evaluate_with_env(arg, context, env)?);
                }
                
                // Call the function
                func(&eval_args, context)
            } else {
                Err(EqError::query_error(format!("Unknown function: {}", name)))
            }
        }
        
        // Composition - evaluate expressions in sequence
        Expr::Comp(exprs) => {
            let mut result = context.clone();
            for expr in exprs {
                let new_env = Environment::with_context(result.clone());
                result = evaluate_with_env(expr, &result, &new_env)?;
            }
            Ok(result)
        }
        
        // Conditionals
        Expr::If { test, then_expr, else_expr } => {
            let test_result = evaluate_with_env(test, context, env)?;
            if test_result.is_truthy() {
                evaluate_with_env(then_expr, context, env)
            } else if let Some(else_expr) = else_expr {
                evaluate_with_env(else_expr, context, env)
            } else {
                Ok(EdnValue::Nil)
            }
        }
        
        // Literals
        Expr::Literal(value) => Ok(value.clone()),
        
        // Raw lists should be analyzed away before evaluation
        Expr::List(_) => {
            Err(EqError::query_error("Unanalyzed list expression found - analysis phase should handle all lists"))
        }
        
        // Macros should have been expanded before evaluation
        Expr::ThreadFirst(_) | Expr::ThreadLast(_) | Expr::When { .. } => {
            Err(EqError::query_error("Unexpanded macro found - macros should be expanded before evaluation".to_string()))
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;

    #[test]
    fn test_identity() {
        let input = EdnValue::Integer(42);
        let result = evaluate(&Expr::Symbol(".".to_string()), &input).unwrap();
        assert_eq!(result, input);
    }

    #[test]
    fn test_keyword_access() {
        let mut map = IndexMap::new();
        map.insert(EdnValue::Keyword("name".to_string()), EdnValue::String("Alice".to_string()));
        let input = EdnValue::Map(map);
        
        let result = evaluate(&Expr::KeywordAccess("name".to_string()), &input).unwrap();
        assert_eq!(result, EdnValue::String("Alice".to_string()));
    }

    #[test]
    fn test_first() {
        let input = EdnValue::Vector(vec![
            EdnValue::Integer(1),
            EdnValue::Integer(2),
            EdnValue::Integer(3),
        ]);
        
        let expr = Expr::Function {
            name: "first".to_string(),
            args: vec![],
        };
        
        let result = evaluate(&expr, &input).unwrap();
        assert_eq!(result, EdnValue::Integer(1));
    }

    #[test]
    fn test_count() {
        let input = EdnValue::Vector(vec![
            EdnValue::Integer(1),
            EdnValue::Integer(2),
            EdnValue::Integer(3),
        ]);
        
        let expr = Expr::Function {
            name: "count".to_string(),
            args: vec![],
        };
        
        let result = evaluate(&expr, &input).unwrap();
        assert_eq!(result, EdnValue::Integer(3));
    }

    #[test]
    fn test_predicates() {
        // Test is-nil
        let nil_expr = Expr::Function {
            name: "nil?".to_string(),
            args: vec![],
        };
        let result = evaluate(&nil_expr, &EdnValue::Nil).unwrap();
        assert_eq!(result, EdnValue::Bool(true));
        
        let result = evaluate(&nil_expr, &EdnValue::Integer(42)).unwrap();
        assert_eq!(result, EdnValue::Bool(false));
        
        // Test is-number
        let number_expr = Expr::Function {
            name: "number?".to_string(),
            args: vec![],
        };
        let result = evaluate(&number_expr, &EdnValue::Integer(42)).unwrap();
        assert_eq!(result, EdnValue::Bool(true));
        
        let result = evaluate(&number_expr, &EdnValue::String("hello".to_string())).unwrap();
        assert_eq!(result, EdnValue::Bool(false));
    }

    #[test]
    fn test_comparison() {
        // Test equality
        let expr = Expr::Function {
            name: "=".to_string(),
            args: vec![Expr::Literal(EdnValue::Integer(42))],
        };
        
        let result = evaluate(&expr, &EdnValue::Integer(42)).unwrap();
        assert_eq!(result, EdnValue::Bool(true));
        
        let result = evaluate(&expr, &EdnValue::Integer(43)).unwrap();
        assert_eq!(result, EdnValue::Bool(false));
    }

    #[test]
    fn test_take_drop() {
        let input = EdnValue::Vector(vec![
            EdnValue::Integer(1),
            EdnValue::Integer(2),
            EdnValue::Integer(3),
            EdnValue::Integer(4),
        ]);
        
        // Test take
        let take_expr = Expr::Function {
            name: "take".to_string(),
            args: vec![Expr::Literal(EdnValue::Integer(2))],
        };
        let result = evaluate(&take_expr, &input).unwrap();
        assert_eq!(result, EdnValue::Vector(vec![
            EdnValue::Integer(1),
            EdnValue::Integer(2),
        ]));
        
        // Test drop
        let drop_expr = Expr::Function {
            name: "drop".to_string(),
            args: vec![Expr::Literal(EdnValue::Integer(2))],
        };
        let result = evaluate(&drop_expr, &input).unwrap();
        assert_eq!(result, EdnValue::Vector(vec![
            EdnValue::Integer(3),
            EdnValue::Integer(4),
        ]));
    }

    #[test]
    fn test_composition() {
        // Test composition: first then count (should fail since first returns a single value)
        let expr = Expr::Comp(vec![
            Expr::Function {
                name: "first".to_string(),
                args: vec![],
            },
            Expr::Function {
                name: "count".to_string(),
                args: vec![],
            },
        ]);
        
        let input = EdnValue::Vector(vec![
            EdnValue::Vector(vec![EdnValue::Integer(1), EdnValue::Integer(2)]),
            EdnValue::Vector(vec![EdnValue::Integer(3), EdnValue::Integer(4)]),
        ]);
        
        // This should get the first vector, then count its elements
        let result = evaluate(&expr, &input).unwrap();
        assert_eq!(result, EdnValue::Integer(2));
    }

    #[test]
    fn test_if_expression() {
        let expr = Expr::If {
            test: Box::new(Expr::Function {
                name: "nil?".to_string(),
                args: vec![],
            }),
            then_expr: Box::new(Expr::Literal(EdnValue::String("it's nil".to_string()))),
            else_expr: Some(Box::new(Expr::Literal(EdnValue::String("not nil".to_string())))),
        };
        
        let result = evaluate(&expr, &EdnValue::Nil).unwrap();
        assert_eq!(result, EdnValue::String("it's nil".to_string()));
        
        let result = evaluate(&expr, &EdnValue::Integer(42)).unwrap();
        assert_eq!(result, EdnValue::String("not nil".to_string()));
    }

    #[test]
    fn test_function_with_higher_order() {
        // Test that frequencies works as expected
        let input = EdnValue::Vector(vec![
            EdnValue::String("a".to_string()),
            EdnValue::String("b".to_string()),
            EdnValue::String("a".to_string()),
        ]);
        
        let expr = Expr::Function {
            name: "frequencies".to_string(),
            args: vec![],
        };
        
        let result = evaluate(&expr, &input).unwrap();
        
        if let EdnValue::Map(map) = result {
            assert_eq!(map.get(&EdnValue::String("a".to_string())), Some(&EdnValue::Integer(2)));
            assert_eq!(map.get(&EdnValue::String("b".to_string())), Some(&EdnValue::Integer(1)));
        } else {
            panic!("Expected map result from frequencies");
        }
    }
}
use crate::edn::EdnValue;
use crate::error::{EqError, EqResult};
use crate::query::ast::{Expr, FunctionRegistry, Environment, FunctionType};
use crate::builtins::create_builtin_registry;

use std::sync::OnceLock;

/// Global function registry - initialized once
static FUNCTION_REGISTRY: OnceLock<FunctionRegistry> = OnceLock::new();

/// Initialize the global function registry
fn get_function_registry() -> &'static FunctionRegistry {
    FUNCTION_REGISTRY.get_or_init(|| {
        let mut registry = create_builtin_registry();
        
        // Add special forms here to avoid circular dependencies
        registry.register_special_form("if".to_string(), special_form_if);
        registry.register_special_form("do".to_string(), special_form_do);
        
        registry
    })
}

/// Special form implementation for 'if'
fn special_form_if(args: &[Expr], context: &EdnValue, env: &Environment) -> EqResult<EdnValue> {
    match args.len() {
        2 => {
            // (if test then)
            let test_result = evaluate_with_env(&args[0], context, env)?;
            if test_result.is_truthy() {
                evaluate_with_env(&args[1], context, env)
            } else {
                Ok(EdnValue::Nil)
            }
        }
        3 => {
            // (if test then else)
            let test_result = evaluate_with_env(&args[0], context, env)?;
            if test_result.is_truthy() {
                evaluate_with_env(&args[1], context, env)
            } else {
                evaluate_with_env(&args[2], context, env)
            }
        }
        _ => Err(EqError::query_error("if takes 2 or 3 arguments".to_string())),
    }
}

/// Special form implementation for 'do'
fn special_form_do(args: &[Expr], context: &EdnValue, env: &Environment) -> EqResult<EdnValue> {
    if args.is_empty() {
        return Ok(EdnValue::Nil);
    }
    
    // Evaluate all expressions in sequence, returning the last result
    let mut result = EdnValue::Nil;
    for expr in args {
        result = evaluate_with_env(expr, context, env)?;
    }
    Ok(result)
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
        
        // Function calls (regular functions and special forms)
        Expr::Function { name, args } => {
            let registry = get_function_registry();
            if let Some(func_type) = registry.get(name) {
                match func_type {
                    FunctionType::Regular(func) => {
                        // Evaluate all arguments for regular functions
                        let mut eval_args = Vec::new();
                        for arg in args {
                            eval_args.push(evaluate_with_env(arg, context, env)?);
                        }
                        
                        // Call the regular function
                        func(&eval_args, context)
                    }
                    FunctionType::SpecialForm(special_func) => {
                        // Pass unevaluated arguments to special forms
                        special_func(args, context, env)
                    }
                    FunctionType::Macro(macro_func) => {
                        // Macros return new expressions that need to be analyzed and evaluated
                        let expanded_expr = macro_func(args)?;
                        // Re-analyze the expanded expression (may contain more macros)
                        let analyzed_expr = crate::analyzer::analyze(expanded_expr)?;
                        // Then evaluate the fully analyzed expression
                        evaluate_with_env(&analyzed_expr, context, env)
                    }
                }
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
        
        
        // Literals
        Expr::Literal(value) => Ok(value.clone()),
        
        // Raw lists should be analyzed away before evaluation
        Expr::List(_) => {
            Err(EqError::query_error("Unanalyzed list expression found - analysis phase should handle all lists"))
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
            args: vec![Expr::Symbol(".".to_string())],
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
            args: vec![Expr::Symbol(".".to_string())],
        };
        
        let result = evaluate(&expr, &input).unwrap();
        assert_eq!(result, EdnValue::Integer(3));
    }

    #[test]
    fn test_predicates() {
        // Test is-nil
        let nil_expr = Expr::Function {
            name: "nil?".to_string(),
            args: vec![Expr::Symbol(".".to_string())],
        };
        let result = evaluate(&nil_expr, &EdnValue::Nil).unwrap();
        assert_eq!(result, EdnValue::Bool(true));
        
        let result = evaluate(&nil_expr, &EdnValue::Integer(42)).unwrap();
        assert_eq!(result, EdnValue::Bool(false));
        
        // Test is-number
        let number_expr = Expr::Function {
            name: "number?".to_string(),
            args: vec![Expr::Symbol(".".to_string())],
        };
        let result = evaluate(&number_expr, &EdnValue::Integer(42)).unwrap();
        assert_eq!(result, EdnValue::Bool(true));
        
        let result = evaluate(&number_expr, &EdnValue::String("hello".to_string())).unwrap();
        assert_eq!(result, EdnValue::Bool(false));
    }

    #[test]
    fn test_comparison() {
        // Test equality with multiple arguments - all equal
        let eq_expr = Expr::Function {
            name: "=".to_string(),
            args: vec![
                Expr::Literal(EdnValue::Integer(42)),
                Expr::Literal(EdnValue::Integer(42)),
                Expr::Literal(EdnValue::Integer(42)),
            ],
        };
        let result = evaluate(&eq_expr, &EdnValue::Nil).unwrap();
        assert_eq!(result, EdnValue::Bool(true));
        
        // Test equality with multiple arguments - not all equal
        let eq_false_expr = Expr::Function {
            name: "=".to_string(),
            args: vec![
                Expr::Literal(EdnValue::Integer(42)),
                Expr::Literal(EdnValue::Integer(42)),
                Expr::Literal(EdnValue::Integer(43)),
            ],
        };
        let result = evaluate(&eq_false_expr, &EdnValue::Nil).unwrap();
        assert_eq!(result, EdnValue::Bool(false));
        
        // Test 0-argument and 1-argument cases (should be true)
        let empty_eq = Expr::Function { name: "=".to_string(), args: vec![] };
        assert_eq!(evaluate(&empty_eq, &EdnValue::Nil).unwrap(), EdnValue::Bool(true));
        
        let single_eq = Expr::Function { 
            name: "=".to_string(), 
            args: vec![Expr::Literal(EdnValue::Integer(42))] 
        };
        assert_eq!(evaluate(&single_eq, &EdnValue::Nil).unwrap(), EdnValue::Bool(true));
        
        // Test < with multiple arguments
        let lt_expr = Expr::Function {
            name: "<".to_string(),
            args: vec![
                Expr::Literal(EdnValue::Integer(1)),
                Expr::Literal(EdnValue::Integer(2)),
                Expr::Literal(EdnValue::Integer(3)),
            ],
        };
        let result = evaluate(&lt_expr, &EdnValue::Nil).unwrap();
        assert_eq!(result, EdnValue::Bool(true));
        
        // Test < with descending sequence (should be false)
        let lt_false_expr = Expr::Function {
            name: "<".to_string(),
            args: vec![
                Expr::Literal(EdnValue::Integer(3)),
                Expr::Literal(EdnValue::Integer(2)),
                Expr::Literal(EdnValue::Integer(1)),
            ],
        };
        let result = evaluate(&lt_false_expr, &EdnValue::Nil).unwrap();
        assert_eq!(result, EdnValue::Bool(false));
        
        // Test <= with equal values (should be true)
        let le_expr = Expr::Function {
            name: "<=".to_string(),
            args: vec![
                Expr::Literal(EdnValue::Integer(1)),
                Expr::Literal(EdnValue::Integer(3)),
                Expr::Literal(EdnValue::Integer(2)),
            ],
        };
        let result = evaluate(&le_expr, &EdnValue::Nil).unwrap();
        assert_eq!(result, EdnValue::Bool(false));
        
        // Test > with descending sequence
        let gt_expr = Expr::Function {
            name: ">".to_string(),
            args: vec![
                Expr::Literal(EdnValue::Integer(5)),
                Expr::Literal(EdnValue::Integer(3)),
                Expr::Literal(EdnValue::Integer(1)),
            ],
        };
        let result = evaluate(&gt_expr, &EdnValue::Nil).unwrap();
        assert_eq!(result, EdnValue::Bool(true));
        
        // Test >= with equal values
        let ge_expr = Expr::Function {
            name: ">=".to_string(),
            args: vec![
                Expr::Literal(EdnValue::Integer(3)),
                Expr::Literal(EdnValue::Integer(3)),
                Expr::Literal(EdnValue::Integer(1)),
            ],
        };
        let result = evaluate(&ge_expr, &EdnValue::Nil).unwrap();
        assert_eq!(result, EdnValue::Bool(true));
        
        // Test 0-argument cases (should all be true)
        let empty_lt = Expr::Function { name: "<".to_string(), args: vec![] };
        assert_eq!(evaluate(&empty_lt, &EdnValue::Nil).unwrap(), EdnValue::Bool(true));
        
        let empty_gt = Expr::Function { name: ">".to_string(), args: vec![] };
        assert_eq!(evaluate(&empty_gt, &EdnValue::Nil).unwrap(), EdnValue::Bool(true));
        
        // Test 1-argument cases (should all be true) 
        let single_lt = Expr::Function { 
            name: "<".to_string(), 
            args: vec![Expr::Literal(EdnValue::Integer(42))] 
        };
        assert_eq!(evaluate(&single_lt, &EdnValue::Nil).unwrap(), EdnValue::Bool(true));
    }

    #[test]
    fn test_take_drop() {
        let input = EdnValue::Vector(vec![
            EdnValue::Integer(1),
            EdnValue::Integer(2),
            EdnValue::Integer(3),
            EdnValue::Integer(4),
        ]);
        
        // Test take - now requires 2 args: count and collection
        let take_expr = Expr::Function {
            name: "take".to_string(),
            args: vec![
                Expr::Literal(EdnValue::Integer(2)),
                Expr::Symbol(".".to_string()),
            ],
        };
        let result = evaluate(&take_expr, &input).unwrap();
        assert_eq!(result, EdnValue::Vector(vec![
            EdnValue::Integer(1),
            EdnValue::Integer(2),
        ]));
        
        // Test drop - now requires 2 args: count and collection
        let drop_expr = Expr::Function {
            name: "drop".to_string(),
            args: vec![
                Expr::Literal(EdnValue::Integer(2)),
                Expr::Symbol(".".to_string()),
            ],
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
                args: vec![Expr::Symbol(".".to_string())],
            },
            Expr::Function {
                name: "count".to_string(),
                args: vec![Expr::Symbol(".".to_string())],
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
        let expr = Expr::Function {
            name: "if".to_string(),
            args: vec![
                Expr::Function {
                    name: "nil?".to_string(),
                    args: vec![Expr::Symbol(".".to_string())],
                },
                Expr::Literal(EdnValue::String("it's nil".to_string())),
                Expr::Literal(EdnValue::String("not nil".to_string())),
            ],
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
            args: vec![Expr::Symbol(".".to_string())],
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
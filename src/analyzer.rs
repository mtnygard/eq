use crate::edn::{EdnValue, value::EdnLambda};
use crate::error::{EqError, EqResult};
use crate::query::ast::{Expr, FunctionRegistry, FunctionType};
use crate::builtins::create_builtin_registry;
use std::sync::OnceLock;

/// Global function registry for macro detection
static ANALYZER_REGISTRY: OnceLock<FunctionRegistry> = OnceLock::new();

fn get_analyzer_registry() -> &'static FunctionRegistry {
    ANALYZER_REGISTRY.get_or_init(|| {
        let registry = create_builtin_registry();
        // Add any analyzer-specific special forms here if needed
        registry
    })
}

/// Analyze and macroexpand expressions until fixed point
pub fn analyze(expr: Expr) -> EqResult<Expr> {
    let mut current = expr;
    
    // Keep analyzing until no more changes occur (fixed point)
    loop {
        let analyzed = analyze_once(current.clone())?;
        if analyzed == current {
            break;
        }
        current = analyzed;
    }
    
    Ok(current)
}

/// Perform one round of analysis and macroexpansion
fn analyze_once(expr: Expr) -> EqResult<Expr> {
    match expr {
        // Raw lists need to be analyzed
        Expr::List(elements) => {
            if elements.is_empty() {
                return Err(EqError::query_error("Empty list expression"));
            }
            
            let head = &elements[0];
            let args = &elements[1..];
            
            match head {
                EdnValue::Symbol(name) => {
                    // Special handling for lambda syntax (fn [params] body)
                    if name == "fn" {
                        return analyze_lambda(args);
                    }
                    
                    let registry = get_analyzer_registry();
                    if let Some(func_type) = registry.get(name) {
                        if let FunctionType::Macro(macro_func) = func_type {
                            // Convert EDN args to Expr args for macro
                            let expr_args = args.iter()
                                .map(|arg| edn_to_expr(arg))
                                .collect::<Result<Vec<_>, _>>()?;
                            // Expand the macro
                            macro_func(&expr_args)
                        } else {
                            // It's a regular function or special form
                            analyze_function_call(name, args)
                        }
                    } else {
                        // Unknown function - treat as regular function call
                        analyze_function_call(name, args)
                    }
                }
                EdnValue::Keyword(name) => analyze_keyword_call(name, args),
                _ => Err(EqError::query_error("First element of list must be a symbol or keyword")),
            }
        }
        
        // Recursively analyze sub-expressions
        Expr::KeywordGet(name, expr) => {
            Ok(Expr::KeywordGet(name, Box::new(analyze(*expr)?)))
        }
        
        Expr::KeywordGetWithDefault(name, expr, default_expr) => {
            Ok(Expr::KeywordGetWithDefault(
                name, 
                Box::new(analyze(*expr)?), 
                Box::new(analyze(*default_expr)?)
            ))
        }
        
        Expr::Function { name, args } => {
            Ok(Expr::Function {
                name,
                args: args.into_iter().map(analyze).collect::<Result<Vec<_>, _>>()?,
            })
        }

        Expr::LambdaCall { func, args } => {
            Ok(Expr::LambdaCall {
                func: Box::new(analyze(*func)?),
                args: args.into_iter().map(analyze).collect::<Result<Vec<_>, _>>()?,
            })
        }
        
        Expr::Comp(exprs) => {
            Ok(Expr::Comp(exprs.into_iter().map(analyze).collect::<Result<Vec<_>, _>>()?))
        }
        
        // All other expressions are already analyzed
        expr => Ok(expr),
    }
}




/// Analyze function calls (symbols in head position)
fn analyze_function_call(name: &str, args: &[EdnValue]) -> EqResult<Expr> {
    // All functions become Function calls - special forms are handled at evaluation time
    let analyzed_args = args.iter()
        .map(|arg| analyze(edn_to_expr(arg)?))
        .collect::<Result<Vec<_>, _>>()?;
    
    Ok(Expr::Function {
        name: name.to_string(),
        args: analyzed_args,
    })
}

/// Analyze keyword calls (keywords in head position) 
fn analyze_keyword_call(name: &str, args: &[EdnValue]) -> EqResult<Expr> {
    match args.len() {
        0 => Err(EqError::query_error(format!("Keyword :{} requires at least 1 argument", name))),
        1 => {
            let arg_expr = edn_to_expr(&args[0])?;
            Ok(Expr::KeywordGet(name.to_string(), Box::new(analyze(arg_expr)?)))
        }
        2 => {
            let arg_expr = edn_to_expr(&args[0])?;
            let default_expr = edn_to_expr(&args[1])?;
            Ok(Expr::KeywordGetWithDefault(
                name.to_string(),
                Box::new(analyze(arg_expr)?),
                Box::new(analyze(default_expr)?)
            ))
        }
        _ => Err(EqError::query_error(format!("Keyword :{} takes 1 or 2 arguments, got {}", name, args.len())))
    }
}

/// Convert EDN value to expression
fn edn_to_expr(value: &EdnValue) -> EqResult<Expr> {
    match value {
        EdnValue::Symbol(name) => Ok(Expr::Symbol(name.clone())),
        EdnValue::List(elements) => Ok(Expr::List(elements.clone())),
        _ => Ok(Expr::Literal(value.clone())),
    }
}

// Helper functions for special cases

/// Analyze lambda syntax: (fn [params] body)
fn analyze_lambda(args: &[EdnValue]) -> EqResult<Expr> {
    if args.len() != 2 {
        return Err(EqError::query_error("fn requires exactly 2 arguments: parameter vector and body"));
    }
    
    // First argument should be a parameter vector
    let params = match &args[0] {
        EdnValue::Vector(params) => {
            let mut param_names = Vec::new();
            for param in params {
                if let EdnValue::Symbol(name) = param {
                    param_names.push(name.clone());
                } else {
                    return Err(EqError::query_error("fn parameters must be symbols"));
                }
            }
            param_names
        }
        _ => return Err(EqError::query_error("fn first argument must be a parameter vector")),
    };
    
    // Second argument is the body
    let body = &args[1];
    
    // Create lambda and return as literal expression
    let lambda = EdnLambda {
        params,
        body: Box::new(body.clone()),
    };
    
    Ok(Expr::Literal(EdnValue::Lambda(lambda)))
}
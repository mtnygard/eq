use crate::edn::EdnValue;
use crate::error::{EqError, EqResult};
use crate::query::ast::Expr;

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
                    // Check if it's a macro first
                    if is_macro(name) {
                        // Expand the macro
                        expand_macro(name, args)
                    } else {
                        // It's a function call
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
        
        Expr::Comp(exprs) => {
            Ok(Expr::Comp(exprs.into_iter().map(analyze).collect::<Result<Vec<_>, _>>()?))
        }
        
        Expr::If { test, then_expr, else_expr } => {
            Ok(Expr::If {
                test: Box::new(analyze(*test)?),
                then_expr: Box::new(analyze(*then_expr)?),
                else_expr: match else_expr {
                    Some(e) => Some(Box::new(analyze(*e)?)),
                    None => None,
                },
            })
        }
        
        
        // These should not appear after macro expansion
        Expr::ThreadFirst(_) | Expr::ThreadLast(_) | Expr::When { .. } => {
            Err(EqError::query_error("Macro variants should not appear after expansion"))
        }
        
        // All other expressions are already analyzed
        expr => Ok(expr),
    }
}

/// Check if a symbol is a macro
fn is_macro(name: &str) -> bool {
    matches!(name, "->" | "->>" | "when")
}

/// Expand a macro
fn expand_macro(name: &str, args: &[EdnValue]) -> EqResult<Expr> {
    match name {
        "->" => expand_thread_first_macro(args),
        "->>" => expand_thread_last_macro(args),
        "when" => expand_when_macro(args),
        _ => Err(EqError::query_error(format!("Unknown macro: {}", name))),
    }
}

/// Expand the -> macro
fn expand_thread_first_macro(args: &[EdnValue]) -> EqResult<Expr> {
    if args.is_empty() {
        return Err(EqError::query_error("-> macro requires at least one argument"));
    }
    
    // Convert first arg to expression
    let mut result = edn_to_expr(&args[0])?;
    
    // Thread through each subsequent form
    for form in args.iter().skip(1) {
        result = thread_first(result, form)?;
    }
    
    Ok(result)
}

/// Expand the ->> macro
fn expand_thread_last_macro(args: &[EdnValue]) -> EqResult<Expr> {
    if args.is_empty() {
        return Err(EqError::query_error("->> macro requires at least one argument"));
    }
    
    // Convert first arg to expression
    let mut result = edn_to_expr(&args[0])?;
    
    // Thread through each subsequent form
    for form in args.iter().skip(1) {
        result = thread_last(result, form)?;
    }
    
    Ok(result)
}

/// Thread first: insert threaded value as first argument
fn thread_first(threaded_value: Expr, form: &EdnValue) -> EqResult<Expr> {
    match form {
        // If it's a symbol like 'first', convert to (first threaded_value)
        EdnValue::Symbol(name) => {
            Ok(Expr::List(vec![
                EdnValue::Symbol(name.clone()),
                edn_value_from_expr(threaded_value)?,
            ]))
        }
        
        // If it's a keyword like :name, convert to (:name threaded_value)
        EdnValue::Keyword(name) => {
            Ok(Expr::List(vec![
                EdnValue::Keyword(name.clone()),
                edn_value_from_expr(threaded_value)?,
            ]))
        }
        
        // If it's a list like (take 3), convert to (take threaded_value 3)
        EdnValue::List(elements) if !elements.is_empty() => {
            let mut new_form = vec![elements[0].clone(), edn_value_from_expr(threaded_value)?];
            new_form.extend_from_slice(&elements[1..]);
            Ok(Expr::List(new_form))
        }
        
        _ => Err(EqError::query_error("Invalid form in -> macro")),
    }
}

/// Thread last: insert threaded value as last argument
fn thread_last(threaded_value: Expr, form: &EdnValue) -> EqResult<Expr> {
    match form {
        // If it's a symbol like 'first', convert to (first threaded_value)
        EdnValue::Symbol(name) => {
            Ok(Expr::List(vec![
                EdnValue::Symbol(name.clone()),
                edn_value_from_expr(threaded_value)?,
            ]))
        }
        
        // If it's a keyword like :name, convert to (:name threaded_value)  
        EdnValue::Keyword(name) => {
            Ok(Expr::List(vec![
                EdnValue::Keyword(name.clone()),
                edn_value_from_expr(threaded_value)?,
            ]))
        }
        
        // If it's a list like (take 3), convert to (take 3 threaded_value)
        EdnValue::List(elements) if !elements.is_empty() => {
            let mut new_form = elements.clone();
            new_form.push(edn_value_from_expr(threaded_value)?);
            Ok(Expr::List(new_form))
        }
        
        _ => Err(EqError::query_error("Invalid form in ->> macro")),
    }
}

/// Expand the when macro: (when test body) -> (if test body nil)
fn expand_when_macro(args: &[EdnValue]) -> EqResult<Expr> {
    if args.len() != 2 {
        return Err(EqError::query_error("when macro requires exactly 2 arguments"));
    }
    
    Ok(Expr::List(vec![
        EdnValue::Symbol("if".to_string()),
        args[0].clone(),
        args[1].clone(),
        EdnValue::Nil,
    ]))
}

/// Convert an expression back to an EDN value (needed for macro expansion)
fn edn_value_from_expr(expr: Expr) -> EqResult<EdnValue> {
    match expr {
        Expr::Symbol(name) => Ok(EdnValue::Symbol(name)),
        Expr::Literal(value) => Ok(value),
        Expr::List(elements) => Ok(EdnValue::List(elements)),
        _ => Err(EqError::query_error("Cannot convert complex expression to EDN value")),
    }
}

/// Analyze function calls (symbols in head position)
fn analyze_function_call(name: &str, args: &[EdnValue]) -> EqResult<Expr> {
    match name {
        // Basic selectors - these keep their special handling
        "get" => analyze_get(args),
        "get-in" => analyze_get_in(args),
        
        // Conditional - special syntax
        "if" => analyze_if(args),
        
        // All other functions become Function calls
        _ => {
            let analyzed_args = args.iter()
                .map(|arg| analyze(edn_to_expr(arg)?))
                .collect::<Result<Vec<_>, _>>()?;
            
            Ok(Expr::Function {
                name: name.to_string(),
                args: analyzed_args,
            })
        }
    }
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

fn analyze_get(args: &[EdnValue]) -> EqResult<Expr> {
    if args.len() != 1 {
        return Err(EqError::query_error("get takes exactly one argument"));
    }
    Ok(Expr::Get(args[0].clone()))
}

fn analyze_get_in(args: &[EdnValue]) -> EqResult<Expr> {
    if args.len() != 2 {
        return Err(EqError::query_error("get-in takes exactly two arguments"));
    }
    let input_expr = edn_to_expr(&args[0])?;
    match &args[1] {
        EdnValue::Vector(path) => Ok(Expr::GetIn(Box::new(input_expr), path.clone())),
        _ => Err(EqError::query_error("get-in requires a vector as second argument")),
    }
}


fn analyze_if(args: &[EdnValue]) -> EqResult<Expr> {
    match args.len() {
        2 => {
            let test_expr = edn_to_expr(&args[0])?;
            let then_expr = edn_to_expr(&args[1])?;
            Ok(Expr::If {
                test: Box::new(analyze(test_expr)?),
                then_expr: Box::new(analyze(then_expr)?),
                else_expr: None,
            })
        }
        3 => {
            let test_expr = edn_to_expr(&args[0])?;
            let then_expr = edn_to_expr(&args[1])?;
            let else_expr = edn_to_expr(&args[2])?;
            Ok(Expr::If {
                test: Box::new(analyze(test_expr)?),
                then_expr: Box::new(analyze(then_expr)?),
                else_expr: Some(Box::new(analyze(else_expr)?)),
            })
        }
        _ => Err(EqError::query_error("if takes 2 or 3 arguments")),
    }
}
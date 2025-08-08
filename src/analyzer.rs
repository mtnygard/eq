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
        
        Expr::First(expr) => Ok(Expr::First(Box::new(analyze(*expr)?))),
        Expr::Last(expr) => Ok(Expr::Last(Box::new(analyze(*expr)?))),
        Expr::Rest(expr) => Ok(Expr::Rest(Box::new(analyze(*expr)?))),
        Expr::Count(expr) => Ok(Expr::Count(Box::new(analyze(*expr)?))),
        Expr::Keys(expr) => Ok(Expr::Keys(Box::new(analyze(*expr)?))),
        Expr::Vals(expr) => Ok(Expr::Vals(Box::new(analyze(*expr)?))),
        Expr::Take(n_expr, coll_expr) => Ok(Expr::Take(Box::new(analyze(*n_expr)?), Box::new(analyze(*coll_expr)?))),
        Expr::Drop(n_expr, coll_expr) => Ok(Expr::Drop(Box::new(analyze(*n_expr)?), Box::new(analyze(*coll_expr)?))),
        Expr::Nth(n_expr, coll_expr) => Ok(Expr::Nth(Box::new(analyze(*n_expr)?), Box::new(analyze(*coll_expr)?))),
        Expr::Map(expr) => Ok(Expr::Map(Box::new(analyze(*expr)?))),
        Expr::Remove(expr) => Ok(Expr::Remove(Box::new(analyze(*expr)?))),
        Expr::Select(expr) => Ok(Expr::Select(Box::new(analyze(*expr)?))),
        Expr::Contains(expr) => Ok(Expr::Contains(Box::new(analyze(*expr)?))),
        
        Expr::Equal(left, right) => {
            Ok(Expr::Equal(Box::new(analyze(*left)?), Box::new(analyze(*right)?)))
        }
        
        Expr::LessThan(expr) => Ok(Expr::LessThan(Box::new(analyze(*expr)?))),
        Expr::GreaterThan(expr) => Ok(Expr::GreaterThan(Box::new(analyze(*expr)?))),
        Expr::LessEqual(expr) => Ok(Expr::LessEqual(Box::new(analyze(*expr)?))),
        Expr::GreaterEqual(expr) => Ok(Expr::GreaterEqual(Box::new(analyze(*expr)?))),
        
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
        
        Expr::Reduce { func, init } => {
            Ok(Expr::Reduce {
                func: Box::new(analyze(*func)?),
                init: match init {
                    Some(e) => Some(Box::new(analyze(*e)?)),
                    None => None,
                },
            })
        }
        
        Expr::Apply(expr) => Ok(Expr::Apply(Box::new(analyze(*expr)?))),
        Expr::GroupBy(expr) => Ok(Expr::GroupBy(Box::new(analyze(*expr)?))),
        
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
        Expr::Identity => Ok(EdnValue::Symbol(".".to_string())),
        Expr::Literal(value) => Ok(value),
        Expr::List(elements) => Ok(EdnValue::List(elements)),
        _ => Err(EqError::query_error("Cannot convert complex expression to EDN value")),
    }
}

/// Analyze function calls (symbols in head position)
fn analyze_function_call(name: &str, args: &[EdnValue]) -> EqResult<Expr> {
    match name {
        // Basic selectors
        "get" => analyze_get(args),
        "get-in" => analyze_get_in(args),
        
        // Collection operations - all take a collection argument
        "first" => analyze_unary("first", args, Expr::First),
        "last" => analyze_unary("last", args, Expr::Last),
        "rest" => analyze_unary("rest", args, Expr::Rest),
        "count" => analyze_unary("count", args, Expr::Count),
        "keys" => analyze_unary("keys", args, Expr::Keys),
        "vals" => analyze_unary("vals", args, Expr::Vals),
        "take" => analyze_binary("take", args, Expr::Take),
        "drop" => analyze_binary("drop", args, Expr::Drop),
        "nth" => analyze_binary("nth", args, Expr::Nth),
        
        // Other operations
        "map" => analyze_unary("map", args, Expr::Map),
        "remove" => analyze_unary("remove", args, Expr::Remove),
        "select" => analyze_unary("select", args, Expr::Select),
        "select-keys" => analyze_select_keys(args),
        "frequencies" => analyze_nullary("frequencies", args, Expr::Frequencies),
        
        // Predicates  
        "nil?" => analyze_nullary("nil?", args, Expr::IsNil),
        "empty?" => analyze_nullary("empty?", args, Expr::IsEmpty),
        "number?" => analyze_nullary("number?", args, Expr::IsNumber),
        "string?" => analyze_nullary("string?", args, Expr::IsString),
        "keyword?" => analyze_nullary("keyword?", args, Expr::IsKeyword),
        "boolean?" => analyze_nullary("boolean?", args, Expr::IsBoolean),
        "contains?" => analyze_unary("contains?", args, Expr::Contains),
        
        // Comparison
        "=" => analyze_binary("=", args, Expr::Equal),
        "<" => analyze_unary("<", args, Expr::LessThan),
        ">" => analyze_unary(">", args, Expr::GreaterThan),
        "<=" => analyze_unary("<=", args, Expr::LessEqual),
        ">=" => analyze_unary(">=", args, Expr::GreaterEqual),
        
        // Conditional
        "if" => analyze_if(args),
        
        _ => Err(EqError::query_error(format!("Unknown function: {}", name))),
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
        EdnValue::Symbol(name) if name == "." => Ok(Expr::Identity),
        EdnValue::List(elements) => Ok(Expr::List(elements.clone())),
        _ => Ok(Expr::Literal(value.clone())),
    }
}

// Helper functions for common patterns
fn analyze_nullary(name: &str, args: &[EdnValue], expr: Expr) -> EqResult<Expr> {
    if !args.is_empty() {
        return Err(EqError::query_error(format!("{} takes no arguments", name)));
    }
    Ok(expr)
}

fn analyze_unary<F>(name: &str, args: &[EdnValue], constructor: F) -> EqResult<Expr> 
where F: Fn(Box<Expr>) -> Expr
{
    if args.len() != 1 {
        return Err(EqError::query_error(format!("{} takes exactly one argument", name)));
    }
    let arg_expr = edn_to_expr(&args[0])?;
    Ok(constructor(Box::new(analyze(arg_expr)?)))
}

fn analyze_binary<F>(name: &str, args: &[EdnValue], constructor: F) -> EqResult<Expr>
where F: Fn(Box<Expr>, Box<Expr>) -> Expr  
{
    if args.len() != 2 {
        return Err(EqError::query_error(format!("{} takes exactly two arguments", name)));
    }
    let left_expr = edn_to_expr(&args[0])?;
    let right_expr = edn_to_expr(&args[1])?;
    Ok(constructor(Box::new(analyze(left_expr)?), Box::new(analyze(right_expr)?)))
}

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

fn analyze_select_keys(args: &[EdnValue]) -> EqResult<Expr> {
    if args.len() != 1 {
        return Err(EqError::query_error("select-keys takes exactly one argument"));
    }
    match &args[0] {
        EdnValue::Vector(keys) => Ok(Expr::SelectKeys(keys.clone())),
        _ => Err(EqError::query_error("select-keys requires a vector argument")),
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
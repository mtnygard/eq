use crate::edn::EdnValue;
use crate::error::{EqError, EqResult};
use crate::query::ast::{FunctionRegistry, Expr};
use indexmap::IndexMap;

/// Initialize the builtin function registry with all standard functions
/// Special forms are added separately in the evaluator module to avoid circular dependencies
pub fn create_builtin_registry() -> FunctionRegistry {
    let mut registry = FunctionRegistry::new();

    // Basic selectors
    registry.register("get".to_string(), builtin_get);
    registry.register("get-in".to_string(), builtin_get_in);

    // Collection operations
    registry.register("first".to_string(), builtin_first);
    registry.register("last".to_string(), builtin_last);
    registry.register("rest".to_string(), builtin_rest);
    registry.register("take".to_string(), builtin_take);
    registry.register("drop".to_string(), builtin_drop);
    registry.register("nth".to_string(), builtin_nth);
    registry.register("count".to_string(), builtin_count);
    registry.register("keys".to_string(), builtin_keys);
    registry.register("vals".to_string(), builtin_vals);

    // Predicates
    registry.register("nil?".to_string(), builtin_is_nil);
    registry.register("empty?".to_string(), builtin_is_empty);
    registry.register("contains?".to_string(), builtin_contains);
    registry.register("number?".to_string(), builtin_is_number);
    registry.register("string?".to_string(), builtin_is_string);
    registry.register("keyword?".to_string(), builtin_is_keyword);
    registry.register("boolean?".to_string(), builtin_is_boolean);

    // Comparison
    registry.register("=".to_string(), builtin_equal);
    registry.register("<".to_string(), builtin_less_than);
    registry.register(">".to_string(), builtin_greater_than);
    registry.register("<=".to_string(), builtin_less_equal);
    registry.register(">=".to_string(), builtin_greater_equal);

    // Higher-order operations
    registry.register("map".to_string(), builtin_map);
    registry.register("remove".to_string(), builtin_remove);
    registry.register("select-keys".to_string(), builtin_select_keys);
    registry.register("select".to_string(), builtin_select);

    // Aggregation
    registry.register("frequencies".to_string(), builtin_frequencies);

    // Threading macros
    registry.register_macro("->".to_string(), macro_thread_first);
    registry.register_macro("->>".to_string(), macro_thread_last);
    
    // Control flow macros
    registry.register_macro("when".to_string(), macro_when);

    registry
}

// Basic selector functions
fn builtin_get(args: &[EdnValue], context: &EdnValue) -> EqResult<EdnValue> {
    match args.len() {
        1 => {
            // (get key) - get key from context
            let key = &args[0];
            Ok(context.get(key).cloned().unwrap_or(EdnValue::Nil))
        }
        2 => {
            // (get map key) - get key from map (threading form)
            let map = &args[0];
            let key = &args[1];
            Ok(map.get(key).cloned().unwrap_or(EdnValue::Nil))
        }
        _ => Err(EqError::query_error("get expects 1 or 2 arguments".to_string())),
    }
}

fn builtin_get_in(args: &[EdnValue], context: &EdnValue) -> EqResult<EdnValue> {
    match args.len() {
        1 => {
            // (get-in path) - get path from context
            match &args[0] {
                EdnValue::Vector(path) => {
                    Ok(context.get_in(path.clone()).cloned().unwrap_or(EdnValue::Nil))
                }
                _ => Err(EqError::type_error("vector", args[0].type_name())),
            }
        }
        2 => {
            // (get-in map path) - get path from map (threading form)
            let map = &args[0];
            match &args[1] {
                EdnValue::Vector(path) => {
                    Ok(map.get_in(path.clone()).cloned().unwrap_or(EdnValue::Nil))
                }
                _ => Err(EqError::type_error("vector", args[1].type_name())),
            }
        }
        _ => Err(EqError::query_error("get-in expects 1 or 2 arguments".to_string())),
    }
}

// Collection operations
fn builtin_first(args: &[EdnValue], _context: &EdnValue) -> EqResult<EdnValue> {
    if args.len() != 1 {
        return Err(EqError::query_error("first expects exactly 1 argument".to_string()));
    }
    
    let target = &args[0];
    
    match target {
        EdnValue::Vector(v) => Ok(v.first().cloned().unwrap_or(EdnValue::Nil)),
        EdnValue::List(l) => Ok(l.first().cloned().unwrap_or(EdnValue::Nil)),
        EdnValue::WithMetadata { value, .. } => builtin_first(&[(**value).clone()], _context),
        _ => Ok(EdnValue::Nil),
    }
}

fn builtin_last(args: &[EdnValue], _context: &EdnValue) -> EqResult<EdnValue> {
    if args.len() != 1 {
        return Err(EqError::query_error("last expects exactly 1 argument".to_string()));
    }
    
    let target = &args[0];

    match target {
        EdnValue::Vector(v) => Ok(v.last().cloned().unwrap_or(EdnValue::Nil)),
        EdnValue::List(l) => Ok(l.last().cloned().unwrap_or(EdnValue::Nil)),
        EdnValue::WithMetadata { value, .. } => builtin_last(&[(**value).clone()], _context),
        _ => Ok(EdnValue::Nil),
    }
}

fn builtin_rest(args: &[EdnValue], _context: &EdnValue) -> EqResult<EdnValue> {
    if args.len() != 1 {
        return Err(EqError::query_error("rest expects exactly 1 argument".to_string()));
    }
    
    let target = &args[0];

    match target {
        EdnValue::Vector(v) => {
            if v.is_empty() {
                Ok(EdnValue::Vector(Vec::new()))
            } else {
                Ok(EdnValue::Vector(v[1..].to_vec()))
            }
        }
        EdnValue::List(l) => {
            if l.is_empty() {
                Ok(EdnValue::List(Vec::new()))
            } else {
                Ok(EdnValue::List(l[1..].to_vec()))
            }
        }
        EdnValue::WithMetadata { value, .. } => builtin_rest(&[(**value).clone()], _context),
        _ => Ok(EdnValue::Vector(Vec::new())),
    }
}

fn builtin_take(args: &[EdnValue], _context: &EdnValue) -> EqResult<EdnValue> {
    if args.len() != 2 {
        return Err(EqError::query_error("take expects exactly 2 arguments".to_string()));
    }
    
    // (take n coll) - take n elements from collection
    if let EdnValue::Integer(count) = &args[0] {
        if *count < 0 {
            return Ok(EdnValue::Vector(Vec::new()));
        }
        
        let count = *count as usize;
        match &args[1] {
            EdnValue::Vector(v) => {
                Ok(EdnValue::Vector(v.iter().take(count).cloned().collect()))
            }
            EdnValue::List(l) => {
                Ok(EdnValue::List(l.iter().take(count).cloned().collect()))
            }
            EdnValue::WithMetadata { value, .. } => builtin_take(&[args[0].clone(), (**value).clone()], _context),
            _ => Ok(EdnValue::Vector(Vec::new())),
        }
    } else {
        Err(EqError::type_error("integer", args[0].type_name()))
    }
}

fn builtin_drop(args: &[EdnValue], _context: &EdnValue) -> EqResult<EdnValue> {
    if args.len() != 2 {
        return Err(EqError::query_error("drop expects exactly 2 arguments".to_string()));
    }
    
    // (drop n coll) - drop n elements from collection
    if let EdnValue::Integer(count) = &args[0] {
        if *count < 0 {
            return Ok(args[1].clone());
        }
        
        let count = *count as usize;
        match &args[1] {
            EdnValue::Vector(v) => {
                Ok(EdnValue::Vector(v.iter().skip(count).cloned().collect()))
            }
            EdnValue::List(l) => {
                Ok(EdnValue::List(l.iter().skip(count).cloned().collect()))
            }
            EdnValue::WithMetadata { value, .. } => builtin_drop(&[args[0].clone(), (**value).clone()], _context),
            _ => Ok(EdnValue::Vector(Vec::new())),
        }
    } else {
        Err(EqError::type_error("integer", args[0].type_name()))
    }
}

fn builtin_nth(args: &[EdnValue], context: &EdnValue) -> EqResult<EdnValue> {
    if args.len() != 1 {
        return Err(EqError::query_error("nth expects 1 argument".to_string()));
    }

    if let EdnValue::Integer(index) = &args[0] {
        Ok(context.get(&EdnValue::Integer(*index)).cloned().unwrap_or(EdnValue::Nil))
    } else {
        Err(EqError::type_error("integer", args[0].type_name()))
    }
}

fn builtin_count(args: &[EdnValue], _context: &EdnValue) -> EqResult<EdnValue> {
    if args.len() != 1 {
        return Err(EqError::query_error("count expects exactly 1 argument".to_string()));
    }
    
    let target = &args[0];

    let count = target.count().unwrap_or(0) as i64;
    Ok(EdnValue::Integer(count))
}

fn builtin_keys(args: &[EdnValue], _context: &EdnValue) -> EqResult<EdnValue> {
    if args.len() != 1 {
        return Err(EqError::query_error("keys expects exactly 1 argument".to_string()));
    }
    
    let target = &args[0];

    match target {
        EdnValue::Map(m) => {
            let keys: Vec<EdnValue> = m.keys().cloned().collect();
            Ok(EdnValue::Vector(keys))
        }
        _ => Ok(EdnValue::Vector(Vec::new())),
    }
}

fn builtin_vals(args: &[EdnValue], _context: &EdnValue) -> EqResult<EdnValue> {
    if args.len() != 1 {
        return Err(EqError::query_error("vals expects exactly 1 argument".to_string()));
    }
    
    let target = &args[0];

    match target {
        EdnValue::Map(m) => {
            let vals: Vec<EdnValue> = m.values().cloned().collect();
            Ok(EdnValue::Vector(vals))
        }
        _ => Ok(EdnValue::Vector(Vec::new())),
    }
}

// Predicates
fn builtin_is_nil(args: &[EdnValue], _context: &EdnValue) -> EqResult<EdnValue> {
    if args.len() != 1 {
        return Err(EqError::query_error("nil? expects exactly 1 argument".to_string()));
    }
    
    let target = &args[0];

    Ok(EdnValue::Bool(matches!(target, EdnValue::Nil)))
}

fn builtin_is_empty(args: &[EdnValue], _context: &EdnValue) -> EqResult<EdnValue> {
    if args.len() != 1 {
        return Err(EqError::query_error("empty? expects exactly 1 argument".to_string()));
    }
    
    let target = &args[0];

    let result = target.count().map(|c| c == 0).unwrap_or(false);
    Ok(EdnValue::Bool(result))
}

fn builtin_contains(args: &[EdnValue], context: &EdnValue) -> EqResult<EdnValue> {
    if args.len() != 1 {
        return Err(EqError::query_error("contains? expects 1 argument".to_string()));
    }

    let key = &args[0];
    let result = match context {
        EdnValue::Map(m) => m.contains_key(key),
        EdnValue::Set(s) => s.contains(key),
        _ => false,
    };
    Ok(EdnValue::Bool(result))
}

fn builtin_is_number(args: &[EdnValue], _context: &EdnValue) -> EqResult<EdnValue> {
    if args.len() != 1 {
        return Err(EqError::query_error("number? expects exactly 1 argument".to_string()));
    }
    
    let target = &args[0];

    Ok(EdnValue::Bool(matches!(target, EdnValue::Integer(_) | EdnValue::Float(_))))
}

fn builtin_is_string(args: &[EdnValue], _context: &EdnValue) -> EqResult<EdnValue> {
    if args.len() != 1 {
        return Err(EqError::query_error("string? expects exactly 1 argument".to_string()));
    }
    
    let target = &args[0];

    Ok(EdnValue::Bool(matches!(target, EdnValue::String(_))))
}

fn builtin_is_keyword(args: &[EdnValue], _context: &EdnValue) -> EqResult<EdnValue> {
    if args.len() != 1 {
        return Err(EqError::query_error("keyword? expects exactly 1 argument".to_string()));
    }
    
    let target = &args[0];

    Ok(EdnValue::Bool(matches!(target, EdnValue::Keyword(_))))
}

fn builtin_is_boolean(args: &[EdnValue], _context: &EdnValue) -> EqResult<EdnValue> {
    if args.len() != 1 {
        return Err(EqError::query_error("boolean? expects exactly 1 argument".to_string()));
    }
    
    let target = &args[0];

    Ok(EdnValue::Bool(matches!(target, EdnValue::Bool(_))))
}

// Comparison
fn builtin_equal(args: &[EdnValue], _context: &EdnValue) -> EqResult<EdnValue> {
    match args.len() {
        0 | 1 => {
            // (=) or (= a) - vacuously true  
            Ok(EdnValue::Bool(true))
        }
        _ => {
            // (= a b c ...) - all arguments must be equal
            let first = &args[0];
            let all_equal = args.iter().skip(1).all(|arg| arg == first);
            Ok(EdnValue::Bool(all_equal))
        }
    }
}

fn builtin_less_than(args: &[EdnValue], _context: &EdnValue) -> EqResult<EdnValue> {
    match args.len() {
        0 | 1 => {
            // (< ) or (< a) - vacuously true
            Ok(EdnValue::Bool(true))
        }
        _ => {
            // (< a b c ...) - check that a < b < c < ...
            for i in 0..args.len()-1 {
                let result = compare_values(&args[i], &args[i+1])?;
                if result >= 0 {
                    return Ok(EdnValue::Bool(false));
                }
            }
            Ok(EdnValue::Bool(true))
        }
    }
}

fn builtin_greater_than(args: &[EdnValue], _context: &EdnValue) -> EqResult<EdnValue> {
    match args.len() {
        0 | 1 => {
            // (> ) or (> a) - vacuously true
            Ok(EdnValue::Bool(true))
        }
        _ => {
            // (> a b c ...) - check that a > b > c > ...
            for i in 0..args.len()-1 {
                let result = compare_values(&args[i], &args[i+1])?;
                if result <= 0 {
                    return Ok(EdnValue::Bool(false));
                }
            }
            Ok(EdnValue::Bool(true))
        }
    }
}

fn builtin_less_equal(args: &[EdnValue], _context: &EdnValue) -> EqResult<EdnValue> {
    match args.len() {
        0 | 1 => {
            // (<= ) or (<= a) - vacuously true
            Ok(EdnValue::Bool(true))
        }
        _ => {
            // (<= a b c ...) - check that a <= b <= c <= ...
            for i in 0..args.len()-1 {
                let result = compare_values(&args[i], &args[i+1])?;
                if result > 0 {
                    return Ok(EdnValue::Bool(false));
                }
            }
            Ok(EdnValue::Bool(true))
        }
    }
}

fn builtin_greater_equal(args: &[EdnValue], _context: &EdnValue) -> EqResult<EdnValue> {
    match args.len() {
        0 | 1 => {
            // (>= ) or (>= a) - vacuously true
            Ok(EdnValue::Bool(true))
        }
        _ => {
            // (>= a b c ...) - check that a >= b >= c >= ...
            for i in 0..args.len()-1 {
                let result = compare_values(&args[i], &args[i+1])?;
                if result < 0 {
                    return Ok(EdnValue::Bool(false));
                }
            }
            Ok(EdnValue::Bool(true))
        }
    }
}

// Higher-order operations (placeholders for now - would need evaluator reference)
fn builtin_map(_args: &[EdnValue], _context: &EdnValue) -> EqResult<EdnValue> {
    Err(EqError::query_error("map not yet implemented with new function system".to_string()))
}

fn builtin_remove(_args: &[EdnValue], _context: &EdnValue) -> EqResult<EdnValue> {
    Err(EqError::query_error("remove not yet implemented with new function system".to_string()))
}

fn builtin_select_keys(args: &[EdnValue], context: &EdnValue) -> EqResult<EdnValue> {
    if args.len() != 1 {
        return Err(EqError::query_error("select-keys expects 1 argument".to_string()));
    }

    let keys = match &args[0] {
        EdnValue::Vector(keys) => keys,
        EdnValue::List(keys) => keys,
        _ => return Err(EqError::type_error("vector or list", args[0].type_name())),
    };

    if let EdnValue::Map(m) = context {
        let mut result = IndexMap::new();
        for key in keys {
            if let Some(value) = m.get(key) {
                result.insert(key.clone(), value.clone());
            }
        }
        Ok(EdnValue::Map(result))
    } else {
        Ok(EdnValue::Map(IndexMap::new()))
    }
}

fn builtin_select(_args: &[EdnValue], _context: &EdnValue) -> EqResult<EdnValue> {
    Err(EqError::query_error("select not yet implemented with new function system".to_string()))
}

// Aggregation
fn builtin_frequencies(args: &[EdnValue], _context: &EdnValue) -> EqResult<EdnValue> {
    if args.len() != 1 {
        return Err(EqError::query_error("frequencies expects exactly 1 argument".to_string()));
    }
    
    let target = &args[0];

    match target {
        EdnValue::Vector(v) => {
            let mut freq_map = IndexMap::new();
            for item in v {
                let count = freq_map.get(item).cloned().unwrap_or(EdnValue::Integer(0));
                if let EdnValue::Integer(n) = count {
                    freq_map.insert(item.clone(), EdnValue::Integer(n + 1));
                }
            }
            Ok(EdnValue::Map(freq_map))
        }
        EdnValue::List(l) => {
            let mut freq_map = IndexMap::new();
            for item in l {
                let count = freq_map.get(item).cloned().unwrap_or(EdnValue::Integer(0));
                if let EdnValue::Integer(n) = count {
                    freq_map.insert(item.clone(), EdnValue::Integer(n + 1));
                }
            }
            Ok(EdnValue::Map(freq_map))
        }
        _ => Ok(EdnValue::Map(IndexMap::new())),
    }
}

/// Compare two values for ordering
fn compare_values(left: &EdnValue, right: &EdnValue) -> EqResult<i32> {
    match (left, right) {
        (EdnValue::Integer(a), EdnValue::Integer(b)) => Ok(a.cmp(b) as i32),
        (EdnValue::Float(a), EdnValue::Float(b)) => {
            if a < b { Ok(-1) }
            else if a > b { Ok(1) }
            else { Ok(0) }
        }
        (EdnValue::Integer(a), EdnValue::Float(b)) => {
            let a_float = *a as f64;
            if a_float < *b { Ok(-1) }
            else if a_float > *b { Ok(1) }
            else { Ok(0) }
        }
        (EdnValue::Float(a), EdnValue::Integer(b)) => {
            let b_float = *b as f64;
            if *a < b_float { Ok(-1) }
            else if *a > b_float { Ok(1) }
            else { Ok(0) }
        }
        (EdnValue::String(a), EdnValue::String(b)) => Ok(a.cmp(b) as i32),
        _ => Err(EqError::type_error("comparable types", 
            &format!("{} and {}", left.type_name(), right.type_name()))),
    }
}

// Macro implementations

/// When macro: (when cond body-exprs) => (if cond (do body-exprs) nil)
fn macro_when(args: &[Expr]) -> EqResult<Expr> {
    if args.len() < 2 {
        return Err(EqError::query_error("when macro requires at least 2 arguments"));
    }
    
    // Extract condition and body expressions
    let condition = args[0].clone();
    let body_exprs = args[1..].to_vec();
    
    // Create (do body-exprs)
    let do_expr = Expr::Function {
        name: "do".to_string(),
        args: body_exprs,
    };
    
    // Create (if cond (do body-exprs) nil)
    Ok(Expr::Function {
        name: "if".to_string(),
        args: vec![
            condition,
            do_expr,
            Expr::Literal(EdnValue::Nil),
        ],
    })
}

/// Threading first macro: (-> x f g h) becomes (h (g (f x)))
fn macro_thread_first(args: &[Expr]) -> EqResult<Expr> {
    if args.is_empty() {
        return Err(EqError::query_error("-> macro requires at least one argument"));
    }
    
    let mut result = args[0].clone();
    
    // Thread through each subsequent form
    for form in args.iter().skip(1) {
        result = thread_first_expr(result, form)?;
    }
    
    Ok(result)
}

/// Threading last macro: (->> x f g h) becomes (h (g (f x))) but arguments go at the end
fn macro_thread_last(args: &[Expr]) -> EqResult<Expr> {
    if args.is_empty() {
        return Err(EqError::query_error("->> macro requires at least one argument"));
    }
    
    let mut result = args[0].clone();
    
    // Thread through each subsequent form
    for form in args.iter().skip(1) {
        result = thread_last_expr(result, form)?;
    }
    
    Ok(result)
}

/// Thread first: insert threaded value as first argument
fn thread_first_expr(threaded_value: Expr, form: &Expr) -> EqResult<Expr> {
    match form {
        // If it's a symbol like 'first', convert to (first threaded_value)
        Expr::Symbol(name) => {
            Ok(Expr::Function {
                name: name.clone(),
                args: vec![threaded_value],
            })
        }
        
        // If it's a keyword access like :name, convert to (:name threaded_value)
        Expr::KeywordAccess(name) => {
            Ok(Expr::KeywordGet(name.clone(), Box::new(threaded_value)))
        }
        
        // If it's a function call like (take 3), convert to (take threaded_value 3)
        Expr::Function { name, args } => {
            let mut new_args = vec![threaded_value];
            new_args.extend_from_slice(args);
            Ok(Expr::Function {
                name: name.clone(),
                args: new_args,
            })
        }
        
        // If it's a raw list like (= 42) or (:name), convert to analyzed form first
        Expr::List(elements) if !elements.is_empty() => {
            match &elements[0] {
                EdnValue::Symbol(name) => {
                    // It's a function call like (= 42)
                    // Convert remaining elements to expressions
                    let args: Result<Vec<Expr>, EqError> = elements[1..]
                        .iter()
                        .map(|edn| Ok(Expr::Literal(edn.clone())))
                        .collect();
                    let args = args?;
                    
                    // Insert threaded value as first argument
                    let mut new_args = vec![threaded_value];
                    new_args.extend(args);
                    
                    Ok(Expr::Function {
                        name: name.clone(),
                        args: new_args,
                    })
                }
                EdnValue::Keyword(name) => {
                    // It's a keyword accessor like (:name)
                    // Thread the value into the keyword get
                    Ok(Expr::KeywordGet(name.clone(), Box::new(threaded_value)))
                }
                _ => {
                    Err(EqError::query_error("Invalid form in -> macro: list must start with symbol or keyword"))
                }
            }
        }
        
        _ => Err(EqError::query_error("Invalid form in -> macro")),
    }
}

/// Thread last: insert threaded value as last argument
fn thread_last_expr(threaded_value: Expr, form: &Expr) -> EqResult<Expr> {
    match form {
        // If it's a symbol like 'first', convert to (first threaded_value)
        Expr::Symbol(name) => {
            Ok(Expr::Function {
                name: name.clone(),
                args: vec![threaded_value],
            })
        }
        
        // If it's a keyword access like :name, convert to (:name threaded_value)
        Expr::KeywordAccess(name) => {
            Ok(Expr::KeywordGet(name.clone(), Box::new(threaded_value)))
        }
        
        // If it's a function call like (take 3), convert to (take 3 threaded_value)
        Expr::Function { name, args } => {
            let mut new_args = args.clone();
            new_args.push(threaded_value);
            Ok(Expr::Function {
                name: name.clone(),
                args: new_args,
            })
        }
        
        // If it's a raw list like (= 42) or (:name), convert to analyzed form first
        Expr::List(elements) if !elements.is_empty() => {
            match &elements[0] {
                EdnValue::Symbol(name) => {
                    // It's a function call like (= 42)
                    // Convert remaining elements to expressions
                    let args: Result<Vec<Expr>, EqError> = elements[1..]
                        .iter()
                        .map(|edn| Ok(Expr::Literal(edn.clone())))
                        .collect();
                    let mut args = args?;
                    
                    // Insert threaded value as last argument
                    args.push(threaded_value);
                    
                    Ok(Expr::Function {
                        name: name.clone(),
                        args,
                    })
                }
                EdnValue::Keyword(name) => {
                    // It's a keyword accessor like (:name)
                    // Thread the value into the keyword get (same as thread-first for keywords)
                    Ok(Expr::KeywordGet(name.clone(), Box::new(threaded_value)))
                }
                _ => {
                    Err(EqError::query_error("Invalid form in ->> macro: list must start with symbol or keyword"))
                }
            }
        }
        
        _ => Err(EqError::query_error("Invalid form in ->> macro")),
    }
}


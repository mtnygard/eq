use crate::edn::EdnValue;
use crate::error::{EqError, EqResult};
use crate::query::ast::FunctionRegistry;
use indexmap::IndexMap;

/// Initialize the builtin function registry with all standard functions
pub fn create_builtin_registry() -> FunctionRegistry {
    let mut registry = FunctionRegistry::new();

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

    registry
}

// Collection operations
fn builtin_first(args: &[EdnValue], context: &EdnValue) -> EqResult<EdnValue> {
    // For collection functions, if we get one argument and it's the same as context,
    // we're operating on the context (this handles (first .) pattern)
    let target = if args.is_empty() {
        context
    } else if args.len() == 1 {
        // Use the provided argument as the target
        &args[0]
    } else {
        return Err(EqError::query_error("first expects 0 or 1 argument".to_string()));
    };
    
    match target {
        EdnValue::Vector(v) => Ok(v.first().cloned().unwrap_or(EdnValue::Nil)),
        EdnValue::List(l) => Ok(l.first().cloned().unwrap_or(EdnValue::Nil)),
        EdnValue::WithMetadata { value, .. } => builtin_first(&[(**value).clone()], context),
        _ => Ok(EdnValue::Nil),
    }
}

fn builtin_last(args: &[EdnValue], context: &EdnValue) -> EqResult<EdnValue> {
    let target = if args.is_empty() {
        context
    } else if args.len() == 1 {
        &args[0]
    } else {
        return Err(EqError::query_error("last expects 0 or 1 argument".to_string()));
    };

    match target {
        EdnValue::Vector(v) => Ok(v.last().cloned().unwrap_or(EdnValue::Nil)),
        EdnValue::List(l) => Ok(l.last().cloned().unwrap_or(EdnValue::Nil)),
        EdnValue::WithMetadata { value, .. } => builtin_last(&[(**value).clone()], context),
        _ => Ok(EdnValue::Nil),
    }
}

fn builtin_rest(args: &[EdnValue], context: &EdnValue) -> EqResult<EdnValue> {
    let target = if args.is_empty() {
        context
    } else if args.len() == 1 {
        &args[0]
    } else {
        return Err(EqError::query_error("rest expects 0 or 1 argument".to_string()));
    };

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
        EdnValue::WithMetadata { value, .. } => builtin_rest(&[(**value).clone()], context),
        _ => Ok(EdnValue::Vector(Vec::new())),
    }
}

fn builtin_take(args: &[EdnValue], context: &EdnValue) -> EqResult<EdnValue> {
    if args.len() != 1 {
        return Err(EqError::query_error("take expects 1 argument".to_string()));
    }

    if let EdnValue::Integer(count) = &args[0] {
        if *count < 0 {
            return Ok(EdnValue::Vector(Vec::new()));
        }
        
        let count = *count as usize;
        match context {
            EdnValue::Vector(v) => {
                Ok(EdnValue::Vector(v.iter().take(count).cloned().collect()))
            }
            EdnValue::List(l) => {
                Ok(EdnValue::List(l.iter().take(count).cloned().collect()))
            }
            EdnValue::WithMetadata { value, .. } => builtin_take(args, value),
            _ => Ok(EdnValue::Vector(Vec::new())),
        }
    } else {
        Err(EqError::type_error("integer", args[0].type_name()))
    }
}

fn builtin_drop(args: &[EdnValue], context: &EdnValue) -> EqResult<EdnValue> {
    if args.len() != 1 {
        return Err(EqError::query_error("drop expects 1 argument".to_string()));
    }

    if let EdnValue::Integer(count) = &args[0] {
        if *count < 0 {
            return Ok(context.clone());
        }
        
        let count = *count as usize;
        match context {
            EdnValue::Vector(v) => {
                Ok(EdnValue::Vector(v.iter().skip(count).cloned().collect()))
            }
            EdnValue::List(l) => {
                Ok(EdnValue::List(l.iter().skip(count).cloned().collect()))
            }
            EdnValue::WithMetadata { value, .. } => builtin_drop(args, value),
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

fn builtin_count(args: &[EdnValue], context: &EdnValue) -> EqResult<EdnValue> {
    let target = if args.is_empty() {
        context
    } else if args.len() == 1 {
        &args[0]
    } else {
        return Err(EqError::query_error("count expects 0 or 1 argument".to_string()));
    };

    let count = target.count().unwrap_or(0) as i64;
    Ok(EdnValue::Integer(count))
}

fn builtin_keys(args: &[EdnValue], context: &EdnValue) -> EqResult<EdnValue> {
    let target = if args.is_empty() {
        context
    } else if args.len() == 1 {
        &args[0]
    } else {
        return Err(EqError::query_error("keys expects 0 or 1 argument".to_string()));
    };

    match target {
        EdnValue::Map(m) => {
            let keys: Vec<EdnValue> = m.keys().cloned().collect();
            Ok(EdnValue::Vector(keys))
        }
        _ => Ok(EdnValue::Vector(Vec::new())),
    }
}

fn builtin_vals(args: &[EdnValue], context: &EdnValue) -> EqResult<EdnValue> {
    let target = if args.is_empty() {
        context
    } else if args.len() == 1 {
        &args[0]
    } else {
        return Err(EqError::query_error("vals expects 0 or 1 argument".to_string()));
    };

    match target {
        EdnValue::Map(m) => {
            let vals: Vec<EdnValue> = m.values().cloned().collect();
            Ok(EdnValue::Vector(vals))
        }
        _ => Ok(EdnValue::Vector(Vec::new())),
    }
}

// Predicates
fn builtin_is_nil(args: &[EdnValue], context: &EdnValue) -> EqResult<EdnValue> {
    let target = if args.is_empty() {
        context
    } else if args.len() == 1 {
        &args[0]
    } else {
        return Err(EqError::query_error("nil? expects 0 or 1 argument".to_string()));
    };

    Ok(EdnValue::Bool(matches!(target, EdnValue::Nil)))
}

fn builtin_is_empty(args: &[EdnValue], context: &EdnValue) -> EqResult<EdnValue> {
    let target = if args.is_empty() {
        context
    } else if args.len() == 1 {
        &args[0]
    } else {
        return Err(EqError::query_error("empty? expects 0 or 1 argument".to_string()));
    };

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

fn builtin_is_number(args: &[EdnValue], context: &EdnValue) -> EqResult<EdnValue> {
    let target = if args.is_empty() {
        context
    } else if args.len() == 1 {
        &args[0]
    } else {
        return Err(EqError::query_error("number? expects 0 or 1 argument".to_string()));
    };

    Ok(EdnValue::Bool(matches!(target, EdnValue::Integer(_) | EdnValue::Float(_))))
}

fn builtin_is_string(args: &[EdnValue], context: &EdnValue) -> EqResult<EdnValue> {
    let target = if args.is_empty() {
        context
    } else if args.len() == 1 {
        &args[0]
    } else {
        return Err(EqError::query_error("string? expects 0 or 1 argument".to_string()));
    };

    Ok(EdnValue::Bool(matches!(target, EdnValue::String(_))))
}

fn builtin_is_keyword(args: &[EdnValue], context: &EdnValue) -> EqResult<EdnValue> {
    let target = if args.is_empty() {
        context
    } else if args.len() == 1 {
        &args[0]
    } else {
        return Err(EqError::query_error("keyword? expects 0 or 1 argument".to_string()));
    };

    Ok(EdnValue::Bool(matches!(target, EdnValue::Keyword(_))))
}

fn builtin_is_boolean(args: &[EdnValue], context: &EdnValue) -> EqResult<EdnValue> {
    let target = if args.is_empty() {
        context
    } else if args.len() == 1 {
        &args[0]
    } else {
        return Err(EqError::query_error("boolean? expects 0 or 1 argument".to_string()));
    };

    Ok(EdnValue::Bool(matches!(target, EdnValue::Bool(_))))
}

// Comparison
fn builtin_equal(args: &[EdnValue], context: &EdnValue) -> EqResult<EdnValue> {
    if args.len() != 1 {
        return Err(EqError::query_error("= expects 1 argument".to_string()));
    }

    Ok(EdnValue::Bool(context == &args[0]))
}

fn builtin_less_than(args: &[EdnValue], context: &EdnValue) -> EqResult<EdnValue> {
    if args.len() != 1 {
        return Err(EqError::query_error("< expects 1 argument".to_string()));
    }

    let result = compare_values(context, &args[0])? < 0;
    Ok(EdnValue::Bool(result))
}

fn builtin_greater_than(args: &[EdnValue], context: &EdnValue) -> EqResult<EdnValue> {
    if args.len() != 1 {
        return Err(EqError::query_error("> expects 1 argument".to_string()));
    }

    let result = compare_values(context, &args[0])? > 0;
    Ok(EdnValue::Bool(result))
}

fn builtin_less_equal(args: &[EdnValue], context: &EdnValue) -> EqResult<EdnValue> {
    if args.len() != 1 {
        return Err(EqError::query_error("<= expects 1 argument".to_string()));
    }

    let result = compare_values(context, &args[0])? <= 0;
    Ok(EdnValue::Bool(result))
}

fn builtin_greater_equal(args: &[EdnValue], context: &EdnValue) -> EqResult<EdnValue> {
    if args.len() != 1 {
        return Err(EqError::query_error(">= expects 1 argument".to_string()));
    }

    let result = compare_values(context, &args[0])? >= 0;
    Ok(EdnValue::Bool(result))
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
fn builtin_frequencies(args: &[EdnValue], context: &EdnValue) -> EqResult<EdnValue> {
    let target = if args.is_empty() {
        context
    } else if args.len() == 1 {
        &args[0]
    } else {
        return Err(EqError::query_error("frequencies expects 0 or 1 argument".to_string()));
    };

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
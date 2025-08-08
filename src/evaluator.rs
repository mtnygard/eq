use crate::edn::EdnValue;
use crate::error::{EqError, EqResult};
use crate::query::ast::Expr;
use indexmap::IndexMap;


/// Direct AST evaluator that treats expressions as functions
/// Each expression takes a context (current data) and returns a value
pub fn evaluate(expr: &Expr, context: &EdnValue) -> EqResult<EdnValue> {
    match expr {
        Expr::Identity => Ok(context.clone()),
        
        Expr::Get(key) => {
            Ok(context.get(key).cloned().unwrap_or(EdnValue::Nil))
        }
        
        Expr::GetIn(path) => {
            Ok(context.get_in(path.clone()).cloned().unwrap_or(EdnValue::Nil))
        }
        
        Expr::KeywordAccess(name) => {
            let key = EdnValue::Keyword(name.clone());
            Ok(context.get(&key).cloned().unwrap_or(EdnValue::Nil))
        }
        
        Expr::KeywordGet(name, expr) => {
            let target = evaluate(expr, context)?;
            let key = EdnValue::Keyword(name.clone());
            Ok(target.get(&key).cloned().unwrap_or(EdnValue::Nil))
        }
        
        Expr::KeywordGetWithDefault(name, expr, default_expr) => {
            let target = evaluate(expr, context)?;
            let key = EdnValue::Keyword(name.clone());
            match target.get(&key) {
                Some(value) => Ok(value.clone()),
                None => evaluate(default_expr, context),
            }
        }
        
        // Collection operations
        Expr::First(coll_expr) => {
            let coll = evaluate(coll_expr, context)?;
            match coll {
                EdnValue::Vector(v) => Ok(v.first().cloned().unwrap_or(EdnValue::Nil)),
                EdnValue::List(l) => Ok(l.first().cloned().unwrap_or(EdnValue::Nil)),
                EdnValue::WithMetadata { value, .. } => {
                    let inner_expr = Expr::First(Box::new(Expr::Literal(*value)));
                    evaluate(&inner_expr, context)
                }
                _ => Ok(EdnValue::Nil),
            }
        }
        
        Expr::Last(coll_expr) => {
            let coll = evaluate(coll_expr, context)?;
            match coll {
                EdnValue::Vector(v) => Ok(v.last().cloned().unwrap_or(EdnValue::Nil)),
                EdnValue::List(l) => Ok(l.last().cloned().unwrap_or(EdnValue::Nil)),
                EdnValue::WithMetadata { value, .. } => {
                    let inner_expr = Expr::Last(Box::new(Expr::Literal(*value)));
                    evaluate(&inner_expr, context)
                }
                _ => Ok(EdnValue::Nil),
            }
        }
        
        Expr::Rest(coll_expr) => {
            let coll = evaluate(coll_expr, context)?;
            match coll {
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
                EdnValue::WithMetadata { value, .. } => {
                    let inner_expr = Expr::Rest(Box::new(Expr::Literal(*value)));
                    evaluate(&inner_expr, context)
                }
                _ => Ok(EdnValue::Vector(Vec::new())),
            }
        }
        
        Expr::Take(n_expr, coll_expr) => {
            let n = evaluate(n_expr, context)?;
            let coll = evaluate(coll_expr, context)?;
            if let EdnValue::Integer(count) = n {
                if count < 0 {
                    return Ok(EdnValue::Vector(Vec::new()));
                }
                
                let count = count as usize;
                match coll {
                    EdnValue::Vector(v) => {
                        Ok(EdnValue::Vector(v.iter().take(count).cloned().collect()))
                    }
                    EdnValue::List(l) => {
                        Ok(EdnValue::List(l.iter().take(count).cloned().collect()))
                    }
                    EdnValue::WithMetadata { value, .. } => {
                        let inner_expr = Expr::Take(n_expr.clone(), Box::new(Expr::Literal(*value)));
                        evaluate(&inner_expr, context)
                    }
                    _ => Ok(EdnValue::Vector(Vec::new())),
                }
            } else {
                Err(EqError::type_error("integer", n.type_name()))
            }
        }
        
        Expr::Drop(n_expr, coll_expr) => {
            let n = evaluate(n_expr, context)?;
            let coll = evaluate(coll_expr, context)?;
            if let EdnValue::Integer(count) = n {
                if count < 0 {
                    return Ok(coll.clone());
                }
                
                let count = count as usize;
                match coll {
                    EdnValue::Vector(v) => {
                        Ok(EdnValue::Vector(v.iter().skip(count).cloned().collect()))
                    }
                    EdnValue::List(l) => {
                        Ok(EdnValue::List(l.iter().skip(count).cloned().collect()))
                    }
                    EdnValue::WithMetadata { value, .. } => {
                        let inner_expr = Expr::Drop(n_expr.clone(), Box::new(Expr::Literal(*value)));
                        evaluate(&inner_expr, context)
                    }
                    _ => Ok(EdnValue::Vector(Vec::new())),
                }
            } else {
                Err(EqError::type_error("integer", n.type_name()))
            }
        }
        
        Expr::Nth(n_expr, coll_expr) => {
            let n = evaluate(n_expr, context)?;
            let coll = evaluate(coll_expr, context)?;
            if let EdnValue::Integer(index) = n {
                Ok(coll.get(&EdnValue::Integer(index)).cloned().unwrap_or(EdnValue::Nil))
            } else {
                Err(EqError::type_error("integer", n.type_name()))
            }
        }
        
        Expr::Count(coll_expr) => {
            let coll = evaluate(coll_expr, context)?;
            let count = coll.count().unwrap_or(0) as i64;
            Ok(EdnValue::Integer(count))
        }
        
        Expr::Keys(coll_expr) => {
            let coll = evaluate(coll_expr, context)?;
            match coll {
                EdnValue::Map(m) => {
                    let keys: Vec<EdnValue> = m.keys().cloned().collect();
                    Ok(EdnValue::Vector(keys))
                }
                _ => Ok(EdnValue::Vector(Vec::new())),
            }
        }
        
        Expr::Vals(coll_expr) => {
            let coll = evaluate(coll_expr, context)?;
            match coll {
                EdnValue::Map(m) => {
                    let vals: Vec<EdnValue> = m.values().cloned().collect();
                    Ok(EdnValue::Vector(vals))
                }
                _ => Ok(EdnValue::Vector(Vec::new())),
            }
        }
        
        // Higher-order operations
        Expr::Map(func_expr) => {
            match context {
                EdnValue::Vector(v) => {
                    let mut results = Vec::new();
                    for item in v {
                        let result = evaluate(func_expr, item)?;
                        results.push(result);
                    }
                    Ok(EdnValue::Vector(results))
                }
                EdnValue::List(l) => {
                    let mut results = Vec::new();
                    for item in l {
                        let result = evaluate(func_expr, item)?;
                        results.push(result);
                    }
                    Ok(EdnValue::List(results))
                }
                _ => Ok(context.clone()),
            }
        }
        
        Expr::Remove(pred_expr) => {
            match context {
                EdnValue::Vector(v) => {
                    let mut results = Vec::new();
                    for item in v {
                        let pred_result = evaluate(pred_expr, item)?;
                        if !pred_result.is_truthy() {
                            results.push(item.clone());
                        }
                    }
                    Ok(EdnValue::Vector(results))
                }
                EdnValue::List(l) => {
                    let mut results = Vec::new();
                    for item in l {
                        let pred_result = evaluate(pred_expr, item)?;
                        if !pred_result.is_truthy() {
                            results.push(item.clone());
                        }
                    }
                    Ok(EdnValue::List(results))
                }
                _ => Ok(context.clone()),
            }
        }
        
        Expr::SelectKeys(keys) => {
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
        
        Expr::Select(pred_expr) => {
            let pred_result = evaluate(pred_expr, context)?;
            if pred_result.is_truthy() {
                Ok(context.clone())
            } else {
                Ok(EdnValue::Nil)
            }
        }
        
        // Predicates
        Expr::IsNil => {
            Ok(EdnValue::Bool(matches!(context, EdnValue::Nil)))
        }
        
        Expr::IsEmpty => {
            let result = context.count().map(|c| c == 0).unwrap_or(false);
            Ok(EdnValue::Bool(result))
        }
        
        Expr::Contains(key_expr) => {
            let key = evaluate(key_expr, context)?;
            let result = match context {
                EdnValue::Map(m) => m.contains_key(&key),
                EdnValue::Set(s) => s.contains(&key),
                _ => false,
            };
            Ok(EdnValue::Bool(result))
        }
        
        Expr::IsNumber => {
            Ok(EdnValue::Bool(matches!(context, EdnValue::Integer(_) | EdnValue::Float(_))))
        }
        
        Expr::IsString => {
            Ok(EdnValue::Bool(matches!(context, EdnValue::String(_))))
        }
        
        Expr::IsKeyword => {
            Ok(EdnValue::Bool(matches!(context, EdnValue::Keyword(_))))
        }
        
        Expr::IsBoolean => {
            Ok(EdnValue::Bool(matches!(context, EdnValue::Bool(_))))
        }
        
        // Comparison
        Expr::Equal(left_expr, right_expr) => {
            let left = evaluate(left_expr, context)?;
            let right = evaluate(right_expr, context)?;
            Ok(EdnValue::Bool(left == right))
        }
        
        Expr::LessThan(val_expr) => {
            let other = evaluate(val_expr, context)?;
            let result = compare_values(context, &other)? < 0;
            Ok(EdnValue::Bool(result))
        }
        
        Expr::GreaterThan(val_expr) => {
            let other = evaluate(val_expr, context)?;
            let result = compare_values(context, &other)? > 0;
            Ok(EdnValue::Bool(result))
        }
        
        Expr::LessEqual(val_expr) => {
            let other = evaluate(val_expr, context)?;
            let result = compare_values(context, &other)? <= 0;
            Ok(EdnValue::Bool(result))
        }
        
        Expr::GreaterEqual(val_expr) => {
            let other = evaluate(val_expr, context)?;
            let result = compare_values(context, &other)? >= 0;
            Ok(EdnValue::Bool(result))
        }
        
        // Composition - evaluate expressions in sequence
        Expr::Comp(exprs) => {
            let mut result = context.clone();
            for expr in exprs {
                result = evaluate(expr, &result)?;
            }
            Ok(result)
        }
        
        // Conditionals
        Expr::If { test, then_expr, else_expr } => {
            let test_result = evaluate(test, context)?;
            if test_result.is_truthy() {
                evaluate(then_expr, context)
            } else if let Some(else_expr) = else_expr {
                evaluate(else_expr, context)
            } else {
                Ok(EdnValue::Nil)
            }
        }
        
        // Aggregation
        Expr::Reduce { func: _, init: _ } => {
            // Placeholder - would need more complex implementation
            Err(EqError::query_error("reduce not yet implemented".to_string()))
        }
        
        Expr::Apply(_func_expr) => {
            // Placeholder - would need function application semantics
            Err(EqError::query_error("apply not yet implemented".to_string()))
        }
        
        Expr::GroupBy(_func_expr) => {
            // Placeholder - would need grouping logic
            Err(EqError::query_error("group-by not yet implemented".to_string()))
        }
        
        Expr::Frequencies => {
            match context {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity() {
        let input = EdnValue::Integer(42);
        let result = evaluate(&Expr::Identity, &input).unwrap();
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
        
        let result = evaluate(&Expr::First(Box::new(Expr::Identity)), &input).unwrap();
        assert_eq!(result, EdnValue::Integer(1));
    }

    #[test]
    fn test_count() {
        let input = EdnValue::Vector(vec![
            EdnValue::Integer(1),
            EdnValue::Integer(2),
            EdnValue::Integer(3),
        ]);
        
        let result = evaluate(&Expr::Count(Box::new(Expr::Identity)), &input).unwrap();
        assert_eq!(result, EdnValue::Integer(3));
    }

    #[test]
    fn test_predicates() {
        // Test is-nil
        let result = evaluate(&Expr::IsNil, &EdnValue::Nil).unwrap();
        assert_eq!(result, EdnValue::Bool(true));
        
        let result = evaluate(&Expr::IsNil, &EdnValue::Integer(42)).unwrap();
        assert_eq!(result, EdnValue::Bool(false));
        
        // Test is-number
        let result = evaluate(&Expr::IsNumber, &EdnValue::Integer(42)).unwrap();
        assert_eq!(result, EdnValue::Bool(true));
        
        let result = evaluate(&Expr::IsNumber, &EdnValue::String("hello".to_string())).unwrap();
        assert_eq!(result, EdnValue::Bool(false));
    }

    #[test]
    fn test_comparison() {
        // Test equality
        let expr = Expr::Equal(
            Box::new(Expr::Identity),
            Box::new(Expr::Literal(EdnValue::Integer(42)))
        );
        
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
        let expr = Expr::Take(Box::new(Expr::Literal(EdnValue::Integer(2))), Box::new(Expr::Identity));
        let result = evaluate(&expr, &input).unwrap();
        assert_eq!(result, EdnValue::Vector(vec![
            EdnValue::Integer(1),
            EdnValue::Integer(2),
        ]));
        
        // Test drop
        let expr = Expr::Drop(Box::new(Expr::Literal(EdnValue::Integer(2))), Box::new(Expr::Identity));
        let result = evaluate(&expr, &input).unwrap();
        assert_eq!(result, EdnValue::Vector(vec![
            EdnValue::Integer(3),
            EdnValue::Integer(4),
        ]));
    }

    #[test]
    fn test_composition() {
        // Test composition: first then count (should fail since first returns a single value)
        let expr = Expr::Comp(vec![
            Expr::First(Box::new(Expr::Identity)),
            Expr::Count(Box::new(Expr::Identity)),
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
            test: Box::new(Expr::IsNil),
            then_expr: Box::new(Expr::Literal(EdnValue::String("it's nil".to_string()))),
            else_expr: Some(Box::new(Expr::Literal(EdnValue::String("not nil".to_string())))),
        };
        
        let result = evaluate(&expr, &EdnValue::Nil).unwrap();
        assert_eq!(result, EdnValue::String("it's nil".to_string()));
        
        let result = evaluate(&expr, &EdnValue::Integer(42)).unwrap();
        assert_eq!(result, EdnValue::String("not nil".to_string()));
    }

    #[test]
    fn test_map_operation() {
        let expr = Expr::Map(Box::new(Expr::KeywordAccess("name".to_string())));
        
        let mut map1 = IndexMap::new();
        map1.insert(EdnValue::Keyword("name".to_string()), EdnValue::String("Alice".to_string()));
        let mut map2 = IndexMap::new();
        map2.insert(EdnValue::Keyword("name".to_string()), EdnValue::String("Bob".to_string()));
        
        let input = EdnValue::Vector(vec![
            EdnValue::Map(map1),
            EdnValue::Map(map2),
        ]);
        
        let result = evaluate(&expr, &input).unwrap();
        assert_eq!(result, EdnValue::Vector(vec![
            EdnValue::String("Alice".to_string()),
            EdnValue::String("Bob".to_string()),
        ]));
    }
}
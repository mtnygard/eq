use crate::edn::EdnValue;
use crate::error::{EqError, EqResult};
use crate::query::compiler::{CompiledQuery, OpCode};
use indexmap::IndexMap;
use std::collections::HashSet;

/// Stack-based virtual machine for executing query bytecode
pub struct QueryVM {
    stack: Vec<EdnValue>,
    pc: usize, // program counter
}

impl QueryVM {
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            pc: 0,
        }
    }

    /// Execute a compiled query on the given input
    pub fn execute(&mut self, query: &CompiledQuery, input: EdnValue) -> EqResult<EdnValue> {
        self.stack.clear();
        self.pc = 0;

        // Push initial input onto stack
        self.stack.push(input);

        while self.pc < query.bytecode.len() {
            let op = &query.bytecode[self.pc];
            self.execute_instruction(op, &query.constants)?;
            self.pc += 1;
        }

        // Return the top of the stack, or nil if empty
        Ok(self.stack.pop().unwrap_or(EdnValue::Nil))
    }

    fn execute_instruction(&mut self, op: &OpCode, constants: &[EdnValue]) -> EqResult<()> {
        match op {
            OpCode::Push(const_idx) => {
                let value = constants[*const_idx].clone();
                self.stack.push(value);
            }

            OpCode::Pop => {
                self.stack.pop();
            }

            OpCode::Dup => {
                if let Some(top) = self.stack.last() {
                    self.stack.push(top.clone());
                }
            }

            OpCode::Identity => {
                // No-op - value stays on stack
            }

            OpCode::Get => {
                let key = self.stack.pop().ok_or_else(|| EqError::runtime_error_str("get", "Missing key"))?;
                let target = self.stack.pop().ok_or_else(|| EqError::runtime_error_str("get", "Missing target"))?;
                
                let result = target.get(&key).cloned().unwrap_or(EdnValue::Nil);
                self.stack.push(result);
            }

            OpCode::GetIn => {
                let path = self.stack.pop().ok_or_else(|| EqError::runtime_error_str("get-in", "Missing path"))?;
                let target = self.stack.pop().ok_or_else(|| EqError::runtime_error_str("get-in", "Missing target"))?;
                
                if let EdnValue::Vector(path_vec) = path {
                    let result = target.get_in(path_vec).cloned().unwrap_or(EdnValue::Nil);
                    self.stack.push(result);
                } else {
                    return Err(EqError::type_error("vector", path.type_name()));
                }
            }

            OpCode::KeywordGet(keyword) => {
                let target = self.stack.pop().ok_or_else(|| EqError::runtime_error_str("keyword-get", "Missing target"))?;
                let key = EdnValue::Keyword(keyword.clone());
                let result = target.get(&key).cloned().unwrap_or(EdnValue::Nil);
                self.stack.push(result);
            }

            OpCode::First => {
                let coll = self.stack.pop().ok_or_else(|| EqError::runtime_error_str("first", "Missing collection"))?;
                let result = match coll {
                    EdnValue::Vector(ref v) => v.first().cloned().unwrap_or(EdnValue::Nil),
                    EdnValue::List(ref l) => l.first().cloned().unwrap_or(EdnValue::Nil),
                    _ => EdnValue::Nil,
                };
                self.stack.push(result);
            }

            OpCode::Last => {
                let coll = self.stack.pop().ok_or_else(|| EqError::runtime_error_str("last", "Missing collection"))?;
                let result = match coll {
                    EdnValue::Vector(ref v) => v.last().cloned().unwrap_or(EdnValue::Nil),
                    EdnValue::List(ref l) => l.last().cloned().unwrap_or(EdnValue::Nil),
                    _ => EdnValue::Nil,
                };
                self.stack.push(result);
            }

            OpCode::Rest => {
                let coll = self.stack.pop().ok_or_else(|| EqError::runtime_error_str("rest", "Missing collection"))?;
                let result = match coll {
                    EdnValue::Vector(ref v) => {
                        if v.is_empty() {
                            EdnValue::Vector(Vec::new())
                        } else {
                            EdnValue::Vector(v[1..].to_vec())
                        }
                    }
                    EdnValue::List(ref l) => {
                        if l.is_empty() {
                            EdnValue::List(Vec::new())
                        } else {
                            EdnValue::List(l[1..].to_vec())
                        }
                    }
                    _ => EdnValue::Vector(Vec::new()),
                };
                self.stack.push(result);
            }

            OpCode::Take => {
                let n = self.stack.pop().ok_or_else(|| EqError::runtime_error_str("take", "Missing count"))?;
                let coll = self.stack.pop().ok_or_else(|| EqError::runtime_error_str("take", "Missing collection"))?;
                
                if let EdnValue::Integer(count) = n {
                    if count < 0 {
                        self.stack.push(EdnValue::Vector(Vec::new()));
                        return Ok(());
                    }
                    
                    let count = count as usize;
                    let result = match coll {
                        EdnValue::Vector(ref v) => EdnValue::Vector(v.iter().take(count).cloned().collect()),
                        EdnValue::List(ref l) => EdnValue::List(l.iter().take(count).cloned().collect()),
                        _ => EdnValue::Vector(Vec::new()),
                    };
                    self.stack.push(result);
                } else {
                    return Err(EqError::type_error("integer", n.type_name()));
                }
            }

            OpCode::Drop => {
                let n = self.stack.pop().ok_or_else(|| EqError::runtime_error_str("drop", "Missing count"))?;
                let coll = self.stack.pop().ok_or_else(|| EqError::runtime_error_str("drop", "Missing collection"))?;
                
                if let EdnValue::Integer(count) = n {
                    if count < 0 {
                        self.stack.push(coll);
                        return Ok(());
                    }
                    
                    let count = count as usize;
                    let result = match coll {
                        EdnValue::Vector(ref v) => EdnValue::Vector(v.iter().skip(count).cloned().collect()),
                        EdnValue::List(ref l) => EdnValue::List(l.iter().skip(count).cloned().collect()),
                        _ => EdnValue::Vector(Vec::new()),
                    };
                    self.stack.push(result);
                } else {
                    return Err(EqError::type_error("integer", n.type_name()));
                }
            }

            OpCode::Nth => {
                let n = self.stack.pop().ok_or_else(|| EqError::runtime_error_str("nth", "Missing index"))?;
                let coll = self.stack.pop().ok_or_else(|| EqError::runtime_error_str("nth", "Missing collection"))?;
                
                if let EdnValue::Integer(index) = n {
                    let result = coll.get(&EdnValue::Integer(index)).cloned().unwrap_or(EdnValue::Nil);
                    self.stack.push(result);
                } else {
                    return Err(EqError::type_error("integer", n.type_name()));
                }
            }

            OpCode::Count => {
                let coll = self.stack.pop().ok_or_else(|| EqError::runtime_error_str("count", "Missing collection"))?;
                let count = coll.count().unwrap_or(0) as i64;
                self.stack.push(EdnValue::Integer(count));
            }

            OpCode::Keys => {
                let map = self.stack.pop().ok_or_else(|| EqError::runtime_error_str("keys", "Missing map"))?;
                let result = match map {
                    EdnValue::Map(ref m) => {
                        let keys: Vec<EdnValue> = m.keys().cloned().collect();
                        EdnValue::Vector(keys)
                    }
                    _ => EdnValue::Vector(Vec::new()),
                };
                self.stack.push(result);
            }

            OpCode::Vals => {
                let map = self.stack.pop().ok_or_else(|| EqError::runtime_error_str("vals", "Missing map"))?;
                let result = match map {
                    EdnValue::Map(ref m) => {
                        let vals: Vec<EdnValue> = m.values().cloned().collect();
                        EdnValue::Vector(vals)
                    }
                    _ => EdnValue::Vector(Vec::new()),
                };
                self.stack.push(result);
            }

            OpCode::Filter => {
                // For now, simplified filter implementation
                // In a full implementation, this would execute the predicate function
                let _predicate = self.stack.pop().ok_or_else(|| EqError::runtime_error_str("filter", "Missing predicate"))?;
                let coll = self.stack.pop().ok_or_else(|| EqError::runtime_error_str("filter", "Missing collection"))?;
                
                // Placeholder: just return the collection as-is
                // A real implementation would apply the predicate to each element
                self.stack.push(coll);
            }

            OpCode::Map => {
                // Simplified map implementation
                let _func = self.stack.pop().ok_or_else(|| EqError::runtime_error_str("map", "Missing function"))?;
                let coll = self.stack.pop().ok_or_else(|| EqError::runtime_error_str("map", "Missing collection"))?;
                
                // Placeholder: just return the collection as-is
                self.stack.push(coll);
            }

            OpCode::Remove => {
                let _predicate = self.stack.pop().ok_or_else(|| EqError::runtime_error_str("remove", "Missing predicate"))?;
                let coll = self.stack.pop().ok_or_else(|| EqError::runtime_error_str("remove", "Missing collection"))?;
                
                // Placeholder: just return the collection as-is
                self.stack.push(coll);
            }

            OpCode::SelectKeys => {
                let keys = self.stack.pop().ok_or_else(|| EqError::runtime_error_str("select-keys", "Missing keys"))?;
                let map = self.stack.pop().ok_or_else(|| EqError::runtime_error_str("select-keys", "Missing map"))?;
                
                if let (EdnValue::Map(ref m), EdnValue::Vector(ref key_vec)) = (&map, &keys) {
                    let mut result = IndexMap::new();
                    for key in key_vec {
                        if let Some(value) = m.get(key) {
                            result.insert(key.clone(), value.clone());
                        }
                    }
                    self.stack.push(EdnValue::Map(result));
                } else {
                    self.stack.push(EdnValue::Map(IndexMap::new()));
                }
            }

            OpCode::IsNil => {
                let value = self.stack.pop().ok_or_else(|| EqError::runtime_error_str("nil?", "Missing value"))?;
                let result = matches!(value, EdnValue::Nil);
                self.stack.push(EdnValue::Bool(result));
            }

            OpCode::IsEmpty => {
                let value = self.stack.pop().ok_or_else(|| EqError::runtime_error_str("empty?", "Missing value"))?;
                let result = value.count().map(|c| c == 0).unwrap_or(false);
                self.stack.push(EdnValue::Bool(result));
            }

            OpCode::Contains => {
                let key = self.stack.pop().ok_or_else(|| EqError::runtime_error_str("contains?", "Missing key"))?;
                let coll = self.stack.pop().ok_or_else(|| EqError::runtime_error_str("contains?", "Missing collection"))?;
                
                let result = match coll {
                    EdnValue::Map(ref m) => m.contains_key(&key),
                    EdnValue::Set(ref s) => s.contains(&key),
                    _ => false,
                };
                self.stack.push(EdnValue::Bool(result));
            }

            OpCode::IsNumber => {
                let value = self.stack.pop().ok_or_else(|| EqError::runtime_error_str("number?", "Missing value"))?;
                let result = matches!(value, EdnValue::Integer(_) | EdnValue::Float(_));
                self.stack.push(EdnValue::Bool(result));
            }

            OpCode::IsString => {
                let value = self.stack.pop().ok_or_else(|| EqError::runtime_error_str("string?", "Missing value"))?;
                let result = matches!(value, EdnValue::String(_));
                self.stack.push(EdnValue::Bool(result));
            }

            OpCode::IsKeyword => {
                let value = self.stack.pop().ok_or_else(|| EqError::runtime_error_str("keyword?", "Missing value"))?;
                let result = matches!(value, EdnValue::Keyword(_));
                self.stack.push(EdnValue::Bool(result));
            }

            OpCode::IsBoolean => {
                let value = self.stack.pop().ok_or_else(|| EqError::runtime_error_str("boolean?", "Missing value"))?;
                let result = matches!(value, EdnValue::Bool(_));
                self.stack.push(EdnValue::Bool(result));
            }

            OpCode::Equal => {
                let expected = self.stack.pop().ok_or_else(|| EqError::runtime_error_str("=", "Missing expected value"))?;
                let actual = self.stack.pop().ok_or_else(|| EqError::runtime_error_str("=", "Missing actual value"))?;
                let result = actual == expected;
                self.stack.push(EdnValue::Bool(result));
            }

            OpCode::LessThan => {
                let right = self.stack.pop().ok_or_else(|| EqError::runtime_error_str("<", "Missing right operand"))?;
                let left = self.stack.pop().ok_or_else(|| EqError::runtime_error_str("<", "Missing left operand"))?;
                let result = self.compare_values(&left, &right)? < 0;
                self.stack.push(EdnValue::Bool(result));
            }

            OpCode::GreaterThan => {
                let right = self.stack.pop().ok_or_else(|| EqError::runtime_error_str(">", "Missing right operand"))?;
                let left = self.stack.pop().ok_or_else(|| EqError::runtime_error_str(">", "Missing left operand"))?;
                let result = self.compare_values(&left, &right)? > 0;
                self.stack.push(EdnValue::Bool(result));
            }

            OpCode::LessEqual => {
                let right = self.stack.pop().ok_or_else(|| EqError::runtime_error_str("<=", "Missing right operand"))?;
                let left = self.stack.pop().ok_or_else(|| EqError::runtime_error_str("<=", "Missing left operand"))?;
                let result = self.compare_values(&left, &right)? <= 0;
                self.stack.push(EdnValue::Bool(result));
            }

            OpCode::GreaterEqual => {
                let right = self.stack.pop().ok_or_else(|| EqError::runtime_error_str(">=", "Missing right operand"))?;
                let left = self.stack.pop().ok_or_else(|| EqError::runtime_error_str(">=", "Missing left operand"))?;
                let result = self.compare_values(&left, &right)? >= 0;
                self.stack.push(EdnValue::Bool(result));
            }

            OpCode::Jump(offset) => {
                self.pc = *offset;
                return Ok(()); // Don't increment pc
            }

            OpCode::JumpIfFalse(offset) => {
                let condition = self.stack.pop().ok_or_else(|| EqError::runtime_error_str("jump-if-false", "Missing condition"))?;
                if !condition.is_truthy() {
                    self.pc = *offset;
                    return Ok(()); // Don't increment pc
                }
            }

            OpCode::JumpIfTrue(offset) => {
                let condition = self.stack.pop().ok_or_else(|| EqError::runtime_error_str("jump-if-true", "Missing condition"))?;
                if condition.is_truthy() {
                    self.pc = *offset;
                    return Ok(()); // Don't increment pc
                }
            }

            OpCode::Frequencies => {
                let coll = self.stack.pop().ok_or_else(|| EqError::runtime_error_str("frequencies", "Missing collection"))?;
                let mut freq_map = IndexMap::new();
                
                match coll {
                    EdnValue::Vector(ref v) => {
                        for item in v {
                            let count = freq_map.get(item).cloned().unwrap_or(EdnValue::Integer(0));
                            if let EdnValue::Integer(n) = count {
                                freq_map.insert(item.clone(), EdnValue::Integer(n + 1));
                            }
                        }
                    }
                    EdnValue::List(ref l) => {
                        for item in l {
                            let count = freq_map.get(item).cloned().unwrap_or(EdnValue::Integer(0));
                            if let EdnValue::Integer(n) = count {
                                freq_map.insert(item.clone(), EdnValue::Integer(n + 1));
                            }
                        }
                    }
                    _ => {}
                }
                
                self.stack.push(EdnValue::Map(freq_map));
            }

            // Placeholder implementations for unimplemented operations
            OpCode::Reduce | OpCode::Apply | OpCode::GroupBy | OpCode::Call(_) | OpCode::Return => {
                return Err(EqError::query_error(format!("Operation {:?} not yet implemented", op)));
            }
        }

        Ok(())
    }

    fn compare_values(&self, left: &EdnValue, right: &EdnValue) -> EqResult<i32> {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::compiler;

    #[test]
    fn test_identity() {
        let mut vm = QueryVM::new();
        let query = compiler::compile(crate::query::ast::Expr::Identity).unwrap();
        let input = EdnValue::Integer(42);
        let result = vm.execute(&query, input.clone()).unwrap();
        assert_eq!(result, input);
    }

    #[test]
    fn test_keyword_get() {
        let mut vm = QueryVM::new();
        let query = compiler::compile(
            crate::query::ast::Expr::KeywordAccess("name".to_string())
        ).unwrap();
        
        let mut map = IndexMap::new();
        map.insert(EdnValue::Keyword("name".to_string()), EdnValue::String("Alice".to_string()));
        let input = EdnValue::Map(map);
        
        let result = vm.execute(&query, input).unwrap();
        assert_eq!(result, EdnValue::String("Alice".to_string()));
    }

    #[test]
    fn test_first() {
        let mut vm = QueryVM::new();
        let query = compiler::compile(crate::query::ast::Expr::First).unwrap();
        let input = EdnValue::Vector(vec![
            EdnValue::Integer(1),
            EdnValue::Integer(2),
            EdnValue::Integer(3),
        ]);
        
        let result = vm.execute(&query, input).unwrap();
        assert_eq!(result, EdnValue::Integer(1));
    }

    #[test]
    fn test_count() {
        let mut vm = QueryVM::new();
        let query = compiler::compile(crate::query::ast::Expr::Count).unwrap();
        let input = EdnValue::Vector(vec![
            EdnValue::Integer(1),
            EdnValue::Integer(2),
            EdnValue::Integer(3),
        ]);
        
        let result = vm.execute(&query, input).unwrap();
        assert_eq!(result, EdnValue::Integer(3));
    }

    #[test]
    fn test_predicates() {
        let mut vm = QueryVM::new();
        
        // Test is-nil
        let query = compiler::compile(crate::query::ast::Expr::IsNil).unwrap();
        let result = vm.execute(&query, EdnValue::Nil).unwrap();
        assert_eq!(result, EdnValue::Bool(true));
        
        let result = vm.execute(&query, EdnValue::Integer(42)).unwrap();
        assert_eq!(result, EdnValue::Bool(false));
        
        // Test is-number
        let query = compiler::compile(crate::query::ast::Expr::IsNumber).unwrap();
        let result = vm.execute(&query, EdnValue::Integer(42)).unwrap();
        assert_eq!(result, EdnValue::Bool(true));
        
        let result = vm.execute(&query, EdnValue::String("hello".to_string())).unwrap();
        assert_eq!(result, EdnValue::Bool(false));
    }

    #[test]
    fn test_comparison() {
        let mut vm = QueryVM::new();
        
        // Test equality
        let query = compiler::compile(
            crate::query::ast::Expr::Equal(Box::new(crate::query::ast::Expr::Literal(EdnValue::Integer(42))))
        ).unwrap();
        
        let result = vm.execute(&query, EdnValue::Integer(42)).unwrap();
        assert_eq!(result, EdnValue::Bool(true));
        
        let result = vm.execute(&query, EdnValue::Integer(43)).unwrap();
        assert_eq!(result, EdnValue::Bool(false));
    }

    #[test]
    fn test_take_drop() {
        let mut vm = QueryVM::new();
        let input = EdnValue::Vector(vec![
            EdnValue::Integer(1),
            EdnValue::Integer(2),
            EdnValue::Integer(3),
            EdnValue::Integer(4),
        ]);
        
        // Test take
        let query = compiler::compile(
            crate::query::ast::Expr::Take(Box::new(crate::query::ast::Expr::Literal(EdnValue::Integer(2))))
        ).unwrap();
        
        let result = vm.execute(&query, input.clone()).unwrap();
        assert_eq!(result, EdnValue::Vector(vec![
            EdnValue::Integer(1),
            EdnValue::Integer(2),
        ]));
        
        // Test drop
        let query = compiler::compile(
            crate::query::ast::Expr::Drop(Box::new(crate::query::ast::Expr::Literal(EdnValue::Integer(2))))
        ).unwrap();
        
        let result = vm.execute(&query, input).unwrap();
        assert_eq!(result, EdnValue::Vector(vec![
            EdnValue::Integer(3),
            EdnValue::Integer(4),
        ]));
    }

    #[test]
    fn test_complex_expression() {
        let mut vm = QueryVM::new();
        
        // Test threading: first then count
        let query = compiler::compile(
            crate::query::ast::Expr::ThreadFirst(vec![
                crate::query::ast::Expr::Identity,
                crate::query::ast::Expr::First,
            ])
        ).unwrap();
        
        let input = EdnValue::Vector(vec![
            EdnValue::Vector(vec![EdnValue::Integer(1), EdnValue::Integer(2)]),
            EdnValue::Vector(vec![EdnValue::Integer(3), EdnValue::Integer(4)]),
        ]);
        
        let result = vm.execute(&query, input).unwrap();
        assert_eq!(result, EdnValue::Vector(vec![EdnValue::Integer(1), EdnValue::Integer(2)]));
    }
}
use super::ast::Expr;
use crate::edn::EdnValue;
use crate::error::EqResult;

/// Bytecode operations for the query VM
#[derive(Debug, Clone, PartialEq)]
pub enum OpCode {
    // Stack operations
    Push(usize),              // Push constant at index
    
    // Basic operations
    Identity,                 // No-op, pass through input
    Get,                      // Get value by key (expects key on stack)
    GetIn,                    // Get nested value (expects path vector on stack)
    KeywordGet(String),       // Get value by keyword literal
    
    // Collection operations
    First,                    // Get first element
    Last,                     // Get last element
    Rest,                     // Get rest of collection
    Take,                     // Take n elements (expects n on stack)
    Drop,                     // Drop n elements (expects n on stack)
    Nth,                      // Get nth element (expects n on stack)
    Count,                    // Get count of collection
    Keys,                     // Get keys of map
    Vals,                     // Get values of map
    
    // Higher-order operations
    Filter,                   // Filter collection (expects predicate bytecode)
    Map,                      // Map over collection (expects function bytecode)
    Remove,                   // Remove elements (expects predicate bytecode)
    SelectKeys,               // Select keys from map (expects keys on stack)
    
    // Predicates
    IsNil,                    // Test if nil
    IsEmpty,                  // Test if empty
    Contains,                 // Test if contains key (expects key on stack)
    IsNumber,                 // Test if number
    IsString,                 // Test if string
    IsKeyword,                // Test if keyword
    IsBoolean,                // Test if boolean
    
    // Comparison
    Equal,                    // Test equality (expects value on stack)
    LessThan,                 // Test less than (expects value on stack)
    GreaterThan,              // Test greater than (expects value on stack)
    LessEqual,                // Test less or equal (expects value on stack)
    GreaterEqual,             // Test greater or equal (expects value on stack)
    
    // Control flow
    Jump(usize),              // Unconditional jump to offset
    JumpIfFalse(usize),       // Jump if top of stack is falsy
    
    // Aggregation
    Reduce,                   // Reduce collection (expects function and init on stack)
    Apply,                    // Apply function to args (expects function on stack)
    GroupBy,                  // Group by function (expects function on stack)
    Frequencies,              // Count frequencies
    
}

/// Compiled query containing bytecode and constants
#[derive(Debug, Clone)]
pub struct CompiledQuery {
    pub bytecode: Vec<OpCode>,
    pub constants: Vec<EdnValue>,
}

impl CompiledQuery {
    pub fn new() -> Self {
        Self {
            bytecode: Vec::new(),
            constants: Vec::new(),
        }
    }
    
    pub fn add_constant(&mut self, value: EdnValue) -> usize {
        self.constants.push(value);
        self.constants.len() - 1
    }
    
    pub fn emit(&mut self, op: OpCode) {
        self.bytecode.push(op);
    }
    
    pub fn current_offset(&self) -> usize {
        self.bytecode.len()
    }
    
    pub fn patch_jump(&mut self, offset: usize, target: usize) {
        match &mut self.bytecode[offset] {
            OpCode::Jump(ref mut addr) => *addr = target,
            OpCode::JumpIfFalse(ref mut addr) => *addr = target,
            _ => panic!("Attempted to patch non-jump instruction"),
        }
    }
}

/// Compiler that converts AST expressions to bytecode
pub struct Compiler {
    query: CompiledQuery,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            query: CompiledQuery::new(),
        }
    }
    
    pub fn compile(expr: Expr) -> EqResult<CompiledQuery> {
        let mut compiler = Self::new();
        compiler.compile_expr(expr)?;
        Ok(compiler.query)
    }
    
    fn compile_expr(&mut self, expr: Expr) -> EqResult<()> {
        match expr {
            Expr::Identity => {
                self.query.emit(OpCode::Identity);
            }
            
            Expr::Get(key) => {
                let const_idx = self.query.add_constant(key);
                self.query.emit(OpCode::Push(const_idx));
                self.query.emit(OpCode::Get);
            }
            
            Expr::GetIn(path) => {
                let const_idx = self.query.add_constant(EdnValue::Vector(path));
                self.query.emit(OpCode::Push(const_idx));
                self.query.emit(OpCode::GetIn);
            }
            
            Expr::KeywordAccess(name) => {
                self.query.emit(OpCode::KeywordGet(name));
            }
            
            Expr::First => self.query.emit(OpCode::First),
            Expr::Last => self.query.emit(OpCode::Last),
            Expr::Rest => self.query.emit(OpCode::Rest),
            Expr::Count => self.query.emit(OpCode::Count),
            Expr::Keys => self.query.emit(OpCode::Keys),
            Expr::Vals => self.query.emit(OpCode::Vals),
            
            Expr::Take(n_expr) => {
                self.compile_expr(*n_expr)?;
                self.query.emit(OpCode::Take);
            }
            
            Expr::Drop(n_expr) => {
                self.compile_expr(*n_expr)?;
                self.query.emit(OpCode::Drop);
            }
            
            Expr::Nth(n_expr) => {
                self.compile_expr(*n_expr)?;
                self.query.emit(OpCode::Nth);
            }
            
            Expr::Filter(pred_expr) => {
                // For now, compile the predicate inline
                // In a full implementation, this would be a closure
                self.compile_expr(*pred_expr)?;
                self.query.emit(OpCode::Filter);
            }
            
            Expr::Map(func_expr) => {
                self.compile_expr(*func_expr)?;
                self.query.emit(OpCode::Map);
            }
            
            Expr::Remove(pred_expr) => {
                self.compile_expr(*pred_expr)?;
                self.query.emit(OpCode::Remove);
            }
            
            Expr::SelectKeys(keys) => {
                let const_idx = self.query.add_constant(EdnValue::Vector(keys));
                self.query.emit(OpCode::Push(const_idx));
                self.query.emit(OpCode::SelectKeys);
            }
            
            Expr::IsNil => self.query.emit(OpCode::IsNil),
            Expr::IsEmpty => self.query.emit(OpCode::IsEmpty),
            Expr::IsNumber => self.query.emit(OpCode::IsNumber),
            Expr::IsString => self.query.emit(OpCode::IsString),
            Expr::IsKeyword => self.query.emit(OpCode::IsKeyword),
            Expr::IsBoolean => self.query.emit(OpCode::IsBoolean),
            
            Expr::Contains(key_expr) => {
                self.compile_expr(*key_expr)?;
                self.query.emit(OpCode::Contains);
            }
            
            Expr::Equal(val_expr) => {
                self.compile_expr(*val_expr)?;
                self.query.emit(OpCode::Equal);
            }
            
            Expr::LessThan(val_expr) => {
                self.compile_expr(*val_expr)?;
                self.query.emit(OpCode::LessThan);
            }
            
            Expr::GreaterThan(val_expr) => {
                self.compile_expr(*val_expr)?;
                self.query.emit(OpCode::GreaterThan);
            }
            
            Expr::LessEqual(val_expr) => {
                self.compile_expr(*val_expr)?;
                self.query.emit(OpCode::LessEqual);
            }
            
            Expr::GreaterEqual(val_expr) => {
                self.compile_expr(*val_expr)?;
                self.query.emit(OpCode::GreaterEqual);
            }
            
            Expr::ThreadFirst(exprs) => {
                self.compile_threading(exprs, true)?;
            }
            
            Expr::ThreadLast(exprs) => {
                self.compile_threading(exprs, false)?;
            }
            
            Expr::Comp(exprs) => {
                // Compose functions in reverse order
                for expr in exprs.into_iter().rev() {
                    self.compile_expr(expr)?;
                }
            }
            
            Expr::If { test, then_expr, else_expr } => {
                self.compile_expr(*test)?;
                let jump_if_false = self.query.current_offset();
                self.query.emit(OpCode::JumpIfFalse(0)); // Will be patched
                
                self.compile_expr(*then_expr)?;
                
                if let Some(else_expr) = else_expr {
                    let jump_end = self.query.current_offset();
                    self.query.emit(OpCode::Jump(0)); // Will be patched
                    
                    let else_start = self.query.current_offset();
                    self.query.patch_jump(jump_if_false, else_start);
                    
                    self.compile_expr(*else_expr)?;
                    
                    let end = self.query.current_offset();
                    self.query.patch_jump(jump_end, end);
                } else {
                    let end = self.query.current_offset();
                    self.query.patch_jump(jump_if_false, end);
                }
            }
            
            Expr::When { test, expr } => {
                self.compile_expr(*test)?;
                let jump_if_false = self.query.current_offset();
                self.query.emit(OpCode::JumpIfFalse(0)); // Will be patched
                
                self.compile_expr(*expr)?;
                
                let end = self.query.current_offset();
                self.query.patch_jump(jump_if_false, end);
            }
            
            Expr::Reduce { func, init } => {
                if let Some(init_expr) = init {
                    self.compile_expr(*init_expr)?;
                } else {
                    // Use nil as default init
                    let const_idx = self.query.add_constant(EdnValue::Nil);
                    self.query.emit(OpCode::Push(const_idx));
                }
                self.compile_expr(*func)?;
                self.query.emit(OpCode::Reduce);
            }
            
            Expr::Apply(func_expr) => {
                self.compile_expr(*func_expr)?;
                self.query.emit(OpCode::Apply);
            }
            
            Expr::GroupBy(func_expr) => {
                self.compile_expr(*func_expr)?;
                self.query.emit(OpCode::GroupBy);
            }
            
            Expr::Frequencies => {
                self.query.emit(OpCode::Frequencies);
            }
            
            Expr::Literal(value) => {
                let const_idx = self.query.add_constant(value);
                self.query.emit(OpCode::Push(const_idx));
            }
            
        }
        
        Ok(())
    }
    
    fn compile_threading(&mut self, exprs: Vec<Expr>, _first: bool) -> EqResult<()> {
        if exprs.is_empty() {
            return Ok(());
        }
        
        // Start with the first expression
        self.compile_expr(exprs[0].clone())?;
        
        // Apply each subsequent expression
        for expr in exprs.into_iter().skip(1) {
            self.compile_expr(expr)?;
        }
        
        Ok(())
    }
}

/// Convenience function to compile an expression
pub fn compile(expr: Expr) -> EqResult<CompiledQuery> {
    Compiler::compile(expr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_identity() {
        let query = compile(Expr::Identity).unwrap();
        assert_eq!(query.bytecode, vec![OpCode::Identity]);
        assert!(query.constants.is_empty());
    }

    #[test]
    fn test_compile_get() {
        let query = compile(Expr::Get(EdnValue::Keyword("name".to_string()))).unwrap();
        assert_eq!(query.bytecode, vec![OpCode::Push(0), OpCode::Get]);
        assert_eq!(query.constants, vec![EdnValue::Keyword("name".to_string())]);
    }

    #[test]
    fn test_compile_keyword_access() {
        let query = compile(Expr::KeywordAccess("age".to_string())).unwrap();
        assert_eq!(query.bytecode, vec![OpCode::KeywordGet("age".to_string())]);
        assert!(query.constants.is_empty());
    }

    #[test]
    fn test_compile_collection_ops() {
        let query = compile(Expr::First).unwrap();
        assert_eq!(query.bytecode, vec![OpCode::First]);

        let query = compile(Expr::Count).unwrap();
        assert_eq!(query.bytecode, vec![OpCode::Count]);
    }

    #[test]
    fn test_compile_filter() {
        let query = compile(Expr::Filter(Box::new(Expr::IsNumber))).unwrap();
        assert_eq!(query.bytecode, vec![OpCode::IsNumber, OpCode::Filter]);
    }

    #[test]
    fn test_compile_if() {
        let query = compile(Expr::If {
            test: Box::new(Expr::IsNil),
            then_expr: Box::new(Expr::Literal(EdnValue::String("empty".to_string()))),
            else_expr: Some(Box::new(Expr::Literal(EdnValue::String("not empty".to_string())))),
        }).unwrap();
        
        // Should have: IsNil, JumpIfFalse, Push(0), Jump, Push(1)
        assert_eq!(query.bytecode.len(), 5);
        assert_eq!(query.bytecode[0], OpCode::IsNil);
        assert!(matches!(query.bytecode[1], OpCode::JumpIfFalse(_)));
        assert_eq!(query.bytecode[2], OpCode::Push(0));
        assert!(matches!(query.bytecode[3], OpCode::Jump(_)));
        assert_eq!(query.bytecode[4], OpCode::Push(1));
    }

    #[test]
    fn test_compile_literals() {
        let query = compile(Expr::Literal(EdnValue::Integer(42))).unwrap();
        assert_eq!(query.bytecode, vec![OpCode::Push(0)]);
        assert_eq!(query.constants, vec![EdnValue::Integer(42)]);
    }

    #[test]
    fn test_compile_threading() {
        let query = compile(Expr::ThreadFirst(vec![
            Expr::Identity,
            Expr::First,
            Expr::KeywordAccess("name".to_string()),
        ])).unwrap();
        
        assert_eq!(query.bytecode, vec![
            OpCode::Identity,
            OpCode::First,
            OpCode::KeywordGet("name".to_string()),
        ]);
    }

    #[test]
    fn test_compile_complex_expression() {
        let query = compile(Expr::Filter(Box::new(
            Expr::Map(Box::new(Expr::KeywordAccess("age".to_string())))
        ))).unwrap();
        
        assert_eq!(query.bytecode, vec![
            OpCode::KeywordGet("age".to_string()),
            OpCode::Map,
            OpCode::Filter,
        ]);
    }
}
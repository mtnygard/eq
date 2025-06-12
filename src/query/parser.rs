use crate::edn::{EdnValue, Parser as EdnParser};
use crate::error::{EqError, EqResult};
use crate::query::ast::Expr;

pub struct QueryParser;

impl QueryParser {
    pub fn parse(input: &str) -> EqResult<Expr> {
        let mut edn_parser = EdnParser::new(input);
        let edn_value = edn_parser.parse()?;
        Self::edn_to_expr(edn_value)
    }

    fn edn_to_expr(value: EdnValue) -> EqResult<Expr> {
        match value {
            // Identity
            EdnValue::Symbol(ref s) if s == "." => Ok(Expr::Identity),
            
            // Keyword access shorthand
            EdnValue::Keyword(name) => Ok(Expr::KeywordAccess(name)),
            
            // List expressions (function calls)
            EdnValue::List(elements) => {
                if elements.is_empty() {
                    return Err(EqError::query_error("Empty list expression"));
                }
                
                let func = &elements[0];
                let args = &elements[1..];
                
                match func {
                    EdnValue::Symbol(name) => Self::parse_function_call(name, args),
                    _ => Err(EqError::query_error("First element of list must be a symbol")),
                }
            }
            
            // Literals
            literal => Ok(Expr::Literal(literal)),
        }
    }

    fn parse_function_call(name: &str, args: &[EdnValue]) -> EqResult<Expr> {
        match name {
            // Basic selectors
            "get" => Self::parse_get(args),
            "get-in" => Self::parse_get_in(args),
            
            // Collection operations
            "first" => Self::parse_nullary("first", args, Expr::First),
            "last" => Self::parse_nullary("last", args, Expr::Last),
            "rest" => Self::parse_nullary("rest", args, Expr::Rest),
            "count" => Self::parse_nullary("count", args, Expr::Count),
            "keys" => Self::parse_nullary("keys", args, Expr::Keys),
            "vals" => Self::parse_nullary("vals", args, Expr::Vals),
            "take" => Self::parse_unary("take", args, Expr::Take),
            "drop" => Self::parse_unary("drop", args, Expr::Drop),
            "nth" => Self::parse_unary("nth", args, Expr::Nth),
            
            // Filtering and mapping
            "filter" => Self::parse_unary("filter", args, Expr::Filter),
            "map" => Self::parse_unary("map", args, Expr::Map),
            "remove" => Self::parse_unary("remove", args, Expr::Remove),
            "select-keys" => Self::parse_select_keys(args),
            
            // Predicates
            "nil?" => Self::parse_nullary("nil?", args, Expr::IsNil),
            "empty?" => Self::parse_nullary("empty?", args, Expr::IsEmpty),
            "number?" => Self::parse_nullary("number?", args, Expr::IsNumber),
            "string?" => Self::parse_nullary("string?", args, Expr::IsString),
            "keyword?" => Self::parse_nullary("keyword?", args, Expr::IsKeyword),
            "boolean?" => Self::parse_nullary("boolean?", args, Expr::IsBoolean),
            "contains?" => Self::parse_unary("contains?", args, Expr::Contains),
            
            // Comparison
            "=" => Self::parse_unary("=", args, Expr::Equal),
            "<" => Self::parse_unary("<", args, Expr::LessThan),
            ">" => Self::parse_unary(">", args, Expr::GreaterThan),
            "<=" => Self::parse_unary("<=", args, Expr::LessEqual),
            ">=" => Self::parse_unary(">=", args, Expr::GreaterEqual),
            
            // Composition
            "->" => Self::parse_threading("->", args, true),
            "->>" => Self::parse_threading("->>", args, false),
            "comp" => Self::parse_comp(args),
            
            // Conditionals
            "if" => Self::parse_if(args),
            "when" => Self::parse_when(args),
            
            // Aggregation
            "reduce" => Self::parse_reduce(args),
            "apply" => Self::parse_unary("apply", args, Expr::Apply),
            "group-by" => Self::parse_unary("group-by", args, Expr::GroupBy),
            "frequencies" => Self::parse_nullary("frequencies", args, Expr::Frequencies),
            
            _ => Err(EqError::query_error(format!("Unknown function: {}", name))),
        }
    }

    fn parse_nullary(name: &str, args: &[EdnValue], expr: Expr) -> EqResult<Expr> {
        if !args.is_empty() {
            return Err(EqError::query_error(format!("{} takes no arguments", name)));
        }
        Ok(expr)
    }

    fn parse_unary<F>(name: &str, args: &[EdnValue], constructor: F) -> EqResult<Expr>
    where
        F: FnOnce(Box<Expr>) -> Expr,
    {
        if args.len() != 1 {
            return Err(EqError::query_error(format!("{} takes exactly one argument", name)));
        }
        let arg_expr = Self::edn_to_expr(args[0].clone())?;
        Ok(constructor(Box::new(arg_expr)))
    }

    fn parse_get(args: &[EdnValue]) -> EqResult<Expr> {
        if args.len() != 1 {
            return Err(EqError::query_error("get takes exactly one argument"));
        }
        Ok(Expr::Get(args[0].clone()))
    }

    fn parse_get_in(args: &[EdnValue]) -> EqResult<Expr> {
        if args.len() != 1 {
            return Err(EqError::query_error("get-in takes exactly one argument"));
        }
        
        match &args[0] {
            EdnValue::Vector(path) => Ok(Expr::GetIn(path.clone())),
            _ => Err(EqError::query_error("get-in requires a vector argument")),
        }
    }

    fn parse_select_keys(args: &[EdnValue]) -> EqResult<Expr> {
        if args.len() != 1 {
            return Err(EqError::query_error("select-keys takes exactly one argument"));
        }
        
        match &args[0] {
            EdnValue::Vector(keys) => Ok(Expr::SelectKeys(keys.clone())),
            _ => Err(EqError::query_error("select-keys requires a vector argument")),
        }
    }

    fn parse_threading(name: &str, args: &[EdnValue], _first: bool) -> EqResult<Expr> {
        if args.is_empty() {
            return Err(EqError::query_error(format!("{} requires at least one argument", name)));
        }
        
        let exprs: Result<Vec<_>, _> = args.iter()
            .map(|arg| Self::edn_to_expr(arg.clone()))
            .collect();
        
        let exprs = exprs?;
        
        match name {
            "->" => Ok(Expr::ThreadFirst(exprs)),
            "->>" => Ok(Expr::ThreadLast(exprs)),
            _ => unreachable!(),
        }
    }

    fn parse_comp(args: &[EdnValue]) -> EqResult<Expr> {
        if args.is_empty() {
            return Err(EqError::query_error("comp requires at least one argument"));
        }
        
        let exprs: Result<Vec<_>, _> = args.iter()
            .map(|arg| Self::edn_to_expr(arg.clone()))
            .collect();
        
        Ok(Expr::Comp(exprs?))
    }

    fn parse_if(args: &[EdnValue]) -> EqResult<Expr> {
        match args.len() {
            2 => {
                let test = Self::edn_to_expr(args[0].clone())?;
                let then_expr = Self::edn_to_expr(args[1].clone())?;
                Ok(Expr::If {
                    test: Box::new(test),
                    then_expr: Box::new(then_expr),
                    else_expr: None,
                })
            }
            3 => {
                let test = Self::edn_to_expr(args[0].clone())?;
                let then_expr = Self::edn_to_expr(args[1].clone())?;
                let else_expr = Self::edn_to_expr(args[2].clone())?;
                Ok(Expr::If {
                    test: Box::new(test),
                    then_expr: Box::new(then_expr),
                    else_expr: Some(Box::new(else_expr)),
                })
            }
            _ => Err(EqError::query_error("if takes 2 or 3 arguments")),
        }
    }

    fn parse_when(args: &[EdnValue]) -> EqResult<Expr> {
        if args.len() != 2 {
            return Err(EqError::query_error("when takes exactly 2 arguments"));
        }
        
        let test = Self::edn_to_expr(args[0].clone())?;
        let expr = Self::edn_to_expr(args[1].clone())?;
        
        Ok(Expr::When {
            test: Box::new(test),
            expr: Box::new(expr),
        })
    }

    fn parse_reduce(args: &[EdnValue]) -> EqResult<Expr> {
        match args.len() {
            1 => {
                let func = Self::edn_to_expr(args[0].clone())?;
                Ok(Expr::Reduce {
                    func: Box::new(func),
                    init: None,
                })
            }
            2 => {
                let func = Self::edn_to_expr(args[0].clone())?;
                let init = Self::edn_to_expr(args[1].clone())?;
                Ok(Expr::Reduce {
                    func: Box::new(func),
                    init: Some(Box::new(init)),
                })
            }
            _ => Err(EqError::query_error("reduce takes 1 or 2 arguments")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_identity() {
        let expr = QueryParser::parse(".").unwrap();
        assert_eq!(expr, Expr::Identity);
    }

    #[test]
    fn test_parse_keyword_access() {
        let expr = QueryParser::parse(":name").unwrap();
        assert_eq!(expr, Expr::KeywordAccess("name".to_string()));
    }

    #[test]
    fn test_parse_get() {
        let expr = QueryParser::parse("(get :name)").unwrap();
        assert_eq!(expr, Expr::Get(EdnValue::Keyword("name".to_string())));
    }

    #[test]
    fn test_parse_get_in() {
        let expr = QueryParser::parse("(get-in [:user :profile :name])").unwrap();
        assert_eq!(expr, Expr::GetIn(vec![
            EdnValue::Keyword("user".to_string()),
            EdnValue::Keyword("profile".to_string()),
            EdnValue::Keyword("name".to_string()),
        ]));
    }

    #[test]
    fn test_parse_collection_ops() {
        let expr = QueryParser::parse("(first)").unwrap();
        assert_eq!(expr, Expr::First);

        let expr = QueryParser::parse("(count)").unwrap();
        assert_eq!(expr, Expr::Count);
    }

    #[test]
    fn test_parse_filter_map() {
        let expr = QueryParser::parse("(filter (number?))").unwrap();
        assert_eq!(expr, Expr::Filter(Box::new(Expr::IsNumber)));

        let expr = QueryParser::parse("(map :name)").unwrap();
        assert_eq!(expr, Expr::Map(Box::new(Expr::KeywordAccess("name".to_string()))));
    }

    #[test]
    fn test_parse_threading() {
        let expr = QueryParser::parse("(-> . (first) :name)").unwrap();
        assert_eq!(expr, Expr::ThreadFirst(vec![
            Expr::Identity,
            Expr::First,
            Expr::KeywordAccess("name".to_string()),
        ]));
    }

    #[test]
    fn test_parse_if() {
        let expr = QueryParser::parse("(if (nil?) :empty :value)").unwrap();
        match expr {
            Expr::If { test, then_expr, else_expr } => {
                assert_eq!(*test, Expr::IsNil);
                assert_eq!(*then_expr, Expr::KeywordAccess("empty".to_string()));
                assert!(else_expr.is_some());
                assert_eq!(*else_expr.unwrap(), Expr::KeywordAccess("value".to_string()));
            }
            _ => panic!("Expected If expression"),
        }
    }

    #[test]
    fn test_parse_literals() {
        let expr = QueryParser::parse("42").unwrap();
        assert_eq!(expr, Expr::Literal(EdnValue::Integer(42)));

        let expr = QueryParser::parse("\"hello\"").unwrap();
        assert_eq!(expr, Expr::Literal(EdnValue::String("hello".to_string())));
    }

    #[test]
    fn test_parse_errors() {
        assert!(QueryParser::parse("()").is_err());
        assert!(QueryParser::parse("(unknown-function)").is_err());
        assert!(QueryParser::parse("(get)").is_err());
        assert!(QueryParser::parse("(get :a :b)").is_err());
    }

    #[test]
    fn test_complex_expressions() {
        let expr = QueryParser::parse("(->> . (filter (number?)) (map :value))").unwrap();
        
        match expr {
            Expr::ThreadLast(exprs) => {
                assert_eq!(exprs.len(), 3);
                assert_eq!(exprs[0], Expr::Identity);
                assert_eq!(exprs[1], Expr::Filter(Box::new(Expr::IsNumber)));
                assert_eq!(exprs[2], Expr::Map(Box::new(Expr::KeywordAccess("value".to_string()))));
            }
            _ => panic!("Expected ThreadLast"),
        }
    }
}
use crate::edn::{EdnValue, Parser as EdnParser};
use crate::error::EqResult;
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
            // Symbols
            EdnValue::Symbol(s) => Ok(Expr::Symbol(s)),
            
            // Keywords are literals unless in function position
            EdnValue::Keyword(name) => Ok(Expr::Literal(EdnValue::Keyword(name))),
            
            // List expressions - just store as lists for analysis phase
            EdnValue::List(elements) => {
                Ok(Expr::List(elements))
            }
            
            // Literals
            literal => Ok(Expr::Literal(literal)),
        }
    }

    // Parser no longer handles function calls - that's done in the analyzer

    // All parsing helpers removed - analysis phase handles function calls
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_identity() {
        let expr = QueryParser::parse(".").unwrap();
        assert_eq!(expr, Expr::Symbol(".".to_string()));
    }

    #[test]
    fn test_parse_keyword_access() {
        // Standalone keywords are literals
        let expr = QueryParser::parse(":name").unwrap();
        assert_eq!(expr, Expr::Literal(EdnValue::Keyword("name".to_string())));
        
        // Keywords in function position - parsed as lists
        let expr = QueryParser::parse("(:name)").unwrap();
        assert_eq!(expr, Expr::List(vec![EdnValue::Keyword("name".to_string())]));
        
        // Keywords with one argument
        let expr = QueryParser::parse("(:name .)").unwrap();
        assert_eq!(expr, Expr::List(vec![EdnValue::Keyword("name".to_string()), EdnValue::Symbol(".".to_string())]));
    }

    #[test]
    fn test_parse_get() {
        let expr = QueryParser::parse("(get :name)").unwrap();
        assert_eq!(expr, Expr::List(vec![EdnValue::Symbol("get".to_string()), EdnValue::Keyword("name".to_string())]));
    }

    #[test]
    fn test_parse_get_in() {
        let expr = QueryParser::parse("(get-in . [:user :profile :name])").unwrap();
        assert_eq!(expr, Expr::List(vec![
            EdnValue::Symbol("get-in".to_string()),
            EdnValue::Symbol(".".to_string()),
            EdnValue::Vector(vec![
                EdnValue::Keyword("user".to_string()),
                EdnValue::Keyword("profile".to_string()),
                EdnValue::Keyword("name".to_string()),
            ])
        ]));
    }

    #[test]
    fn test_parse_collection_ops() {
        let expr = QueryParser::parse("(first)").unwrap();
        assert_eq!(expr, Expr::List(vec![EdnValue::Symbol("first".to_string())]));

        let expr = QueryParser::parse("(count)").unwrap();
        assert_eq!(expr, Expr::List(vec![EdnValue::Symbol("count".to_string())]));
    }

    #[test]
    fn test_parse_map() {
        let expr = QueryParser::parse("(map :name)").unwrap();
        assert_eq!(expr, Expr::List(vec![EdnValue::Symbol("map".to_string()), EdnValue::Keyword("name".to_string())]));
    }

    #[test]
    fn test_parse_threading() {
        let expr = QueryParser::parse("(-> . (first) :name)").unwrap();
        assert_eq!(expr, Expr::List(vec![
            EdnValue::Symbol("->".to_string()),
            EdnValue::Symbol(".".to_string()),
            EdnValue::List(vec![EdnValue::Symbol("first".to_string())]),
            EdnValue::Keyword("name".to_string()),
        ]));
    }

    #[test]
    fn test_parse_if() {
        let expr = QueryParser::parse("(if (nil?) :empty :value)").unwrap();
        assert_eq!(expr, Expr::List(vec![
            EdnValue::Symbol("if".to_string()),
            EdnValue::List(vec![EdnValue::Symbol("nil?".to_string())]),
            EdnValue::Keyword("empty".to_string()),
            EdnValue::Keyword("value".to_string()),
        ]));
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
        // Empty lists should parse successfully (error checking happens during analysis)
        let expr = QueryParser::parse("()").unwrap();
        assert_eq!(expr, Expr::List(vec![]));
        
        // Unknown functions should parse as lists (error checking happens during analysis)
        let expr = QueryParser::parse("(unknown-function)").unwrap();
        assert_eq!(expr, Expr::List(vec![EdnValue::Symbol("unknown-function".to_string())]));
        
        // Parser accepts any list structure (analysis phase handles arity checking)
        let expr = QueryParser::parse("(get)").unwrap();
        assert_eq!(expr, Expr::List(vec![EdnValue::Symbol("get".to_string())]));
        
        let expr = QueryParser::parse("(get :a :b)").unwrap();
        assert_eq!(expr, Expr::List(vec![
            EdnValue::Symbol("get".to_string()), 
            EdnValue::Keyword("a".to_string()), 
            EdnValue::Keyword("b".to_string())
        ]));
    }

    #[test]
    fn test_complex_expressions() {
        let expr = QueryParser::parse("(->> . (select (number?)) (map :value))").unwrap();
        
        assert_eq!(expr, Expr::List(vec![
            EdnValue::Symbol("->>".to_string()),
            EdnValue::Symbol(".".to_string()),
            EdnValue::List(vec![
                EdnValue::Symbol("select".to_string()),
                EdnValue::List(vec![EdnValue::Symbol("number?".to_string())])
            ]),
            EdnValue::List(vec![
                EdnValue::Symbol("map".to_string()),
                EdnValue::Keyword("value".to_string())
            ])
        ]));
    }
}
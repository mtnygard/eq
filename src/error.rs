use thiserror::Error;

pub type EqResult<T> = Result<T, EqError>;

#[derive(Error, Debug)]
pub enum EqError {
    #[error("Parse error at line {line}, column {column}: {message}")]
    ParseError {
        line: usize,
        column: usize,
        message: String,
    },
    
    #[error("Query error: {message}")]
    QueryError { message: String },
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Runtime error in {context}: {source}")]
    RuntimeError {
        context: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    
    #[error("Type error: expected {expected}, got {actual}")]
    TypeError { expected: String, actual: String },
    
    #[error("Glob pattern error: {0}")]
    GlobError(#[from] glob::PatternError),
    
    #[error("WalkDir error: {0}")]
    WalkDirError(#[from] walkdir::Error),
}

impl EqError {
    pub fn parse_error(line: usize, column: usize, message: impl Into<String>) -> Self {
        Self::ParseError {
            line,
            column,
            message: message.into(),
        }
    }
    
    pub fn query_error(message: impl Into<String>) -> Self {
        Self::QueryError {
            message: message.into(),
        }
    }
    
    pub fn runtime_error_str(context: impl Into<String>, message: impl Into<String>) -> Self {
        Self::RuntimeError {
            context: context.into(),
            source: Box::new(std::io::Error::new(std::io::ErrorKind::Other, message.into())),
        }
    }
    
    pub fn type_error(expected: impl Into<String>, actual: impl Into<String>) -> Self {
        Self::TypeError {
            expected: expected.into(),
            actual: actual.into(),
        }
    }
}
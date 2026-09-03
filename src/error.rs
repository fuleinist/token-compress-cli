use std::fmt;

/// All failures surface as TcError; the CLI maps them to exit code 2.
#[derive(Debug)]
pub struct TcError {
    pub msg: String,
}

impl TcError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self { msg: msg.into() }
    }
}

impl fmt::Display for TcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl std::error::Error for TcError {}

impl From<std::io::Error> for TcError {
    fn from(e: std::io::Error) -> Self {
        TcError::new(format!("io error: {e}"))
    }
}

impl From<serde_json::Error> for TcError {
    fn from(e: serde_json::Error) -> Self {
        TcError::new(format!("invalid map: {e}"))
    }
}

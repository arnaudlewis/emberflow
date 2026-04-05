use std::error::Error;
use std::fmt;

pub type Result<T> = std::result::Result<T, EmberFlowError>;

#[derive(Debug)]
pub enum EmberFlowError {
    UnsupportedValue { field: &'static str, value: String },
    NotFound(String),
    Io(std::io::Error),
    Sqlite(rusqlite::Error),
    Json(serde_json::Error),
}

impl fmt::Display for EmberFlowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedValue { field, value } => {
                write!(f, "unsupported {field}: {value}")
            }
            Self::NotFound(value) => write!(f, "record not found: {value}"),
            Self::Io(error) => write!(f, "io error: {error}"),
            Self::Sqlite(error) => write!(f, "sqlite error: {error}"),
            Self::Json(error) => write!(f, "json error: {error}"),
        }
    }
}

impl Error for EmberFlowError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Sqlite(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::UnsupportedValue { .. } | Self::NotFound(_) => None,
        }
    }
}

impl From<std::io::Error> for EmberFlowError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<rusqlite::Error> for EmberFlowError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sqlite(error)
    }
}

impl From<serde_json::Error> for EmberFlowError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

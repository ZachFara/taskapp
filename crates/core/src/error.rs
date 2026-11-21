use serde::{Serialize, Serializer};
use specta::{Type, Types, datatype::DataType};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid: {0}")]
    Invalid(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

/// What crosses the Tauri and HTTP boundary.
#[derive(Serialize, Type)]
struct ErrorPayload {
    kind: &'static str,
    message: String,
}

impl Error {
    #[must_use]
    pub fn kind(&self) -> &'static str {
        match self {
            Error::NotFound(_) => "NotFound",
            Error::Invalid(_) => "Invalid",
            Error::Conflict(_) => "Conflict",
            Error::Db(_) => "Db",
            Error::Io(_) => "Io",
        }
    }
}

impl Serialize for Error {
    fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        ErrorPayload {
            kind: self.kind(),
            message: self.to_string(),
        }
        .serialize(s)
    }
}

impl Type for Error {
    fn definition(types: &mut Types) -> DataType {
        ErrorPayload::definition(types)
    }
}

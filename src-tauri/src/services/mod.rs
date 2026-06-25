pub mod actions;
pub mod archive;
pub mod capture;
pub mod draft;
pub mod parse_queue;
pub mod search;
pub mod settings;

use crate::db::DatabaseError;
use crate::parser::ParserError;
use crate::repositories::RepositoryError;

#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error(transparent)]
    Repository(#[from] RepositoryError),
    #[error(transparent)]
    Database(#[from] DatabaseError),
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Parser(#[from] ParserError),
    #[error("invalid input: {0}")]
    InvalidInput(&'static str),
    #[error("{entity} not found: {id}")]
    NotFound { entity: &'static str, id: String },
    #[error("{name} state lock poisoned")]
    StatePoisoned { name: &'static str },
}

pub type ServiceResult<T> = Result<T, ServiceError>;

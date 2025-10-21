use thiserror::Error;

#[derive(Error, Debug)]
pub enum VectorLawDBError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Key not found: {0}")]
    KeyNotFound(String),

    #[error("Transaction conflict")]
    TransactionConflict,

    #[error("Compaction error: {0}")]
    Compaction(String),

    #[error("Index error: {0}")]
    Index(String),
}

pub type Result<T> = std::result::Result<T, VectorLawDBError>;

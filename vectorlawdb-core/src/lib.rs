pub mod lsm;
pub mod storage;
pub mod error;
pub mod types;

pub use error::{Result, VectorLawDBError};

// Re-exports
pub use lsm::{LSMTree, MemTable, SSTable, WriteAheadLog};
pub use types::{CaseId, Embedding, CaseMetadata, CaseDocument};

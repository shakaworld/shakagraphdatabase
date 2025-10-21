pub mod parser;
pub mod optimizer;
pub mod executor;

pub use parser::VQLParser;

// VQL (Vector Query Language) example:
// FIND cases
// WHERE jurisdiction = "Federal"
//   AND year BETWEEN 2000 AND 2023
//   AND vector_similar("patent infringement", threshold=0.8)
// ORDER BY citation_rank DESC
// LIMIT 10

pub mod parser;
pub mod optimizer;
pub mod executor;

pub use parser::{VQLParser, VQLQuery, QueryType, Predicate, Filter};
pub use optimizer::{QueryOptimizer, QueryPlan, PlanStep, StepType};
pub use executor::{QueryExecutor, QueryResult, ExecutionContext};

// VQL (Vector Query Language) examples:
// - FIND cases LIMIT 10
// - FIND cases NEAR [0.1, 0.2, 0.3] RADIUS 0.5
// - FIND cases CITES 'case_123'
// - FIND cases WHERE date AFTER '2020-01-01'
// - FIND cases WHERE jurisdiction = 'Federal' LIMIT 20

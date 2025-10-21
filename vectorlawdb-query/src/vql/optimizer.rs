//! VQL Query Optimizer
//!
//! Cost-based optimization of VQL queries.

use super::parser::{VQLQuery, QueryType};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPlan {
    pub steps: Vec<PlanStep>,
    pub estimated_cost: f64,
    pub estimated_rows: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub step_type: StepType,
    pub description: String,
    pub estimated_cost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepType {
    SpatialIndexScan,
    CitationGraphTraversal,
    TemporalFilter,
    FullScan,
    Limit,
}

pub struct QueryOptimizer;

impl QueryOptimizer {
    /// Optimize a VQL query into an execution plan
    pub fn optimize(query: VQLQuery) -> QueryPlan {
        let mut steps = Vec::new();
        let mut total_cost = 0.0;

        match query.query_type {
            QueryType::Vector => {
                steps.push(PlanStep {
                    step_type: StepType::SpatialIndexScan,
                    description: "Scan spatial index with NEAR predicate".to_string(),
                    estimated_cost: 10.0,
                });
                total_cost += 10.0;
            }
            QueryType::Citation => {
                steps.push(PlanStep {
                    step_type: StepType::CitationGraphTraversal,
                    description: "Traverse citation graph".to_string(),
                    estimated_cost: 50.0,
                });
                total_cost += 50.0;
            }
            QueryType::Temporal => {
                steps.push(PlanStep {
                    step_type: StepType::TemporalFilter,
                    description: "Filter by temporal predicate".to_string(),
                    estimated_cost: 20.0,
                });
                total_cost += 20.0;
            }
            QueryType::Simple => {
                steps.push(PlanStep {
                    step_type: StepType::FullScan,
                    description: "Full table scan".to_string(),
                    estimated_cost: 100.0,
                });
                total_cost += 100.0;
            }
        }

        if query.limit.is_some() {
            steps.push(PlanStep {
                step_type: StepType::Limit,
                description: format!("Limit to {} rows", query.limit.unwrap()),
                estimated_cost: 1.0,
            });
            total_cost += 1.0;
        }

        QueryPlan {
            steps,
            estimated_cost: total_cost,
            estimated_rows: query.limit.unwrap_or(1000),
        }
    }
}

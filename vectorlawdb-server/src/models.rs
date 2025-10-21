use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Request Models
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct QueryRequest {
    pub query: String,
    #[serde(default)]
    pub explain: bool,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct CaseInsertRequest {
    pub case_id: String,
    pub name: String,
    pub text: String,
    pub date: String, // ISO format: YYYY-MM-DD
    pub jurisdiction: Option<String>,
    pub court: Option<String>,
    #[serde(default)]
    pub citations: Vec<String>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct BatchInsertRequest {
    pub cases: Vec<CaseInsertRequest>,
    #[serde(default = "default_true")]
    pub generate_embeddings: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct SimilarCasesRequest {
    pub case_id: Option<String>,
    pub embedding: Option<Vec<f32>>,
    #[serde(default = "default_k")]
    pub k: usize,
    #[serde(default = "default_radius")]
    pub radius: f32,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub jurisdiction: Option<String>,
}

fn default_k() -> usize {
    10
}

fn default_radius() -> f32 {
    0.5
}

#[derive(Debug, Deserialize)]
pub struct SimilarQueryParams {
    #[serde(default = "default_k")]
    pub k: usize,
    #[serde(default = "default_radius")]
    pub radius: f32,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct IndexRebuildRequest {
    pub index_type: String, // "spatial", "citation", or "all"
    #[serde(default = "default_true")]
    pub background: bool,
}

// ============================================================================
// Response Models
// ============================================================================

#[derive(Debug, Serialize)]
pub struct QueryResponse {
    pub results: Vec<serde_json::Value>,
    pub execution_time_ms: f64,
    pub rows_returned: usize,
    pub rows_scanned: usize,
    pub explanation: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CaseResponse {
    pub case_id: String,
    pub name: String,
    pub date: String,
    pub jurisdiction: Option<String>,
    pub court: Option<String>,
    pub citations: Vec<String>,
    pub text_preview: Option<String>,
    pub embedding_dims: Option<usize>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct BatchInsertResponse {
    pub total_cases: usize,
    pub successful: usize,
    pub failed: usize,
    pub errors: Vec<BatchError>,
    pub processing_time_ms: f64,
}

#[derive(Debug, Serialize)]
pub struct BatchError {
    pub case_id: String,
    pub error: String,
}

#[derive(Debug, Serialize)]
pub struct SimilarCase {
    pub case_id: String,
    pub name: String,
    pub distance: f32,
    pub similarity_score: f32,
    pub date: String,
    pub jurisdiction: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SimilarCasesResponse {
    pub query_case_id: Option<String>,
    pub similar_cases: Vec<SimilarCase>,
    pub query_time_ms: f64,
    pub method: String,
}

#[derive(Debug, Serialize)]
pub struct CitationNetworkResponse {
    pub case_id: String,
    pub case_name: String,
    pub direct_citations: Vec<CitationInfo>,
    pub cited_by: Vec<CitationInfo>,
    pub transitive_closure_size: usize,
    pub pagerank_score: f32,
    pub hub_score: f32,
    pub authority_score: f32,
    pub co_cited_cases: Vec<CoCitedCase>,
}

#[derive(Debug, Serialize)]
pub struct CitationInfo {
    pub case_id: String,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct CoCitedCase {
    pub case_id: String,
    pub name: String,
    pub co_citation_count: usize,
}

#[derive(Debug, Serialize)]
pub struct CitationChainsResponse {
    pub case_id: String,
    pub applications: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct StatsResponse {
    pub total_cases: usize,
    pub total_citations: usize,
    pub date_range: DateRange,
    pub spatial_index_stats: serde_json::Value,
    pub citation_graph_stats: serde_json::Value,
    pub storage_stats: serde_json::Value,
    pub query_stats: serde_json::Value,
    pub uptime_seconds: f64,
}

#[derive(Debug, Serialize)]
pub struct DateRange {
    pub min: String,
    pub max: String,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_seconds: f64,
    pub components: HashMap<String, String>,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub detail: Option<String>,
}

// ============================================================================
// Internal Models
// ============================================================================

#[derive(Debug, Clone)]
pub struct CaseData {
    pub case_id: String,
    pub name: String,
    pub text: String,
    pub date: String,
    pub jurisdiction: Option<String>,
    pub court: Option<String>,
    pub citations: Vec<String>,
    pub embedding: Vec<f32>,
    pub metadata: HashMap<String, serde_json::Value>,
}

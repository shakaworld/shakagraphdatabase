use axum::{
    extract::{Path, State, Query as QueryParams},
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use std::sync::Arc;
use std::time::Instant;
use chrono::{Utc, Duration};

use crate::state::AppState;
use crate::models::*;

// Import citation types
use vectorlawdb_citations::{Citation, CitationType};

//=============================================================================
// Health & Root Endpoints
//=============================================================================

pub async fn health_check(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: state.uptime_seconds(),
        components: state.health_check(),
    })
}

//=============================================================================
// Case Management Endpoints
//=============================================================================

pub async fn get_case(
    State(state): State<Arc<AppState>>,
    Path(case_id): Path<String>,
) -> Result<Json<CaseResponse>, ApiError> {
    let storage = state.storage.read();

    let case = storage.get(&case_id)
        .ok_or_else(|| ApiError::NotFound(format!("Case not found: {}", case_id)))?;

    Ok(Json(CaseResponse {
        case_id: case.case_id.clone(),
        name: case.name.clone(),
        date: case.date.clone(),
        jurisdiction: case.jurisdiction.clone(),
        court: case.court.clone(),
        citations: case.citations.clone(),
        text_preview: Some(case.text.chars().take(500).collect()),
        embedding_dims: Some(case.embedding.len()),
        metadata: case.metadata.clone(),
    }))
}

pub async fn insert_case(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CaseInsertRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    tracing::info!("Inserting case: {}", req.case_id);

    // Parse the date
    let case_date = chrono::DateTime::parse_from_rfc3339(&req.date)
        .map_err(|e| ApiError::BadRequest(format!("Invalid date format: {}", e)))?
        .with_timezone(&Utc);

    // Generate embedding (simple hash-based for now)
    let embedding = generate_embedding(&req.text);

    // Store in HashMap
    let case_data = CaseData {
        case_id: req.case_id.clone(),
        name: req.name.clone(),
        text: req.text.clone(),
        date: req.date.clone(),
        jurisdiction: req.jurisdiction.clone(),
        court: req.court.clone(),
        citations: req.citations.clone(),
        embedding: embedding.clone(),
        metadata: req.metadata.clone(),
    };

    {
        let mut storage = state.storage.write();
        storage.insert(req.case_id.clone(), case_data);
    }

    // Index in spatial index
    let position = embedding_to_vector3(&embedding);
    state.spatial_index.insert(position, req.case_id.clone(), case_date);

    // Add citations to citation graph
    if !req.citations.is_empty() {
        let mut graph = state.citation_graph.write();
        for cited_case_id in &req.citations {
            let citation = Citation {
                from_case_id: req.case_id.clone(),
                to_case_id: cited_case_id.clone(),
                citation_type: CitationType::Cites,
                weight: 1.0,
            };
            graph.add_citation(citation);
        }
    }

    Ok(Json(serde_json::json!({
        "status": "success",
        "case_id": req.case_id,
        "indexed": true,
    })))
}

pub async fn batch_insert(
    State(state): State<Arc<AppState>>,
    Json(req): Json<BatchInsertRequest>,
) -> Result<Json<BatchInsertResponse>, ApiError> {
    let start_time = Instant::now();
    let total_cases = req.cases.len();
    let mut successful = 0;
    let mut failed = 0;
    let mut errors = Vec::new();

    for case_req in req.cases {
        match insert_case_internal(&state, case_req).await {
            Ok(_) => successful += 1,
            Err(e) => {
                failed += 1;
                errors.push(BatchError {
                    case_id: "unknown".to_string(),
                    error: e.to_string(),
                });
            }
        }
    }

    // Rebuild citation chains after batch insert
    if successful > 0 {
        let graph = state.citation_graph.read();
        let mut chains = state.citation_chains.write();
        chains.build_from_graph(&graph);
        tracing::info!("Rebuilt citation chains after batch insert");
    }

    Ok(Json(BatchInsertResponse {
        total_cases,
        successful,
        failed,
        errors,
        processing_time_ms: start_time.elapsed().as_secs_f64() * 1000.0,
    }))
}

async fn insert_case_internal(
    state: &AppState,
    req: CaseInsertRequest,
) -> Result<(), String> {
    let case_date = chrono::DateTime::parse_from_rfc3339(&req.date)
        .map_err(|e| format!("Invalid date: {}", e))?
        .with_timezone(&Utc);

    let embedding = generate_embedding(&req.text);

    let case_data = CaseData {
        case_id: req.case_id.clone(),
        name: req.name.clone(),
        text: req.text.clone(),
        date: req.date.clone(),
        jurisdiction: req.jurisdiction.clone(),
        court: req.court.clone(),
        citations: req.citations.clone(),
        embedding: embedding.clone(),
        metadata: req.metadata.clone(),
    };

    {
        let mut storage = state.storage.write();
        storage.insert(req.case_id.clone(), case_data);
    }

    let position = embedding_to_vector3(&embedding);
    state.spatial_index.insert(position, req.case_id.clone(), case_date);

    if !req.citations.is_empty() {
        let mut graph = state.citation_graph.write();
        for cited_case_id in &req.citations {
            let citation = Citation {
                from_case_id: req.case_id.clone(),
                to_case_id: cited_case_id.clone(),
                citation_type: CitationType::Cites,
                weight: 1.0,
            };
            graph.add_citation(citation);
        }
    }

    Ok(())
}

//=============================================================================
// Query Endpoint
//=============================================================================

pub async fn execute_query(
    State(state): State<Arc<AppState>>,
    Json(req): Json<QueryRequest>,
) -> Json<QueryResponse> {
    let start_time = Instant::now();

    // TODO: Implement VQL parser
    // For now, return empty results
    tracing::warn!("VQL query not implemented yet: {}", req.query);

    Json(QueryResponse {
        results: vec![],
        execution_time_ms: start_time.elapsed().as_secs_f64() * 1000.0,
        rows_returned: 0,
        rows_scanned: 0,
        explanation: if req.explain {
            Some("VQL parser not yet implemented".to_string())
        } else {
            None
        },
    })
}

//=============================================================================
// Similarity Search Endpoints
//=============================================================================

pub async fn find_similar_by_id(
    State(state): State<Arc<AppState>>,
    Path(case_id): Path<String>,
    QueryParams(params): QueryParams<SimilarQueryParams>,
) -> Result<Json<SimilarCasesResponse>, ApiError> {
    let start_time = Instant::now();

    // Get the case to find its embedding
    let storage = state.storage.read();
    let query_case = storage.get(&case_id)
        .ok_or_else(|| ApiError::NotFound(format!("Case not found: {}", case_id)))?;

    let query_embedding = &query_case.embedding;
    let query_position = embedding_to_vector3(query_embedding);

    // Parse date filters
    let start_date = params.start_date.as_ref()
        .and_then(|d| chrono::DateTime::parse_from_rfc3339(d).ok())
        .map(|d| d.with_timezone(&Utc));
    let end_date = params.end_date.as_ref()
        .and_then(|d| chrono::DateTime::parse_from_rfc3339(d).ok())
        .map(|d| d.with_timezone(&Utc));

    // Query spatial index
    let candidate_ids = state.spatial_index.query(
        query_position,
        params.radius,
        start_date,
        end_date,
    );

    // Calculate actual similarities and sort
    let mut similar_cases: Vec<SimilarCase> = candidate_ids
        .iter()
        .filter_map(|id| {
            let case = storage.get(id)?;
            if id == &case_id {
                return None; // Skip the query case itself
            }

            let similarity = cosine_similarity(query_embedding, &case.embedding);
            let distance = euclidean_distance(query_embedding, &case.embedding);

            Some(SimilarCase {
                case_id: case.case_id.clone(),
                name: case.name.clone(),
                distance,
                similarity_score: similarity,
                date: case.date.clone(),
                jurisdiction: case.jurisdiction.clone(),
            })
        })
        .collect();

    // Sort by similarity (descending)
    similar_cases.sort_by(|a, b| b.similarity_score.partial_cmp(&a.similarity_score).unwrap());

    // Take only top K
    similar_cases.truncate(params.k);

    Ok(Json(SimilarCasesResponse {
        query_case_id: Some(case_id),
        similar_cases,
        query_time_ms: start_time.elapsed().as_secs_f64() * 1000.0,
        method: "hierarchical_spatial_index".to_string(),
    }))
}

pub async fn find_similar_by_vector(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SimilarCasesRequest>,
) -> Result<Json<SimilarCasesResponse>, ApiError> {
    let start_time = Instant::now();

    let query_embedding = req.embedding
        .ok_or_else(|| ApiError::BadRequest("embedding field required".to_string()))?;

    let query_position = embedding_to_vector3(&query_embedding);

    let start_date = req.start_date.as_ref()
        .and_then(|d| chrono::DateTime::parse_from_rfc3339(d).ok())
        .map(|d| d.with_timezone(&Utc));
    let end_date = req.end_date.as_ref()
        .and_then(|d| chrono::DateTime::parse_from_rfc3339(d).ok())
        .map(|d| d.with_timezone(&Utc));

    let candidate_ids = state.spatial_index.query(
        query_position,
        req.radius,
        start_date,
        end_date,
    );

    let storage = state.storage.read();
    let mut similar_cases: Vec<SimilarCase> = candidate_ids
        .iter()
        .filter_map(|id| {
            let case = storage.get(id)?;
            let similarity = cosine_similarity(&query_embedding, &case.embedding);
            let distance = euclidean_distance(&query_embedding, &case.embedding);

            Some(SimilarCase {
                case_id: case.case_id.clone(),
                name: case.name.clone(),
                distance,
                similarity_score: similarity,
                date: case.date.clone(),
                jurisdiction: case.jurisdiction.clone(),
            })
        })
        .collect();

    similar_cases.sort_by(|a, b| b.similarity_score.partial_cmp(&a.similarity_score).unwrap());
    similar_cases.truncate(req.k);

    Ok(Json(SimilarCasesResponse {
        query_case_id: req.case_id,
        similar_cases,
        query_time_ms: start_time.elapsed().as_secs_f64() * 1000.0,
        method: "hierarchical_spatial_index".to_string(),
    }))
}

//=============================================================================
// Citation Endpoints
//=============================================================================

pub async fn get_citation_network(
    State(state): State<Arc<AppState>>,
    Path(case_id): Path<String>,
) -> Result<Json<CitationNetworkResponse>, ApiError> {
    let storage = state.storage.read();
    let case = storage.get(&case_id)
        .ok_or_else(|| ApiError::NotFound(format!("Case not found: {}", case_id)))?;

    let graph = state.citation_graph.read();

    // Get direct citations
    let direct_citations: Vec<CitationInfo> = graph.get_citations_from(&case_id)
        .iter()
        .filter_map(|citation| {
            let cited_case = storage.get(&citation.to_case_id)?;
            Some(CitationInfo {
                case_id: cited_case.case_id.clone(),
                name: cited_case.name.clone(),
            })
        })
        .collect();

    // Get cases that cite this case
    let cited_by: Vec<CitationInfo> = graph.get_citations_to(&case_id)
        .iter()
        .filter_map(|citation| {
            let citing_case = storage.get(&citation.from_case_id)?;
            Some(CitationInfo {
                case_id: citing_case.case_id.clone(),
                name: citing_case.name.clone(),
            })
        })
        .collect();

    // Calculate citation rank (simple version)
    let pagerank_score = graph.citation_rank(&case_id) as f32;

    Ok(Json(CitationNetworkResponse {
        case_id: case_id.clone(),
        case_name: case.name.clone(),
        direct_citations,
        cited_by,
        transitive_closure_size: 0, // TODO: Implement BFS for transitive closure
        pagerank_score,
        hub_score: 0.0, // TODO: Implement HITS algorithm
        authority_score: pagerank_score,
        co_cited_cases: vec![], // TODO: Implement co-citation analysis
    }))
}

pub async fn get_citation_chains(
    State(state): State<Arc<AppState>>,
    Path(case_id): Path<String>,
) -> Result<Json<CitationChainsResponse>, ApiError> {
    let chains = state.citation_chains.read();
    let case_chains = chains.get_chains_for_case(&case_id);

    let mut applications = std::collections::HashMap::new();

    for chain in case_chains {
        let chain_info = serde_json::json!({
            "type": format!("{:?}", chain.chain_type),
            "length": chain.length(),
            "strength": chain.strength,
            "cases": chain.cases,
        });

        applications.insert(
            format!("{:?}", chain.chain_type),
            chain_info,
        );
    }

    Ok(Json(CitationChainsResponse {
        case_id,
        applications,
    }))
}

//=============================================================================
// Statistics & Maintenance Endpoints
//=============================================================================

pub async fn get_statistics(
    State(state): State<Arc<AppState>>,
) -> Json<StatsResponse> {
    let storage = state.storage.read();
    let spatial_stats = state.spatial_index.get_stats();
    let graph = state.citation_graph.read();

    // Calculate date range
    let dates: Vec<&String> = storage.values().map(|c| &c.date).collect();
    let (min_date, max_date) = if dates.is_empty() {
        ("unknown".to_string(), "unknown".to_string())
    } else {
        (
            dates.iter().min().unwrap().to_string(),
            dates.iter().max().unwrap().to_string(),
        )
    };

    Json(StatsResponse {
        total_cases: storage.len(),
        total_citations: graph.get_all_cases().len(),
        date_range: DateRange {
            min: min_date,
            max: max_date,
        },
        spatial_index_stats: serde_json::to_value(&spatial_stats).unwrap(),
        citation_graph_stats: serde_json::json!({
            "total_cases": graph.get_all_cases().len(),
        }),
        storage_stats: serde_json::json!({
            "type": "HashMap",
            "size": storage.len(),
        }),
        query_stats: serde_json::json!({
            "total_queries": spatial_stats.total_queries,
            "avg_query_time_ms": spatial_stats.avg_query_time_ms,
        }),
        uptime_seconds: state.uptime_seconds(),
    })
}

pub async fn rebuild_index(
    State(state): State<Arc<AppState>>,
    Json(req): Json<IndexRebuildRequest>,
) -> Json<serde_json::Value> {
    tracing::info!("Rebuilding index: {}", req.index_type);

    match req.index_type.as_str() {
        "spatial" | "all" => {
            state.spatial_index.rebuild();
            tracing::info!("Spatial index rebuilt");
        }
        "citation" | "all" => {
            let graph = state.citation_graph.read();
            let mut chains = state.citation_chains.write();
            chains.build_from_graph(&graph);
            tracing::info!("Citation chains rebuilt");
        }
        _ => {}
    }

    Json(serde_json::json!({
        "status": "success",
        "index_type": req.index_type,
    }))
}

pub async fn clear_caches(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    state.spatial_index.clear_stats();

    Json(serde_json::json!({
        "status": "success",
        "message": "Caches cleared",
    }))
}

//=============================================================================
// Helper Functions
//=============================================================================

fn generate_embedding(text: &str) -> Vec<f32> {
    // Simple hash-based embedding for demonstration
    // In production, use a proper embedding model
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut embedding = vec![0.0; 384]; // 384-dim like all-MiniLM-L6-v2

    // Hash different n-grams
    for (i, word) in text.split_whitespace().enumerate() {
        let mut hasher = DefaultHasher::new();
        word.hash(&mut hasher);
        let hash = hasher.finish();

        let idx = (hash as usize) % embedding.len();
        embedding[idx] += 1.0;

        if i > 100 {
            break; // Limit processing
        }
    }

    // Normalize
    let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in &mut embedding {
            *x /= norm;
        }
    }

    embedding
}

fn embedding_to_vector3(embedding: &[f32]) -> nalgebra::Vector3<f32> {
    // Take first 3 dimensions for spatial indexing
    if embedding.len() >= 3 {
        nalgebra::Vector3::new(embedding[0], embedding[1], embedding[2])
    } else {
        nalgebra::Vector3::zeros()
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot / (norm_a * norm_b)
}

fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return f32::MAX;
    }

    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f32>()
        .sqrt()
}

//=============================================================================
// Error Handling
//=============================================================================

pub enum ApiError {
    NotFound(String),
    BadRequest(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
        };

        let body = Json(ErrorResponse {
            error: message.clone(),
            detail: Some(message),
        });

        (status, body).into_response()
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::NotFound(msg) => write!(f, "Not found: {}", msg),
            ApiError::BadRequest(msg) => write!(f, "Bad request: {}", msg),
        }
    }
}

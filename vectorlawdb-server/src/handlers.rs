use axum::{extract::State, Json};
use std::sync::Arc;

use crate::{AppState, QueryRequest, QueryResponse};

pub async fn query_cases(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<QueryRequest>,
) -> Json<QueryResponse> {
    tracing::info!("Received query: {} (type: {})", req.query, req.query_type);

    // TODO: Execute actual query
    let results = vec![];

    Json(QueryResponse {
        results,
        count: 0,
        query_time_ms: 0.0,
    })
}

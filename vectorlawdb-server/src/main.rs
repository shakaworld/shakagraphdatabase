use axum::{
    Router,
    routing::{get, post, delete},
    extract::{Path, State, Query as QueryParams},
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tower_http::cors::{CorsLayer, Any};
use std::time::Instant;

mod models;
mod handlers;
mod state;

use models::*;
use handlers::*;
use state::AppState;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Initialize app state
    let state = Arc::new(AppState::new().await);

    // Build router with all endpoints
    let app = Router::new()
        // Root
        .route("/", get(root))
        .route("/health", get(health_check))

        // Query endpoints
        .route("/query", post(execute_query))

        // Case endpoints
        .route("/case/:case_id", get(get_case))
        .route("/case", post(insert_case))
        .route("/batch", post(batch_insert))

        // Similarity search
        .route("/similar/:case_id", get(find_similar_by_id))
        .route("/similar/vector", post(find_similar_by_vector))

        // Citation endpoints
        .route("/citations/:case_id", get(get_citation_network))
        .route("/citations/:case_id/chains", get(get_citation_chains))

        // Statistics and maintenance
        .route("/stats", get(get_statistics))
        .route("/index/rebuild", post(rebuild_index))
        .route("/cache", delete(clear_caches))

        // Add state
        .with_state(state)

        // Add middleware
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any)
        );

    let listener = TcpListener::bind("0.0.0.0:8000").await.unwrap();

    tracing::info!("🚀 VectorLawDB server listening on {}", listener.local_addr().unwrap());
    tracing::info!("📚 API documentation: http://localhost:8000/");

    axum::serve(listener, app).await.unwrap();
}

async fn root() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "message": "VectorLawDB API",
        "version": env!("CARGO_PKG_VERSION"),
        "endpoints": {
            "health": "GET /health",
            "query": "POST /query",
            "case": "GET /case/:id, POST /case",
            "similar": "GET /similar/:id, POST /similar/vector",
            "citations": "GET /citations/:id, GET /citations/:id/chains",
            "stats": "GET /stats"
        }
    }))
}

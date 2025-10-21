use axum::{
    Router,
    routing::{get, post},
    extract::State,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;

mod routes;
mod handlers;

#[derive(Clone)]
struct AppState {
    // TODO: Initialize storage, spatial index, etc.
}

#[derive(Deserialize)]
struct QueryRequest {
    query: String,
    query_type: String,
    limit: usize,
}

#[derive(Serialize)]
struct QueryResponse {
    results: Vec<serde_json::Value>,
    count: usize,
    query_time_ms: f64,
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let state = AppState {
        // TODO: Initialize components
    };

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/query", post(handlers::query_cases))
        .layer(TraceLayer::new_for_http())
        .with_state(Arc::new(state));

    let listener = TcpListener::bind("0.0.0.0:8000").await.unwrap();

    tracing::info!("VectorLawDB server listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, app).await.unwrap();
}

async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

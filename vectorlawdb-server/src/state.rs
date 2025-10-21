use std::sync::Arc;
use std::time::Instant;
use parking_lot::RwLock;
use std::collections::HashMap;
use crate::models::CaseData;

/// Application state shared across all requests
#[derive(Clone)]
pub struct AppState {
    pub start_time: Instant,
    pub storage: Arc<RwLock<HashMap<String, CaseData>>>,
    // In production, these would be actual components:
    // pub lsm_tree: Arc<LSMTree>,
    // pub spatial_index: Arc<HierarchicalSpatialIndex>,
    // pub citation_graph: Arc<CitationGraph>,
    // pub citation_chains: Arc<JurisCitationChains>,
}

impl AppState {
    pub async fn new() -> Self {
        tracing::info!("Initializing VectorLawDB application state...");

        // For now, use in-memory HashMap for storage
        // In production, this would initialize the LSM-tree and other components
        let storage = Arc::new(RwLock::new(HashMap::new()));

        tracing::info!("✓ Application state initialized");

        Self {
            start_time: Instant::now(),
            storage,
        }
    }

    pub fn uptime_seconds(&self) -> f64 {
        self.start_time.elapsed().as_secs_f64()
    }

    pub fn health_check(&self) -> HashMap<String, String> {
        let mut components = HashMap::new();
        components.insert("storage".to_string(), "ok".to_string());
        components.insert("spatial_index".to_string(), "stub".to_string());
        components.insert("citation_graph".to_string(), "stub".to_string());
        components.insert("query_executor".to_string(), "stub".to_string());
        components
    }
}

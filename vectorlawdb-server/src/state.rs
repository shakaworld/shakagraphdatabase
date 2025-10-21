use std::sync::Arc;
use std::time::Instant;
use parking_lot::RwLock;
use std::collections::HashMap;
use crate::models::CaseData;

// Import our components
use vectorlawdb_spatial::{HierarchicalSpatialIndex, IndexConfig};
use vectorlawdb_citations::{CitationGraph, JurisCitationChains};

/// Application state shared across all requests
#[derive(Clone)]
pub struct AppState {
    pub start_time: Instant,

    /// In-memory storage for case data
    /// TODO: Replace with LSMTree for production persistence
    pub storage: Arc<RwLock<HashMap<String, CaseData>>>,

    /// Hierarchical spatial index (Icosahedron -> Tesseract -> Octree)
    pub spatial_index: Arc<HierarchicalSpatialIndex>,

    /// Citation graph for tracking case citations
    pub citation_graph: Arc<RwLock<CitationGraph>>,

    /// Citation chains for legal analysis
    pub citation_chains: Arc<RwLock<JurisCitationChains>>,
}

impl AppState {
    pub async fn new() -> Self {
        tracing::info!("Initializing VectorLawDB application state...");

        // Initialize in-memory storage
        let storage = Arc::new(RwLock::new(HashMap::new()));
        tracing::info!("✓ Storage initialized");

        // Initialize hierarchical spatial index with default config
        let spatial_config = IndexConfig::default();
        let spatial_index = Arc::new(HierarchicalSpatialIndex::new(spatial_config));
        tracing::info!("✓ Hierarchical spatial index initialized (20 faces, 8 temporal slices each)");

        // Initialize citation graph
        let citation_graph = Arc::new(RwLock::new(CitationGraph::new()));
        tracing::info!("✓ Citation graph initialized");

        // Initialize citation chains
        let citation_chains = Arc::new(RwLock::new(JurisCitationChains::new()));
        tracing::info!("✓ JurisCitationChains initialized (15 chain types)");

        tracing::info!("✅ Application state fully initialized");

        Self {
            start_time: Instant::now(),
            storage,
            spatial_index,
            citation_graph,
            citation_chains,
        }
    }

    /// Get uptime in seconds
    pub fn uptime_seconds(&self) -> f64 {
        self.start_time.elapsed().as_secs_f64()
    }

    /// Health check for all components
    pub fn health_check(&self) -> HashMap<String, String> {
        let mut components = HashMap::new();

        // Storage check
        let storage_count = self.storage.read().len();
        components.insert("storage".to_string(), format!("ok ({} cases)", storage_count));

        // Spatial index check
        let spatial_stats = self.spatial_index.get_stats();
        components.insert(
            "spatial_index".to_string(),
            format!("ok ({} cases indexed)", spatial_stats.total_cases)
        );

        // Citation graph check
        let graph = self.citation_graph.read();
        let graph_cases = graph.get_all_cases().len();
        components.insert(
            "citation_graph".to_string(),
            format!("ok ({} cases)", graph_cases)
        );

        // Citation chains check
        let chains = self.citation_chains.read();
        let chains_count = chains.total_chains();
        components.insert(
            "citation_chains".to_string(),
            format!("ok ({} chains)", chains_count)
        );

        components.insert("query_executor".to_string(), "stub".to_string());

        components
    }

    /// Get comprehensive statistics
    pub fn get_stats(&self) -> serde_json::Value {
        let spatial_stats = self.spatial_index.get_stats();

        let citation_graph = self.citation_graph.read();
        let all_cases = citation_graph.get_all_cases();

        serde_json::json!({
            "storage": {
                "total_cases": self.storage.read().len(),
                "type": "in-memory HashMap"
            },
            "spatial_index": {
                "total_cases": spatial_stats.total_cases,
                "total_inserts": spatial_stats.total_inserts,
                "total_queries": spatial_stats.total_queries,
                "avg_query_time_ms": spatial_stats.avg_query_time_ms,
                "num_faces": spatial_stats.num_faces,
                "num_temporal_slices": spatial_stats.num_temporal_slices,
            },
            "citation_graph": {
                "total_cases": all_cases.len(),
            },
            "citation_chains": {
                "total_chains": self.citation_chains.read().total_chains(),
            },
            "uptime_seconds": self.uptime_seconds(),
        })
    }
}

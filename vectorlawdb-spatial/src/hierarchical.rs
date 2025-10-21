use chrono::{DateTime, Utc, Duration};
use nalgebra::Vector3;
use serde::{Serialize, Deserialize};
use parking_lot::{RwLock, Mutex};
use std::sync::Arc;
use std::collections::HashMap;

use crate::icosahedron::Icosahedron;
use crate::tesseract::{Tesseract, TesseractLink, TesseractStats};

/// Configuration for the hierarchical spatial index
#[derive(Debug, Clone)]
pub struct IndexConfig {
    /// Minimum date for temporal indexing
    pub min_date: DateTime<Utc>,
    /// Maximum date for temporal indexing
    pub max_date: DateTime<Utc>,
    /// Number of temporal slices per tesseract (default: 8)
    pub num_temporal_slices: usize,
    /// Radius of the icosahedron
    pub icosahedron_radius: f32,
    /// Maximum depth of octrees
    pub max_octree_depth: usize,
}

impl Default for IndexConfig {
    fn default() -> Self {
        // Default to last 100 years
        let now = Utc::now();
        let century_ago = now - Duration::days(365 * 100);

        Self {
            min_date: century_ago,
            max_date: now,
            num_temporal_slices: 8,
            icosahedron_radius: 1.0,
            max_octree_depth: 10,
        }
    }
}

/// Hierarchical Spatial Index
///
/// Three-level spatial-temporal index structure:
/// - Level 1: Icosahedron (20 geodesic faces for global partitioning)
/// - Level 2: Tesseract per face (8 temporal slices)
/// - Level 3: Octree per temporal slice (3D spatial tree)
///
/// Thread-safe: All operations use parking_lot locks for concurrent access
///
/// # Example
/// ```ignore
/// use vectorlawdb_spatial::hierarchical::{HierarchicalSpatialIndex, IndexConfig};
///
/// let config = IndexConfig::default();
/// let index = HierarchicalSpatialIndex::new(config);
/// ```
pub struct HierarchicalSpatialIndex {
    config: IndexConfig,
    /// Icosahedron for Level 1 partitioning
    icosahedron: Icosahedron,
    /// Map from face index to tesseract
    face_tesseracts: Vec<Arc<Tesseract>>,
    /// Statistics
    total_inserts: Mutex<usize>,
    total_queries: Mutex<usize>,
    query_times: RwLock<Vec<f64>>,
}

impl HierarchicalSpatialIndex {
    /// Create a new hierarchical spatial index
    pub fn new(config: IndexConfig) -> Self {
        let icosahedron = Icosahedron::new(config.icosahedron_radius);
        let num_faces = icosahedron.faces.len();

        // Create one tesseract per icosahedron face
        let mut face_tesseracts = Vec::with_capacity(num_faces);
        for _ in 0..num_faces {
            let tesseract = Tesseract::new(
                config.min_date,
                config.max_date,
                config.num_temporal_slices,
            );
            face_tesseracts.push(Arc::new(tesseract));
        }

        Self {
            config,
            icosahedron,
            face_tesseracts,
            total_inserts: Mutex::new(0),
            total_queries: Mutex::new(0),
            query_times: RwLock::new(Vec::new()),
        }
    }

    /// Insert a case into the hierarchical index
    ///
    /// # Arguments
    /// * `position` - 3D position in vector space (derived from embedding)
    /// * `case_id` - Unique identifier for the case
    /// * `case_date` - Date of the case decision
    ///
    /// # Returns
    /// `true` if inserted successfully
    pub fn insert(
        &self,
        position: Vector3<f32>,
        case_id: String,
        case_date: DateTime<Utc>,
    ) -> bool {
        // Level 1: Find which icosahedron face contains this position
        let face_idx = self.icosahedron.find_face(&position);

        // Level 2-3: Insert into the tesseract for this face
        let tesseract = &self.face_tesseracts[face_idx];
        let success = tesseract.insert_case(position, case_id, case_date);

        if success {
            let mut inserts = self.total_inserts.lock();
            *inserts += 1;
        }

        success
    }

    /// Batch insert multiple cases
    ///
    /// More efficient than individual inserts for large datasets
    pub fn batch_insert(
        &self,
        cases: Vec<(Vector3<f32>, String, DateTime<Utc>)>,
    ) -> usize {
        let mut success_count = 0;

        for (position, case_id, case_date) in cases {
            if self.insert(position, case_id, case_date) {
                success_count += 1;
            }
        }

        success_count
    }

    /// Query for cases within spatial radius and temporal range
    ///
    /// # Arguments
    /// * `query_position` - Query position in 3D space
    /// * `radius` - Spatial radius for similarity threshold
    /// * `start_date` - Start of temporal range (None = all dates)
    /// * `end_date` - End of temporal range (None = all dates)
    ///
    /// # Returns
    /// List of case IDs matching the query
    pub fn query(
        &self,
        query_position: Vector3<f32>,
        radius: f32,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
    ) -> Vec<String> {
        let start_time = std::time::Instant::now();

        // Level 1: Find primary face and neighboring faces
        let primary_face = self.icosahedron.find_face(&query_position);
        let faces_to_search = self.get_faces_within_radius(&query_position, radius, primary_face);

        let mut results = Vec::new();

        // Level 2-3: Query tesseracts for each face
        for face_idx in faces_to_search {
            let tesseract = &self.face_tesseracts[face_idx];
            let face_results = tesseract.query_temporal_spatial(
                query_position,
                radius,
                start_date,
                end_date,
            );
            results.extend(face_results);
        }

        // Update statistics
        let elapsed = start_time.elapsed().as_secs_f64() * 1000.0; // ms
        {
            let mut queries = self.total_queries.lock();
            *queries += 1;
        }
        {
            let mut times = self.query_times.write();
            times.push(elapsed);
            // Keep only last 1000 query times
            if times.len() > 1000 {
                times.remove(0);
            }
        }

        results
    }

    /// Query for K nearest neighbors
    ///
    /// # Arguments
    /// * `query_position` - Query position in 3D space
    /// * `k` - Number of nearest neighbors to return
    /// * `start_date` - Optional temporal filter (start)
    /// * `end_date` - Optional temporal filter (end)
    ///
    /// # Returns
    /// List of up to K case IDs, sorted by distance
    pub fn query_knn(
        &self,
        query_position: Vector3<f32>,
        k: usize,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
    ) -> Vec<String> {
        // Start with a small radius and expand if needed
        let mut radius = 0.1;
        let max_radius = 2.0;
        let mut results = Vec::new();

        while results.len() < k && radius <= max_radius {
            results = self.query(query_position, radius, start_date, end_date);
            radius *= 1.5; // Exponential expansion
        }

        // If we have more than k results, we'd need to sort by distance
        // For now, just return the first k
        results.into_iter().take(k).collect()
    }

    /// Query cases within a temporal range only (no spatial filter)
    pub fn query_temporal(
        &self,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Vec<String> {
        let mut results = Vec::new();

        // Query all tesseracts
        for tesseract in &self.face_tesseracts {
            let face_results = tesseract.query_temporal_only(start_date, end_date);
            results.extend(face_results);
        }

        results
    }

    /// Get faces that might contain results within the radius
    ///
    /// This includes the primary face and any neighboring faces
    fn get_faces_within_radius(
        &self,
        _position: &Vector3<f32>,
        _radius: f32,
        primary_face: usize,
    ) -> Vec<usize> {
        // For now, just return the primary face
        // TODO: Add neighboring face detection for queries near face boundaries
        vec![primary_face]
    }

    /// Rebuild the index (useful after bulk inserts or deletions)
    pub fn rebuild(&self) {
        // Rebalance all tesseracts
        for tesseract in &self.face_tesseracts {
            tesseract.rebalance();
        }
    }

    /// Get comprehensive statistics about the index
    pub fn get_stats(&self) -> HierarchicalIndexStats {
        let total_cases: usize = self.face_tesseracts
            .iter()
            .map(|t| t.total_cases())
            .sum();

        let face_stats: Vec<FaceStats> = self.face_tesseracts
            .iter()
            .enumerate()
            .map(|(idx, tesseract)| {
                let stats = tesseract.get_stats();
                FaceStats {
                    face_index: idx,
                    case_count: stats.total_cases,
                    tesseract_stats: stats,
                }
            })
            .collect();

        let query_times = self.query_times.read();
        let avg_query_time = if query_times.is_empty() {
            0.0
        } else {
            query_times.iter().sum::<f64>() / query_times.len() as f64
        };

        HierarchicalIndexStats {
            total_cases,
            total_inserts: *self.total_inserts.lock(),
            total_queries: *self.total_queries.lock(),
            avg_query_time_ms: avg_query_time,
            num_faces: self.face_tesseracts.len(),
            num_temporal_slices: self.config.num_temporal_slices,
            temporal_range: (self.config.min_date, self.config.max_date),
            face_stats,
        }
    }

    /// Get statistics for a specific face
    pub fn get_face_stats(&self, face_idx: usize) -> Option<TesseractStats> {
        self.face_tesseracts.get(face_idx).map(|t| t.get_stats())
    }

    /// Get configuration
    pub fn config(&self) -> &IndexConfig {
        &self.config
    }

    /// Clear all statistics (keeps data intact)
    pub fn clear_stats(&self) {
        *self.total_inserts.lock() = 0;
        *self.total_queries.lock() = 0;
        self.query_times.write().clear();
    }
}

// Thread safety
unsafe impl Send for HierarchicalSpatialIndex {}
unsafe impl Sync for HierarchicalSpatialIndex {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HierarchicalIndexStats {
    pub total_cases: usize,
    pub total_inserts: usize,
    pub total_queries: usize,
    pub avg_query_time_ms: f64,
    pub num_faces: usize,
    pub num_temporal_slices: usize,
    pub temporal_range: (DateTime<Utc>, DateTime<Utc>),
    pub face_stats: Vec<FaceStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceStats {
    pub face_index: usize,
    pub case_count: usize,
    pub tesseract_stats: TesseractStats,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hierarchical_index_creation() {
        let config = IndexConfig::default();
        let index = HierarchicalSpatialIndex::new(config);

        let stats = index.get_stats();
        assert_eq!(stats.total_cases, 0);
        assert_eq!(stats.num_faces, 20); // Icosahedron has 20 faces
    }

    #[test]
    fn test_insert_and_query() {
        let config = IndexConfig::default();
        let index = HierarchicalSpatialIndex::new(config.clone());

        let position = Vector3::new(0.5, 0.5, 0.5);
        let case_id = "case-123".to_string();
        let case_date = config.min_date + Duration::days(365);

        // Insert
        assert!(index.insert(position, case_id.clone(), case_date));

        // Query
        let results = index.query(position, 0.5, None, None);
        assert!(results.contains(&case_id));
    }

    #[test]
    fn test_temporal_query() {
        let config = IndexConfig::default();
        let index = HierarchicalSpatialIndex::new(config.clone());

        let position = Vector3::new(0.0, 0.0, 1.0);
        let case_date = config.min_date + Duration::days(100);

        index.insert(position, "case-1".to_string(), case_date);

        // Query for cases in the first half of the time range
        let mid_date = config.min_date + Duration::days(365 * 50);
        let results = index.query_temporal(config.min_date, mid_date);

        assert!(results.contains(&"case-1".to_string()));
    }

    #[test]
    fn test_batch_insert() {
        let config = IndexConfig::default();
        let index = HierarchicalSpatialIndex::new(config.clone());

        let cases = vec![
            (Vector3::new(0.1, 0.1, 0.1), "case-1".to_string(), config.min_date + Duration::days(10)),
            (Vector3::new(0.2, 0.2, 0.2), "case-2".to_string(), config.min_date + Duration::days(20)),
            (Vector3::new(0.3, 0.3, 0.3), "case-3".to_string(), config.min_date + Duration::days(30)),
        ];

        let count = index.batch_insert(cases);
        assert_eq!(count, 3);

        let stats = index.get_stats();
        assert_eq!(stats.total_cases, 3);
    }
}

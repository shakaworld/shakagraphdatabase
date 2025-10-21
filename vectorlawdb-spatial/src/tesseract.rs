use chrono::{DateTime, Utc, Duration};
use nalgebra::Vector3;
use serde::{Serialize, Deserialize};
use crate::octree::Octree3D;
use parking_lot::{RwLock, Mutex};
use std::sync::Arc;

/// A single temporal slice within the tesseract.
/// Each slice represents a time period and contains an octree.
/// Uses parking_lot for high-performance concurrent access.
#[derive(Debug)]
pub struct TemporalSlice {
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    octree: RwLock<Octree3D>,
    case_count: Mutex<usize>,
}

impl TemporalSlice {
    pub fn new(start_date: DateTime<Utc>, end_date: DateTime<Utc>) -> Self {
        Self {
            start_date,
            end_date,
            octree: RwLock::new(Octree3D::new(Vector3::zeros(), 1.0, 10)),
            case_count: Mutex::new(0),
        }
    }

    /// Check if a date falls within this slice's time range
    pub fn contains_date(&self, date: &DateTime<Utc>) -> bool {
        *date >= self.start_date && *date < self.end_date
    }

    /// Insert a case into this slice's octree
    /// Thread-safe with write lock
    pub fn insert_case(&self, position: Vector3<f32>, case_id: String, case_date: DateTime<Utc>) -> bool {
        if !self.contains_date(&case_date) {
            return false;
        }

        // Get write lock on octree
        let mut octree = self.octree.write();
        octree.insert(position, case_id);

        // Update count atomically
        let mut count = self.case_count.lock();
        *count += 1;

        true
    }

    /// Query cases within spatial radius in this time slice
    /// Thread-safe with read lock
    pub fn query_spatial(&self, query_vector: Vector3<f32>, radius: f32) -> Vec<String> {
        let octree = self.octree.read();
        octree.query_radius(query_vector, radius)
    }

    /// Get statistics about this temporal slice
    pub fn get_stats(&self) -> SliceStats {
        SliceStats {
            start_date: self.start_date,
            end_date: self.end_date,
            case_count: *self.case_count.lock(),
        }
    }

    /// Get current case count
    pub fn case_count(&self) -> usize {
        *self.case_count.lock()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SliceStats {
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub case_count: usize,
}

/// 4D Hypercube (Tesseract) for temporal-spatial indexing.
///
/// The tesseract divides time into 8 slices, each containing a 3D octree
/// for spatial partitioning. This enables efficient temporal range queries
/// combined with spatial constraints.
///
/// Thread-safe: All operations use parking_lot locks for high-performance
/// concurrent access.
///
/// # Temporal Structure
/// - 8 time slices evenly distributed across date range
/// - Each slice is a 3D cube (octree) in vector space
/// - Queries can span multiple slices for temporal ranges
///
/// # Example
/// ```ignore
/// use chrono::Utc;
/// use vectorlawdb_spatial::tesseract::Tesseract;
///
/// let tesseract = Tesseract::new(
///     Utc::now() - Duration::days(365 * 100),
///     Utc::now(),
///     8
/// );
/// ```
pub struct Tesseract {
    pub min_date: DateTime<Utc>,
    pub max_date: DateTime<Utc>,
    pub num_slices: usize,
    slices: Vec<Arc<TemporalSlice>>,
    total_cases: Mutex<usize>,
}

impl Tesseract {
    /// Create a new Tesseract with the specified temporal range
    ///
    /// # Arguments
    /// * `min_date` - Earliest date in dataset
    /// * `max_date` - Latest date in dataset
    /// * `num_slices` - Number of temporal slices (default 8 for hypercube)
    pub fn new(min_date: DateTime<Utc>, max_date: DateTime<Utc>, num_slices: usize) -> Self {
        let slices = Self::create_temporal_slices(min_date, max_date, num_slices);

        Self {
            min_date,
            max_date,
            num_slices,
            slices,
            total_cases: Mutex::new(0),
        }
    }

    fn create_temporal_slices(
        min_date: DateTime<Utc>,
        max_date: DateTime<Utc>,
        num_slices: usize,
    ) -> Vec<Arc<TemporalSlice>> {
        let total_duration = max_date - min_date;
        let slice_duration = total_duration / num_slices as i32;

        let mut slices = Vec::with_capacity(num_slices);
        let mut current_date = min_date;

        for i in 0..num_slices {
            let start_date = current_date;
            let end_date = if i == num_slices - 1 {
                max_date // Last slice includes max_date
            } else {
                current_date + slice_duration
            };

            slices.push(Arc::new(TemporalSlice::new(start_date, end_date)));
            current_date = end_date;
        }

        slices
    }

    /// Find which temporal slice contains a given date
    /// Uses binary search for O(log n) time complexity
    fn find_slice_index(&self, date: &DateTime<Utc>) -> Option<usize> {
        if *date < self.min_date || *date >= self.max_date {
            return None;
        }

        // Binary search for efficiency
        let mut left = 0;
        let mut right = self.num_slices - 1;

        while left <= right {
            let mid = (left + right) / 2;
            let slice = &self.slices[mid];

            if slice.contains_date(date) {
                return Some(mid);
            } else if *date < slice.start_date {
                if mid == 0 {
                    break;
                }
                right = mid - 1;
            } else {
                left = mid + 1;
            }
        }

        None
    }

    /// Insert a case into the appropriate temporal slice
    /// Thread-safe: Can be called concurrently from multiple threads
    ///
    /// # Arguments
    /// * `position` - Spatial position in vector space
    /// * `case_id` - Unique identifier for the case
    /// * `case_date` - Date of the case decision
    ///
    /// # Returns
    /// `true` if inserted successfully, `false` if date out of range
    pub fn insert_case(
        &self,
        position: Vector3<f32>,
        case_id: String,
        case_date: DateTime<Utc>,
    ) -> bool {
        let slice_idx = match self.find_slice_index(&case_date) {
            Some(idx) => idx,
            None => return false,
        };

        let success = self.slices[slice_idx].insert_case(position, case_id, case_date);

        if success {
            let mut total = self.total_cases.lock();
            *total += 1;
        }

        success
    }

    /// Query cases within temporal range and spatial radius
    /// Thread-safe: Can be called concurrently with inserts
    ///
    /// # Arguments
    /// * `query_vector` - Query vector for spatial similarity
    /// * `radius` - Spatial radius for similarity threshold
    /// * `start_date` - Start of temporal range (None = min_date)
    /// * `end_date` - End of temporal range (None = max_date)
    ///
    /// # Returns
    /// List of case IDs matching both temporal and spatial constraints
    pub fn query_temporal_spatial(
        &self,
        query_vector: Vector3<f32>,
        radius: f32,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
    ) -> Vec<String> {
        let start_date = start_date.unwrap_or(self.min_date);
        let end_date = end_date.unwrap_or(self.max_date);

        let start_idx = match self.find_slice_index(&start_date) {
            Some(idx) => idx,
            None => return vec![],
        };

        let end_idx = match self.find_slice_index(&end_date) {
            Some(idx) => idx,
            None => return vec![],
        };

        let mut results = Vec::new();

        // Query each slice in the temporal range
        // Slices can be queried concurrently
        for i in start_idx..=end_idx {
            let slice_results = self.slices[i].query_spatial(query_vector, radius);
            results.extend(slice_results);
        }

        results
    }

    /// Query all cases within a temporal range (no spatial filter)
    pub fn query_temporal_only(
        &self,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Vec<String> {
        let start_idx = match self.find_slice_index(&start_date) {
            Some(idx) => idx,
            None => return vec![],
        };

        let end_idx = match self.find_slice_index(&end_date) {
            Some(idx) => idx,
            None => return vec![],
        };

        let mut results = Vec::new();

        for i in start_idx..=end_idx {
            // Get all cases from this slice
            let slice_results = self.slices[i].query_spatial(Vector3::zeros(), f32::MAX);
            results.extend(slice_results);
        }

        results
    }

    /// Get case distribution across temporal slices
    pub fn get_slice_distribution(&self) -> Vec<SliceStats> {
        self.slices.iter().map(|s| s.get_stats()).collect()
    }

    /// Rebalance octrees in slices that are too deep or unbalanced
    pub fn rebalance(&self) {
        for slice in &self.slices {
            if slice.case_count() > 1000 {
                // Threshold for rebalancing
                // TODO: Implement octree rebalancing
            }
        }
    }

    /// Get comprehensive statistics about the tesseract
    pub fn get_stats(&self) -> TesseractStats {
        TesseractStats {
            total_cases: *self.total_cases.lock(),
            num_slices: self.num_slices,
            min_date: self.min_date,
            max_date: self.max_date,
            slice_distribution: self.get_slice_distribution(),
        }
    }

    /// Get total number of cases
    pub fn total_cases(&self) -> usize {
        *self.total_cases.lock()
    }

    /// Get reference to a specific temporal slice
    pub fn get_slice(&self, index: usize) -> Option<Arc<TemporalSlice>> {
        self.slices.get(index).cloned()
    }
}

// Implement Send and Sync for thread safety
unsafe impl Send for Tesseract {}
unsafe impl Sync for Tesseract {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TesseractStats {
    pub total_cases: usize,
    pub num_slices: usize,
    pub min_date: DateTime<Utc>,
    pub max_date: DateTime<Utc>,
    pub slice_distribution: Vec<SliceStats>,
}

/// Lightweight link from Icosahedron face to Tesseract
/// Uses Arc for zero-cost sharing
pub struct TesseractLink {
    pub tesseract: Arc<Tesseract>,
}

impl TesseractLink {
    pub fn new(tesseract: Arc<Tesseract>) -> Self {
        Self { tesseract }
    }

    /// Create from owned tesseract
    pub fn from_owned(tesseract: Tesseract) -> Self {
        Self {
            tesseract: Arc::new(tesseract),
        }
    }

    /// Insert case through the link
    pub fn insert_case(
        &self,
        position: Vector3<f32>,
        case_id: String,
        case_date: DateTime<Utc>,
    ) -> bool {
        self.tesseract.insert_case(position, case_id, case_date)
    }

    /// Query through the link
    pub fn query(
        &self,
        query_vector: Vector3<f32>,
        radius: f32,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
    ) -> Vec<String> {
        self.tesseract.query_temporal_spatial(query_vector, radius, start_date, end_date)
    }

    /// Get statistics through the link
    pub fn get_stats(&self) -> TesseractStats {
        self.tesseract.get_stats()
    }

    /// Clone the Arc reference (cheap)
    pub fn clone_ref(&self) -> Arc<Tesseract> {
        Arc::clone(&self.tesseract)
    }
}

impl Clone for TesseractLink {
    fn clone(&self) -> Self {
        Self {
            tesseract: Arc::clone(&self.tesseract),
        }
    }
}

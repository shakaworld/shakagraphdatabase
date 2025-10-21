//! Offset Geometric Contact Detection
//!
//! Detects proximity-based relationships between legal cases using
//! spatial and temporal distance thresholds.

use chrono::{DateTime, Utc};
use nalgebra::Vector3;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use parking_lot::{RwLock, Mutex};

/// Type of geometric contact
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContactType {
    Direct,      // Very close (< 30% of offset)
    Influence,   // Medium (30-70% of offset)
    Proximity,   // Weak (70-100% of offset)
}

/// Represents a geometric contact between two cases
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub source_id: String,
    pub target_id: String,
    pub spatial_distance: f32,
    pub temporal_distance_days: i64,
    pub contact_type: ContactType,
    pub strength: f32,
}

/// Influence zone around a case
#[derive(Debug, Clone)]
pub struct InfluenceZone {
    pub case_id: String,
    pub center: Vector3<f32>,
    pub date: DateTime<Utc>,
    pub spatial_radius: f32,
    pub temporal_radius_days: i64,
}

impl InfluenceZone {
    /// Check if a point is within spatial radius
    pub fn contains_spatial(&self, point: &Vector3<f32>) -> bool {
        let distance = (point - self.center).norm();
        distance <= self.spatial_radius
    }

    /// Check if a date is within temporal radius
    pub fn contains_temporal(&self, other_date: &DateTime<Utc>) -> bool {
        let days_diff = (other_date.signed_duration_since(self.date)).num_days().abs();
        days_diff <= self.temporal_radius_days
    }

    /// Check if within both spatial and temporal radius
    pub fn contains(&self, point: &Vector3<f32>, date: &DateTime<Utc>) -> bool {
        self.contains_spatial(point) && self.contains_temporal(date)
    }

    /// Calculate contact strength for a point
    pub fn calculate_strength(&self, point: &Vector3<f32>, date: &DateTime<Utc>) -> f32 {
        let spatial_dist = (point - self.center).norm();
        let temporal_dist = (date.signed_duration_since(self.date)).num_days().abs();

        // Exponential decay
        let spatial_weight = (-spatial_dist / self.spatial_radius).exp();
        let temporal_weight = (-temporal_dist as f32 / self.temporal_radius_days as f32).exp();

        // Geometric mean
        (spatial_weight * temporal_weight).sqrt()
    }
}

/// Offset Geometric Contact Detector
pub struct OffsetGeometricContact {
    spatial_offset: f32,
    temporal_offset_days: i64,
    min_contact_strength: f32,

    // Cache for influence zones
    influence_zone_cache: RwLock<HashMap<String, InfluenceZone>>,

    // Statistics
    total_contacts_detected: Mutex<usize>,
    contacts_by_type: RwLock<HashMap<ContactType, usize>>,
}

impl OffsetGeometricContact {
    /// Create a new contact detector
    pub fn new(spatial_offset: f32, temporal_offset_days: i64, min_contact_strength: f32) -> Self {
        Self {
            spatial_offset,
            temporal_offset_days,
            min_contact_strength,
            influence_zone_cache: RwLock::new(HashMap::new()),
            total_contacts_detected: Mutex::new(0),
            contacts_by_type: RwLock::new(HashMap::new()),
        }
    }

    /// Create with default parameters
    pub fn default_config() -> Self {
        Self::new(
            0.3,    // 30% spatial distance threshold
            730,    // 2 years temporal threshold
            0.1,    // 10% minimum strength
        )
    }

    /// Create an influence zone around a case
    pub fn create_influence_zone(
        &self,
        case_id: String,
        position: Vector3<f32>,
        date: DateTime<Utc>,
        radius_multiplier: f32,
    ) -> InfluenceZone {
        let zone = InfluenceZone {
            case_id: case_id.clone(),
            center: position,
            date,
            spatial_radius: self.spatial_offset * radius_multiplier,
            temporal_radius_days: (self.temporal_offset_days as f32 * radius_multiplier) as i64,
        };

        // Cache the zone
        let mut cache = self.influence_zone_cache.write();
        cache.insert(case_id, zone.clone());

        zone
    }

    /// Get cached influence zone
    pub fn get_influence_zone(&self, case_id: &str) -> Option<InfluenceZone> {
        let cache = self.influence_zone_cache.read();
        cache.get(case_id).cloned()
    }

    /// Calculate contact strength based on distances
    pub fn calculate_contact_strength(
        &self,
        spatial_distance: f32,
        temporal_distance_days: i64,
    ) -> f32 {
        // Exponential decay for spatial proximity
        let spatial_weight = (-spatial_distance / self.spatial_offset).exp();

        // Exponential decay for temporal proximity
        let temporal_weight = (-temporal_distance_days as f32 / self.temporal_offset_days as f32).exp();

        // Geometric mean (penalizes if either is weak)
        (spatial_weight * temporal_weight).sqrt()
    }

    /// Detect if there's a geometric contact between two cases
    pub fn detect_contact(
        &self,
        source_id: String,
        source_position: Vector3<f32>,
        source_date: DateTime<Utc>,
        target_id: String,
        target_position: Vector3<f32>,
        target_date: DateTime<Utc>,
    ) -> Option<Contact> {
        // Don't detect self-contact
        if source_id == target_id {
            return None;
        }

        // Calculate distances
        let spatial_dist = (source_position - target_position).norm();
        let temporal_dist = (source_date.signed_duration_since(target_date)).num_days().abs();

        // Check if within offset thresholds
        if spatial_dist > self.spatial_offset || temporal_dist > self.temporal_offset_days {
            return None;
        }

        // Calculate strength
        let strength = self.calculate_contact_strength(spatial_dist, temporal_dist);

        // Filter by minimum strength
        if strength < self.min_contact_strength {
            return None;
        }

        // Determine contact type based on spatial distance
        let contact_type = if spatial_dist < self.spatial_offset * 0.3 {
            ContactType::Direct
        } else if spatial_dist < self.spatial_offset * 0.7 {
            ContactType::Influence
        } else {
            ContactType::Proximity
        };

        // Update statistics
        {
            let mut count = self.total_contacts_detected.lock();
            *count += 1;
        }
        {
            let mut by_type = self.contacts_by_type.write();
            *by_type.entry(contact_type).or_insert(0) += 1;
        }

        Some(Contact {
            source_id,
            target_id,
            spatial_distance: spatial_dist,
            temporal_distance_days: temporal_dist,
            contact_type,
            strength,
        })
    }

    /// Detect all contacts for a case within the database
    pub fn detect_contacts_for_case(
        &self,
        case_id: String,
        position: Vector3<f32>,
        date: DateTime<Utc>,
        all_cases: &[(String, Vector3<f32>, DateTime<Utc>)],
    ) -> Vec<Contact> {
        let mut contacts = Vec::new();

        for (other_id, other_pos, other_date) in all_cases {
            if let Some(contact) = self.detect_contact(
                case_id.clone(),
                position,
                date,
                other_id.clone(),
                *other_pos,
                *other_date,
            ) {
                contacts.push(contact);
            }
        }

        // Sort by strength (descending)
        contacts.sort_by(|a, b| b.strength.partial_cmp(&a.strength).unwrap());

        contacts
    }

    /// Find cases within an influence zone
    pub fn find_cases_in_zone(
        &self,
        zone: &InfluenceZone,
        all_cases: &[(String, Vector3<f32>, DateTime<Utc>)],
    ) -> Vec<(String, f32)> {
        all_cases
            .iter()
            .filter(|(id, pos, date)| {
                id != &zone.case_id && zone.contains(pos, date)
            })
            .map(|(id, pos, date)| {
                let strength = zone.calculate_strength(pos, date);
                (id.clone(), strength)
            })
            .filter(|(_, strength)| *strength >= self.min_contact_strength)
            .collect()
    }

    /// Get statistics
    pub fn statistics(&self) -> ContactStatistics {
        let total = *self.total_contacts_detected.lock();
        let by_type = self.contacts_by_type.read().clone();

        ContactStatistics {
            total_contacts_detected: total,
            contacts_by_type: by_type,
            spatial_offset: self.spatial_offset,
            temporal_offset_days: self.temporal_offset_days,
            min_contact_strength: self.min_contact_strength,
            cached_zones: self.influence_zone_cache.read().len(),
        }
    }

    /// Clear cache
    pub fn clear_cache(&self) {
        let mut cache = self.influence_zone_cache.write();
        cache.clear();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactStatistics {
    pub total_contacts_detected: usize,
    pub contacts_by_type: HashMap<ContactType, usize>,
    pub spatial_offset: f32,
    pub temporal_offset_days: i64,
    pub min_contact_strength: f32,
    pub cached_zones: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_contact_detection() {
        let detector = OffsetGeometricContact::new(0.2, 730, 0.1);

        let source_id = "case_001".to_string();
        let source_pos = Vector3::new(0.1, 0.1, 0.1);
        let source_date = Utc::now();

        let target_id = "case_002".to_string();
        let target_pos = Vector3::new(0.15, 0.15, 0.15);
        let target_date = source_date + Duration::days(100);

        let contact = detector.detect_contact(
            source_id.clone(),
            source_pos,
            source_date,
            target_id.clone(),
            target_pos,
            target_date,
        );

        assert!(contact.is_some());
        if let Some(c) = contact {
            assert!(c.strength > 0.0);
            assert!(c.strength <= 1.0);
        }
    }

    #[test]
    fn test_influence_zone() {
        let detector = OffsetGeometricContact::new(0.3, 365, 0.1);

        let zone = detector.create_influence_zone(
            "case_001".to_string(),
            Vector3::new(0.5, 0.5, 0.5),
            Utc::now(),
            1.0,
        );

        // Point inside zone
        let inside_point = Vector3::new(0.55, 0.55, 0.55);
        assert!(zone.contains_spatial(&inside_point));

        // Point outside zone
        let outside_point = Vector3::new(1.0, 1.0, 1.0);
        assert!(!zone.contains_spatial(&outside_point));
    }

    #[test]
    fn test_contact_type_classification() {
        let detector = OffsetGeometricContact::new(1.0, 1000, 0.1);

        // Direct contact (< 30%)
        let direct = detector.detect_contact(
            "a".to_string(),
            Vector3::zeros(),
            Utc::now(),
            "b".to_string(),
            Vector3::new(0.2, 0.0, 0.0),
            Utc::now(),
        );
        assert!(matches!(direct.unwrap().contact_type, ContactType::Direct));

        // Influence contact (30-70%)
        let influence = detector.detect_contact(
            "a".to_string(),
            Vector3::zeros(),
            Utc::now(),
            "c".to_string(),
            Vector3::new(0.5, 0.0, 0.0),
            Utc::now(),
        );
        assert!(matches!(influence.unwrap().contact_type, ContactType::Influence));

        // Proximity contact (70-100%)
        let proximity = detector.detect_contact(
            "a".to_string(),
            Vector3::zeros(),
            Utc::now(),
            "d".to_string(),
            Vector3::new(0.85, 0.0, 0.0),
            Utc::now(),
        );
        assert!(matches!(proximity.unwrap().contact_type, ContactType::Proximity));
    }

    #[test]
    fn test_statistics() {
        let detector = OffsetGeometricContact::new(0.5, 500, 0.1);

        let now = Utc::now();

        // Detect some contacts
        detector.detect_contact(
            "a".to_string(),
            Vector3::zeros(),
            now,
            "b".to_string(),
            Vector3::new(0.1, 0.0, 0.0),
            now,
        );

        detector.detect_contact(
            "a".to_string(),
            Vector3::zeros(),
            now,
            "c".to_string(),
            Vector3::new(0.4, 0.0, 0.0),
            now,
        );

        let stats = detector.statistics();
        assert_eq!(stats.total_contacts_detected, 2);
        assert!(stats.contacts_by_type.len() > 0);
    }
}

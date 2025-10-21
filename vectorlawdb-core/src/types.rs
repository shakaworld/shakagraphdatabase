use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::fmt;
use chrono::{DateTime, Utc};

/// Strong type for case identifiers
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CaseId(pub String);

impl CaseId {
    /// Create a new CaseId with validation
    pub fn new(id: String) -> Result<Self, String> {
        if id.is_empty() {
            return Err("Case ID cannot be empty".to_string());
        }
        if id.len() > 256 {
            return Err("Case ID too long (max 256 characters)".to_string());
        }
        Ok(CaseId(id))
    }

    /// Create without validation (use carefully)
    pub fn unchecked(id: String) -> Self {
        CaseId(id)
    }

    /// Get the inner string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CaseId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for CaseId {
    fn from(s: String) -> Self {
        CaseId(s)
    }
}

impl From<&str> for CaseId {
    fn from(s: &str) -> Self {
        CaseId(s.to_string())
    }
}

/// Vector embedding with helper methods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding(pub Vec<f32>);

impl Embedding {
    /// Create a new embedding with validation
    pub fn new(vec: Vec<f32>) -> Result<Self, String> {
        if vec.is_empty() {
            return Err("Embedding cannot be empty".to_string());
        }
        if vec.len() > 4096 {
            return Err("Embedding dimension too large (max 4096)".to_string());
        }
        // Check for NaN or infinite values
        if vec.iter().any(|&x| !x.is_finite()) {
            return Err("Embedding contains NaN or infinite values".to_string());
        }
        Ok(Embedding(vec))
    }

    /// Create without validation (use carefully)
    pub fn unchecked(vec: Vec<f32>) -> Self {
        Embedding(vec)
    }

    /// Get dimension of the embedding
    pub fn dim(&self) -> usize {
        self.0.len()
    }

    /// Calculate cosine similarity with another embedding
    pub fn cosine_similarity(&self, other: &Embedding) -> f32 {
        if self.dim() != other.dim() {
            return 0.0;
        }

        let dot: f32 = self.0.iter().zip(&other.0).map(|(a, b)| a * b).sum();
        let norm_a: f32 = self.0.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = other.0.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }

        dot / (norm_a * norm_b)
    }

    /// Calculate Euclidean distance to another embedding
    pub fn euclidean_distance(&self, other: &Embedding) -> f32 {
        if self.dim() != other.dim() {
            return f32::MAX;
        }

        self.0
            .iter()
            .zip(&other.0)
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f32>()
            .sqrt()
    }

    /// Normalize the embedding to unit length
    pub fn normalize(&mut self) {
        let norm: f32 = self.0.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for x in &mut self.0 {
                *x /= norm;
            }
        }
    }

    /// Get normalized copy
    pub fn normalized(&self) -> Self {
        let mut copy = self.clone();
        copy.normalize();
        copy
    }

    /// Get the inner vector
    pub fn as_slice(&self) -> &[f32] {
        &self.0
    }

    /// Convert to nalgebra Vector3 for spatial indexing (take first 3 dims)
    pub fn to_vector3(&self) -> nalgebra::Vector3<f32> {
        if self.dim() >= 3 {
            nalgebra::Vector3::new(self.0[0], self.0[1], self.0[2])
        } else if self.dim() == 2 {
            nalgebra::Vector3::new(self.0[0], self.0[1], 0.0)
        } else if self.dim() == 1 {
            nalgebra::Vector3::new(self.0[0], 0.0, 0.0)
        } else {
            nalgebra::Vector3::zeros()
        }
    }
}

impl From<Vec<f32>> for Embedding {
    fn from(v: Vec<f32>) -> Self {
        Embedding(v)
    }
}

/// Metadata for a legal case
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseMetadata {
    pub case_id: CaseId,
    pub name: String,
    pub date: DateTime<Utc>,
    pub jurisdiction: Option<String>,
    pub court: Option<String>,
    pub citations: Vec<String>,
    #[serde(flatten)]
    pub custom: HashMap<String, serde_json::Value>,
}

impl CaseMetadata {
    /// Create new case metadata
    pub fn new(
        case_id: CaseId,
        name: String,
        date: DateTime<Utc>,
    ) -> Self {
        Self {
            case_id,
            name,
            date,
            jurisdiction: None,
            court: None,
            citations: Vec::new(),
            custom: HashMap::new(),
        }
    }

    /// Builder pattern for jurisdiction
    pub fn with_jurisdiction(mut self, jurisdiction: String) -> Self {
        self.jurisdiction = Some(jurisdiction);
        self
    }

    /// Builder pattern for court
    pub fn with_court(mut self, court: String) -> Self {
        self.court = Some(court);
        self
    }

    /// Builder pattern for citations
    pub fn with_citations(mut self, citations: Vec<String>) -> Self {
        self.citations = citations;
        self
    }

    /// Add custom metadata field
    pub fn add_custom(&mut self, key: String, value: serde_json::Value) {
        self.custom.insert(key, value);
    }

    /// Get year of the case
    pub fn year(&self) -> i32 {
        self.date.year()
    }

    /// Check if case is from a specific jurisdiction
    pub fn is_from_jurisdiction(&self, jurisdiction: &str) -> bool {
        self.jurisdiction.as_ref().map(|j| j == jurisdiction).unwrap_or(false)
    }
}

/// Complete case document with text, metadata, and embedding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseDocument {
    pub metadata: CaseMetadata,
    pub text: String,
    pub embedding: Embedding,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl CaseDocument {
    /// Create a new case document
    pub fn new(
        metadata: CaseMetadata,
        text: String,
        embedding: Embedding,
    ) -> Self {
        let now = Utc::now();
        Self {
            metadata,
            text,
            embedding,
            summary: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Get the case ID
    pub fn case_id(&self) -> &CaseId {
        &self.metadata.case_id
    }

    /// Get the case name
    pub fn name(&self) -> &str {
        &self.metadata.name
    }

    /// Get the case date
    pub fn date(&self) -> DateTime<Utc> {
        self.metadata.date
    }

    /// Get text preview (first N chars)
    pub fn text_preview(&self, max_chars: usize) -> String {
        if self.text.len() <= max_chars {
            self.text.clone()
        } else {
            format!("{}...", &self.text[..max_chars])
        }
    }

    /// Update the embedding
    pub fn update_embedding(&mut self, embedding: Embedding) {
        self.embedding = embedding;
        self.updated_at = Utc::now();
    }

    /// Set summary
    pub fn with_summary(mut self, summary: String) -> Self {
        self.summary = Some(summary);
        self
    }

    /// Calculate similarity to another case
    pub fn similarity_to(&self, other: &CaseDocument) -> f32 {
        self.embedding.cosine_similarity(&other.embedding)
    }

    /// Serialize to bytes for storage
    pub fn to_bytes(&self) -> Result<Vec<u8>, String> {
        bincode::serialize(self)
            .map_err(|e| format!("Serialization error: {}", e))
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        bincode::deserialize(bytes)
            .map_err(|e| format!("Deserialization error: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_case_id_creation() {
        let id = CaseId::new("case-123".to_string()).unwrap();
        assert_eq!(id.as_str(), "case-123");

        let empty = CaseId::new("".to_string());
        assert!(empty.is_err());
    }

    #[test]
    fn test_embedding_cosine_similarity() {
        let emb1 = Embedding::new(vec![1.0, 0.0, 0.0]).unwrap();
        let emb2 = Embedding::new(vec![1.0, 0.0, 0.0]).unwrap();
        let emb3 = Embedding::new(vec![0.0, 1.0, 0.0]).unwrap();

        assert_eq!(emb1.cosine_similarity(&emb2), 1.0);
        assert_eq!(emb1.cosine_similarity(&emb3), 0.0);
    }

    #[test]
    fn test_embedding_normalization() {
        let mut emb = Embedding::new(vec![3.0, 4.0, 0.0]).unwrap();
        emb.normalize();

        let norm: f32 = emb.0.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_case_metadata_builder() {
        let meta = CaseMetadata::new(
            CaseId::from("case-1"),
            "Test v. Case".to_string(),
            Utc::now(),
        )
        .with_jurisdiction("US-CA".to_string())
        .with_court("Supreme Court".to_string());

        assert_eq!(meta.jurisdiction, Some("US-CA".to_string()));
        assert!(meta.is_from_jurisdiction("US-CA"));
    }

    #[test]
    fn test_case_document_creation() {
        let meta = CaseMetadata::new(
            CaseId::from("case-1"),
            "Test v. Case".to_string(),
            Utc::now(),
        );
        let emb = Embedding::new(vec![1.0, 2.0, 3.0]).unwrap();
        let doc = CaseDocument::new(meta, "Full text here".to_string(), emb);

        assert_eq!(doc.name(), "Test v. Case");
        assert_eq!(doc.text_preview(4), "Full...");
    }
}

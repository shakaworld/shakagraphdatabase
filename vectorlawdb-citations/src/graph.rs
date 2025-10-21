//! Citation Graph Structure
//!
//! Directed graph of legal case citations using petgraph for advanced graph algorithms.

use serde::{Serialize, Deserialize};
use std::collections::{HashMap, HashSet};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::Direction;
use petgraph::algo::{dijkstra, connected_components};
use parking_lot::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    pub from_case_id: String,
    pub to_case_id: String,
    pub citation_type: CitationType,
    pub weight: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CitationType {
    Follows,
    Distinguishes,
    Overrules,
    Questions,
    Cites,
}

/// Citation edge metadata
#[derive(Debug, Clone)]
struct EdgeData {
    citation_type: CitationType,
    weight: f32,
}

/// Citation graph using petgraph for advanced algorithms
pub struct CitationGraph {
    // Petgraph directed graph
    graph: RwLock<DiGraph<String, EdgeData>>,
    // Map from case_id to node index for O(1) lookup
    node_indices: RwLock<HashMap<String, NodeIndex>>,
    // Legacy adjacency lists for backward compatibility
    outgoing: RwLock<HashMap<String, Vec<Citation>>>,
    incoming: RwLock<HashMap<String, Vec<Citation>>>,
}

impl CitationGraph {
    pub fn new() -> Self {
        Self {
            graph: RwLock::new(DiGraph::new()),
            node_indices: RwLock::new(HashMap::new()),
            outgoing: RwLock::new(HashMap::new()),
            incoming: RwLock::new(HashMap::new()),
        }
    }

    /// Ensure a case node exists in the graph
    fn ensure_node(&self, case_id: &str) -> NodeIndex {
        let mut indices = self.node_indices.write();

        if let Some(&idx) = indices.get(case_id) {
            return idx;
        }

        let mut graph = self.graph.write();
        let idx = graph.add_node(case_id.to_string());
        indices.insert(case_id.to_string(), idx);
        idx
    }

    /// Add a citation to the graph
    pub fn add_citation(&self, citation: Citation) {
        // Add nodes
        let from_idx = self.ensure_node(&citation.from_case_id);
        let to_idx = self.ensure_node(&citation.to_case_id);

        // Add edge to petgraph
        {
            let mut graph = self.graph.write();
            graph.add_edge(from_idx, to_idx, EdgeData {
                citation_type: citation.citation_type,
                weight: citation.weight,
            });
        }

        // Update legacy adjacency lists
        {
            let mut outgoing = self.outgoing.write();
            outgoing
                .entry(citation.from_case_id.clone())
                .or_insert_with(Vec::new)
                .push(citation.clone());
        }

        {
            let mut incoming = self.incoming.write();
            incoming
                .entry(citation.to_case_id.clone())
                .or_insert_with(Vec::new)
                .push(citation);
        }
    }

    /// Get citations from a case (outgoing edges)
    pub fn get_citations_from(&self, case_id: &str) -> Vec<&Citation> {
        let outgoing = self.outgoing.read();
        outgoing
            .get(case_id)
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }

    /// Get citations to a case (incoming edges)
    pub fn get_citations_to(&self, case_id: &str) -> Vec<&Citation> {
        let incoming = self.incoming.read();
        incoming
            .get(case_id)
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }

    /// Get citation rank (number of incoming citations)
    pub fn citation_rank(&self, case_id: &str) -> usize {
        let incoming = self.incoming.read();
        incoming.get(case_id).map(|v| v.len()).unwrap_or(0)
    }

    /// Find shortest path between two cases using Dijkstra's algorithm
    pub fn find_path(&self, from: &str, to: &str, max_depth: usize) -> Option<Vec<String>> {
        let indices = self.node_indices.read();
        let graph = self.graph.read();

        let from_idx = indices.get(from)?;
        let to_idx = indices.get(to)?;

        // Use Dijkstra to find shortest path
        let distances = dijkstra(&*graph, *from_idx, Some(*to_idx), |e| e.weight() as f64);

        if !distances.contains_key(to_idx) {
            return None;
        }

        // Reconstruct path using BFS with distance constraints
        let mut path = vec![from.to_string()];
        let mut current_idx = *from_idx;

        while current_idx != *to_idx && path.len() < max_depth {
            let neighbors: Vec<_> = graph
                .neighbors_directed(current_idx, Direction::Outgoing)
                .collect();

            let mut best_neighbor = None;
            let mut best_distance = f64::MAX;

            for neighbor in neighbors {
                if let Some(&dist) = distances.get(&neighbor) {
                    if dist < best_distance {
                        best_distance = dist;
                        best_neighbor = Some(neighbor);
                    }
                }
            }

            if let Some(next_idx) = best_neighbor {
                let next_case = graph[next_idx].clone();
                path.push(next_case);
                current_idx = next_idx;
            } else {
                break;
            }
        }

        if current_idx == *to_idx {
            Some(path)
        } else {
            None
        }
    }

    /// Get all case IDs in the graph
    pub fn get_all_cases(&self) -> Vec<String> {
        let indices = self.node_indices.read();
        indices.keys().cloned().collect()
    }

    /// Get outgoing neighbors of a case
    pub fn get_outgoing_neighbors(&self, case_id: &str) -> Vec<String> {
        let indices = self.node_indices.read();
        let graph = self.graph.read();

        if let Some(&idx) = indices.get(case_id) {
            graph.neighbors_directed(idx, Direction::Outgoing)
                .map(|neighbor_idx| graph[neighbor_idx].clone())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get incoming neighbors of a case
    pub fn get_incoming_neighbors(&self, case_id: &str) -> Vec<String> {
        let indices = self.node_indices.read();
        let graph = self.graph.read();

        if let Some(&idx) = indices.get(case_id) {
            graph.neighbors_directed(idx, Direction::Incoming)
                .map(|neighbor_idx| graph[neighbor_idx].clone())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Calculate transitive closure up to max depth
    pub fn get_transitive_closure(&self, case_id: &str, max_depth: usize) -> HashSet<String> {
        let mut visited = HashSet::new();
        let mut queue = vec![(case_id.to_string(), 0)];

        while let Some((current, depth)) = queue.pop() {
            if depth >= max_depth {
                continue;
            }

            if !visited.insert(current.clone()) {
                continue;
            }

            let neighbors = self.get_outgoing_neighbors(&current);
            for neighbor in neighbors {
                if !visited.contains(&neighbor) {
                    queue.push((neighbor, depth + 1));
                }
            }
        }

        visited.remove(case_id);
        visited
    }

    /// Get strongly connected components
    pub fn get_connected_components(&self) -> usize {
        let graph = self.graph.read();
        connected_components(&*graph)
    }

    /// Calculate PageRank scores for all cases
    pub fn compute_pagerank(&self, damping: f64, max_iterations: usize) -> HashMap<String, f64> {
        let all_nodes = self.get_all_cases();
        let n = all_nodes.len() as f64;

        if n == 0.0 {
            return HashMap::new();
        }

        // Initialize PageRank values
        let mut pagerank: HashMap<String, f64> = all_nodes
            .iter()
            .map(|node| (node.clone(), 1.0 / n))
            .collect();

        // Power iteration
        for _ in 0..max_iterations {
            let mut new_pagerank = HashMap::new();

            for node in &all_nodes {
                let mut rank = (1.0 - damping) / n;

                let incoming = self.get_incoming_neighbors(node);
                for predecessor in incoming {
                    let outgoing_count = self.get_outgoing_neighbors(&predecessor).len() as f64;
                    if outgoing_count > 0.0 {
                        rank += damping * pagerank.get(&predecessor).unwrap_or(&0.0) / outgoing_count;
                    }
                }

                new_pagerank.insert(node.clone(), rank);
            }

            pagerank = new_pagerank;
        }

        // Normalize
        let total: f64 = pagerank.values().sum();
        if total > 0.0 {
            for value in pagerank.values_mut() {
                *value /= total;
            }
        }

        pagerank
    }

    /// Compute HITS (Hubs and Authorities) scores
    /// Returns a HashMap mapping case_id -> (hub_score, authority_score)
    pub fn compute_hits(&self, max_iterations: usize) -> HashMap<String, (f64, f64)> {
        let all_nodes = self.get_all_cases();
        let n = all_nodes.len() as f64;

        if n == 0.0 {
            return HashMap::new();
        }

        // Initialize hub and authority scores to 1.0
        let mut hub_scores: HashMap<String, f64> = all_nodes
            .iter()
            .map(|node| (node.clone(), 1.0))
            .collect();
        let mut auth_scores: HashMap<String, f64> = all_nodes
            .iter()
            .map(|node| (node.clone(), 1.0))
            .collect();

        // HITS algorithm iteration
        for _ in 0..max_iterations {
            let mut new_auth_scores = HashMap::new();
            let mut new_hub_scores = HashMap::new();

            // Update authority scores: sum of hub scores of incoming neighbors
            for node in &all_nodes {
                let incoming = self.get_incoming_neighbors(node);
                let auth_score: f64 = incoming
                    .iter()
                    .map(|neighbor| hub_scores.get(neighbor).unwrap_or(&0.0))
                    .sum();
                new_auth_scores.insert(node.clone(), auth_score);
            }

            // Update hub scores: sum of authority scores of outgoing neighbors
            for node in &all_nodes {
                let outgoing = self.get_outgoing_neighbors(node);
                let hub_score: f64 = outgoing
                    .iter()
                    .map(|neighbor| new_auth_scores.get(neighbor).unwrap_or(&0.0))
                    .sum();
                new_hub_scores.insert(node.clone(), hub_score);
            }

            // Normalize authority scores
            let auth_norm: f64 = new_auth_scores.values().map(|x| x * x).sum::<f64>().sqrt();
            if auth_norm > 0.0 {
                for score in new_auth_scores.values_mut() {
                    *score /= auth_norm;
                }
            }

            // Normalize hub scores
            let hub_norm: f64 = new_hub_scores.values().map(|x| x * x).sum::<f64>().sqrt();
            if hub_norm > 0.0 {
                for score in new_hub_scores.values_mut() {
                    *score /= hub_norm;
                }
            }

            auth_scores = new_auth_scores;
            hub_scores = new_hub_scores;
        }

        // Combine into single HashMap
        all_nodes
            .iter()
            .map(|node| {
                let hub = *hub_scores.get(node).unwrap_or(&0.0);
                let auth = *auth_scores.get(node).unwrap_or(&0.0);
                (node.clone(), (hub, auth))
            })
            .collect()
    }

    /// Get most cited cases (authorities)
    pub fn get_most_cited(&self, n: usize) -> Vec<(String, usize)> {
        let all_cases = self.get_all_cases();
        let mut citations: Vec<_> = all_cases
            .iter()
            .map(|case_id| {
                let count = self.citation_rank(case_id);
                (case_id.clone(), count)
            })
            .collect();

        citations.sort_by(|a, b| b.1.cmp(&a.1));
        citations.truncate(n);
        citations
    }

    /// Get graph statistics
    pub fn statistics(&self) -> GraphStatistics {
        let graph = self.graph.read();
        let indices = self.node_indices.read();

        GraphStatistics {
            total_nodes: indices.len(),
            total_edges: graph.edge_count(),
            connected_components: self.get_connected_components(),
        }
    }
}

impl Default for CitationGraph {
    fn default() -> Self {
        Self::new()
    }
}

// Thread safety
unsafe impl Send for CitationGraph {}
unsafe impl Sync for CitationGraph {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStatistics {
    pub total_nodes: usize,
    pub total_edges: usize,
    pub connected_components: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_citation_graph() {
        let graph = CitationGraph::new();

        let citation1 = Citation {
            from_case_id: "case_a".to_string(),
            to_case_id: "case_b".to_string(),
            citation_type: CitationType::Follows,
            weight: 1.0,
        };

        let citation2 = Citation {
            from_case_id: "case_a".to_string(),
            to_case_id: "case_c".to_string(),
            citation_type: CitationType::Cites,
            weight: 1.0,
        };

        let citation3 = Citation {
            from_case_id: "case_b".to_string(),
            to_case_id: "case_c".to_string(),
            citation_type: CitationType::Follows,
            weight: 1.0,
        };

        graph.add_citation(citation1);
        graph.add_citation(citation2);
        graph.add_citation(citation3);

        // Test outgoing
        let outgoing = graph.get_outgoing_neighbors("case_a");
        assert_eq!(outgoing.len(), 2);

        // Test incoming
        let incoming = graph.get_incoming_neighbors("case_c");
        assert_eq!(incoming.len(), 2);

        // Test citation rank
        assert_eq!(graph.citation_rank("case_c"), 2);

        // Test path finding
        let path = graph.find_path("case_a", "case_c", 5);
        assert!(path.is_some());
    }

    #[test]
    fn test_pagerank() {
        let graph = CitationGraph::new();

        // Create a simple citation network
        for i in 1..=5 {
            for j in (i+1)..=5 {
                graph.add_citation(Citation {
                    from_case_id: format!("case_{}", i),
                    to_case_id: format!("case_{}", j),
                    citation_type: CitationType::Cites,
                    weight: 1.0,
                });
            }
        }

        let pagerank = graph.compute_pagerank(0.85, 100);
        assert_eq!(pagerank.len(), 5);

        // Sum should be approximately 1.0
        let sum: f64 = pagerank.values().sum();
        assert!((sum - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_transitive_closure() {
        let graph = CitationGraph::new();

        graph.add_citation(Citation {
            from_case_id: "a".to_string(),
            to_case_id: "b".to_string(),
            citation_type: CitationType::Cites,
            weight: 1.0,
        });

        graph.add_citation(Citation {
            from_case_id: "b".to_string(),
            to_case_id: "c".to_string(),
            citation_type: CitationType::Cites,
            weight: 1.0,
        });

        graph.add_citation(Citation {
            from_case_id: "c".to_string(),
            to_case_id: "d".to_string(),
            citation_type: CitationType::Cites,
            weight: 1.0,
        });

        let closure = graph.get_transitive_closure("a", 5);
        assert_eq!(closure.len(), 3); // b, c, d
        assert!(closure.contains("b"));
        assert!(closure.contains("c"));
        assert!(closure.contains("d"));
    }

    #[test]
    fn test_hits_algorithm() {
        let graph = CitationGraph::new();

        // Create a hub-and-spoke pattern
        // Central hub 'h' cites many authorities
        // Authority 'a1' is cited by many hubs
        graph.add_citation(Citation {
            from_case_id: "h".to_string(),
            to_case_id: "a1".to_string(),
            citation_type: CitationType::Cites,
            weight: 1.0,
        });

        graph.add_citation(Citation {
            from_case_id: "h".to_string(),
            to_case_id: "a2".to_string(),
            citation_type: CitationType::Cites,
            weight: 1.0,
        });

        graph.add_citation(Citation {
            from_case_id: "h2".to_string(),
            to_case_id: "a1".to_string(),
            citation_type: CitationType::Cites,
            weight: 1.0,
        });

        let hits_scores = graph.compute_hits(50);

        // 'h' should have a high hub score (cites many)
        let (hub_h, _) = hits_scores.get("h").unwrap();
        assert!(*hub_h > 0.0);

        // 'a1' should have a high authority score (cited by many)
        let (_, auth_a1) = hits_scores.get("a1").unwrap();
        assert!(*auth_a1 > 0.0);

        // Verify scores are normalized (between 0 and 1)
        for (_case, (hub, auth)) in hits_scores.iter() {
            assert!(*hub >= 0.0 && *hub <= 1.0);
            assert!(*auth >= 0.0 && *auth <= 1.0);
        }
    }
}

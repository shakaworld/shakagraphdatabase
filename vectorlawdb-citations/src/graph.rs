use serde::{Serialize, Deserialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    pub from_case_id: String,
    pub to_case_id: String,
    pub citation_type: CitationType,
    pub weight: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CitationType {
    Follows,
    Distinguishes,
    Overrules,
    Questions,
    Cites,
}

pub struct CitationGraph {
    // Adjacency list: case_id -> list of citations
    outgoing: HashMap<String, Vec<Citation>>,
    incoming: HashMap<String, Vec<Citation>>,
}

impl CitationGraph {
    pub fn new() -> Self {
        Self {
            outgoing: HashMap::new(),
            incoming: HashMap::new(),
        }
    }

    pub fn add_citation(&mut self, citation: Citation) {
        // Add to outgoing
        self.outgoing
            .entry(citation.from_case_id.clone())
            .or_insert_with(Vec::new)
            .push(citation.clone());

        // Add to incoming
        self.incoming
            .entry(citation.to_case_id.clone())
            .or_insert_with(Vec::new)
            .push(citation);
    }

    pub fn get_citations_from(&self, case_id: &str) -> Vec<&Citation> {
        self.outgoing
            .get(case_id)
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }

    pub fn get_citations_to(&self, case_id: &str) -> Vec<&Citation> {
        self.incoming
            .get(case_id)
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }

    pub fn citation_rank(&self, case_id: &str) -> usize {
        self.incoming.get(case_id).map(|v| v.len()).unwrap_or(0)
    }

    pub fn find_path(&self, from: &str, to: &str, max_depth: usize) -> Option<Vec<String>> {
        let mut visited = HashSet::new();
        let mut queue = vec![(from.to_string(), vec![from.to_string()])];

        while let Some((current, path)) = queue.pop() {
            if current == to {
                return Some(path);
            }

            if path.len() > max_depth {
                continue;
            }

            if !visited.insert(current.clone()) {
                continue;
            }

            if let Some(citations) = self.outgoing.get(&current) {
                for citation in citations {
                    let mut new_path = path.clone();
                    new_path.push(citation.to_case_id.clone());
                    queue.push((citation.to_case_id.clone(), new_path));
                }
            }
        }

        None
    }

    pub fn get_all_cases(&self) -> Vec<String> {
        let mut cases = HashSet::new();

        for case_id in self.outgoing.keys() {
            cases.insert(case_id.clone());
        }

        for case_id in self.incoming.keys() {
            cases.insert(case_id.clone());
        }

        cases.into_iter().collect()
    }
}

impl Default for CitationGraph {
    fn default() -> Self {
        Self::new()
    }
}

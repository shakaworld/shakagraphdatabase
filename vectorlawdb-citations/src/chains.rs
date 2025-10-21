use serde::{Serialize, Deserialize};
use std::collections::{HashMap, HashSet, VecDeque};
use crate::graph::{CitationGraph, Citation, CitationType};

/// Represents the 15 specialized legal citation chain types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum JurisChainType {
    /// 1. Precedent Chain - Following established legal precedents
    Precedent,
    /// 2. Overruling Chain - Cases that overrule previous decisions
    Overruling,
    /// 3. Distinguishing Chain - Cases that distinguish from precedents
    Distinguishing,
    /// 4. Statutory Interpretation Chain - Interpreting statutes
    StatutoryInterpretation,
    /// 5. Constitutional Review Chain - Constitutional law matters
    ConstitutionalReview,
    /// 6. Procedural Chain - Procedural law citations
    Procedural,
    /// 7. Remedial Chain - Remedy and relief citations
    Remedial,
    /// 8. Jurisdictional Chain - Jurisdictional questions
    Jurisdictional,
    /// 9. Evidentiary Chain - Evidence law citations
    Evidentiary,
    /// 10. Contractual Chain - Contract law citations
    Contractual,
    /// 11. Tort Chain - Tort law citations
    Tort,
    /// 12. Criminal Chain - Criminal law citations
    Criminal,
    /// 13. Administrative Chain - Administrative law citations
    Administrative,
    /// 14. International Law Chain - International law citations
    InternationalLaw,
    /// 15. Comparative Law Chain - Comparative law citations
    ComparativeLaw,
}

impl JurisChainType {
    /// Get all chain types
    pub fn all() -> Vec<JurisChainType> {
        vec![
            JurisChainType::Precedent,
            JurisChainType::Overruling,
            JurisChainType::Distinguishing,
            JurisChainType::StatutoryInterpretation,
            JurisChainType::ConstitutionalReview,
            JurisChainType::Procedural,
            JurisChainType::Remedial,
            JurisChainType::Jurisdictional,
            JurisChainType::Evidentiary,
            JurisChainType::Contractual,
            JurisChainType::Tort,
            JurisChainType::Criminal,
            JurisChainType::Administrative,
            JurisChainType::InternationalLaw,
            JurisChainType::ComparativeLaw,
        ]
    }

    /// Get description of the chain type
    pub fn description(&self) -> &'static str {
        match self {
            JurisChainType::Precedent => "Following established legal precedents",
            JurisChainType::Overruling => "Cases that overrule previous decisions",
            JurisChainType::Distinguishing => "Cases that distinguish from precedents",
            JurisChainType::StatutoryInterpretation => "Interpreting statutes and legislation",
            JurisChainType::ConstitutionalReview => "Constitutional law and judicial review",
            JurisChainType::Procedural => "Procedural law and court procedures",
            JurisChainType::Remedial => "Remedies, relief, and damages",
            JurisChainType::Jurisdictional => "Jurisdictional questions and conflicts",
            JurisChainType::Evidentiary => "Evidence law and admissibility",
            JurisChainType::Contractual => "Contract law and obligations",
            JurisChainType::Tort => "Tort law and civil wrongs",
            JurisChainType::Criminal => "Criminal law and prosecution",
            JurisChainType::Administrative => "Administrative law and agencies",
            JurisChainType::InternationalLaw => "International law and treaties",
            JurisChainType::ComparativeLaw => "Comparative law across jurisdictions",
        }
    }
}

/// Represents a citation chain - a sequence of related cases
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitationChain {
    pub chain_type: JurisChainType,
    pub cases: Vec<String>,
    pub strength: f32,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

impl CitationChain {
    pub fn new(chain_type: JurisChainType) -> Self {
        Self {
            chain_type,
            cases: Vec::new(),
            strength: 0.0,
            start_date: None,
            end_date: None,
        }
    }

    pub fn add_case(&mut self, case_id: String) {
        self.cases.push(case_id);
    }

    pub fn length(&self) -> usize {
        self.cases.len()
    }

    pub fn calculate_strength(&mut self) {
        // Strength based on chain length and density
        // Longer chains with more connections are stronger
        self.strength = (self.cases.len() as f32).sqrt();
    }
}

/// Manager for tracking and analyzing all 15 types of citation chains
pub struct JurisCitationChains {
    chains: HashMap<JurisChainType, Vec<CitationChain>>,
    case_to_chains: HashMap<String, Vec<(JurisChainType, usize)>>,
}

impl JurisCitationChains {
    pub fn new() -> Self {
        let mut chains = HashMap::new();
        for chain_type in JurisChainType::all() {
            chains.insert(chain_type, Vec::new());
        }

        Self {
            chains,
            case_to_chains: HashMap::new(),
        }
    }

    /// Build citation chains from a citation graph
    pub fn build_from_graph(&mut self, graph: &CitationGraph) {
        // Build chains for each type
        for chain_type in JurisChainType::all() {
            self.build_chains_for_type(graph, chain_type);
        }
    }

    fn build_chains_for_type(&mut self, graph: &CitationGraph, chain_type: JurisChainType) {
        let mut visited = HashSet::new();
        let chains_vec = self.chains.get_mut(&chain_type).unwrap();

        // Get all cases from graph
        let all_cases = graph.get_all_cases();

        for case_id in all_cases {
            if visited.contains(&case_id) {
                continue;
            }

            // Try to build a chain starting from this case
            let chain = self.build_chain_from_case(
                graph,
                &case_id,
                chain_type,
                &mut visited,
            );

            if chain.length() > 1 {
                let chain_idx = chains_vec.len();
                chains_vec.push(chain);

                // Update reverse index
                for case in &chains_vec[chain_idx].cases {
                    self.case_to_chains
                        .entry(case.clone())
                        .or_insert_with(Vec::new)
                        .push((chain_type, chain_idx));
                }
            }
        }
    }

    fn build_chain_from_case(
        &self,
        graph: &CitationGraph,
        start_case: &str,
        chain_type: JurisChainType,
        visited: &mut HashSet<String>,
    ) -> CitationChain {
        let mut chain = CitationChain::new(chain_type);
        let mut queue = VecDeque::new();
        queue.push_back(start_case.to_string());

        while let Some(case_id) = queue.pop_front() {
            if visited.contains(&case_id) {
                continue;
            }

            visited.insert(case_id.clone());
            chain.add_case(case_id.clone());

            // Get citations from this case
            let citations = graph.get_citations_from(&case_id);

            for citation in citations {
                // Filter by chain type relevance
                if self.is_relevant_citation(&citation.citation_type, chain_type) {
                    queue.push_back(citation.to_case_id.clone());
                }
            }
        }

        chain.calculate_strength();
        chain
    }

    fn is_relevant_citation(&self, citation_type: &CitationType, chain_type: JurisChainType) -> bool {
        // Map citation types to chain types
        match (citation_type, chain_type) {
            (CitationType::Follows, JurisChainType::Precedent) => true,
            (CitationType::Overrules, JurisChainType::Overruling) => true,
            (CitationType::Distinguishes, JurisChainType::Distinguishing) => true,
            // Default: Cites can apply to many chain types
            (CitationType::Cites, _) => true,
            _ => false,
        }
    }

    /// Get all chains of a specific type
    pub fn get_chains_by_type(&self, chain_type: JurisChainType) -> &Vec<CitationChain> {
        self.chains.get(&chain_type).unwrap()
    }

    /// Get chains containing a specific case
    pub fn get_chains_for_case(&self, case_id: &str) -> Vec<&CitationChain> {
        let mut result = Vec::new();

        if let Some(chain_refs) = self.case_to_chains.get(case_id) {
            for (chain_type, chain_idx) in chain_refs {
                if let Some(chains) = self.chains.get(chain_type) {
                    if let Some(chain) = chains.get(*chain_idx) {
                        result.push(chain);
                    }
                }
            }
        }

        result
    }

    /// Get statistics for all chain types
    pub fn get_statistics(&self) -> ChainStatistics {
        let mut stats = ChainStatistics::new();

        for chain_type in JurisChainType::all() {
            let chains = self.get_chains_by_type(chain_type);

            let type_stats = ChainTypeStats {
                chain_type,
                total_chains: chains.len(),
                avg_length: if chains.is_empty() {
                    0.0
                } else {
                    chains.iter().map(|c| c.length()).sum::<usize>() as f32 / chains.len() as f32
                },
                max_length: chains.iter().map(|c| c.length()).max().unwrap_or(0),
                total_cases: chains.iter().map(|c| c.length()).sum(),
            };

            stats.by_type.insert(chain_type, type_stats);
        }

        stats
    }

    /// Find the strongest chain of a given type
    pub fn get_strongest_chain(&self, chain_type: JurisChainType) -> Option<&CitationChain> {
        let chains = self.get_chains_by_type(chain_type);
        chains.iter().max_by(|a, b| a.strength.partial_cmp(&b.strength).unwrap())
    }

    /// Analyze chain overlap (cases appearing in multiple chains)
    pub fn analyze_overlap(&self) -> OverlapAnalysis {
        let mut overlap_counts: HashMap<String, usize> = HashMap::new();

        for case_chains in self.case_to_chains.values() {
            for (case_id, _) in case_chains {
                // Count is already implicit in the number of entries
            }
        }

        // Find cases in multiple chains
        let multi_chain_cases: Vec<String> = self.case_to_chains
            .iter()
            .filter(|(_, chains)| chains.len() > 1)
            .map(|(case_id, _)| case_id.clone())
            .collect();

        OverlapAnalysis {
            total_cases: self.case_to_chains.len(),
            multi_chain_cases: multi_chain_cases.len(),
            avg_chains_per_case: if self.case_to_chains.is_empty() {
                0.0
            } else {
                self.case_to_chains.values().map(|v| v.len()).sum::<usize>() as f32
                    / self.case_to_chains.len() as f32
            },
        }
    }
}

impl Default for JurisCitationChains {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainTypeStats {
    pub chain_type: JurisChainType,
    pub total_chains: usize,
    pub avg_length: f32,
    pub max_length: usize,
    pub total_cases: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainStatistics {
    pub by_type: HashMap<JurisChainType, ChainTypeStats>,
}

impl ChainStatistics {
    fn new() -> Self {
        Self {
            by_type: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlapAnalysis {
    pub total_cases: usize,
    pub multi_chain_cases: usize,
    pub avg_chains_per_case: f32,
}

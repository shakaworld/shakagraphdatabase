//! VQL (Vector Query Language) Parser
//!
//! Parses VQL queries into an Abstract Syntax Tree (AST).
//!
//! Example VQL queries:
//! - `FIND cases WHERE date > '2020-01-01' LIMIT 10`
//! - `FIND cases NEAR [0.1, 0.2, 0.3] RADIUS 0.5`
//! - `FIND cases CITES 'case_123'`

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VQLQuery {
    pub entity: String,
    pub filters: Vec<Filter>,
    pub limit: Option<usize>,
    pub query_type: QueryType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryType {
    Simple,
    Vector,
    Citation,
    Temporal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Filter {
    pub predicate: Predicate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Predicate {
    /// NEAR [vector] RADIUS radius
    Near { vector: Vec<f32>, radius: f32 },
    /// CITES case_id
    Cites { case_id: String },
    /// CITED_BY case_id
    CitedBy { case_id: String },
    /// date AFTER 'date'
    After { date: String },
    /// date BEFORE 'date'
    Before { date: String },
    /// date BETWEEN 'start' AND 'end'
    Between { start: String, end: String },
    /// field = value
    Equals { field: String, value: String },
    /// field > value
    GreaterThan { field: String, value: String },
    /// field < value
    LessThan { field: String, value: String },
}

pub struct VQLParser;

impl VQLParser {
    /// Parse a VQL query string into an AST
    pub fn parse(input: &str) -> Result<VQLQuery, String> {
        let input = input.trim();

        // Check for FIND keyword
        if !input.to_uppercase().starts_with("FIND") {
            return Err("Query must start with FIND".to_string());
        }

        let rest = input[4..].trim();

        // Parse entity (e.g., "cases")
        let (entity, rest) = Self::parse_entity(rest)?;

        // Parse filters
        let (filters, rest) = Self::parse_filters(rest)?;

        // Parse LIMIT clause
        let limit = Self::parse_limit(rest)?;

        // Determine query type
        let query_type = Self::determine_query_type(&filters);

        Ok(VQLQuery {
            entity,
            filters,
            limit,
            query_type,
        })
    }

    fn parse_entity(input: &str) -> Result<(String, &str), String> {
        let parts: Vec<&str> = input.splitn(2, |c: char| c.is_whitespace()).collect();
        if parts.is_empty() {
            return Err("Missing entity name".to_string());
        }

        let entity = parts[0].to_string();
        let rest = if parts.len() > 1 { parts[1].trim() } else { "" };

        Ok((entity, rest))
    }

    fn parse_filters(input: &str) -> Result<(Vec<Filter>, &str), String> {
        let mut filters = Vec::new();

        if input.is_empty() {
            return Ok((filters, input));
        }

        let upper = input.to_uppercase();

        // Check for WHERE clause
        if let Some(where_pos) = upper.find("WHERE") {
            let rest = &input[where_pos + 5..].trim();
            let (filter_str, remaining) = Self::extract_until_limit(rest);

            // Parse predicates from WHERE clause
            if let Ok(predicate) = Self::parse_predicate(filter_str) {
                filters.push(Filter { predicate });
            }

            return Ok((filters, remaining));
        }

        // Check for NEAR clause
        if let Some(near_pos) = upper.find("NEAR") {
            let rest = &input[near_pos + 4..].trim();
            let (filter_str, remaining) = Self::extract_until_limit(rest);

            if let Ok(predicate) = Self::parse_near_predicate(filter_str) {
                filters.push(Filter { predicate });
            }

            return Ok((filters, remaining));
        }

        // Check for CITES clause
        if let Some(cites_pos) = upper.find("CITES") {
            let rest = &input[cites_pos + 5..].trim();
            let (filter_str, remaining) = Self::extract_until_limit(rest);

            let case_id = filter_str.trim().trim_matches('\'').trim_matches('"').to_string();
            filters.push(Filter {
                predicate: Predicate::Cites { case_id },
            });

            return Ok((filters, remaining));
        }

        // Check for CITED_BY clause
        if let Some(cited_pos) = upper.find("CITED_BY") {
            let rest = &input[cited_pos + 8..].trim();
            let (filter_str, remaining) = Self::extract_until_limit(rest);

            let case_id = filter_str.trim().trim_matches('\'').trim_matches('"').to_string();
            filters.push(Filter {
                predicate: Predicate::CitedBy { case_id },
            });

            return Ok((filters, remaining));
        }

        Ok((filters, input))
    }

    fn extract_until_limit(input: &str) -> (&str, &str) {
        let upper = input.to_uppercase();
        if let Some(limit_pos) = upper.find("LIMIT") {
            (&input[..limit_pos].trim(), &input[limit_pos..].trim())
        } else {
            (input.trim(), "")
        }
    }

    fn parse_predicate(input: &str) -> Result<Predicate, String> {
        let input = input.trim();

        // Try to parse different predicate types
        if input.contains("BETWEEN") {
            return Self::parse_between_predicate(input);
        }

        if input.contains("AFTER") {
            let parts: Vec<&str> = input.splitn(2, "AFTER").collect();
            if parts.len() == 2 {
                let date = parts[1].trim().trim_matches('\'').trim_matches('"').to_string();
                return Ok(Predicate::After { date });
            }
        }

        if input.contains("BEFORE") {
            let parts: Vec<&str> = input.splitn(2, "BEFORE").collect();
            if parts.len() == 2 {
                let date = parts[1].trim().trim_matches('\'').trim_matches('"').to_string();
                return Ok(Predicate::Before { date });
            }
        }

        // Try equals, greater than, less than
        if let Some(pos) = input.find('=') {
            let field = input[..pos].trim().to_string();
            let value = input[pos + 1..].trim().trim_matches('\'').trim_matches('"').to_string();
            return Ok(Predicate::Equals { field, value });
        }

        if let Some(pos) = input.find('>') {
            let field = input[..pos].trim().to_string();
            let value = input[pos + 1..].trim().trim_matches('\'').trim_matches('"').to_string();
            return Ok(Predicate::GreaterThan { field, value });
        }

        if let Some(pos) = input.find('<') {
            let field = input[..pos].trim().to_string();
            let value = input[pos + 1..].trim().trim_matches('\'').trim_matches('"').to_string();
            return Ok(Predicate::LessThan { field, value });
        }

        Err(format!("Cannot parse predicate: {}", input))
    }

    fn parse_between_predicate(input: &str) -> Result<Predicate, String> {
        let parts: Vec<&str> = input.split("BETWEEN").collect();
        if parts.len() != 2 {
            return Err("Invalid BETWEEN syntax".to_string());
        }

        let values = parts[1].trim();
        let value_parts: Vec<&str> = values.split("AND").collect();
        if value_parts.len() != 2 {
            return Err("BETWEEN requires two values separated by AND".to_string());
        }

        let start = value_parts[0].trim().trim_matches('\'').trim_matches('"').to_string();
        let end = value_parts[1].trim().trim_matches('\'').trim_matches('"').to_string();

        Ok(Predicate::Between { start, end })
    }

    fn parse_near_predicate(input: &str) -> Result<Predicate, String> {
        // Format: [0.1, 0.2, 0.3] RADIUS 0.5
        let parts: Vec<&str> = input.split("RADIUS").collect();
        if parts.len() != 2 {
            return Err("NEAR requires RADIUS clause".to_string());
        }

        // Parse vector
        let vector_str = parts[0].trim();
        let vector = Self::parse_vector(vector_str)?;

        // Parse radius
        let radius: f32 = parts[1]
            .trim()
            .parse()
            .map_err(|_| "Invalid radius value".to_string())?;

        Ok(Predicate::Near { vector, radius })
    }

    fn parse_vector(input: &str) -> Result<Vec<f32>, String> {
        let input = input.trim().trim_matches('[').trim_matches(']');
        let parts: Vec<&str> = input.split(',').collect();

        let mut vector = Vec::new();
        for part in parts {
            let value: f32 = part
                .trim()
                .parse()
                .map_err(|_| format!("Invalid vector value: {}", part))?;
            vector.push(value);
        }

        if vector.is_empty() {
            return Err("Vector cannot be empty".to_string());
        }

        Ok(vector)
    }

    fn parse_limit(input: &str) -> Result<Option<usize>, String> {
        let upper = input.to_uppercase();
        if let Some(limit_pos) = upper.find("LIMIT") {
            let rest = input[limit_pos + 5..].trim();
            let limit_str = rest.split_whitespace().next().unwrap_or("10");
            let limit: usize = limit_str
                .parse()
                .map_err(|_| "Invalid LIMIT value".to_string())?;
            Ok(Some(limit))
        } else {
            Ok(None)
        }
    }

    fn determine_query_type(filters: &[Filter]) -> QueryType {
        for filter in filters {
            match &filter.predicate {
                Predicate::Near { .. } => return QueryType::Vector,
                Predicate::Cites { .. } | Predicate::CitedBy { .. } => return QueryType::Citation,
                Predicate::After { .. } | Predicate::Before { .. } | Predicate::Between { .. } => {
                    return QueryType::Temporal
                }
                _ => {}
            }
        }
        QueryType::Simple
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_query() {
        let query = VQLParser::parse("FIND cases LIMIT 10").unwrap();
        assert_eq!(query.entity, "cases");
        assert_eq!(query.limit, Some(10));
    }

    #[test]
    fn test_near_query() {
        let query = VQLParser::parse("FIND cases NEAR [0.1, 0.2, 0.3] RADIUS 0.5 LIMIT 5").unwrap();
        assert!(matches!(query.query_type, QueryType::Vector));
        assert_eq!(query.limit, Some(5));

        if let Some(filter) = query.filters.first() {
            if let Predicate::Near { vector, radius } = &filter.predicate {
                assert_eq!(vector.len(), 3);
                assert_eq!(*radius, 0.5);
            }
        }
    }

    #[test]
    fn test_cites_query() {
        let query = VQLParser::parse("FIND cases CITES 'case_123'").unwrap();
        assert!(matches!(query.query_type, QueryType::Citation));
    }

    #[test]
    fn test_temporal_query() {
        let query = VQLParser::parse("FIND cases WHERE date AFTER '2020-01-01' LIMIT 20").unwrap();
        assert!(matches!(query.query_type, QueryType::Temporal));
        assert_eq!(query.limit, Some(20));
    }
}

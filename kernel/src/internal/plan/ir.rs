use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub signals: Option<Signals>,
    pub nodes: Vec<Node>,
    pub edges: Option<Vec<Edge>>,
    pub stop_conditions: Option<StopConditions>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signals {
    pub latency_budget_ms: Option<u64>,
    pub cost_cap_usd: Option<f64>,
    pub risk: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub op: Operation,
    pub tool: Option<String>,
    pub args: Option<HashMap<String, serde_json::Value>>,
    pub bind: Option<HashMap<String, String>>,
    pub out: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Operation {
    #[serde(rename = "call")]
    Call,
    #[serde(rename = "map")]
    Map,
    #[serde(rename = "reduce")]
    Reduce,
    #[serde(rename = "branch")]
    Branch,
    #[serde(rename = "assert")]
    Assert,
    #[serde(rename = "spawn")]
    Spawn,
    #[serde(rename = "mem.read")]
    MemRead,
    #[serde(rename = "mem.write")]
    MemWrite,
    #[serde(rename = "verify")]
    Verify,
    #[serde(rename = "retry")]
    Retry,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopConditions {
    pub max_nodes: Option<u32>,
    pub min_confidence: Option<f64>,
}

impl Plan {
    pub fn validate(&self) -> Result<(), PlanValidationError> {
        // Basic validation
        if self.nodes.is_empty() {
            return Err(PlanValidationError::EmptyPlan);
        }

        // Check for duplicate node IDs
        let mut seen_ids = std::collections::HashSet::new();
        for node in &self.nodes {
            if seen_ids.contains(&node.id) {
                return Err(PlanValidationError::DuplicateNodeId(node.id.clone()));
            }
            seen_ids.insert(node.id.clone());
        }

        // Check edges reference valid nodes
        if let Some(edges) = &self.edges {
            for edge in edges {
                if !seen_ids.contains(&edge.from) {
                    return Err(PlanValidationError::InvalidEdge(format!(
                        "Edge references non-existent 'from' node: {}",
                        edge.from
                    )));
                }
                if !seen_ids.contains(&edge.to) {
                    return Err(PlanValidationError::InvalidEdge(format!(
                        "Edge references non-existent 'to' node: {}",
                        edge.to
                    )));
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PlanValidationError {
    #[error("Plan cannot be empty")]
    EmptyPlan,
    #[error("Duplicate node ID: {0}")]
    DuplicateNodeId(String),
    #[error("Invalid edge: {0}")]
    InvalidEdge(String),
}
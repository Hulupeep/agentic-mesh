use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

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
    pub capability: Option<String>,
    pub args: Option<HashMap<String, serde_json::Value>>,
    pub bind: Option<HashMap<String, String>>,
    pub out: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

    pub fn validate_with_tools<I, T>(&self, tools: I) -> Result<(), PlanValidationError>
    where
        I: IntoIterator<Item = T>,
        T: AsRef<str>,
    {
        self.validate()?;

        let available: HashSet<String> = tools
            .into_iter()
            .map(|tool| tool.as_ref().to_string())
            .collect();

        for node in &self.nodes {
            if Self::operation_requires_tool(&node.op) {
                if node.tool.is_none() && node.capability.is_none() {
                    return Err(PlanValidationError::MissingToolOrCapability(
                        node.id.clone(),
                    ));
                }

                if let Some(tool_name) = &node.tool {
                    if !available.contains(tool_name) {
                        return Err(PlanValidationError::UnknownTool(tool_name.clone()));
                    }
                }
            }

            if let Some(tool_name) = &node.tool {
                if !available.contains(tool_name) {
                    return Err(PlanValidationError::UnknownTool(tool_name.clone()));
                }
            }

            if Self::operation_requires_output(&node.op) {
                match &node.out {
                    Some(out_map) if !out_map.is_empty() => {
                        if out_map.keys().any(|key| key.trim().is_empty()) {
                            return Err(PlanValidationError::MissingOutputBinding(node.id.clone()));
                        }
                    }
                    _ => {
                        return Err(PlanValidationError::MissingOutputBinding(node.id.clone()));
                    }
                }
            }
        }

        Ok(())
    }

    fn operation_requires_tool(op: &Operation) -> bool {
        matches!(
            op,
            Operation::Call
                | Operation::Map
                | Operation::Reduce
                | Operation::Verify
                | Operation::MemRead
                | Operation::MemWrite
                | Operation::Retry
        )
    }

    fn operation_requires_output(op: &Operation) -> bool {
        matches!(
            op,
            Operation::Call
                | Operation::Map
                | Operation::Reduce
                | Operation::Verify
                | Operation::MemRead
                | Operation::Retry
        )
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
    #[error("Unknown tool referenced: {0}")]
    UnknownTool(String),
    #[error("Node {0} is missing output bindings")]
    MissingOutputBinding(String),
    #[error("Node {0} requires either a tool or capability")]
    MissingToolOrCapability(String),
}

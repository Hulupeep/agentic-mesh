use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: Option<String>,
    pub io: IoSpec,
    pub capabilities: Option<Vec<String>>,
    pub constraints: Option<Constraints>,
    pub provenance: Option<Provenance>,
    pub quality: Option<Quality>,
    pub policy: Option<Policy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoSpec {
    pub input: Schema,
    pub output: Schema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    #[serde(rename = "type")]
    pub schema_type: String,
    pub properties: Option<HashMap<String, Box<Schema>>>,
    pub required: Option<Vec<String>>,
    pub items: Option<Box<Schema>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraints {
    pub input_tokens_max: Option<u32>,
    pub latency_p50_ms: Option<u32>,
    pub cost_per_call_usd: Option<f64>,
    pub rate_limit_qps: Option<u32>,
    pub side_effects: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provenance {
    pub attribution_required: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quality {
    pub freshness_window: Option<String>, // ISO 8601 duration
    pub coverage_tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub deny_if: Option<Vec<String>>,
}

// Tool client for invoking tools over HTTP
#[derive(Debug, Clone)]
pub struct ToolClient {
    client: reqwest::Client,
}

#[derive(Debug, Serialize, Deserialize)]
struct InvokeRequest {
    args: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct InvokeResponse {
    result: serde_json::Value,
    error: Option<String>,
}

impl ToolClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    pub async fn invoke_tool(
        &self,
        tool_url: &str,
        tool_name: &str,
        args: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, ToolError> {
        let request = InvokeRequest { args };
        let base_url = tool_url.trim_end_matches('/');
        let invoke_url = format!("{}/invoke/{}", base_url, tool_name);

        let response = self
            .client
            .post(invoke_url)
            .json(&request)
            .send()
            .await
            .map_err(|e| ToolError::Communication(e.to_string()))?;

        let invoke_response: InvokeResponse = response
            .json()
            .await
            .map_err(|e| ToolError::Communication(e.to_string()))?;

        if let Some(error) = invoke_response.error {
            return Err(ToolError::Invocation(error));
        }

        Ok(invoke_response.result)
    }

    pub async fn get_tool_spec(
        &self,
        tool_url: &str,
        tool_name: &str,
    ) -> Result<ToolSpec, ToolError> {
        let base_url = tool_url.trim_end_matches('/');
        let spec_url = format!("{}/spec/{}", base_url, tool_name);
        let response = self
            .client
            .get(spec_url)
            .send()
            .await
            .map_err(|e| ToolError::Communication(e.to_string()))?;

        let spec: ToolSpec = response
            .json()
            .await
            .map_err(|e| ToolError::Communication(e.to_string()))?;

        Ok(spec)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("Communication error: {0}")]
    Communication(String),
    #[error("Invocation error: {0}")]
    Invocation(String),
    #[error("Validation error: {0}")]
    Validation(String),
}

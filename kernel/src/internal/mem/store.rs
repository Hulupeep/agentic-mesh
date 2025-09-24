use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::internal::evidence::verify::Evidence;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub key: String,
    pub value: serde_json::Value,
    pub provenance: Option<Vec<String>>,
    pub confidence: Option<f64>,
    pub ttl: Option<String>, // ISO 8601 duration
    pub timestamp: String,   // ISO 8601 datetime
}

pub struct MemoryStore {
    client: reqwest::Client,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    pub async fn read(&self, memory_url: &str, key: &str) -> Result<Option<MemoryEntry>, MemoryError> {
        let response = self
            .client
            .post(format!("{}/invoke", memory_url))
            .json(&serde_json::json!({
                "operation": "read",
                "key": key
            }))
            .send()
            .await
            .map_err(|e| MemoryError::Communication(e.to_string()))?;

        let result: serde_json::Value = response
            .json()
            .await
            .map_err(|e| MemoryError::Communication(e.to_string()))?;

        if result.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
            if let Some(value) = result.get("value") {
                // For now, return a simple memory entry with the value
                // In a full implementation, we'd parse all fields properly
                Ok(Some(MemoryEntry {
                    key: key.to_string(),
                    value: value.clone(),
                    provenance: None,
                    confidence: None,
                    ttl: None,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                }))
            } else {
                Ok(None)
            }
        } else {
            Ok(None) // Key not found
        }
    }

    pub async fn write(
        &self,
        memory_url: &str,
        key: &str,
        value: &serde_json::Value,
        provenance: Option<&Vec<String>>,
        confidence: Option<f64>,
        ttl: Option<&str>,
    ) -> Result<(), MemoryError> {
        // First, validate the evidence if confidence is provided
        if let Some(conf) = confidence {
            if conf < 0.8 {
                return Err(MemoryError::InsufficientConfidence(conf));
            }
        }

        let response = self
            .client
            .post(format!("{}/invoke", memory_url))
            .json(&serde_json::json!({
                "operation": "write",
                "key": key,
                "value": value,
                "provenance": provenance,
                "confidence": confidence,
                "ttl": ttl.unwrap_or("P90D") // Default TTL is 90 days
            }))
            .send()
            .await
            .map_err(|e| MemoryError::Communication(e.to_string()))?;

        let result: serde_json::Value = response
            .json()
            .await
            .map_err(|e| MemoryError::Communication(e.to_string()))?;

        if result.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
            Ok(())
        } else {
            let message = result.get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error");
            Err(MemoryError::StorageError(message.to_string()))
        }
    }

    pub async fn forget(&self, memory_url: &str, key: &str) -> Result<(), MemoryError> {
        let response = self
            .client
            .post(format!("{}/invoke", memory_url))
            .json(&serde_json::json!({
                "operation": "forget",
                "key": key
            }))
            .send()
            .await
            .map_err(|e| MemoryError::Communication(e.to_string()))?;

        let result: serde_json::Value = response
            .json()
            .await
            .map_err(|e| MemoryError::Communication(e.to_string()))?;

        if result.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
            Ok(())
        } else {
            let message = result.get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error");
            Err(MemoryError::StorageError(message.to_string()))
        }
    }

    pub async fn write_with_evidence(
        &self,
        memory_url: &str,
        key: &str,
        value: &serde_json::Value,
        evidence: &Evidence,
        min_confidence: f64,
    ) -> Result<(), MemoryError> {
        // Validate the evidence before writing
        let verifier = crate::internal::evidence::verify::EvidenceVerifier;
        verifier.validate_evidence_for_storage(evidence, min_confidence)
            .map_err(|e| MemoryError::EvidenceValidation(e.to_string()))?;

        // Extract provenance from evidence if available
        let provenance = evidence.verdicts.as_ref().map(|verdicts| {
            verdicts.iter()
                .filter_map(|v| {
                    if v.needs_citation {
                        Some(v.claim_id.clone())
                    } else {
                        None
                    }
                })
                .collect::<Vec<String>>()
        });

        // Calculate mean confidence from evidence
        let verification_result = verifier.verify_evidence(evidence);
        let confidence = Some(verification_result.mean_confidence);

        self.write(memory_url, key, value, provenance.as_ref(), confidence, None).await
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    #[error("Communication error: {0}")]
    Communication(String),
    #[error("Storage error: {0}")]
    StorageError(String),
    #[error("Insufficient confidence: {0}")]
    InsufficientConfidence(f64),
    #[error("Evidence validation error: {0}")]
    EvidenceValidation(String),
}
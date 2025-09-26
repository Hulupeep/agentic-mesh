use crate::internal::evidence::verify::{Evidence, EvidenceVerifier};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

static TTL_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^P(?:(\d+)D)?(?:T(?:(\d+)H)?(?:(\d+)M)?(?:(\d+)S)?)?$").expect("valid TTL regex")
});

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub key: String,
    pub value: serde_json::Value,
    pub provenance: Option<Vec<String>>,
    pub confidence: f64,
    pub ttl: String,
    pub timestamp: String,
    pub expires_at: Option<String>,
    pub evidence_summary: Option<serde_json::Value>,
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

    pub async fn read(
        &self,
        memory_url: &str,
        key: &str,
    ) -> Result<Option<MemoryEntry>, MemoryError> {
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

        let payload = result.get("result").unwrap_or(&result);

        if payload
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            if let Some(entry) = payload.get("entry") {
                Ok(Some(self.parse_entry(key, entry)?))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
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
        evidence_summary: Option<&serde_json::Value>,
    ) -> Result<(), MemoryError> {
        let provenance = provenance.ok_or(MemoryError::MissingProvenance)?;
        if provenance.is_empty() {
            return Err(MemoryError::MissingProvenance);
        }

        let confidence = confidence.ok_or_else(|| MemoryError::InsufficientConfidence(0.0))?;
        if confidence < 0.8 {
            return Err(MemoryError::InsufficientConfidence(confidence));
        }

        let ttl_value = validate_ttl(ttl)?;

        let mut payload = serde_json::json!({
            "operation": "write",
            "key": key,
            "value": value,
            "provenance": provenance,
            "confidence": confidence,
            "ttl": ttl_value,
        });

        if let Some(summary) = evidence_summary {
            payload["evidence_summary"] = summary.clone();
        }

        let response = self
            .client
            .post(format!("{}/invoke", memory_url))
            .json(&payload)
            .send()
            .await
            .map_err(|e| MemoryError::Communication(e.to_string()))?;

        let result: serde_json::Value = response
            .json()
            .await
            .map_err(|e| MemoryError::Communication(e.to_string()))?;

        let payload = result.get("result").unwrap_or(&result);

        if payload
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            Ok(())
        } else {
            let message = payload
                .get("message")
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

        let payload = result.get("result").unwrap_or(&result);

        if payload
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            Ok(())
        } else {
            let message = payload
                .get("message")
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
        let verifier = EvidenceVerifier;
        verifier
            .validate_evidence_for_storage(evidence, min_confidence)
            .map_err(|e| MemoryError::EvidenceValidation(e.to_string()))?;

        let provenance = evidence.verdicts.as_ref().map(|verdicts| {
            verdicts
                .iter()
                .filter_map(|v| if v.needs_citation { Some(v.claim_id.clone()) } else { None })
                .collect::<Vec<String>>()
        });

        let verification_result = verifier.verify_evidence(evidence);
        let confidence = Some(verification_result.mean_confidence);
        let summary_value = serde_json::to_value(&verification_result)
            .map_err(|e| MemoryError::EvidenceValidation(e.to_string()))?;

        self.write(
            memory_url,
            key,
            value,
            provenance.as_ref(),
            confidence,
            None,
            Some(&summary_value),
        )
        .await
    }

    fn parse_entry(
        &self,
        key: &str,
        entry: &serde_json::Value,
    ) -> Result<MemoryEntry, MemoryError> {
        let value = entry.get("value").cloned().ok_or_else(|| {
            MemoryError::StorageError("Missing value in memory entry".to_string())
        })?;

        let provenance = entry
            .get("provenance")
            .and_then(|p| p.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect::<Vec<String>>()
            });

        let confidence = entry
            .get("confidence")
            .and_then(|c| c.as_f64())
            .ok_or_else(|| MemoryError::StorageError("Missing confidence in memory entry".to_string()))?;

        let ttl = entry
            .get("ttl")
            .and_then(|t| t.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| MemoryError::StorageError("Missing TTL in memory entry".to_string()))?;

        let timestamp = entry
            .get("timestamp")
            .and_then(|t| t.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());

        let expires_at = entry
            .get("expires_at")
            .and_then(|t| t.as_str())
            .map(|s| s.to_string());

        let evidence_summary = entry.get("evidence_summary").cloned();

        Ok(MemoryEntry {
            key: key.to_string(),
            value,
            provenance,
            confidence,
            ttl,
            timestamp,
            expires_at,
            evidence_summary,
        })
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
    #[error("Memory writes require provenance")]
    MissingProvenance,
    #[error("Invalid TTL: {0}")]
    InvalidTtl(String),
}

fn validate_ttl(ttl: Option<&str>) -> Result<String, MemoryError> {
    let canonical = ttl
        .map(|s| s.trim().to_uppercase())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "P90D".to_string());
    if !TTL_REGEX.is_match(&canonical) {
        return Err(MemoryError::InvalidTtl(canonical));
    }
    if canonical == "P0D" || canonical == "PT0S" {
        return Err(MemoryError::InvalidTtl(canonical));
    }
    Ok(canonical)
}

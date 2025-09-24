use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub claims: Option<Vec<String>>,
    pub supports: Option<Vec<Support>>,
    pub contradicts: Option<Vec<Contradiction>>,
    pub verdicts: Option<Vec<Verdict>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Support {
    pub claim_id: String,
    pub source: String,
    pub confidence: f64,
    pub explanation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contradiction {
    pub claim_id: String,
    pub source: String,
    pub confidence: f64,
    pub explanation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Verdict {
    pub claim_id: String,
    pub verdict: VerdictType,
    pub confidence: f64,
    pub needs_citation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VerdictType {
    Supported,
    Contradicted,
    Neutral,
}

pub struct EvidenceVerifier;

impl EvidenceVerifier {
    pub fn verify_evidence(&self, evidence: &Evidence) -> VerificationResult {
        let mut total_claims = 0;
        let mut supported_claims = 0;
        let mut contradicted_claims = 0;
        let mut mean_confidence = 0.0;
        let mut confidence_sum = 0.0;

        // Count verdicts and calculate metrics
        if let Some(ref verdicts) = evidence.verdicts {
            total_claims = verdicts.len();
            
            for verdict in verdicts {
                match verdict.verdict {
                    VerdictType::Supported => supported_claims += 1,
                    VerdictType::Contradicted => contradicted_claims += 1,
                    VerdictType::Neutral => {}
                }
                
                confidence_sum += verdict.confidence;
            }
            
            if total_claims > 0 {
                mean_confidence = confidence_sum / total_claims as f64;
            }
        }

        let verification_result = VerificationResult {
            total_claims,
            supported_claims,
            contradicted_claims,
            mean_confidence,
            needs_citation_count: evidence.verdicts.as_ref()
                .map(|v| v.iter().filter(|verdict| verdict.needs_citation).count())
                .unwrap_or(0),
        };

        verification_result
    }

    pub fn validate_evidence_for_storage(&self, evidence: &Evidence, min_confidence: f64) -> Result<(), EvidenceValidationError> {
        // Check if mean confidence meets minimum threshold
        let verification_result = self.verify_evidence(evidence);
        
        if verification_result.mean_confidence < min_confidence {
            return Err(EvidenceValidationError::InsufficientConfidence {
                mean_confidence: verification_result.mean_confidence,
                min_required: min_confidence,
            });
        }

        // Check for contradictions exceeding threshold
        if let Some(verdicts) = &evidence.verdicts {
            let contradiction_count = verdicts
                .iter()
                .filter(|v| matches!(v.verdict, VerdictType::Contradicted))
                .count();
                
            let total_verdicts = verdicts.len();
            if total_verdicts > 0 {
                let contradiction_ratio = contradiction_count as f64 / total_verdicts as f64;
                // For now, we'll use a threshold of 0.5 (more than 50% contradictions is an issue)
                if contradiction_ratio > 0.5 {
                    return Err(EvidenceValidationError::TooManyContradictions {
                        contradiction_ratio,
                        threshold: 0.5,
                    });
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct VerificationResult {
    pub total_claims: usize,
    pub supported_claims: usize,
    pub contradicted_claims: usize,
    pub mean_confidence: f64,
    pub needs_citation_count: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum EvidenceValidationError {
    #[error("Insufficient evidence confidence: {mean_confidence:.2} < required {min_required:.2}")]
    InsufficientConfidence { mean_confidence: f64, min_required: f64 },
    #[error("Too many contradictions: ratio {contradiction_ratio:.2} > threshold {threshold:.2}")]
    TooManyContradictions { contradiction_ratio: f64, threshold: f64 },
}
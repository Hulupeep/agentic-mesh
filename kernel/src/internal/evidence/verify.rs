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
        let mut confidence_sum = 0.0;
        let mut global_min_conf: f64 = f64::INFINITY;
        let mut global_max_conf: f64 = 0.0;

        let mut claim_accumulators: HashMap<String, ClaimAccumulator> = HashMap::new();

        if let Some(claims) = &evidence.claims {
            for claim in claims {
                claim_accumulators.entry(claim.clone()).or_default();
            }
        }

        if let Some(supports) = &evidence.supports {
            for support in supports {
                let entry = claim_accumulators
                    .entry(support.claim_id.clone())
                    .or_default();
                entry.supports += 1;
                entry.add_confidence(support.confidence);
            }
        }

        if let Some(contradictions) = &evidence.contradicts {
            for contradiction in contradictions {
                let entry = claim_accumulators
                    .entry(contradiction.claim_id.clone())
                    .or_default();
                entry.contradictions += 1;
                entry.add_confidence(contradiction.confidence);
            }
        }

        if let Some(verdicts) = &evidence.verdicts {
            total_claims = verdicts.len();

            for verdict in verdicts {
                match verdict.verdict {
                    VerdictType::Supported => supported_claims += 1,
                    VerdictType::Contradicted => contradicted_claims += 1,
                    VerdictType::Neutral => {}
                }

                confidence_sum += verdict.confidence;
                global_min_conf = global_min_conf.min(verdict.confidence);
                global_max_conf = global_max_conf.max(verdict.confidence);

                let entry = claim_accumulators
                    .entry(verdict.claim_id.clone())
                    .or_default();
                entry.add_confidence(verdict.confidence);
            }
        }

        if global_min_conf == f64::INFINITY {
            global_min_conf = 0.0;
        }

        let per_claim = claim_accumulators
            .into_iter()
            .map(|(claim_id, acc)| (claim_id, acc.into_summary()))
            .collect();

        let mean_confidence = if total_claims > 0 {
            confidence_sum / total_claims as f64
        } else {
            0.0
        };

        VerificationResult {
            total_claims,
            supported_claims,
            contradicted_claims,
            mean_confidence,
            needs_citation_count: evidence
                .verdicts
                .as_ref()
                .map(|v| v.iter().filter(|verdict| verdict.needs_citation).count())
                .unwrap_or(0),
            max_confidence: global_max_conf,
            min_confidence: global_min_conf,
            per_claim,
        }
    }

    pub fn validate_evidence_for_storage(
        &self,
        evidence: &Evidence,
        min_confidence: f64,
    ) -> Result<(), EvidenceValidationError> {
        let verification_result = self.verify_evidence(evidence);

        if verification_result.mean_confidence < min_confidence {
            return Err(EvidenceValidationError::InsufficientConfidence {
                mean_confidence: verification_result.mean_confidence,
                min_required: min_confidence,
            });
        }

        for (claim, summary) in &verification_result.per_claim {
            if summary.supports == 0 {
                return Err(EvidenceValidationError::MissingSupport {
                    claim_id: claim.clone(),
                });
            }
        }

        if let Some(verdicts) = &evidence.verdicts {
            let contradiction_count = verdicts
                .iter()
                .filter(|v| matches!(v.verdict, VerdictType::Contradicted))
                .count();

            let total_verdicts = verdicts.len();
            if total_verdicts > 0 {
                let contradiction_ratio = contradiction_count as f64 / total_verdicts as f64;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub total_claims: usize,
    pub supported_claims: usize,
    pub contradicted_claims: usize,
    pub mean_confidence: f64,
    pub needs_citation_count: usize,
    pub max_confidence: f64,
    pub min_confidence: f64,
    pub per_claim: HashMap<String, ClaimSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimSummary {
    pub supports: usize,
    pub contradictions: usize,
    pub average_confidence: Option<f64>,
    pub max_confidence: Option<f64>,
    pub min_confidence: Option<f64>,
}

struct ClaimAccumulator {
    supports: usize,
    contradictions: usize,
    confidence_sum: f64,
    confidence_count: usize,
    min_confidence: f64,
    max_confidence: f64,
}

impl Default for ClaimAccumulator {
    fn default() -> Self {
        Self {
            supports: 0,
            contradictions: 0,
            confidence_sum: 0.0,
            confidence_count: 0,
            min_confidence: f64::INFINITY,
            max_confidence: 0.0,
        }
    }
}

impl ClaimAccumulator {
    fn add_confidence(&mut self, value: f64) {
        self.confidence_sum += value;
        self.confidence_count += 1;
        self.max_confidence = if self.confidence_count == 1 {
            value
        } else {
            self.max_confidence.max(value)
        };
        self.min_confidence = if self.confidence_count == 1 {
            value
        } else {
            self.min_confidence.min(value)
        };
    }

    fn into_summary(self) -> ClaimSummary {
        let average = if self.confidence_count > 0 {
            Some(self.confidence_sum / self.confidence_count as f64)
        } else {
            None
        };

        ClaimSummary {
            supports: self.supports,
            contradictions: self.contradictions,
            average_confidence: average,
            max_confidence: if self.confidence_count > 0 {
                Some(self.max_confidence)
            } else {
                None
            },
            min_confidence: if self.confidence_count > 0 {
                Some(self.min_confidence)
            } else {
                None
            },
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EvidenceValidationError {
    #[error("Insufficient evidence confidence: {mean_confidence:.2} < required {min_required:.2}")]
    InsufficientConfidence {
        mean_confidence: f64,
        min_required: f64,
    },
    #[error("Too many contradictions: ratio {contradiction_ratio:.2} > threshold {threshold:.2}")]
    TooManyContradictions {
        contradiction_ratio: f64,
        threshold: f64,
    },
    #[error("Claim {claim_id} is missing supporting evidence")]
    MissingSupport { claim_id: String },
}

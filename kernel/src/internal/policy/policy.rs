use crate::internal::{
    evidence::verify::{Evidence, VerificationResult},
    tools::spec::ToolSpec,
    trace::trace::Trace,
};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct PolicyContext {
    pub evidence: Option<Evidence>,
    pub tool_specs: Vec<ToolSpec>,
    pub traces: Vec<Trace>,
    pub variables: HashMap<String, Value>,
}

pub struct PolicyEngine;

const MIN_EVIDENCE_CONFIDENCE: f64 = 0.8;

impl PolicyEngine {
    pub fn enforce_policies(&self, ctx: &PolicyContext) -> Result<PolicyResult, PolicyError> {
        let mut violations = Vec::new();
        let mut enforcement_actions = Vec::new();
        let mut evidence_summary_count = 0usize;

        // Check evidence confidence requirements
        if let Some(ref evidence) = ctx.evidence {
            let verifier = crate::internal::evidence::verify::EvidenceVerifier;
            let verification = verifier.verify_evidence(evidence);

            // If mean confidence is too low, add a violation
            if verification.mean_confidence < 0.7 {
                violations.push(PolicyViolation {
                    rule: "minimum_confidence".to_string(),
                    severity: PolicySeverity::Warning,
                    message: format!(
                        "Mean evidence confidence {:.2} is below threshold",
                        verification.mean_confidence
                    ),
                    details: None,
                });
            }
        }

        // Check tool usage policies
        for tool_spec in &ctx.tool_specs {
            if let Some(ref policy) = tool_spec.policy {
                if let Some(ref deny_patterns) = policy.deny_if {
                    for pattern in deny_patterns {
                        enforcement_actions.push(EnforcementAction {
                            action: "check_pattern".to_string(),
                            target: tool_spec.name.clone(),
                            details: Some(serde_json::json!({ "pattern": pattern })),
                        });
                    }
                }
            }

            // Check provenance requirements
            if let Some(ref provenance) = tool_spec.provenance {
                if provenance.attribution_required == Some(true) {
                    // Check if responses using this tool include citations
                    // This would require checking the final response, which we don't have here
                    // In a real implementation, we'd pass the response to check for citations
                    enforcement_actions.push(EnforcementAction {
                        action: "verify_attribution".to_string(),
                        target: tool_spec.name.clone(),
                        details: Some(serde_json::json!({ "attribution_required": true })),
                    });
                }
            }
        }

        // Check trace patterns for violations
        let mut budget_summary_seen = false;
        for trace in &ctx.traces {
            if trace.event_type == "policy_violation" {
                let message = trace
                    .data
                    .as_ref()
                    .and_then(|data| data.get("description"))
                    .and_then(|value| value.as_str())
                    .unwrap_or("Tool policy violation detected")
                    .to_string();

                violations.push(PolicyViolation {
                    rule: "tool_policy".to_string(),
                    severity: PolicySeverity::Error,
                    message,
                    details: trace.data.clone(),
                });
                continue;
            }

            if trace.event_type == "budget_summary" {
                budget_summary_seen = true;
                if let Some(data) = &trace.data {
                    if let (Some(total_latency), Some(latency_budget)) = (
                        data.get("total_latency_ms").and_then(|v| v.as_f64()),
                        data.get("latency_budget_ms").and_then(|v| v.as_f64()),
                    ) {
                        if latency_budget > 0.0 && total_latency > latency_budget {
                            violations.push(PolicyViolation {
                                rule: "latency_budget".to_string(),
                                severity: PolicySeverity::Error,
                                message: format!(
                                    "Latency budget exceeded: {:.2}ms > {:.2}ms",
                                    total_latency, latency_budget
                                ),
                                details: Some(serde_json::json!({
                                    "total_latency_ms": total_latency,
                                    "latency_budget_ms": latency_budget,
                                })),
                            });
                        }
                    }

                    if let (Some(total_cost), Some(cost_cap)) = (
                        data.get("total_cost_usd").and_then(|v| v.as_f64()),
                        data.get("cost_cap_usd").and_then(|v| v.as_f64()),
                    ) {
                        if cost_cap > 0.0 && total_cost > cost_cap {
                            violations.push(PolicyViolation {
                                rule: "cost_cap".to_string(),
                                severity: PolicySeverity::Error,
                                message: format!(
                                    "Cost budget exceeded: ${:.4} > ${:.4}",
                                    total_cost, cost_cap
                                ),
                                details: Some(serde_json::json!({
                                    "total_cost_usd": total_cost,
                                    "cost_cap_usd": cost_cap,
                                })),
                            });
                        }
                    }
                }
                continue;
            }

            if trace.event_type == "evidence_summary" {
                evidence_summary_count += 1;
                if let Some(data) = &trace.data {
                    PolicyEngine::evaluate_summary_value(
                        "trace:evidence_summary",
                        data,
                        &mut violations,
                        &mut enforcement_actions,
                    );
                } else {
                    violations.push(PolicyViolation {
                        rule: "invalid_evidence_summary".to_string(),
                        severity: PolicySeverity::Error,
                        message: "Evidence summary trace missing payload".to_string(),
                        details: None,
                    });
                }
                continue;
            }

            if let Some(cost) = trace.cost_usd {
                if cost > 1.0 {
                    // arbitrary threshold
                    violations.push(PolicyViolation {
                        rule: "cost_limit".to_string(),
                        severity: PolicySeverity::Warning,
                        message: format!("Trace cost ${:.4} exceeded threshold", cost),
                        details: Some(serde_json::json!({ "cost": cost })),
                    });
                }
            }
        }

        if !budget_summary_seen {
            enforcement_actions.push(EnforcementAction {
                action: "emit_budget_summary".to_string(),
                target: "plan".to_string(),
                details: Some(serde_json::json!({ "note": "Missing budget summary trace" })),
            });
        }

        // Inspect variable bindings for evidence summaries the plan persisted
        for (key, value) in &ctx.variables {
            if key.ends_with("_summary") || key.contains("summary") {
                evidence_summary_count += 1;
                PolicyEngine::evaluate_summary_value(
                    &format!("variable:{}", key),
                    value,
                    &mut violations,
                    &mut enforcement_actions,
                );
            }
        }

        if ctx.evidence.is_some() && evidence_summary_count == 0 {
            violations.push(PolicyViolation {
                rule: "missing_evidence_summary".to_string(),
                severity: PolicySeverity::Error,
                message: "Evidence supplied but no verification summary found".to_string(),
                details: None,
            });
        }

        let allowed = violations.is_empty();

        Ok(PolicyResult {
            violations,
            enforcement_actions,
            allowed,
        })
    }

    fn evaluate_summary_value(
        origin: &str,
        value: &Value,
        violations: &mut Vec<PolicyViolation>,
        enforcement_actions: &mut Vec<EnforcementAction>,
    ) -> bool {
        match serde_json::from_value::<VerificationResult>(value.clone()) {
            Ok(summary) => {
                let mut summary_valid = true;

                if summary.total_claims == 0 {
                    summary_valid = false;
                    violations.push(PolicyViolation {
                        rule: "evidence_missing_claims".to_string(),
                        severity: PolicySeverity::Error,
                        message: format!("{} provided an evidence summary with no claims", origin),
                        details: Some(value.clone()),
                    });
                }

                if summary.mean_confidence < MIN_EVIDENCE_CONFIDENCE {
                    summary_valid = false;
                    violations.push(PolicyViolation {
                        rule: "evidence_confidence".to_string(),
                        severity: PolicySeverity::Error,
                        message: format!(
                            "{} mean confidence {:.2} below {:.2}",
                            origin, summary.mean_confidence, MIN_EVIDENCE_CONFIDENCE
                        ),
                        details: Some(value.clone()),
                    });
                }

                if summary.supported_claims == 0 {
                    summary_valid = false;
                    violations.push(PolicyViolation {
                        rule: "evidence_missing_support".to_string(),
                        severity: PolicySeverity::Error,
                        message: format!("{} verification summary has no supported claims", origin),
                        details: Some(value.clone()),
                    });
                }

                if summary_valid {
                    enforcement_actions.push(EnforcementAction {
                        action: "evidence_summary_valid".to_string(),
                        target: origin.to_string(),
                        details: Some(value.clone()),
                    });
                }

                summary_valid
            }
            Err(err) => {
                violations.push(PolicyViolation {
                    rule: "invalid_evidence_summary".to_string(),
                    severity: PolicySeverity::Error,
                    message: format!("{} evidence summary failed to parse: {}", origin, err),
                    details: Some(value.clone()),
                });
                false
            }
        }
    }

    pub fn check_memory_write_policy(
        &self,
        evidence: Option<&Evidence>,
    ) -> Result<(), PolicyError> {
        if let Some(evidence) = evidence {
            let verifier = crate::internal::evidence::verify::EvidenceVerifier;
            let verification = verifier.verify_evidence(evidence);

            // Memory writes require a minimum confidence level
            if verification.mean_confidence < 0.8 {
                return Err(PolicyError::InsufficientEvidenceConfidence {
                    mean_confidence: verification.mean_confidence,
                    required: 0.8,
                });
            }
        } else {
            return Err(PolicyError::InsufficientEvidenceConfidence {
                mean_confidence: 0.0,
                required: 0.8,
            });
        }

        Ok(())
    }

    pub fn check_response_policy(
        &self,
        response: &str,
        ctx: &PolicyContext,
    ) -> Result<String, PolicyError> {
        // Check if response meets citation requirements
        let mut modified_response = response.to_string();

        // If any tools required attribution, ensure citations are present
        for tool_spec in &ctx.tool_specs {
            if let Some(ref provenance) = tool_spec.provenance {
                if provenance.attribution_required == Some(true) {
                    // In a real implementation, we'd check if the response contains citations
                    // For now, we'll just ensure citations are mentioned if there's evidence
                    if let Some(ref evidence) = ctx.evidence {
                        if evidence.verdicts.as_ref().map_or(0, |v| v.len()) > 0 {
                            // Add a note about citations if they're not already present
                            if !response.contains("citation") && !response.contains("source") {
                                modified_response.push_str(
                                    "\n\nSources: [Citations based on verification evidence]",
                                );
                            }
                        }
                    }
                }
            }
        }

        Ok(modified_response)
    }
}

#[derive(Debug)]
pub struct PolicyResult {
    pub violations: Vec<PolicyViolation>,
    pub enforcement_actions: Vec<EnforcementAction>,
    pub allowed: bool,
}

#[derive(Debug)]
pub struct PolicyViolation {
    pub rule: String,
    pub severity: PolicySeverity,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

#[derive(Debug)]
pub enum PolicySeverity {
    Warning,
    Error,
    Info,
}

#[derive(Debug)]
pub struct EnforcementAction {
    pub action: String,
    pub target: String,
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, thiserror::Error)]
pub enum PolicyError {
    #[error("Insufficient evidence confidence: {mean_confidence:.2} < required {required:.2}")]
    InsufficientEvidenceConfidence { mean_confidence: f64, required: f64 },
    #[error("Policy violation: {0}")]
    Violation(String),
}

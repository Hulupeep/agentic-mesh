//! Policy engine tests

use amp::internal::{
    evidence::verify::{Evidence, Support, Verdict, VerdictType},
    policy::policy::{PolicyContext, PolicyEngine},
    tools::spec::{IoSpec, Provenance, Schema, ToolSpec},
    trace::trace::Trace,
};
use std::collections::HashMap;

#[test]
fn test_policy_engine_evidence_verification() {
    let engine = PolicyEngine;

    // Test with high confidence evidence - should pass
    let high_confidence_evidence = Evidence {
        claims: Some(vec!["claim_0".to_string()]),
        supports: Some(vec![Support {
            claim_id: "claim_0".to_string(),
            source: "source_0".to_string(),
            confidence: 0.92,
            explanation: Some("High confidence support".to_string()),
        }]),
        contradicts: None,
        verdicts: Some(vec![Verdict {
            claim_id: "claim_0".to_string(),
            verdict: VerdictType::Supported,
            confidence: 0.9,
            needs_citation: true,
        }]),
    };

    let summary = serde_json::json!({
        "total_claims": 1,
        "supported_claims": 1,
        "contradicted_claims": 0,
        "mean_confidence": 0.9,
        "needs_citation_count": 0,
        "max_confidence": 0.9,
        "min_confidence": 0.9,
        "per_claim": {
            "claim_0": {
                "supports": 1,
                "contradictions": 0,
                "average_confidence": 0.9,
                "max_confidence": 0.9,
                "min_confidence": 0.9
            }
        }
    });

    let mut summary_trace = Trace::new(
        "evidence_summary".to_string(),
        "verify_step".to_string(),
        "Verification summary".to_string(),
    );
    summary_trace.data = Some(summary.clone());

    let ctx = PolicyContext {
        evidence: Some(high_confidence_evidence),
        tool_specs: vec![],
        traces: vec![summary_trace],
        variables: HashMap::from([("verification_summary".to_string(), summary.clone())]),
    };

    let result = engine.enforce_policies(&ctx);
    assert!(result.is_ok());

    // The result should be allowed
    if let Ok(policy_result) = result {
        assert!(policy_result.allowed);
        // Should not have violations for high confidence
        assert!(policy_result
            .violations
            .iter()
            .find(|v| v.message.contains("confidence"))
            .is_none());
    }

    println!("Policy engine evidence verification test passed");
}

#[test]
fn test_policy_engine_budget_violation_detection() {
    let engine = PolicyEngine;

    let mut budget_trace = Trace::new(
        "budget_summary".to_string(),
        "plan".to_string(),
        "Budget snapshot".to_string(),
    );
    budget_trace.data = Some(serde_json::json!({
        "total_latency_ms": 250.0,
        "latency_budget_ms": 120.0,
        "total_cost_usd": 0.25,
        "cost_cap_usd": 0.10,
        "total_tokens": 1_024,
    }));

    let ctx = PolicyContext {
        evidence: None,
        tool_specs: vec![],
        traces: vec![budget_trace],
        variables: HashMap::new(),
    };

    let result = engine
        .enforce_policies(&ctx)
        .expect("policy evaluation should succeed");
    assert!(!result.allowed);
    assert!(result
        .violations
        .iter()
        .any(|violation| violation.rule == "cost_cap"));
}

#[test]
fn test_policy_engine_surface_tool_policy_violation() {
    let engine = PolicyEngine;

    let mut violation_trace = Trace::new(
        "policy_violation".to_string(),
        "doc.search.local".to_string(),
        "Policy violation".to_string(),
    );
    violation_trace.data = Some(serde_json::json!({
        "description": "Tool doc.search.local invocation blocked by policy pattern 'pii'",
        "pattern": "pii",
    }));

    let ctx = PolicyContext {
        evidence: None,
        tool_specs: vec![],
        traces: vec![violation_trace],
        variables: HashMap::new(),
    };

    let result = engine
        .enforce_policies(&ctx)
        .expect("policy evaluation should succeed");
    assert!(!result.allowed);
    assert!(result
        .violations
        .iter()
        .any(|violation| violation.rule == "tool_policy"));
}

#[test]
fn test_policy_engine_requires_evidence_summary_when_evidence_present() {
    let engine = PolicyEngine;

    let evidence = Evidence {
        claims: Some(vec!["c1".to_string()]),
        supports: None,
        contradicts: None,
        verdicts: Some(vec![Verdict {
            claim_id: "c1".to_string(),
            verdict: VerdictType::Supported,
            confidence: 0.85,
            needs_citation: false,
        }]),
    };

    let ctx = PolicyContext {
        evidence: Some(evidence),
        tool_specs: vec![],
        traces: vec![],
        variables: HashMap::new(),
    };

    let result = engine
        .enforce_policies(&ctx)
        .expect("policy evaluation should succeed");
    assert!(!result.allowed);
    assert!(result
        .violations
        .iter()
        .any(|violation| violation.rule == "missing_evidence_summary"));
}

#[test]
fn test_policy_engine_rejects_low_confidence_summary() {
    let engine = PolicyEngine;

    let evidence = Evidence {
        claims: Some(vec!["c1".to_string()]),
        supports: None,
        contradicts: None,
        verdicts: Some(vec![Verdict {
            claim_id: "c1".to_string(),
            verdict: VerdictType::Supported,
            confidence: 0.85,
            needs_citation: false,
        }]),
    };

    let low_summary = serde_json::json!({
        "total_claims": 1,
        "supported_claims": 0,
        "contradicted_claims": 1,
        "mean_confidence": 0.6,
        "needs_citation_count": 1,
        "max_confidence": 0.6,
        "min_confidence": 0.6,
        "per_claim": {
            "c1": {
                "supports": 0,
                "contradictions": 1,
                "average_confidence": 0.6,
                "max_confidence": 0.6,
                "min_confidence": 0.6
            }
        }
    });

    let mut trace = Trace::new(
        "evidence_summary".to_string(),
        "verify_step".to_string(),
        "Low confidence summary".to_string(),
    );
    trace.data = Some(low_summary.clone());

    let ctx = PolicyContext {
        evidence: Some(evidence),
        tool_specs: vec![],
        traces: vec![trace],
        variables: HashMap::from([("verification_summary".to_string(), low_summary)]),
    };

    let result = engine
        .enforce_policies(&ctx)
        .expect("policy evaluation should succeed");
    assert!(!result.allowed);
    assert!(result
        .violations
        .iter()
        .any(|violation| violation.rule == "evidence_confidence"));
    assert!(result
        .violations
        .iter()
        .any(|violation| violation.rule == "evidence_missing_support"));
}

#[test]
fn test_policy_engine_handles_malformed_summary_payload() {
    let engine = PolicyEngine;

    let evidence = Evidence {
        claims: Some(vec!["c1".to_string()]),
        supports: None,
        contradicts: None,
        verdicts: Some(vec![Verdict {
            claim_id: "c1".to_string(),
            verdict: VerdictType::Supported,
            confidence: 0.85,
            needs_citation: false,
        }]),
    };

    let mut malformed_trace = Trace::new(
        "evidence_summary".to_string(),
        "verify_step".to_string(),
        "Malformed summary".to_string(),
    );
    malformed_trace.data = Some(serde_json::json!("not a summary"));

    let ctx = PolicyContext {
        evidence: Some(evidence),
        tool_specs: vec![],
        traces: vec![malformed_trace],
        variables: HashMap::new(),
    };

    let result = engine
        .enforce_policies(&ctx)
        .expect("policy evaluation should succeed");
    assert!(!result.allowed);
    assert!(result
        .violations
        .iter()
        .any(|violation| violation.rule == "invalid_evidence_summary"));
}

#[test]
fn test_policy_memory_write_validation() {
    let engine = PolicyEngine;

    // Test with high confidence evidence - should pass
    let high_confidence_evidence = Evidence {
        claims: Some(vec!["claim_0".to_string()]),
        supports: Some(vec![Support {
            claim_id: "claim_0".to_string(),
            source: "source_0".to_string(),
            confidence: 0.91,
            explanation: Some("Reliable support".to_string()),
        }]),
        contradicts: None,
        verdicts: Some(vec![Verdict {
            claim_id: "claim_0".to_string(),
            verdict: VerdictType::Supported,
            confidence: 0.9,
            needs_citation: true,
        }]),
    };

    let result = engine.check_memory_write_policy(Some(&high_confidence_evidence));
    assert!(result.is_ok());

    // Test with low confidence evidence - should fail
    let low_confidence_evidence = Evidence {
        claims: Some(vec!["claim_0".to_string()]),
        supports: Some(vec![Support {
            claim_id: "claim_0".to_string(),
            source: "source_1".to_string(),
            confidence: 0.55,
            explanation: Some("Weak support".to_string()),
        }]),
        contradicts: None,
        verdicts: Some(vec![Verdict {
            claim_id: "claim_0".to_string(),
            verdict: VerdictType::Supported,
            confidence: 0.5, // Below 0.8 threshold
            needs_citation: true,
        }]),
    };

    let result = engine.check_memory_write_policy(Some(&low_confidence_evidence));
    assert!(result.is_err());

    // Test with no evidence - should fail
    let result = engine.check_memory_write_policy(None);
    assert!(result.is_err());

    println!("Policy memory write validation test passed");
}

#[test]
fn test_policy_response_citation_check() {
    let engine = PolicyEngine;

    // Create a context with a tool that requires citations
    let tool_spec = ToolSpec {
        name: "citation_required_tool".to_string(),
        description: None,
        io: IoSpec {
            input: Schema {
                schema_type: "object".to_string(),
                properties: None,
                required: None,
                items: None,
            },
            output: Schema {
                schema_type: "object".to_string(),
                properties: None,
                required: None,
                items: None,
            },
        },
        capabilities: None,
        constraints: None,
        provenance: Some(Provenance {
            attribution_required: Some(true), // Requires citations
        }),
        quality: None,
        policy: None,
    };

    // Evidence that supports citations being needed
    let evidence = Evidence {
        claims: Some(vec!["claim_0".to_string()]),
        supports: Some(vec![Support {
            claim_id: "claim_0".to_string(),
            source: "source_0".to_string(),
            confidence: 0.9,
            explanation: Some("Citation support".to_string()),
        }]),
        contradicts: None,
        verdicts: Some(vec![Verdict {
            claim_id: "claim_0".to_string(),
            verdict: VerdictType::Supported,
            confidence: 0.9,
            needs_citation: true,
        }]),
    };

    let ctx = PolicyContext {
        evidence: Some(evidence),
        tool_specs: vec![tool_spec],
        traces: vec![],
        variables: HashMap::new(),
    };

    // Test response without citations - should be modified to include citations
    let response = "This is an answer based on the provided information.".to_string();
    let modified_response = engine.check_response_policy(&response, &ctx);

    assert!(modified_response.is_ok());
    let final_response = modified_response.unwrap();

    // The response should mention citations since evidence requires them
    assert!(final_response.contains("Citations") || final_response.contains("source"));

    println!("Policy response citation check test passed");
}

#[test]
fn test_policy_context_empty() {
    let engine = PolicyEngine;

    let ctx = PolicyContext {
        evidence: None,
        tool_specs: vec![],
        traces: vec![],
        variables: HashMap::new(),
    };

    let result = engine.enforce_policies(&ctx);
    assert!(result.is_ok());

    if let Ok(policy_result) = result {
        assert!(policy_result.allowed);
        // Should have fewer violations with empty context
        println!(
            "Policy result violations: {}",
            policy_result.violations.len()
        );
    }

    println!("Policy engine empty context test passed");
}

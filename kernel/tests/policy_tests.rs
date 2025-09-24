//! Policy engine tests

use amp::internal::{
    policy::policy::{PolicyEngine, PolicyContext, PolicyResult, PolicyError},
    evidence::verify::{Evidence, Verdict, VerdictType},
    tools::spec::{ToolSpec, Provenance},
    trace::trace::Trace,
};
use serde_json::Value;
use std::collections::HashMap;

#[test]
fn test_policy_engine_evidence_verification() {
    let engine = PolicyEngine;
    
    // Test with high confidence evidence - should pass
    let high_confidence_evidence = Evidence {
        claims: Some(vec!["Test claim".to_string()]),
        supports: None,
        contradicts: None,
        verdicts: Some(vec![Verdict {
            claim_id: "claim_0".to_string(),
            verdict: VerdictType::Supported,
            confidence: 0.9,
            needs_citation: true,
        }]),
    };

    let ctx = PolicyContext {
        evidence: Some(high_confidence_evidence),
        tool_specs: vec![],
        traces: vec![],
        variables: HashMap::new(),
    };

    let result = engine.enforce_policies(&ctx);
    assert!(result.is_ok());
    
    // The result should be allowed
    if let Ok(policy_result) = result {
        assert!(policy_result.allowed);
        // Should not have violations for high confidence
        assert!(policy_result.violations.iter()
            .find(|v| v.message.contains("confidence")).is_none());
    }

    println!("Policy engine evidence verification test passed");
}

#[test]
fn test_policy_memory_write_validation() {
    let engine = PolicyEngine;
    
    // Test with high confidence evidence - should pass
    let high_confidence_evidence = Evidence {
        claims: Some(vec!["Test claim".to_string()]),
        supports: None,
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
        claims: Some(vec!["Test claim".to_string()]),
        supports: None,
        contradicts: None,
        verdicts: Some(vec![Verdict {
            claim_id: "claim_0".to_string(),
            verdict: VerdictType::Supported,
            confidence: 0.5,  // Below 0.8 threshold
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
        io: Default::default(),  // Using default for brevity
        constraints: None,
        provenance: Some(Provenance {
            attribution_required: Some(true),  // Requires citations
        }),
        quality: None,
        policy: None,
    };

    // Evidence that supports citations being needed
    let evidence = Evidence {
        claims: Some(vec!["Test claim".to_string()]),
        supports: None,
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
        println!("Policy result violations: {}", policy_result.violations.len());
    }

    println!("Policy engine empty context test passed");
}
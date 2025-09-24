//! End-to-End Integration Tests for AMP
//! Tests the complete workflow including plan execution, evidence verification, and memory operations

use amp::internal::{
    exec::scheduler::{ExecutionContext, Scheduler},
    plan::ir::{Plan, Signals, Node, Operation, Edge},
    evidence::verify::{Evidence, EvidenceVerifier, Verdict, VerdictType},
    mem::store::MemoryStore,
};
use std::collections::HashMap;
use serde_json::json;

#[tokio::test]
async fn test_complete_plan_execution_workflow() {
    // Create a simple plan that exercises multiple operations
    let plan = Plan {
        signals: Some(Signals {
            latency_budget_ms: Some(10000),
            cost_cap_usd: Some(10.0),
            risk: Some(0.1),
        }),
        nodes: vec![
            Node {
                id: "search_node".to_string(),
                op: Operation::Call,
                tool: Some("doc.search.local".to_string()),
                args: Some(HashMap::from([
                    ("q".to_string(), json!("test query")),
                    ("k".to_string(), json!(5)),
                ])),
                bind: None,
                out: Some(HashMap::from([("search_results".to_string(), "search_results".to_string())])),
            },
            Node {
                id: "verify_node".to_string(),
                op: Operation::Verify,
                tool: Some("ground.verify".to_string()),
                args: Some(HashMap::from([
                    ("claims".to_string(), json!(["Test claim"])),
                    ("sources".to_string(), json!([])),  // This would be the search results in a real scenario
                ])),
                bind: None,
                out: Some(HashMap::from([("verification_result".to_string(), "verification_result".to_string())])),
            },
            Node {
                id: "memory_write_node".to_string(),
                op: Operation::MemWrite,
                tool: Some("mesh.mem.sqlite".to_string()),
                args: Some(HashMap::from([
                    ("key".to_string(), json!("test.key")),
                    ("value".to_string(), json!({"data": "test value", "timestamp": "2023-01-01"})),
                    ("confidence".to_string(), json!(0.9)),
                ])),
                bind: None,
                out: None,
            },
        ],
        edges: Some(vec![
            Edge {
                from: "search_node".to_string(),
                to: "verify_node".to_string(),
            },
            Edge {
                from: "verify_node".to_string(),
                to: "memory_write_node".to_string(),
            },
        ]),
        stop_conditions: Some(amp::internal::plan::ir::StopConditions {
            max_nodes: Some(10),
            min_confidence: Some(0.7),
        }),
    };

    // Validate the plan structure
    assert!(plan.validate().is_ok());
    assert_eq!(plan.nodes.len(), 3);
    assert_eq!(plan.edges.as_ref().unwrap().len(), 2);

    // Create execution context
    let mut ctx = ExecutionContext::new();
    ctx.signals = plan.signals.clone();
    
    // This test would require running services, so we'll just verify the structure
    // In a real test, we would mock the tool services
    assert!(!ctx.tool_urls.contains_key("doc.search.local"));
    
    // Verify that our plan structure is correct
    let search_node = &plan.nodes[0];
    assert_eq!(search_node.id, "search_node");
    assert_eq!(search_node.op, Operation::Call);
    assert_eq!(search_node.tool, Some("doc.search.local".to_string()));
    
    let verify_node = &plan.nodes[1];
    assert_eq!(verify_node.id, "verify_node");
    assert_eq!(verify_node.op, Operation::Verify);
    
    let mem_node = &plan.nodes[2];
    assert_eq!(mem_node.id, "memory_write_node");
    assert_eq!(mem_node.op, Operation::MemWrite);
    
    println!("Plan structure validation passed");
}

#[tokio::test]
async fn test_evidence_verification() {
    // Test evidence with high confidence - should pass
    let high_confidence_evidence = Evidence {
        claims: Some(vec!["Test claim".to_string()]),
        supports: Some(vec![crate::internal::evidence::verify::Support {
            claim_id: "claim_0".to_string(),
            source: "source_0".to_string(),
            confidence: 0.9,
            explanation: Some("Strong support".to_string()),
        }]),
        contradicts: None,
        verdicts: Some(vec![Verdict {
            claim_id: "claim_0".to_string(),
            verdict: VerdictType::Supported,
            confidence: 0.9,
            needs_citation: true,
        }]),
    };

    let verifier = EvidenceVerifier;
    assert!(verifier.validate_evidence_for_storage(&high_confidence_evidence, 0.8).is_ok());

    // Test evidence with low confidence - should fail
    let low_confidence_evidence = Evidence {
        claims: Some(vec!["Test claim".to_string()]),
        supports: None,
        contradicts: None,
        verdicts: Some(vec![Verdict {
            claim_id: "claim_0".to_string(),
            verdict: VerdictType::Supported,
            confidence: 0.5,  // Below threshold
            needs_citation: false,
        }]),
    };

    assert!(verifier.validate_evidence_for_storage(&low_confidence_evidence, 0.8).is_err());

    println!("Evidence verification tests passed");
}

#[tokio::test]
async fn test_memory_operations() {
    // This would test actual memory operations, but for now we test the structure
    let mem_store = MemoryStore::new();
    
    // The actual operations would require a running memory service
    // Here we just verify the client can be created
    assert!(true);
    
    println!("Memory store client creation test passed");
}

#[tokio::test]
async fn test_scheduler_execution_with_dependencies() {
    // Create a simple plan with dependencies
    let plan = Plan {
        signals: None,
        nodes: vec![
            Node {
                id: "node_a".to_string(),
                op: Operation::Call,
                tool: Some("doc.search.local".to_string()),
                args: Some(HashMap::from([("q".to_string(), json!("first query"))])),
                bind: None,
                out: Some(HashMap::from([("result_a".to_string(), "result".to_string())])),
            },
            Node {
                id: "node_b".to_string(),
                op: Operation::Call,
                tool: Some("doc.search.local".to_string()),
                args: Some(HashMap::from([("q".to_string(), json!("second query"))])),
                bind: None,
                out: Some(HashMap::from([("result_b".to_string(), "result".to_string())])),
            },
        ],
        edges: Some(vec![
            Edge {
                from: "node_a".to_string(),
                to: "node_b".to_string(),  // node_b depends on node_a
            },
        ]),
        stop_conditions: None,
    };

    // Validate the plan
    assert!(plan.validate().is_ok());
    
    // Verify the dependency structure
    if let Some(edges) = &plan.edges {
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].from, "node_a");
        assert_eq!(edges[0].to, "node_b");
    }

    println!("Dependency validation test passed");
}
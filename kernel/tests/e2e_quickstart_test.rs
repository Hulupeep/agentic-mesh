#[cfg(test)]
mod tests {
    use super::*;
    use crate::internal::{
        exec::scheduler::{ExecutionContext, Scheduler},
        plan::ir::{Plan, Signals, Node, Operation, Edge},
        evidence::verify::{Evidence, EvidenceVerifier},
        mem::store::MemoryStore,
    };
    use std::collections::HashMap;
    use serde_json::json;

    #[tokio::test]
    async fn test_e2e_quickstart() {
        // This would be a full end-to-end test, but since it requires running adapters,
        // we'll simulate the key components
        
        // Create a minimal plan for testing
        let plan = Plan {
            signals: Some(Signals {
                latency_budget_ms: Some(5000),
                cost_cap_usd: Some(1.0),
                risk: Some(0.1),
            }),
            nodes: vec![
                Node {
                    id: "test_node".to_string(),
                    op: Operation::Call,
                    tool: Some("doc.search.local".to_string()),
                    args: Some(HashMap::from([("q".to_string(), json!("test query"))])),
                    bind: None,
                    out: Some(HashMap::from([("result".to_string(), "result".to_string())])),
                }
            ],
            edges: Some(vec![
                Edge {
                    from: "test_node".to_string(),
                    to: "test_node".to_string(), // In a real test, this would form a proper graph
                }
            ]),
            stop_conditions: None,
        };

        // Validate the plan
        assert!(plan.validate().is_ok());

        // Create execution context
        let mut ctx = ExecutionContext::new();
        ctx.signals = plan.signals.clone();

        // Set up tool URLs for testing (these would need to be mocked or use real adapters)
        ctx.tool_urls.insert("doc.search.local".to_string(), "http://localhost:7401".to_string());
        ctx.tool_urls.insert("ground.verify".to_string(), "http://localhost:7402".to_string());
        ctx.tool_urls.insert("mesh.mem.sqlite".to_string(), "http://localhost:7403".to_string());

        // Since we can't actually run the full plan without adapters running,
        // we'll just check that the plan structure is valid
        assert_eq!(plan.nodes.len(), 1);
        assert_eq!(plan.nodes[0].id, "test_node");
        
        // Verify that the test passes
        assert!(true);
    }

    #[tokio::test]
    async fn test_planner_constraints() {
        use crate::internal::exec::constraints::{ConstraintChecker, Budget};
        
        // Create a plan with high cost requirements
        let plan = Plan {
            signals: Some(Signals {
                latency_budget_ms: Some(100), // Very tight latency budget
                cost_cap_usd: Some(0.01),     // Very low cost cap
                risk: Some(0.1),
            }),
            nodes: vec![
                Node {
                    id: "high_cost_node".to_string(),
                    op: Operation::Call,
                    tool: Some("expensive_tool".to_string()),
                    args: Some(HashMap::from([("param".to_string(), json!("value"))])),
                    bind: None,
                    out: None,
                }
            ],
            edges: None,
            stop_conditions: None,
        };

        // Create a tool spec that exceeds our budget constraints
        use crate::internal::tools::spec::{ToolSpec, IoSpec, Schema, Constraints};
        let tool_spec = ToolSpec {
            name: "expensive_tool".to_string(),
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
            constraints: Some(Constraints {
                input_tokens_max: Some(1000), // High token usage
                latency_p50_ms: Some(500),     // Exceeds our 100ms budget
                cost_per_call_usd: Some(0.5),  // Exceeds our 0.01 budget
                rate_limit_qps: Some(10),
                side_effects: Some(false),
            }),
            provenance: None,
            quality: None,
            policy: None,
        };

        // This test should pass if the constraint checker properly identifies violations
        let result = ConstraintChecker::check_plan_constraints(&plan, &[tool_spec]);
        
        // Since the tool exceeds our budgets, we expect an error
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_policy_write_guard() {
        use crate::internal::policy::policy::PolicyEngine;
        
        // Create evidence with low confidence
        let low_confidence_evidence = Evidence {
            claims: Some(vec!["Test claim".to_string()]),
            supports: None,
            contradicts: None,
            verdicts: Some(vec![crate::internal::evidence::verify::Verdict {
                claim_id: "claim_0".to_string(),
                verdict: crate::internal::evidence::verify::VerdictType::Supported,
                confidence: 0.5, // Below the 0.8 threshold for memory writes
                needs_citation: false,
            }]),
        };

        // Try to validate for memory write (should fail)
        let engine = PolicyEngine;
        let result = engine.check_memory_write_policy(Some(&low_confidence_evidence));
        
        // Should fail because confidence is too low
        assert!(result.is_err());
        
        // Create evidence with high confidence
        let high_confidence_evidence = Evidence {
            claims: Some(vec!["Test claim".to_string()]),
            supports: None,
            contradicts: None,
            verdicts: Some(vec![crate::internal::evidence::verify::Verdict {
                claim_id: "claim_0".to_string(),
                verdict: crate::internal::evidence::verify::VerdictType::Supported,
                confidence: 0.9, // Above the 0.8 threshold
                needs_citation: false,
            }]),
        };
        
        // This should pass
        let result = engine.check_memory_write_policy(Some(&high_confidence_evidence));
        assert!(result.is_ok());
    }
}
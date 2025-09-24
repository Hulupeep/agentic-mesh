#[cfg(test)]
mod tests {
    use super::*;
    use crate::internal::{
        exec::scheduler::{ExecutionContext, Scheduler},
        plan::ir::{Plan, Signals, Node, Operation},
        exec::constraints::{ConstraintChecker, Budget},
    };
    use std::collections::HashMap;
    use serde_json::json;

    #[tokio::test]
    async fn test_planner_constraints_with_low_budget() {
        // Create a plan with default parameters
        let plan = Plan {
            signals: Some(Signals {
                latency_budget_ms: Some(10),   // Very tight latency budget
                cost_cap_usd: Some(0.001),     // Very low cost cap
                risk: Some(0.1),
            }),
            nodes: vec![
                Node {
                    id: "search_node".to_string(),
                    op: Operation::Call,
                    tool: Some("doc.search.local".to_string()),
                    args: Some(HashMap::from([("q".to_string(), json!("refund policy"))])),
                    bind: None,
                    out: None,
                }
            ],
            edges: None,
            stop_conditions: None,
        };

        // Create tool specs with constraints that exceed our budget
        use crate::internal::tools::spec::{ToolSpec, IoSpec, Schema, Constraints};
        let tool_spec = ToolSpec {
            name: "doc.search.local".to_string(),
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
                input_tokens_max: Some(500),
                latency_p50_ms: Some(100),      // Exceeds our 10ms budget
                cost_per_call_usd: Some(0.01),  // Exceeds our 0.001 budget
                rate_limit_qps: Some(50),
                side_effects: Some(false),
            }),
            provenance: Some(crate::internal::tools::spec::Provenance {
                attribution_required: Some(true),
            }),
            quality: None,
            policy: None,
        };

        // Check constraints - this should fail
        let result = ConstraintChecker::check_plan_constraints(&plan, &[tool_spec]);
        assert!(result.is_err());

        // The error should be either latency or cost budget exceeded
        match result {
            Err(e) => {
                println!("Expected constraint error: {:?}", e);
                // Check that it's the expected kind of error
                assert!(e.to_string().contains("budget"));
            },
            Ok(_) => panic!("Expected constraint check to fail"),
        }
    }

    #[test]
    fn test_plan_validation() {
        use crate::internal::plan::ir::{PlanValidationError};
        
        // Test empty plan
        let empty_plan = Plan {
            signals: None,
            nodes: vec![],
            edges: None,
            stop_conditions: None,
        };
        
        assert!(matches!(empty_plan.validate(), Err(PlanValidationError::EmptyPlan)));
        
        // Test plan with duplicate nodes
        let plan_with_duplicates = Plan {
            signals: None,
            nodes: vec![
                Node {
                    id: "node1".to_string(),
                    op: Operation::Call,
                    tool: Some("tool1".to_string()),
                    args: None,
                    bind: None,
                    out: None,
                },
                Node {
                    id: "node1".to_string(),  // Duplicate ID
                    op: Operation::Call,
                    tool: Some("tool2".to_string()),
                    args: None,
                    bind: None,
                    out: None,
                }
            ],
            edges: None,
            stop_conditions: None,
        };
        
        assert!(matches!(plan_with_duplicates.validate(), Err(PlanValidationError::DuplicateNodeId(_))));
    }
}
use amp::internal::{
    exec::constraints::ConstraintChecker,
    plan::ir::{Node, Operation, Plan, PlanValidationError, Signals},
    tools::spec::{Constraints, IoSpec, Provenance, Schema, ToolSpec},
};
use serde_json::json;
use std::collections::HashMap;

#[tokio::test]
async fn test_planner_constraints_with_low_budget() {
    let plan = Plan {
        signals: Some(Signals {
            latency_budget_ms: Some(10),
            cost_cap_usd: Some(0.001),
            risk: Some(0.1),
        }),
        nodes: vec![Node {
            id: "search_node".to_string(),
            op: Operation::Call,
            tool: Some("doc.search.local".to_string()),
            capability: None,
            args: Some(HashMap::from([("q".to_string(), json!("refund policy"))])),
            bind: None,
            out: None,
        }],
        edges: None,
        stop_conditions: None,
    };

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
        capabilities: None,
        constraints: Some(Constraints {
            input_tokens_max: Some(500),
            latency_p50_ms: Some(100),
            cost_per_call_usd: Some(0.01),
            rate_limit_qps: Some(50),
            side_effects: Some(false),
        }),
        provenance: Some(Provenance {
            attribution_required: Some(true),
        }),
        quality: None,
        policy: None,
    };

    let result = ConstraintChecker::check_plan_constraints(&plan, &[tool_spec]);
    assert!(result.is_err());
}

#[test]
fn test_plan_validation() {
    let empty_plan = Plan {
        signals: None,
        nodes: vec![],
        edges: None,
        stop_conditions: None,
    };
    assert!(matches!(
        empty_plan.validate(),
        Err(PlanValidationError::EmptyPlan)
    ));

    let plan_with_duplicates = Plan {
        signals: None,
        nodes: vec![
            Node {
                id: "node1".to_string(),
                op: Operation::Call,
                tool: Some("tool1".to_string()),
                capability: None,
                args: None,
                bind: None,
                out: None,
            },
            Node {
                id: "node1".to_string(),
                op: Operation::Call,
                tool: Some("tool2".to_string()),
                capability: None,
                args: None,
                bind: None,
                out: None,
            },
        ],
        edges: None,
        stop_conditions: None,
    };
    assert!(matches!(
        plan_with_duplicates.validate(),
        Err(PlanValidationError::DuplicateNodeId(_))
    ));
}

#[test]
fn test_plan_validate_with_tools_checks() {
    let plan = Plan {
        signals: None,
        nodes: vec![Node {
            id: "node1".to_string(),
            op: Operation::Call,
            tool: Some("doc.search.local".to_string()),
            capability: None,
            args: None,
            bind: None,
            out: Some(HashMap::from([(
                "result".to_string(),
                "result".to_string(),
            )])),
        }],
        edges: None,
        stop_conditions: None,
    };

    let tools = vec!["doc.search.local".to_string()];
    assert!(plan.validate_with_tools(&tools).is_ok());

    let missing_tool_plan = Plan {
        signals: None,
        nodes: vec![Node {
            id: "node1".to_string(),
            op: Operation::Call,
            tool: None,
            capability: None,
            args: None,
            bind: None,
            out: Some(HashMap::from([(
                "result".to_string(),
                "result".to_string(),
            )])),
        }],
        edges: None,
        stop_conditions: None,
    };
    assert!(matches!(
        missing_tool_plan.validate_with_tools(&tools),
        Err(PlanValidationError::MissingToolOrCapability(node)) if node == "node1"
    ));

    assert!(matches!(
        plan.validate_with_tools(&["other.tool".to_string()]),
        Err(PlanValidationError::UnknownTool(tool)) if tool == "doc.search.local"
    ));

    let missing_out_plan = Plan {
        signals: None,
        nodes: vec![Node {
            id: "node1".to_string(),
            op: Operation::Verify,
            tool: Some("doc.search.local".to_string()),
            capability: None,
            args: None,
            bind: None,
            out: None,
        }],
        edges: None,
        stop_conditions: None,
    };
    assert!(matches!(
        missing_out_plan.validate_with_tools(&tools),
        Err(PlanValidationError::MissingOutputBinding(node)) if node == "node1"
    ));
}

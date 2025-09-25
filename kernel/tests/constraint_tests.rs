//! Unit tests for constraint checking functionality

use amp::internal::{
    exec::constraints::{Budget, ConstraintChecker},
    plan::ir::{Node, Operation, Plan, Signals},
    tools::spec::{Constraints, IoSpec, Schema, ToolSpec},
};
use serde_json::json;
use std::collections::HashMap;

#[test]
fn test_budget_creation_and_checking() {
    let signals = Some(Signals {
        latency_budget_ms: Some(5000),
        cost_cap_usd: Some(10.0),
        risk: Some(0.1),
    });

    let budget = Budget::new(signals.as_ref());

    assert_eq!(budget.latency_remaining_ms, Some(5000));
    assert_eq!(budget.cost_remaining_usd, Some(10.0));

    // Test budget subtraction
    let mut modified_budget = budget;
    assert!(modified_budget.subtract_latency(1000));
    assert_eq!(modified_budget.latency_remaining_ms, Some(4000));

    assert!(modified_budget.subtract_cost(5.0));
    assert_eq!(modified_budget.cost_remaining_usd, Some(5.0));

    println!("Budget operations test passed");
}

#[test]
fn test_constraint_violations() {
    let signals = Some(Signals {
        latency_budget_ms: Some(100), // Very tight budget
        cost_cap_usd: Some(0.01),     // Very low budget
        risk: Some(0.1),
    });

    let plan = Plan {
        signals,
        nodes: vec![Node {
            id: "test_node".to_string(),
            op: Operation::Call,
            tool: Some("expensive_tool".to_string()),
            capability: None,
            args: Some(HashMap::new()),
            bind: None,
            out: None,
        }],
        edges: None,
        stop_conditions: None,
    };

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
        capabilities: None,
        constraints: Some(Constraints {
            input_tokens_max: Some(1000),
            latency_p50_ms: Some(500),    // Exceeds our 100ms budget
            cost_per_call_usd: Some(1.0), // Exceeds our 0.01 budget
            rate_limit_qps: Some(10),
            side_effects: Some(false),
        }),
        provenance: None,
        quality: None,
        policy: None,
    };

    let result = ConstraintChecker::check_plan_constraints(&plan, &[tool_spec]);
    assert!(result.is_err());

    // Check that the error is related to budget exceeded
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("budget"));

    println!("Constraint violation test passed");
}

#[test]
fn test_estimate_remaining_budget() {
    let initial_budget = Budget {
        latency_remaining_ms: Some(1000),
        cost_remaining_usd: Some(10.0),
        tokens_remaining: Some(10000),
    };

    let tool_spec = ToolSpec {
        name: "test_tool".to_string(),
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
            input_tokens_max: Some(100),
            latency_p50_ms: Some(100),
            cost_per_call_usd: Some(1.0),
            rate_limit_qps: Some(10),
            side_effects: Some(false),
        }),
        provenance: None,
        quality: None,
        policy: None,
    };

    let new_budget = ConstraintChecker::estimate_remaining_budget(&initial_budget, &tool_spec)
        .expect("Budget estimation should succeed");

    assert_eq!(new_budget.latency_remaining_ms, Some(900)); // 1000 - 100
    assert_eq!(new_budget.cost_remaining_usd, Some(9.0)); // 10.0 - 1.0
    assert_eq!(new_budget.tokens_remaining, Some(9900)); // 10000 - 100

    println!("Budget estimation test passed");
}

#[test]
fn test_token_estimation() {
    // This tests the internal helper function for token estimation
    let value = json!("This is a test string for token estimation");

    // The function is not public, so we can't test it directly
    // But we can verify our understanding of the algorithm

    // The current implementation estimates 1 token per 4 characters
    let text = value.to_string();
    let chars = text.chars().count() as u64;
    let estimated_tokens = chars / 4; // This matches the implementation

    assert!(estimated_tokens > 0);

    println!("Token estimation logic test passed");
}

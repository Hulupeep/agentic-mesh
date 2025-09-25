//! CLI functionality tests

use amp::internal::{
    exec::scheduler::ExecutionContext,
    plan::ir::{Node, Operation, Plan, Signals},
};
use serde_json::json;
use std::collections::HashMap;

#[tokio::test]
async fn test_plan_execution_with_variables() {
    // Create a simple plan that exercises variable resolution
    let plan = Plan {
        signals: Some(Signals {
            latency_budget_ms: Some(5000),
            cost_cap_usd: Some(1.0),
            risk: Some(0.1),
        }),
        nodes: vec![Node {
            id: "initial_node".to_string(),
            op: Operation::Call,
            tool: Some("doc.search.local".to_string()),
            capability: None,
            args: Some(HashMap::from([("q".to_string(), json!("initial query"))])),
            bind: None,
            out: Some(HashMap::from([(
                "query_result".to_string(),
                "result".to_string(),
            )])),
        }],
        edges: None,
        stop_conditions: None,
    };

    // Create execution context with initial variables
    let mut ctx = ExecutionContext::new();
    ctx.signals = plan.signals.clone();

    // Add a tool URL so the execution can proceed (though it will fail without a real service)
    ctx.tool_urls.insert(
        "doc.search.local".to_string(),
        "http://localhost:7401".to_string(),
    );

    // Validate plan structure
    assert!(plan.validate().is_ok());
    assert_eq!(plan.nodes.len(), 1);

    // Test variable resolution
    let node = &plan.nodes[0];
    if let Some(ref out_map) = node.out {
        for (var_name, _result_path) in out_map {
            // After execution, this variable should be populated (in a real scenario)
            assert_eq!(var_name, "query_result");
        }
    }

    // For this test, we won't actually execute the plan since it requires real services
    // but we verify that the structure is correct
    assert_eq!(node.id, "initial_node");
    assert_eq!(node.op, Operation::Call);

    println!("Plan execution structure test passed");
}

#[tokio::test]
async fn test_context_variable_resolution() {
    let mut ctx = ExecutionContext::new();

    // Add some variables to context
    ctx.variables
        .insert("query_var".to_string(), json!("test query"));
    ctx.variables.insert("limit_var".to_string(), json!(10));

    // Create args with variable references
    let args_with_refs = HashMap::from([
        ("q".to_string(), json!("$query_var")),
        ("limit".to_string(), json!("$limit_var")),
        ("literal".to_string(), json!("not_a_variable")),
    ]);

    // Resolve the arguments
    let resolved = ctx.resolve_args(Some(&args_with_refs));

    if let Some(serde_json::Value::Object(resolved_map)) = resolved {
        // Check that variables were resolved
        assert_eq!(
            resolved_map.get("q").unwrap().as_str().unwrap(),
            "test query"
        );
        assert_eq!(resolved_map.get("limit").unwrap().as_i64().unwrap(), 10);
        assert_eq!(
            resolved_map.get("literal").unwrap().as_str().unwrap(),
            "not_a_variable"
        );
    } else {
        panic!("Resolved args should be an object");
    }

    // Test with non-existent variable (should use literal)
    let args_with_bad_ref = HashMap::from([("missing".to_string(), json!("$nonexistent_var"))]);

    let resolved_bad = ctx.resolve_args(Some(&args_with_bad_ref));
    if let Some(serde_json::Value::Object(resolved_map)) = resolved_bad {
        // Non-existent variable should be preserved as literal string
        assert_eq!(
            resolved_map.get("missing").unwrap().as_str().unwrap(),
            "$nonexistent_var"
        );
    } else {
        panic!("Resolved args should be an object");
    }

    println!("Variable resolution test passed");
}

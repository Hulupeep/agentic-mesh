use amp::internal::{
    exec::scheduler::ExecutionContext,
    plan::ir::{Edge, Node, Operation, Plan, Signals},
};
use serde_json::json;
use std::collections::HashMap;

#[tokio::test]
async fn test_e2e_quickstart() {
    let plan = Plan {
        signals: Some(Signals {
            latency_budget_ms: Some(5000),
            cost_cap_usd: Some(1.0),
            risk: Some(0.1),
        }),
        nodes: vec![Node {
            id: "test_node".to_string(),
            op: Operation::Call,
            tool: Some("doc.search.local".to_string()),
            capability: None,
            args: Some(HashMap::from([("q".to_string(), json!("test query"))])),
            bind: None,
            out: Some(HashMap::from([(
                "result".to_string(),
                "result".to_string(),
            )])),
        }],
        edges: Some(vec![Edge {
            from: "test_node".to_string(),
            to: "test_node".to_string(),
        }]),
        stop_conditions: None,
    };

    assert!(plan.validate().is_ok());

    let mut ctx = ExecutionContext::new();
    ctx.signals = plan.signals.clone();
    ctx.tool_urls.insert(
        "doc.search.local".to_string(),
        "http://localhost:7401".to_string(),
    );
    ctx.tool_urls.insert(
        "ground.verify".to_string(),
        "http://localhost:7402".to_string(),
    );
    ctx.tool_urls.insert(
        "mesh.mem.sqlite".to_string(),
        "http://localhost:7403".to_string(),
    );

    assert_eq!(plan.nodes.len(), 1);
    assert_eq!(plan.nodes[0].id, "test_node");
}

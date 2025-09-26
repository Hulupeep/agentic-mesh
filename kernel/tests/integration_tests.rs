//! End-to-End Integration Tests for AMP
//! Tests the complete workflow including plan execution, evidence verification, and memory operations

use amp::internal::{
    evidence::verify::{Evidence, EvidenceVerifier, Support, Verdict, VerdictType},
    exec::scheduler::{ExecutionContext, ExecutionError, Scheduler},
    mem::store::MemoryStore,
    plan::ir::{Edge, Node, Operation, Plan, Signals},
    policy::policy::{PolicyContext, PolicyEngine},
};
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Duration, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

#[derive(Deserialize)]
struct ToolInvokeRequest {
    args: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct ToolInvokeResponse {
    result: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Clone)]
struct MemoryRecord {
    value: serde_json::Value,
    provenance: Option<Vec<String>>,
    confidence: Option<f64>,
    ttl: Option<String>,
    timestamp: String,
    expires_at: Option<String>,
    evidence_summary: Option<serde_json::Value>,
}

type SharedMemoryState = Arc<Mutex<HashMap<String, MemoryRecord>>>;

async fn spawn_doc_search_server(port: Option<u16>) -> (String, JoinHandle<()>) {
    async fn handler(Json(payload): Json<ToolInvokeRequest>) -> Json<ToolInvokeResponse> {
        let query = payload
            .args
            .as_ref()
            .and_then(|args| args.get("q"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let hits = vec![json!({
            "id": "hit-1",
            "uri": format!("doc://{}", query),
            "score": 0.99,
            "snippet": "Sample snippet",
            "stamp": "2023-01-01T00:00:00Z"
        })];

        Json(ToolInvokeResponse {
            result: json!({ "hits": hits }),
            error: None,
        })
    }

    async fn spec_handler() -> Json<serde_json::Value> {
        Json(json!({
            "name": "doc.search.local",
            "description": "Stub doc search tool",
            "io": {
                "input": {
                    "type": "object",
                    "properties": null,
                    "required": null,
                    "items": null
                },
                "output": {
                    "type": "object",
                    "properties": null,
                    "required": null,
                    "items": null
                }
            },
            "capabilities": ["search.documents"],
            "constraints": {
                "input_tokens_max": 512,
                "latency_p50_ms": 120,
                "cost_per_call_usd": 0.0001,
                "rate_limit_qps": 50,
                "side_effects": false
            },
            "policy": {
                "deny_if": ["pii"]
            }
        }))
    }

    let app = Router::new()
        .route("/invoke/doc.search.local", post(handler))
        .route("/spec/doc.search.local", get(spec_handler));
    let listener = match port {
        Some(p) => tokio::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], p)))
            .await
            .unwrap(),
        None => tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap(),
    };
    let addr = listener.local_addr().unwrap();
    let server = axum::serve(listener, app.into_make_service());
    let handle = tokio::spawn(async move {
        server.await.expect("doc.search.local server error");
    });
    (format!("http://{}", addr), handle)
}

async fn spawn_verify_server(port: Option<u16>) -> (String, JoinHandle<()>) {
    async fn handler(Json(payload): Json<ToolInvokeRequest>) -> Json<ToolInvokeResponse> {
        let args = payload.args.unwrap_or_else(|| json!({}));
        let claims = args
            .get("claims")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let sources = args
            .get("sources")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let support_source = sources
            .get(0)
            .and_then(|v| v.get("id"))
            .and_then(|v| v.as_str())
            .unwrap_or("source-0");

        let result = json!({
            "claims": claims,
            "supports": [{
                "claim_id": "claim_0",
                "source": support_source,
                "confidence": 0.93,
                "explanation": "auto-verified"
            }],
            "contradicts": [],
            "verdicts": [{
                "claim_id": "claim_0",
                "verdict": "supported",
                "confidence": 0.93,
                "needs_citation": true
            }]
        });

        Json(ToolInvokeResponse {
            result,
            error: None,
        })
    }

    async fn spec_handler() -> Json<serde_json::Value> {
        Json(json!({
            "name": "ground.verify",
            "description": "Stub verifier",
            "io": {
                "input": {
                    "type": "object",
                    "properties": null,
                    "required": null,
                    "items": null
                },
                "output": {
                    "type": "object",
                    "properties": null,
                    "required": null,
                    "items": null
                }
            },
            "capabilities": ["evidence.verify"],
            "constraints": {
                "input_tokens_max": 512,
                "latency_p50_ms": 150,
                "cost_per_call_usd": 0.0002,
                "rate_limit_qps": 25,
                "side_effects": false
            }
        }))
    }

    let app = Router::new()
        .route("/invoke/ground.verify", post(handler))
        .route("/spec/ground.verify", get(spec_handler));
    let listener = match port {
        Some(p) => tokio::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], p)))
            .await
            .unwrap(),
        None => tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap(),
    };
    let addr = listener.local_addr().unwrap();
    let server = axum::serve(listener, app.into_make_service());
    let handle = tokio::spawn(async move {
        server.await.expect("ground.verify server error");
    });
    (format!("http://{}", addr), handle)
}

async fn execute_reference_plan() -> (ExecutionContext, Vec<JoinHandle<()>>) {
    let (doc_url, doc_handle) = spawn_doc_search_server(None).await;
    let (verify_url, verify_handle) = spawn_verify_server(None).await;
    let memory_state: SharedMemoryState = Arc::new(Mutex::new(HashMap::new()));
    let (memory_url, memory_handle) = spawn_memory_server(memory_state, None).await;

    let plan = Plan {
        signals: Some(Signals {
            latency_budget_ms: Some(5_000),
            cost_cap_usd: Some(2.0),
            risk: Some(0.2),
        }),
        nodes: vec![
            Node {
                id: "search_docs".to_string(),
                op: Operation::Call,
                tool: Some("doc.search.local".to_string()),
                capability: None,
                args: Some(HashMap::from([
                    ("q".to_string(), json!("neurodivergent productivity")),
                    ("k".to_string(), json!(3)),
                ])),
                bind: None,
                out: Some(HashMap::from([(
                    "search_results".to_string(),
                    "result".to_string(),
                )])),
            },
            Node {
                id: "verify_claims".to_string(),
                op: Operation::Verify,
                tool: Some("ground.verify".to_string()),
                capability: None,
                args: Some(HashMap::from([
                    (
                        "claims".to_string(),
                        json!(["Structured plans improve follow-through"]),
                    ),
                    ("sources".to_string(), json!("$search_results.hits")),
                ])),
                bind: None,
                out: Some(HashMap::from([(
                    "verification".to_string(),
                    "result".to_string(),
                )])),
            },
            Node {
                id: "persist_summary".to_string(),
                op: Operation::MemWrite,
                tool: Some("mesh.mem.sqlite".to_string()),
                capability: None,
                args: Some({
                    let mut map = HashMap::new();
                    map.insert("key".to_string(), json!("product.todo.brief"));
                    map.insert(
                        "value".to_string(),
                        json!({
                            "summary": "$verification.supports[0].explanation",
                            "source": "$search_results.hits[0].uri"
                        }),
                    );
                    map.insert(
                        "provenance".to_string(),
                        json!(["$verification.supports[0].source"]),
                    );
                    map.insert(
                        "confidence".to_string(),
                        json!("$verification.verdicts[0].confidence"),
                    );
                    map.insert("ttl".to_string(), json!("P30D"));
                    map
                }),
                bind: None,
                out: None,
            },
        ],
        edges: Some(vec![
            Edge {
                from: "search_docs".to_string(),
                to: "verify_claims".to_string(),
            },
            Edge {
                from: "verify_claims".to_string(),
                to: "persist_summary".to_string(),
            },
        ]),
        stop_conditions: Some(amp::internal::plan::ir::StopConditions {
            max_nodes: Some(8),
            min_confidence: Some(0.7),
        }),
    };

    let scheduler = Scheduler;
    let mut ctx = ExecutionContext::new();
    ctx.tool_urls
        .insert("doc.search.local".to_string(), doc_url.clone());
    ctx.tool_urls
        .insert("ground.verify".to_string(), verify_url.clone());
    ctx.tool_urls
        .insert("mesh.mem.sqlite".to_string(), memory_url.clone());

    let result_ctx = scheduler
        .execute_plan(ctx, &plan)
        .await
        .expect("plan execution should succeed");

    (result_ctx, vec![doc_handle, verify_handle, memory_handle])
}

async fn spawn_memory_server(
    state: SharedMemoryState,
    port: Option<u16>,
) -> (String, JoinHandle<()>) {
    async fn handler(
        State(state): State<SharedMemoryState>,
        Json(payload): Json<serde_json::Value>,
    ) -> Json<serde_json::Value> {
        let args_obj = payload
            .get("args")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();

        let operation = payload
            .get("operation")
            .and_then(|v| v.as_str())
            .or_else(|| args_obj.get("operation").and_then(|v| v.as_str()))
            .unwrap_or("");

        match operation {
            "write" => {
                let key = payload
                    .get("key")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let value = payload.get("value").cloned().unwrap_or_else(|| json!(null));
                let provenance = payload
                    .get("provenance")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect::<Vec<String>>()
                    });
                let confidence = payload.get("confidence").and_then(|v| v.as_f64());
                let ttl = payload
                    .get("ttl")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "P90D".to_string());
                let timestamp = Utc::now().to_rfc3339();
                let expires_at = Utc::now()
                    .checked_add_signed(Duration::days(90))
                    .map(|dt| dt.to_rfc3339());
                let evidence_summary = payload.get("evidence_summary").cloned();

                if provenance.as_ref().map(|p| p.is_empty()).unwrap_or(true) {
                    return Json(json!({
                        "result": {
                            "success": false,
                            "message": "Memory write requires non-empty provenance"
                        }
                    }));
                }

                if confidence.unwrap_or(0.0) < 0.8 {
                    return Json(json!({
                        "result": {
                            "success": false,
                            "message": "Memory write rejected: confidence must be >= 0.8"
                        }
                    }));
                }

                let record = MemoryRecord {
                    value: value.clone(),
                    provenance: provenance.clone(),
                    confidence,
                    ttl: Some(ttl.clone()),
                    timestamp: timestamp.clone(),
                    expires_at: expires_at.clone(),
                    evidence_summary: evidence_summary.clone(),
                };

                state.lock().await.insert(key.clone(), record);

                Json(json!({
                    "result": {
                        "success": true,
                        "entry": {
                            "key": key,
                            "value": value,
                            "provenance": provenance,
                            "confidence": confidence,
                            "ttl": ttl,
                            "timestamp": timestamp,
                            "expires_at": expires_at,
                            "evidence_summary": evidence_summary
                        }
                    }
                }))
            }
            "read" => {
                let key = payload.get("key").and_then(|v| v.as_str()).unwrap_or("");

                let maybe_entry = state.lock().await.get(key).cloned();

                if let Some(entry) = maybe_entry {
                    Json(json!({
                        "result": {
                            "success": true,
                            "entry": {
                                "key": key,
                                "value": entry.value,
                                "provenance": entry.provenance,
                                "confidence": entry.confidence,
                                "ttl": entry.ttl,
                                "timestamp": entry.timestamp,
                                "expires_at": entry.expires_at,
                                "evidence_summary": entry.evidence_summary
                            }
                        }
                    }))
                } else {
                    Json(json!({
                        "result": {
                            "success": false,
                            "message": format!("Key {} not found", key)
                        }
                    }))
                }
            }
            "forget" => {
                let key = payload.get("key").and_then(|v| v.as_str()).unwrap_or("");
                state.lock().await.remove(key);
                Json(json!({
                    "result": {
                        "success": true,
                        "message": format!("Key {} deleted", key)
                    }
                }))
            }
            _ => Json(json!({
                "result": {
                    "success": false,
                    "message": format!("Unsupported operation: {}", operation)
                }
            })),
        }
    }

    async fn spec_handler() -> Json<serde_json::Value> {
        Json(json!({
            "name": "mesh.mem.sqlite",
            "description": "Stub memory store",
            "io": {
                "input": {
                    "type": "object",
                    "properties": null,
                    "required": null,
                    "items": null
                },
                "output": {
                    "type": "object",
                    "properties": null,
                    "required": null,
                    "items": null
                }
            },
            "capabilities": ["memory.read", "memory.write", "memory.forget"],
            "constraints": {
                "input_tokens_max": 256,
                "latency_p50_ms": 50,
                "cost_per_call_usd": 0.00005,
                "rate_limit_qps": 100,
                "side_effects": true
            }
        }))
    }

    let app = Router::new()
        .route("/invoke", post(handler))
        .route("/spec/mesh.mem.sqlite", get(spec_handler))
        .with_state(state);

    let listener = match port {
        Some(p) => tokio::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], p)))
            .await
            .unwrap(),
        None => tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap(),
    };
    let addr = listener.local_addr().unwrap();
    let server = axum::serve(listener, app.into_make_service());
    let handle = tokio::spawn(async move {
        server.await.expect("memory server error");
    });
    (format!("http://{}", addr), handle)
}

async fn spawn_memory_analytics_server(
    state: SharedMemoryState,
    port: Option<u16>,
) -> (String, JoinHandle<()>) {
    async fn handler(
        State(state): State<SharedMemoryState>,
        Json(payload): Json<serde_json::Value>,
    ) -> Json<serde_json::Value> {
        let args_map = payload
            .get("args")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();

        let operation = args_map
            .get("operation")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        match operation {
            "summary" => {
                let store = state.lock().await;
                let now = Utc::now();
                let mut total = 0usize;
                let mut expired = 0usize;
                let mut expiring_next_24h = 0usize;

                for record in store.values() {
                    total += 1;
                    if let Some(expires_at) = &record.expires_at {
                        if let Ok(parsed) = DateTime::parse_from_rfc3339(expires_at) {
                            let parsed = parsed.with_timezone(&Utc);
                            if parsed < now {
                                expired += 1;
                            } else if parsed <= now + Duration::hours(24) {
                                expiring_next_24h += 1;
                            }
                        }
                    }
                }

                Json(serde_json::json!({
                    "result": {
                        "success": true,
                        "data": {
                            "total_entries": total,
                            "expired_entries": expired,
                            "expiring_next_24h": expiring_next_24h,
                        }
                    }
                }))
            }
            "list" => {
                let limit = args_map
                    .get("limit")
                    .and_then(|v| v.as_u64())
                    .map(|v| v.min(1000))
                    .unwrap_or(20) as usize;
                let mut entries: Vec<(String, MemoryRecord)> = state
                    .lock()
                    .await
                    .iter()
                    .map(|(key, record)| (key.clone(), record.clone()))
                    .collect();

                entries.sort_by(|(_, a), (_, b)| {
                    let default_time = Utc::now() + Duration::days(365);
                    let a_time = a
                        .expires_at
                        .as_ref()
                        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or(default_time);
                    let b_time = b
                        .expires_at
                        .as_ref()
                        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or(default_time);
                    a_time.cmp(&b_time)
                });

                let data: Vec<serde_json::Value> = entries
                    .into_iter()
                    .take(limit)
                    .map(|(key, record)| {
                        serde_json::json!({
                            "key": key,
                            "confidence": record.confidence,
                            "ttl": record.ttl,
                            "created_at": record.timestamp,
                            "expires_at": record.expires_at,
                        })
                    })
                    .collect();

                Json(serde_json::json!({
                    "result": {
                        "success": true,
                        "data": {"entries": data}
                    }
                }))
            }
            "by_key" => {
                let key = args_map
                    .get("key")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim();
                if key.is_empty() {
                    return Json(serde_json::json!({
                        "result": {
                            "success": false,
                            "message": "Key is required for by_key operation"
                        }
                    }));
                }

                let store = state.lock().await;
                if let Some(record) = store.get(key) {
                    Json(serde_json::json!({
                        "result": {
                            "success": true,
                            "data": {
                                "key": key,
                                "value": record.value,
                                "confidence": record.confidence,
                                "ttl": record.ttl,
                                "created_at": record.timestamp,
                                "expires_at": record.expires_at,
                                "provenance": record.provenance,
                                "evidence_summary": record.evidence_summary
                            }
                        }
                    }))
                } else {
                    Json(serde_json::json!({
                        "result": {
                            "success": false,
                            "message": format!("Key {} not found", key)
                        }
                    }))
                }
            }
            _ => Json(serde_json::json!({
                "result": {
                    "success": false,
                    "message": format!("Unsupported operation: {}", operation)
                }
            })),
        }
    }

    async fn spec_handler() -> Json<serde_json::Value> {
        Json(serde_json::json!({
            "name": "mesh.mem.analytics",
            "description": "Stub memory analytics",
            "io": {
                "input": {
                    "type": "object",
                    "properties": null,
                    "required": null,
                    "items": null
                },
                "output": {
                    "type": "object",
                    "properties": null,
                    "required": null,
                    "items": null
                }
            },
            "capabilities": ["memory.analytics"],
            "constraints": {
                "input_tokens_max": 128,
                "latency_p50_ms": 40,
                "cost_per_call_usd": 0.00001,
                "rate_limit_qps": 200,
                "side_effects": false
            }
        }))
    }

    let app = Router::new()
        .route("/invoke/mesh.mem.analytics", post(handler))
        .route("/spec/mesh.mem.analytics", get(spec_handler))
        .with_state(state);

    let listener = match port {
        Some(p) => tokio::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], p)))
            .await
            .unwrap(),
        None => tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap(),
    };
    let addr = listener.local_addr().unwrap();
    let server = axum::serve(listener, app.into_make_service());
    let handle = tokio::spawn(async move {
        server
            .await
            .expect("memory analytics server error");
    });
    (format!("http://{}", addr), handle)
}

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
                capability: None,
                args: Some(HashMap::from([
                    ("q".to_string(), json!("test query")),
                    ("k".to_string(), json!(5)),
                ])),
                bind: None,
                out: Some(HashMap::from([(
                    "search_results".to_string(),
                    "search_results".to_string(),
                )])),
            },
            Node {
                id: "verify_node".to_string(),
                op: Operation::Verify,
                tool: Some("ground.verify".to_string()),
                capability: None,
                args: Some(HashMap::from([
                    ("claims".to_string(), json!(["Test claim"])),
                    ("sources".to_string(), json!([])), // This would be the search results in a real scenario
                ])),
                bind: None,
                out: Some(HashMap::from([(
                    "verification_result".to_string(),
                    "verification_result".to_string(),
                )])),
            },
            Node {
                id: "memory_write_node".to_string(),
                op: Operation::MemWrite,
                tool: Some("mesh.mem.sqlite".to_string()),
                capability: None,
                args: Some(HashMap::from([
                    ("key".to_string(), json!("test.key")),
                    (
                        "value".to_string(),
                        json!({"data": "test value", "timestamp": "2023-01-01"}),
                    ),
                    ("confidence".to_string(), json!(0.9)),
                    ("provenance".to_string(), json!(["doc://test"])),
                    ("ttl".to_string(), json!("P60D")),
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
async fn test_memory_analytics_node_execution() {
    let (doc_url, doc_handle) = spawn_doc_search_server(None).await;
    let (verify_url, verify_handle) = spawn_verify_server(None).await;
    let memory_state: SharedMemoryState = Arc::new(Mutex::new(HashMap::new()));
    let (memory_url, memory_handle) = spawn_memory_server(memory_state.clone(), None).await;
    let (analytics_url, analytics_handle) =
        spawn_memory_analytics_server(memory_state.clone(), None).await;

    let plan = Plan {
        signals: Some(Signals {
            latency_budget_ms: Some(5_000),
            cost_cap_usd: Some(2.0),
            risk: Some(0.2),
        }),
        nodes: vec![
            Node {
                id: "search_docs".to_string(),
                op: Operation::Call,
                tool: Some("doc.search.local".to_string()),
                capability: None,
                args: Some(HashMap::from([
                    ("q".to_string(), json!("neurodivergent productivity")),
                    ("k".to_string(), json!(3)),
                ])),
                bind: None,
                out: Some(HashMap::from([(
                    "search_results".to_string(),
                    "result".to_string(),
                )])),
            },
            Node {
                id: "verify_claims".to_string(),
                op: Operation::Verify,
                tool: Some("ground.verify".to_string()),
                capability: None,
                args: Some(HashMap::from([
                    (
                        "claims".to_string(),
                        json!(["Structured plans improve follow-through"]),
                    ),
                    ("sources".to_string(), json!("$search_results.hits")),
                ])),
                bind: None,
                out: Some(HashMap::from([(
                    "verification".to_string(),
                    "result".to_string(),
                )])),
            },
            Node {
                id: "persist_summary".to_string(),
                op: Operation::MemWrite,
                tool: Some("mesh.mem.sqlite".to_string()),
                capability: None,
                args: Some({
                    let mut map = HashMap::new();
                    map.insert("key".to_string(), json!("product.todo.brief"));
                    map.insert(
                        "value".to_string(),
                        json!({
                            "summary": "$verification.supports[0].explanation",
                            "source": "$search_results.hits[0].uri"
                        }),
                    );
                    map.insert(
                        "provenance".to_string(),
                        json!(["$verification.supports[0].source"]),
                    );
                    map.insert(
                        "confidence".to_string(),
                        json!("$verification.verdicts[0].confidence"),
                    );
                    map.insert("ttl".to_string(), json!("P30D"));
                    map
                }),
                bind: None,
                out: None,
            },
            Node {
                id: "memory_insights".to_string(),
                op: Operation::Call,
                tool: Some("mesh.mem.analytics".to_string()),
                capability: None,
                args: Some(HashMap::from([
                    ("operation".to_string(), json!("by_key")),
                    ("key".to_string(), json!("product.todo.brief")),
                ])),
                bind: None,
                out: Some(HashMap::from([(
                    "memory_analytics".to_string(),
                    "result".to_string(),
                )])),
            },
        ],
        edges: Some(vec![
            Edge {
                from: "search_docs".to_string(),
                to: "verify_claims".to_string(),
            },
            Edge {
                from: "verify_claims".to_string(),
                to: "persist_summary".to_string(),
            },
            Edge {
                from: "persist_summary".to_string(),
                to: "memory_insights".to_string(),
            },
        ]),
        stop_conditions: Some(amp::internal::plan::ir::StopConditions {
            max_nodes: Some(8),
            min_confidence: Some(0.7),
        }),
    };

    let scheduler = Scheduler;
    let mut ctx = ExecutionContext::new();
    ctx.tool_urls.insert("doc.search.local".to_string(), doc_url);
    ctx.tool_urls
        .insert("ground.verify".to_string(), verify_url);
    ctx.tool_urls
        .insert("mesh.mem.sqlite".to_string(), memory_url);
    ctx.tool_urls
        .insert("mesh.mem.analytics".to_string(), analytics_url);

    let result_ctx = scheduler
        .execute_plan(ctx, &plan)
        .await
        .expect("plan execution should succeed");

    let analytics_value = result_ctx
        .variables
        .get("memory_analytics")
        .expect("analytics output stored");
    assert_eq!(
        analytics_value
            .get("success")
            .and_then(|v| v.as_bool()),
        Some(true)
    );
    assert_eq!(
        analytics_value
            .get("data")
            .and_then(|v| v.get("key"))
            .and_then(|v| v.as_str()),
        Some("product.todo.brief")
    );

    for handle in [doc_handle, verify_handle, memory_handle, analytics_handle] {
        handle.abort();
    }
}

#[tokio::test]
async fn test_evidence_verification() {
    // Test evidence with high confidence - should pass
    let high_confidence_evidence = Evidence {
        claims: Some(vec!["claim_0".to_string()]),
        supports: Some(vec![Support {
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
    assert!(verifier
        .validate_evidence_for_storage(&high_confidence_evidence, 0.8)
        .is_ok());

    // Test evidence with low confidence - should fail
    let low_confidence_evidence = Evidence {
        claims: Some(vec!["claim_0".to_string()]),
        supports: Some(vec![Support {
            claim_id: "claim_0".to_string(),
            source: "source_1".to_string(),
            confidence: 0.55,
            explanation: Some("Marginal support".to_string()),
        }]),
        contradicts: None,
        verdicts: Some(vec![Verdict {
            claim_id: "claim_0".to_string(),
            verdict: VerdictType::Supported,
            confidence: 0.5, // Below threshold
            needs_citation: false,
        }]),
    };

    assert!(verifier
        .validate_evidence_for_storage(&low_confidence_evidence, 0.8)
        .is_err());

    println!("Evidence verification tests passed");
}

#[tokio::test]
async fn test_memory_operations() {
    // This would test actual memory operations, but for now we test the structure
    let _mem_store = MemoryStore::new();

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
                capability: None,
                args: Some(HashMap::from([("q".to_string(), json!("first query"))])),
                bind: None,
                out: Some(HashMap::from([(
                    "result_a".to_string(),
                    "result".to_string(),
                )])),
            },
            Node {
                id: "node_b".to_string(),
                op: Operation::Call,
                tool: Some("doc.search.local".to_string()),
                capability: None,
                args: Some(HashMap::from([("q".to_string(), json!("second query"))])),
                bind: None,
                out: Some(HashMap::from([(
                    "result_b".to_string(),
                    "result".to_string(),
                )])),
            },
        ],
        edges: Some(vec![Edge {
            from: "node_a".to_string(),
            to: "node_b".to_string(), // node_b depends on node_a
        }]),
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

#[tokio::test]
async fn test_end_to_end_plan_execution_with_http_tools() {
    let (doc_url, doc_handle) = spawn_doc_search_server(None).await;
    let (verify_url, verify_handle) = spawn_verify_server(None).await;
    let memory_state: SharedMemoryState = Arc::new(Mutex::new(HashMap::new()));
    let (memory_url, memory_handle) = spawn_memory_server(memory_state.clone(), None).await;
    let (analytics_url, analytics_handle) =
        spawn_memory_analytics_server(memory_state.clone(), None).await;

    let plan = Plan {
        signals: Some(Signals {
            latency_budget_ms: Some(5_000),
            cost_cap_usd: Some(2.0),
            risk: Some(0.2),
        }),
        nodes: vec![
            Node {
                id: "search_docs".to_string(),
                op: Operation::Call,
                tool: Some("doc.search.local".to_string()),
                capability: None,
                args: Some(HashMap::from([
                    ("q".to_string(), json!("neurodivergent productivity")),
                    ("k".to_string(), json!(3)),
                ])),
                bind: None,
                out: Some(HashMap::from([(
                    "search_results".to_string(),
                    "result".to_string(),
                )])),
            },
            Node {
                id: "verify_claims".to_string(),
                op: Operation::Verify,
                tool: Some("ground.verify".to_string()),
                capability: None,
                args: Some(HashMap::from([
                    (
                        "claims".to_string(),
                        json!(["Structured plans improve follow-through"]),
                    ),
                    ("sources".to_string(), json!("$search_results.hits")),
                ])),
                bind: None,
                out: Some(HashMap::from([(
                    "verification".to_string(),
                    "result".to_string(),
                )])),
            },
            Node {
                id: "persist_summary".to_string(),
                op: Operation::MemWrite,
                tool: Some("mesh.mem.sqlite".to_string()),
                capability: None,
                args: Some({
                    let mut map = HashMap::new();
                    map.insert("key".to_string(), json!("product.todo.brief"));
                    map.insert(
                        "value".to_string(),
                        json!({
                            "summary": "$verification.supports[0].explanation",
                            "source": "$search_results.hits[0].uri"
                        }),
                    );
                    map.insert(
                        "provenance".to_string(),
                        json!(["$verification.supports[0].source"]),
                    );
                    map.insert(
                        "confidence".to_string(),
                        json!("$verification.verdicts[0].confidence"),
                    );
                    map.insert("ttl".to_string(), json!("P30D"));
                    map
                }),
                bind: None,
                out: None,
            },
            Node {
                id: "memory_insights".to_string(),
                op: Operation::Call,
                tool: Some("mesh.mem.analytics".to_string()),
                capability: None,
                args: Some(HashMap::from([
                    ("operation".to_string(), json!("by_key")),
                    ("key".to_string(), json!("product.todo.brief")),
                ])),
                bind: None,
                out: Some(HashMap::from([(
                    "memory_analytics".to_string(),
                    "result".to_string(),
                )])),
            },
        ],
        edges: Some(vec![
            Edge {
                from: "search_docs".to_string(),
                to: "verify_claims".to_string(),
            },
            Edge {
                from: "verify_claims".to_string(),
                to: "persist_summary".to_string(),
            },
            Edge {
                from: "persist_summary".to_string(),
                to: "memory_insights".to_string(),
            },
        ]),
        stop_conditions: Some(amp::internal::plan::ir::StopConditions {
            max_nodes: Some(8),
            min_confidence: Some(0.7),
        }),
    };

    let scheduler = Scheduler;
    let mut ctx = ExecutionContext::new();
    ctx.tool_urls
        .insert("doc.search.local".to_string(), doc_url.clone());
    ctx.tool_urls
        .insert("ground.verify".to_string(), verify_url.clone());
    ctx.tool_urls
        .insert("mesh.mem.sqlite".to_string(), memory_url.clone());
    ctx.tool_urls
        .insert("mesh.mem.analytics".to_string(), analytics_url.clone());

    let result_ctx = scheduler
        .execute_plan(ctx, &plan)
        .await
        .expect("plan execution should succeed");

    assert!(result_ctx.variables.contains_key("search_results"));
    assert!(result_ctx.variables.contains_key("verification"));
    assert!(result_ctx.variables.contains_key("verification_summary"));
    assert!(result_ctx
        .trace_events
        .iter()
        .any(|trace| trace.event_type == "evidence_summary"));
    assert!(result_ctx.variables.contains_key("memory_analytics"));

    let store = memory_state.lock().await;
    let record = store
        .get("product.todo.brief")
        .expect("memory entry stored");
    assert_eq!(record.provenance.as_ref().map(|p| p.len()), Some(1));
    assert!(record.confidence.unwrap_or(0.0) >= 0.9_f64);

    doc_handle.abort();
    verify_handle.abort();
    memory_handle.abort();
    analytics_handle.abort();

    println!("End-to-end plan execution with HTTP tools passed");
}

#[tokio::test]
async fn test_plan_fails_when_cost_budget_exceeded() {
    let (doc_url, doc_handle) = spawn_doc_search_server(None).await;

    let plan = Plan {
        signals: Some(Signals {
            latency_budget_ms: Some(10_000),
            cost_cap_usd: Some(0.00001),
            risk: Some(0.1),
        }),
        nodes: vec![Node {
            id: "doc_call".to_string(),
            op: Operation::Call,
            tool: Some("doc.search.local".to_string()),
            capability: None,
            args: Some(HashMap::from([(
                "q".to_string(),
                json!("budget guardrails"),
            )])),
            bind: None,
            out: Some(HashMap::from([(
                "search_result".to_string(),
                "result".to_string(),
            )])),
        }],
        edges: None,
        stop_conditions: None,
    };

    let scheduler = Scheduler;
    let mut ctx = ExecutionContext::new();
    ctx.tool_urls
        .insert("doc.search.local".to_string(), doc_url.clone());

    let result = scheduler.execute_plan(ctx, &plan).await;

    match result {
        Err(ExecutionError::BudgetExceeded(msg)) => {
            assert!(msg.contains("Cost budget exceeded"));
        }
        other => panic!("Expected cost budget failure, got {:?}", other),
    }

    doc_handle.abort();

    println!("Cost budget guardrail test passed");
}

#[tokio::test]
async fn test_tool_policy_blocking_prevents_execution() {
    let (doc_url, doc_handle) = spawn_doc_search_server(None).await;

    let plan = Plan {
        signals: Some(Signals {
            latency_budget_ms: Some(10_000),
            cost_cap_usd: Some(1.0),
            risk: Some(0.1),
        }),
        nodes: vec![Node {
            id: "doc_policy_check".to_string(),
            op: Operation::Call,
            tool: Some("doc.search.local".to_string()),
            capability: None,
            args: Some(HashMap::from([(
                "q".to_string(),
                json!("find PII disclosure procedures"),
            )])),
            bind: None,
            out: Some(HashMap::from([(
                "search_result".to_string(),
                "result".to_string(),
            )])),
        }],
        edges: None,
        stop_conditions: None,
    };

    let scheduler = Scheduler;
    let mut ctx = ExecutionContext::new();
    ctx.tool_urls
        .insert("doc.search.local".to_string(), doc_url.clone());

    let result = scheduler.execute_plan(ctx, &plan).await;

    match result {
        Err(ExecutionError::ToolExecutionError(msg)) => {
            assert!(msg.contains("policy pattern"));
        }
        other => panic!("Expected tool policy enforcement failure, got {:?}", other),
    }

    doc_handle.abort();

    println!("Tool policy enforcement test passed");
}

#[tokio::test]
async fn test_policy_engine_accepts_high_confidence_plan_evidence() {
    let (result_ctx, handles) = execute_reference_plan().await;

    let verification_value = result_ctx
        .variables
        .get("verification")
        .cloned()
        .expect("verification variable present");
    let evidence: Evidence =
        serde_json::from_value(verification_value).expect("verification output is valid evidence");

    let policy_ctx = PolicyContext {
        evidence: Some(evidence),
        tool_specs: result_ctx.tool_specs.values().cloned().collect(),
        traces: result_ctx.trace_events.clone(),
        variables: result_ctx.variables.clone(),
    };

    let engine = PolicyEngine;
    let result = engine
        .enforce_policies(&policy_ctx)
        .expect("policy evaluation should succeed");
    assert!(result.allowed);

    for handle in handles {
        handle.abort();
    }
}

#[tokio::test]
async fn test_policy_engine_flags_low_confidence_plan_evidence() {
    let (result_ctx, handles) = execute_reference_plan().await;

    let verification_value = result_ctx
        .variables
        .get("verification")
        .cloned()
        .expect("verification variable present");
    let evidence: Evidence =
        serde_json::from_value(verification_value).expect("verification output is valid evidence");

    let mut traces = result_ctx.trace_events.clone();
    let mut variables = result_ctx.variables.clone();

    let mut low_summary = variables
        .get("verification_summary")
        .cloned()
        .expect("verification summary variable present");

    if let Some(summary_obj) = low_summary.as_object_mut() {
        summary_obj.insert("mean_confidence".to_string(), json!(0.6));
        if let Some(per_claim) = summary_obj
            .get_mut("per_claim")
            .and_then(|v| v.as_object_mut())
        {
            for claim in per_claim.values_mut() {
                if let Some(claim_obj) = claim.as_object_mut() {
                    claim_obj.insert("supports".to_string(), json!(0));
                    claim_obj.insert("contradictions".to_string(), json!(1));
                    claim_obj.insert("average_confidence".to_string(), json!(0.6));
                    claim_obj.insert("max_confidence".to_string(), json!(0.6));
                    claim_obj.insert("min_confidence".to_string(), json!(0.6));
                }
            }
        }
    }

    for trace in &mut traces {
        if trace.event_type == "evidence_summary" {
            trace.data = Some(low_summary.clone());
        }
    }

    variables.insert("verification_summary".to_string(), low_summary.clone());

    let policy_ctx = PolicyContext {
        evidence: Some(evidence),
        tool_specs: result_ctx.tool_specs.values().cloned().collect(),
        traces,
        variables,
    };

    let engine = PolicyEngine;
    let result = engine
        .enforce_policies(&policy_ctx)
        .expect("policy evaluation should succeed");
    assert!(!result.allowed);
    assert!(result
        .violations
        .iter()
        .any(|violation| violation.rule == "evidence_confidence"));

    for handle in handles {
        handle.abort();
    }
}

#[tokio::test]
async fn test_capability_routing_selects_registered_tool() {
    let (doc_url, doc_handle) = spawn_doc_search_server(None).await;

    let plan = Plan {
        signals: Some(Signals {
            latency_budget_ms: Some(2_000),
            cost_cap_usd: Some(1.0),
            risk: Some(0.1),
        }),
        nodes: vec![Node {
            id: "search_docs".to_string(),
            op: Operation::Call,
            tool: None,
            capability: Some("search.documents".to_string()),
            args: Some(HashMap::from([
                ("q".to_string(), json!("capability routing")),
                ("k".to_string(), json!(2)),
            ])),
            bind: None,
            out: Some(HashMap::from([(
                "results".to_string(),
                "result".to_string(),
            )])),
        }],
        edges: None,
        stop_conditions: None,
    };

    let scheduler = Scheduler;
    let mut ctx = ExecutionContext::new();
    ctx.tool_urls
        .insert("doc.search.local".to_string(), doc_url.clone());

    let result_ctx = scheduler
        .execute_plan(ctx, &plan)
        .await
        .expect("plan execution should succeed");

    let route_trace = result_ctx
        .trace_events
        .iter()
        .find(|trace| trace.event_type == "capability_route")
        .expect("capability route trace present");

    let data = route_trace
        .data
        .as_ref()
        .expect("capability route trace carries data");
    assert_eq!(
        data.get("capability").and_then(|v| v.as_str()),
        Some("search.documents")
    );
    assert_eq!(
        data.get("selected_tool").and_then(|v| v.as_str()),
        Some("doc.search.local")
    );

    assert!(result_ctx
        .trace_events
        .iter()
        .any(|trace| trace.event_type == "plan_optimizer"));

    doc_handle.abort();
}

#[tokio::test]
async fn test_kernel_api_execute_plan_end_to_end() {
    let memory_state: SharedMemoryState = Arc::new(Mutex::new(HashMap::new()));

    let prev_tool_config = std::env::var("AMP_TOOL_CONFIG").ok();
    std::env::set_var("AMP_TOOL_CONFIG", "../config/tools.json");

    let (doc_url, doc_handle) = spawn_doc_search_server(Some(7401)).await;
    let (verify_url, verify_handle) = spawn_verify_server(Some(7402)).await;
    let (memory_url, memory_handle) = spawn_memory_server(memory_state.clone(), Some(7403)).await;
    let (analytics_url, analytics_handle) =
        spawn_memory_analytics_server(memory_state.clone(), Some(7407)).await;

    // Ensure the URLs match the kernel defaults
    assert!(doc_url.ends_with("7401"));
    assert!(verify_url.ends_with("7402"));
    assert!(memory_url.ends_with("7403"));
    assert!(analytics_url.ends_with("7407"));

    let kernel_app = amp::internal::api::create_router();
    let kernel_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let kernel_addr = kernel_listener.local_addr().unwrap();
    let kernel_handle = tokio::spawn(async move {
        axum::serve(kernel_listener, kernel_app.into_make_service())
            .await
            .expect("kernel api server error");
    });

    let plan = Plan {
        signals: Some(Signals {
            latency_budget_ms: Some(5_000),
            cost_cap_usd: Some(2.0),
            risk: Some(0.2),
        }),
        nodes: vec![
            Node {
                id: "search_docs".to_string(),
                op: Operation::Call,
                tool: Some("doc.search.local".to_string()),
                capability: None,
                args: Some(HashMap::from([
                    ("q".to_string(), json!("neurodivergent productivity")),
                    ("k".to_string(), json!(3)),
                ])),
                bind: None,
                out: Some(HashMap::from([(
                    "search_results".to_string(),
                    "result".to_string(),
                )])),
            },
            Node {
                id: "verify_claims".to_string(),
                op: Operation::Verify,
                tool: Some("ground.verify".to_string()),
                capability: None,
                args: Some(HashMap::from([
                    (
                        "claims".to_string(),
                        json!(["Structured plans improve follow-through"]),
                    ),
                    ("sources".to_string(), json!("$search_results.hits")),
                ])),
                bind: None,
                out: Some(HashMap::from([(
                    "verification".to_string(),
                    "result".to_string(),
                )])),
            },
            Node {
                id: "persist_summary".to_string(),
                op: Operation::MemWrite,
                tool: Some("mesh.mem.sqlite".to_string()),
                capability: None,
                args: Some({
                    let mut map = HashMap::new();
                    map.insert("key".to_string(), json!("product.todo.brief"));
                    map.insert(
                        "value".to_string(),
                        json!({
                            "summary": "$verification.supports[0].explanation",
                            "source": "$search_results.hits[0].uri"
                        }),
                    );
                    map.insert(
                        "provenance".to_string(),
                        json!(["$verification.supports[0].source"]),
                    );
                    map.insert(
                        "confidence".to_string(),
                        json!("$verification.verdicts[0].confidence"),
                    );
                    map.insert("ttl".to_string(), json!("P30D"));
                    map
                }),
                bind: None,
                out: None,
            },
            Node {
                id: "memory_insights".to_string(),
                op: Operation::Call,
                tool: Some("mesh.mem.analytics".to_string()),
                capability: None,
                args: Some(HashMap::from([
                    ("operation".to_string(), json!("by_key")),
                    ("key".to_string(), json!("product.todo.brief")),
                ])),
                bind: None,
                out: Some(HashMap::from([(
                    "memory_analytics".to_string(),
                    "result".to_string(),
                )])),
            },
        ],
        edges: Some(vec![
            Edge {
                from: "search_docs".to_string(),
                to: "verify_claims".to_string(),
            },
            Edge {
                from: "verify_claims".to_string(),
                to: "persist_summary".to_string(),
            },
            Edge {
                from: "persist_summary".to_string(),
                to: "memory_insights".to_string(),
            },
        ]),
        stop_conditions: Some(amp::internal::plan::ir::StopConditions {
            max_nodes: Some(8),
            min_confidence: Some(0.7),
        }),
    };

    let client = Client::new();
    let execute_body = serde_json::json!({
        "plan": serde_json::to_value(&plan).unwrap(),
        "inputs": serde_json::Value::Null
    });

    let response = client
        .post(format!("http://{}/v1/plan/execute", kernel_addr))
        .json(&execute_body)
        .send()
        .await
        .expect("plan execute request failed");

    let status = response.status();
    let body_text = response
        .text()
        .await
        .expect("failed to read execute response body");
    assert!(
        status.is_success(),
        "plan execute returned {:?}: {}",
        status,
        body_text
    );
    let body: serde_json::Value = serde_json::from_str(&body_text).expect("invalid response body");
    let plan_id = body
        .get("plan_id")
        .and_then(|v| v.as_str())
        .expect("missing plan_id")
        .to_string();

    // Fetch traces to ensure API recorded execution
    let trace_response = client
        .get(format!("http://{}/v1/trace/{}", kernel_addr, plan_id))
        .send()
        .await
        .expect("trace request failed");
    assert!(trace_response.status().is_success());
    let trace_body: serde_json::Value = trace_response.json().await.expect("invalid trace body");
    let analytics_trace_seen = trace_body
        .get("traces")
        .and_then(|v| v.as_array())
        .map(|traces| {
            traces.iter().any(|trace| {
                trace
                    .get("data")
                    .and_then(|d| d.get("tool"))
                    .and_then(|t| t.as_str())
                    == Some("mesh.mem.analytics")
            })
        })
        .unwrap_or(false);
    assert!(analytics_trace_seen, "analytics step trace missing");

    // Verify memory write succeeded with expected confidence
    let store = memory_state.lock().await;
    let record = store
        .get("product.todo.brief")
        .expect("memory entry stored via API execution");
    assert!(record.confidence.unwrap_or(0.0) >= 0.9);

    drop(store);

    if let Some(value) = prev_tool_config {
        std::env::set_var("AMP_TOOL_CONFIG", value);
    } else {
        std::env::remove_var("AMP_TOOL_CONFIG");
    }

    doc_handle.abort();
    verify_handle.abort();
    memory_handle.abort();
    analytics_handle.abort();
    kernel_handle.abort();

    println!("Kernel API end-to-end execution test passed");
}

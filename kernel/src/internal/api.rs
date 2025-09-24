use axum::{
    extract::{Path, State, Json},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::internal::{
    exec::scheduler::{ExecutionContext, Scheduler},
    plan::ir::Plan,
    trace::trace::Trace,
};

// State to hold execution context and traces
#[derive(Clone)]
pub struct AppState {
    pub exec_context: Arc<RwLock<ExecutionContext>>,
    pub plans: Arc<RwLock<std::collections::HashMap<String, Plan>>>,
    pub plan_traces: Arc<RwLock<std::collections::HashMap<String, Vec<Trace>>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            exec_context: Arc::new(RwLock::new(ExecutionContext::new())),
            plans: Arc::new(RwLock::new(std::collections::HashMap::new())),
            plan_traces: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }
}

pub fn create_router() -> Router {
    Router::new()
        .route("/v1/plan/execute", post(execute_plan))
        .route("/v1/trace/:plan_id", get(get_trace))
        .route("/v1/replay/bundle", post(create_bundle))
        .with_state(AppState::new())
}

#[derive(Deserialize)]
pub struct ExecuteRequest {
    pub plan: Plan,
    pub inputs: Option<serde_json::Value>,
}

#[derive(Serialize)]
pub struct ExecuteResponse {
    pub plan_id: String,
    pub stream_url: String,
    pub status: String,
}

#[derive(Serialize)]
pub struct TraceResponse {
    pub plan_id: String,
    pub traces: Vec<Trace>,
}

async fn execute_plan(
    State(state): State<AppState>,
    Json(request): Json<ExecuteRequest>,
) -> Result<Json<ExecuteResponse>, (StatusCode, Json<serde_json::Value>)> {
    let plan_id = Uuid::new_v4().to_string();
    
    // Store the plan
    {
        let mut plans = state.plans.write().await;
        plans.insert(plan_id.clone(), request.plan.clone());
    }
    
    // Validate the plan first
    if let Err(e) = request.plan.validate() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": format!("Plan validation failed: {}", e)}))
        ));
    }
    
    // Prepare execution context with inputs if provided
    let mut ctx = ExecutionContext::new();
    if let Some(inputs) = request.inputs {
        if let serde_json::Value::Object(map) = inputs {
            ctx.variables = map;
        }
    }
    ctx.signals = request.plan.signals.clone();
    
    // Set up tool URLs (in a real implementation, these would be configured properly)
    ctx.tool_urls.insert("doc.search.local".to_string(), "http://localhost:7401".to_string());
    ctx.tool_urls.insert("ground.verify".to_string(), "http://localhost:7402".to_string());
    ctx.tool_urls.insert("mesh.mem.sqlite".to_string(), "http://localhost:7403".to_string());
    
    // Execute the plan
    let scheduler = Scheduler;
    let result = scheduler.execute_plan(ctx, &request.plan).await;
    
    match result {
        Ok(mut final_ctx) => {
            // Store traces for this plan
            {
                let mut plan_traces = state.plan_traces.write().await;
                plan_traces.insert(plan_id.clone(), final_ctx.trace_events.clone());
            }
            
            Ok(Json(ExecuteResponse {
                plan_id: plan_id.clone(),
                stream_url: format!("/v1/trace/{}", plan_id),
                status: "completed".to_string(),
            }))
        }
        Err(e) => {
            tracing::error!("Plan execution failed for plan {}: {}", plan_id, e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Plan execution failed: {}", e)}))
            ))
        }
    }
}

async fn get_trace(
    Path(plan_id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<TraceResponse>, (StatusCode, Json<serde_json::Value>)> {
    let plan_traces = state.plan_traces.read().await;
    let traces = plan_traces.get(&plan_id)
        .ok_or_else(|| {
            (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": format!("Plan {} not found", plan_id)})))
        })?
        .clone();
    
    Ok(Json(TraceResponse {
        plan_id: plan_id.clone(),
        traces,
    }))
}

#[derive(Deserialize)]
pub struct BundleRequest {
    pub plan_id: String,
}

async fn create_bundle(
    State(state): State<AppState>,
    Json(request): Json<BundleRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // In a real implementation, this would create a tar.gz bundle
    // For now, return a dummy response
    let plans = state.plans.read().await;
    let plan = plans.get(&request.plan_id)
        .ok_or_else(|| {
            (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": format!("Plan {} not found", request.plan_id)})))
        })?;
    
    let plan_traces = state.plan_traces.read().await;
    let traces = plan_traces.get(&request.plan_id)
        .ok_or_else(|| {
            (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": format!("Traces for plan {} not found", request.plan_id)})))
        })?
        .clone();
    
    let bundle_data = serde_json::json!({
        "plan": plan,
        "traces": traces,
        "plan_id": request.plan_id
    });
    
    let serialized = serde_json::to_vec(&bundle_data)
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Serialization error: {}", e)})))
        })?;
    
    Ok((StatusCode::OK, serialized))
}
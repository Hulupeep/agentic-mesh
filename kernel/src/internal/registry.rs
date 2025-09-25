use serde::{Deserialize, Serialize};
use std::{collections::HashMap, env, fs, path::Path, sync::Arc};
use tokio::sync::RwLock;

const DEFAULT_CONFIG_PATH: &str = "config/tools.json";
const DEFAULT_ENTRIES: &[(&str, &str)] = &[
    ("doc.search.local", "http://localhost:7401"),
    ("ground.verify", "http://localhost:7402"),
    ("mesh.mem.sqlite", "http://localhost:7403"),
];

#[derive(Debug, Deserialize, Serialize, Clone)]
struct ToolEntry {
    name: String,
    url: String,
}

#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("Failed to read tool registry: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid tool registry JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("HTTP error accessing tool registry: {0}")]
    Http(String),
}

pub fn load_tool_registry() -> HashMap<String, String> {
    let path = env::var("AMP_TOOL_CONFIG").unwrap_or_else(|_| DEFAULT_CONFIG_PATH.to_string());
    match read_registry(Path::new(&path)) {
        Ok(registry) if !registry.is_empty() => registry,
        Ok(_) | Err(_) => default_registry(),
    }
}

fn read_registry(path: &Path) -> Result<HashMap<String, String>, RegistryError> {
    let contents = fs::read_to_string(path)?;
    let entries: Vec<ToolEntry> = serde_json::from_str(&contents)?;
    Ok(entries
        .into_iter()
        .map(|entry| (entry.name, entry.url))
        .collect())
}

pub fn default_registry() -> HashMap<String, String> {
    DEFAULT_ENTRIES
        .iter()
        .map(|(name, url)| ((*name).to_string(), (*url).to_string()))
        .collect()
}

pub async fn fetch_remote_registry(
    base_url: &str,
) -> Result<HashMap<String, String>, RegistryError> {
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/tools", base_url.trim_end_matches('/')))
        .send()
        .await
        .map_err(|e| RegistryError::Http(e.to_string()))?;

    if !response.status().is_success() {
        return Err(RegistryError::Http(format!(
            "Registry responded with status {}",
            response.status()
        )));
    }

    let entries: Vec<ToolEntry> = response
        .json()
        .await
        .map_err(|e| RegistryError::Http(e.to_string()))?;

    Ok(entries
        .into_iter()
        .map(|entry| (entry.name, entry.url))
        .collect())
}

#[derive(Clone, Default)]
pub struct RegistryState {
    inner: Arc<RwLock<HashMap<String, String>>>,
}

impl RegistryState {
    pub fn new(initial: HashMap<String, String>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(initial)),
        }
    }

    pub async fn list(&self) -> HashMap<String, String> {
        self.inner.read().await.clone()
    }

    pub async fn register(&self, name: String, url: String) {
        self.inner.write().await.insert(name, url);
    }

    pub async fn unregister(&self, name: &str) {
        self.inner.write().await.remove(name);
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RegisterRequest {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub success: bool,
}

pub fn create_registry_router(state: RegistryState) -> axum::Router {
    use axum::{
        extract::{Path, State},
        routing::{delete, get, post},
        Json, Router,
    };

    async fn list(State(state): State<RegistryState>) -> Json<Vec<ToolEntry>> {
        let registry = state.list().await;
        let entries = registry
            .into_iter()
            .map(|(name, url)| ToolEntry { name, url })
            .collect();
        Json(entries)
    }

    async fn register(
        State(state): State<RegistryState>,
        Json(payload): Json<RegisterRequest>,
    ) -> Json<RegisterResponse> {
        state.register(payload.name, payload.url).await;
        Json(RegisterResponse { success: true })
    }

    async fn unregister(
        State(state): State<RegistryState>,
        Path(name): Path<String>,
    ) -> Json<RegisterResponse> {
        state.unregister(&name).await;
        Json(RegisterResponse { success: true })
    }

    Router::new()
        .route("/tools", get(list))
        .route("/register", post(register))
        .route("/register/:name", delete(unregister))
        .with_state(state)
}

use amp::internal::registry::{create_registry_router, RegisterRequest, RegistryState};
use serde_json::json;
use std::collections::HashMap;

#[tokio::test]
async fn test_registry_service_registers_and_lists() {
    let initial = HashMap::new();
    let state = RegistryState::new(initial);
    let app = create_registry_router(state.clone());

    let client = reqwest::Client::new();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .expect("registry server error");
    });

    let base_url = format!("http://{}", addr);
    let register_body = RegisterRequest {
        name: "test.tool".to_string(),
        url: "http://localhost:9999".to_string(),
    };

    client
        .post(format!("{}/register", base_url))
        .json(&register_body)
        .send()
        .await
        .expect("register request failed");

    let tools: Vec<serde_json::Value> = client
        .get(format!("{}/tools", base_url))
        .send()
        .await
        .expect("list request failed")
        .json()
        .await
        .expect("invalid response body");

    assert!(tools
        .iter()
        .any(|entry| entry == &json!({"name": "test.tool", "url": "http://localhost:9999"})));

    handle.abort();
}

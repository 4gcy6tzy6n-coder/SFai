//! M7 backend API tests (router-level, no network): task lifecycle, demo runs,
//! artifacts, verification, wiki, events, auth and error mapping.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

use sfai::app::{build_app, App};
use sfai::server::{router, ServerState};

fn state(app: App, token: Option<String>) -> Arc<ServerState> {
    Arc::new(ServerState::new(Arc::new(app), token, None))
}

async fn call(router: &axum::Router, request: Request<Body>) -> (StatusCode, Value) {
    let (status, bytes) = call_bytes(router, request).await;
    let value: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, value)
}

async fn call_bytes(router: &axum::Router, request: Request<Body>) -> (StatusCode, bytes::Bytes) {
    let response = router.clone().oneshot(request).await.expect("router call");
    let status = response.status();
    let body = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    (status, body)
}

fn get(uri: &str) -> Request<Body> {
    Request::builder()
        .uri(uri)
        .body(Body::empty())
        .expect("request")
}

fn post_json(uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("request")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn health_works_and_auth_is_enforced() {
    let dir = tempfile::tempdir().expect("tmp");
    let app = build_app(dir.path()).expect("app");
    let open = router(state(app, None));

    let (status, body) = call(&open, get("/api/health")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
    assert!(body["fixtures"].as_array().unwrap().len() == 5);

    // With a token configured, /api/* requires it; health stays open.
    let dir2 = tempfile::tempdir().expect("tmp2");
    let app2 = build_app(dir2.path()).expect("app2");
    let guarded = router(state(app2, Some("secret".into())));

    let (status, _) = call(&guarded, get("/api/health")).await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = call(&guarded, get("/api/tasks")).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let authed = Request::builder()
        .uri("/api/tasks")
        .header("authorization", "Bearer secret")
        .body(Body::empty())
        .expect("request");
    let (status, body) = call(&guarded, authed).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["tasks"].is_array());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn task_lifecycle_and_artifacts() {
    let dir = tempfile::tempdir().expect("tmp");
    let app = build_app(dir.path()).expect("app");
    let api = router(state(app, None));

    let (status, body) = call(
        &api,
        post_json(
            "/api/tasks",
            json!({"goal": "say hello", "acceptance_rules": []}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let task_id = body["task"]["task_id"]
        .as_str()
        .expect("task id")
        .to_string();

    let (status, body) = call(&api, get(&format!("/api/tasks/{task_id}"))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["task"]["goal"], "say hello");
    assert_eq!(body["state"]["status"], "created");

    // One-shot demo: tool-use fixture produces a verifiable artifact.
    let (status, demo) = call(&api, post_json("/api/demo/tool-use", json!({}))).await;
    assert_eq!(status, StatusCode::OK, "demo failed: {demo}");
    assert_eq!(demo["status"], "completed");
    let demo_task = demo["task_id"].as_str().expect("demo task").to_string();
    assert!(demo["steps"].as_u64().unwrap_or(0) >= 3);

    let (status, body) = call(&api, get(&format!("/api/tasks/{demo_task}/artifacts"))).await;
    assert_eq!(status, StatusCode::OK);
    let artifacts = body["artifacts"].as_array().expect("artifacts");
    assert!(!artifacts.is_empty());
    let artifact_id = artifacts[0]["artifact_id"].as_str().expect("artifact id");

    let (status, bytes) = call_bytes(
        &api,
        get(&format!("/api/tasks/{demo_task}/artifacts/{artifact_id}")),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let content = String::from_utf8_lossy(&bytes).to_string();
    assert!(
        content.contains("Citations"),
        "artifact text must be served: {content}"
    );

    let (status, body) = call(&api, get(&format!("/api/tasks/{demo_task}/verify"))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["all_pass"], true);

    let (status, body) = call(&api, get(&format!("/api/tasks/{demo_task}/wiki"))).await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        body["summary"]["nodes_by_kind"]["evidence"]
            .as_u64()
            .unwrap_or(0)
            >= 1
    );

    let (status, body) = call(&api, get(&format!("/api/tasks/{demo_task}/events"))).await;
    assert_eq!(status, StatusCode::OK);
    assert!(!body["events"].as_array().expect("events").is_empty());

    let (status, body) = call(&api, get(&format!("/api/tasks/{demo_task}/trace"))).await;
    assert_eq!(status, StatusCode::OK);
    assert!(!body["trace"].as_array().expect("trace").is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn error_mapping_and_validation() {
    let dir = tempfile::tempdir().expect("tmp");
    let app = build_app(dir.path()).expect("app");
    let api = router(state(app, None));

    let (status, _) = call(&api, get("/api/tasks/does-not-exist")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, _) = call(
        &api,
        post_json("/api/tasks", json!({"goal": "   ", "acceptance_rules": []})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, body) = call(
        &api,
        post_json("/api/tasks", json!({"goal": "x", "acceptance_rules": []})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let task_id = body["task"]["task_id"].as_str().unwrap().to_string();

    let (status, _) = call(
        &api,
        post_json(
            &format!("/api/tasks/{task_id}/run"),
            json!({"fixture": "nope"}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // Cancel on a task that is not running is a conflict.
    let (status, _) = call(
        &api,
        post_json(&format!("/api/tasks/{task_id}/cancel"), json!({})),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn run_endpoint_executes_fixture_on_existing_task() {
    let dir = tempfile::tempdir().expect("tmp");
    let app = build_app(dir.path()).expect("app");
    let api = router(state(app, None));

    let (_, body) = call(
        &api,
        post_json(
            "/api/tasks",
            json!({
                "goal": "run the direct answer fixture",
                "constraints": ["be brief"],
                "acceptance_rules": []
            }),
        ),
    )
    .await;
    let task_id = body["task"]["task_id"].as_str().unwrap().to_string();

    let (status, result) = call(
        &api,
        post_json(
            &format!("/api/tasks/{task_id}/run"),
            json!({"fixture": "direct-answer", "max_steps": 5}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "run failed: {result}");
    assert_eq!(result["status"], "completed");
    assert_eq!(result["steps"], 1);

    let (status, body) = call(&api, get(&format!("/api/tasks/{task_id}"))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["state"]["status"], "completed");
    assert_eq!(body["task"]["constraints"][0], "be brief");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn providers_endpoint_lists_builtin_profiles() {
    let dir = tempfile::tempdir().expect("tmp");
    let app = build_app(dir.path()).expect("app");
    let api = router(state(app, None));

    let (status, body) = call(&api, get("/api/providers")).await;
    assert_eq!(status, StatusCode::OK);
    let names: Vec<&str> = body["providers"]
        .as_array()
        .expect("providers")
        .iter()
        .map(|p| p["name"].as_str().unwrap_or(""))
        .collect();
    assert_eq!(names, vec!["deepseek", "qwen", "mimo", "bailian", "custom"]);
    assert!(body["default"].is_null());
    assert_eq!(body["providers"][0]["model"], "deepseek-chat");
    assert_eq!(body["providers"][0]["api_key_envs"][0], "DEEPSEEK_API_KEY");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn run_without_provider_is_rejected_clearly() {
    let dir = tempfile::tempdir().expect("tmp");
    let app = build_app(dir.path()).expect("app");
    let api = router(state(app, None));

    let (_, body) = call(
        &api,
        post_json(
            "/api/tasks",
            json!({"goal": "hello", "acceptance_rules": []}),
        ),
    )
    .await;
    let task_id = body["task"]["task_id"].as_str().unwrap().to_string();
    let (status, body) = call(
        &api,
        post_json(&format!("/api/tasks/{task_id}/run"), json!({})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        body["error"]
            .as_str()
            .unwrap_or("")
            .contains("no provider configured"),
        "unexpected error: {body}"
    );
}

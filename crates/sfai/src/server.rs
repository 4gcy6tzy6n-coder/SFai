//! M7 backend API server (`sfai serve`).
//!
//! A local, single-user HTTP API so the CLI UI and the desktop/web frontend
//! share one engine instead of parsing CLI output:
//!
//! - tasks: create / list / get / run (fixture or provider) / cancel
//! - events: polling plus an SSE stream
//! - artifacts, trace, wiki summary, verification results, capabilities, skills
//! - `POST /api/demo/<fixture>` for one-shot demos
//!
//! Concurrency: axum handlers run on a multi-threaded runtime; kernel runs are
//! offloaded to blocking threads. Per-task run locks reject duplicate runs and
//! `cancel` flips a cooperative flag the kernel checks each step.

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::{self, Next};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::convert::Infallible;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::app::App;
use crate::contracts::{Budget, TaskStatus};
use crate::core::decision_kernel::{NoopObserver, TaskRunResult};
use crate::core::model_adapter::ModelAdapter;
use crate::error::SfaiError;
use crate::fixtures::{get_fixture, FIXTURE_NAMES};
use crate::integrations::providers::openai_compat::OpenAICompatibleAdapter;

pub struct ServerState {
    pub app: Arc<App>,
    pub token: Option<String>,
    pub running: Mutex<HashSet<String>>,
    pub cancels: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

impl ServerState {
    pub fn new(app: Arc<App>, token: Option<String>) -> Self {
        Self {
            app,
            token,
            running: Mutex::new(HashSet::new()),
            cancels: Mutex::new(HashMap::new()),
        }
    }
}

pub fn router(state: Arc<ServerState>) -> Router {
    let cors = tower_http::cors::CorsLayer::permissive();
    Router::new()
        .route("/api/health", get(health))
        .route("/api/capabilities", get(list_capabilities))
        .route("/api/skills", get(list_skills))
        .route("/api/tasks", get(list_tasks).post(create_task))
        .route("/api/tasks/:task_id", get(get_task))
        .route("/api/tasks/:task_id/events", get(get_events))
        .route("/api/tasks/:task_id/stream", get(stream_events))
        .route("/api/tasks/:task_id/run", post(run_task))
        .route("/api/tasks/:task_id/cancel", post(cancel_task))
        .route("/api/tasks/:task_id/artifacts", get(list_artifacts))
        .route(
            "/api/tasks/:task_id/artifacts/:artifact_id",
            get(get_artifact_text),
        )
        .route("/api/tasks/:task_id/trace", get(get_trace))
        .route("/api/tasks/:task_id/wiki", get(get_wiki_summary))
        .route("/api/tasks/:task_id/verify", get(verify_task))
        .route("/api/demo/:fixture", post(run_demo))
        .layer(cors)
        .layer(middleware::from_fn_with_state(state.clone(), auth))
        .with_state(state)
}

/// Serve the API until the process stops. Bind to loopback by default.
pub async fn serve(
    app: Arc<App>,
    addr: std::net::SocketAddr,
    token: Option<String>,
) -> std::io::Result<()> {
    let state = Arc::new(ServerState::new(app, token));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router(state)).await
}

// ---- middleware ----

async fn auth(
    State(state): State<Arc<ServerState>>,
    headers: HeaderMap,
    request: axum::extract::Request,
    next: Next,
) -> Response {
    if request.uri().path() == "/api/health" {
        return next.run(request).await;
    }
    if let Some(expected) = &state.token {
        let provided = headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if provided != format!("Bearer {expected}") {
            return ApiError::unauthorized("missing or invalid bearer token").into_response();
        }
    }
    next.run(request).await
}

// ---- errors ----

pub struct ApiError(StatusCode, String);

impl ApiError {
    fn bad_request(message: impl Into<String>) -> Self {
        Self(StatusCode::BAD_REQUEST, message.into())
    }
    fn unauthorized(message: impl Into<String>) -> Self {
        Self(StatusCode::UNAUTHORIZED, message.into())
    }
    fn not_found(message: impl Into<String>) -> Self {
        Self(StatusCode::NOT_FOUND, message.into())
    }
    fn conflict(message: impl Into<String>) -> Self {
        Self(StatusCode::CONFLICT, message.into())
    }
    fn internal(message: impl Into<String>) -> Self {
        Self(StatusCode::INTERNAL_SERVER_ERROR, message.into())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({"error": self.1}))).into_response()
    }
}

impl From<SfaiError> for ApiError {
    fn from(error: SfaiError) -> Self {
        match &error {
            SfaiError::NotFound(_) => ApiError::not_found(error.to_string()),
            SfaiError::Validation(_) | SfaiError::Intent(_) | SfaiError::Policy(_) => {
                ApiError::bad_request(error.to_string())
            }
            _ => ApiError::internal(error.to_string()),
        }
    }
}

// ---- handlers ----

async fn health(State(state): State<Arc<ServerState>>) -> Json<Value> {
    let running = state.running.lock().expect("server mutex poisoned").len();
    Json(json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "running_tasks": running,
        "fixtures": FIXTURE_NAMES,
    }))
}

async fn list_capabilities(State(state): State<Arc<ServerState>>) -> Json<Value> {
    Json(json!({"capabilities": state.app.registry.discover("", 100, None)}))
}

async fn list_skills(State(state): State<Arc<ServerState>>) -> Json<Value> {
    Json(json!({"skills": state.app.skills.discover("", 100)}))
}

#[derive(Deserialize)]
struct CreateTaskReq {
    goal: String,
    #[serde(default)]
    constraints: Vec<String>,
    #[serde(default)]
    acceptance_rules: Vec<String>,
    #[serde(default)]
    max_tool_calls: Option<u32>,
}

async fn create_task(
    State(state): State<Arc<ServerState>>,
    Json(req): Json<CreateTaskReq>,
) -> Result<Json<Value>, ApiError> {
    if req.goal.trim().is_empty() {
        return Err(ApiError::bad_request("goal must not be empty"));
    }
    let budget = req.max_tool_calls.map(|n| Budget {
        max_tool_calls: Some(n),
        ..Default::default()
    });
    let task = state
        .app
        .create_task(&req.goal, req.constraints, req.acceptance_rules, budget)?;
    Ok(Json(json!({"task": task})))
}

#[derive(Deserialize)]
struct ListTasksQuery {
    #[serde(default = "default_limit")]
    limit: usize,
}

fn default_limit() -> usize {
    20
}

async fn list_tasks(
    State(state): State<Arc<ServerState>>,
    Query(query): Query<ListTasksQuery>,
) -> Result<Json<Value>, ApiError> {
    let tasks = state.app.task_store.list(query.limit.min(200))?;
    Ok(Json(json!({"tasks": tasks})))
}

async fn get_task(
    State(state): State<Arc<ServerState>>,
    Path(task_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let task = state.app.task_store.load(&task_id)?;
    let state_view = state.app.task_store.state(&task_id)?;
    Ok(Json(json!({"task": task, "state": state_view})))
}

#[derive(Deserialize)]
struct EventsQuery {
    #[serde(default)]
    after: i64,
}

async fn get_events(
    State(state): State<Arc<ServerState>>,
    Path(task_id): Path<String>,
    Query(query): Query<EventsQuery>,
) -> Result<Json<Value>, ApiError> {
    state.app.task_store.load(&task_id)?;
    let events = state.app.ledger.events_for(&task_id, query.after)?;
    Ok(Json(json!({"events": events})))
}

async fn stream_events(
    State(state): State<Arc<ServerState>>,
    Path(task_id): Path<String>,
    Query(query): Query<EventsQuery>,
) -> Result<Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>>, ApiError> {
    state.app.task_store.load(&task_id)?;
    let app = state.app.clone();
    let stream = futures_util::stream::unfold(
        (app, task_id, query.after),
        |(app, task_id, after)| async move {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            let events = app.ledger.events_for(&task_id, after).unwrap_or_default();
            let (next_after, event) = match events.last() {
                Some(last) => (
                    last.seq,
                    Event::default()
                        .event("ledger")
                        .data(serde_json::to_string(&events).unwrap_or_default()),
                ),
                None => (after, Event::default().event("heartbeat").data("{}")),
            };
            Some((Ok::<Event, Infallible>(event), (app, task_id, next_after)))
        },
    );
    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

#[derive(Deserialize, Default)]
struct RunTaskReq {
    #[serde(default)]
    fixture: Option<String>,
    #[serde(default)]
    max_steps: Option<usize>,
}

async fn run_task(
    State(state): State<Arc<ServerState>>,
    Path(task_id): Path<String>,
    Json(req): Json<RunTaskReq>,
) -> Result<Json<Value>, ApiError> {
    state.app.task_store.load(&task_id)?;
    if let Some(name) = &req.fixture {
        get_fixture(name).map_err(|e| ApiError::bad_request(e.to_string()))?;
    }

    {
        let mut running = state.running.lock().expect("server mutex poisoned");
        if !running.insert(task_id.clone()) {
            return Err(ApiError::conflict("task is already running"));
        }
    }
    let cancel = Arc::new(AtomicBool::new(false));
    state
        .cancels
        .lock()
        .expect("server mutex poisoned")
        .insert(task_id.clone(), cancel.clone());

    let app = state.app.clone();
    let tid = task_id.clone();
    let fixture = req.fixture.clone();
    let max_steps = req.max_steps.unwrap_or(50);

    let joined = tokio::task::spawn_blocking(move || -> Result<TaskRunResult, String> {
        let mut model: Box<dyn ModelAdapter> = match fixture {
            Some(name) => {
                let spec = get_fixture(&name).map_err(|e| e.to_string())?;
                (spec.build_model)()
            }
            None => Box::new(OpenAICompatibleAdapter::from_env()),
        };
        let mut observer = NoopObserver;
        app.run_observed_with_cancel(model.as_mut(), &tid, max_steps, &mut observer, Some(cancel))
            .map_err(|e| e.to_string())
    })
    .await;

    state
        .running
        .lock()
        .expect("server mutex poisoned")
        .remove(&task_id);
    state
        .cancels
        .lock()
        .expect("server mutex poisoned")
        .remove(&task_id);

    match joined {
        Ok(Ok(result)) => Ok(Json(json!(result))),
        Ok(Err(message)) => Err(ApiError::internal(message)),
        Err(join_error) => Err(ApiError::internal(format!("run join error: {join_error}"))),
    }
}

async fn cancel_task(
    State(state): State<Arc<ServerState>>,
    Path(task_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    state.app.task_store.load(&task_id)?;
    let flag = state
        .cancels
        .lock()
        .expect("server mutex poisoned")
        .get(&task_id)
        .cloned();
    match flag {
        Some(flag) => {
            flag.store(true, Ordering::Relaxed);
            Ok(Json(json!({"cancel_requested": true, "task_id": task_id})))
        }
        None => Err(ApiError::conflict("task is not running")),
    }
}

async fn list_artifacts(
    State(state): State<Arc<ServerState>>,
    Path(task_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    state.app.task_store.load(&task_id)?;
    let artifacts = state.app.objects.task_artifacts(&task_id)?;
    Ok(Json(json!({"artifacts": artifacts})))
}

async fn get_artifact_text(
    State(state): State<Arc<ServerState>>,
    Path((task_id, artifact_id)): Path<(String, String)>,
) -> Result<Response, ApiError> {
    let artifact = state
        .app
        .objects
        .get_artifact(&artifact_id)?
        .ok_or_else(|| ApiError::not_found(format!("unknown artifact: {artifact_id}")))?;
    if artifact.task_id != task_id {
        return Err(ApiError::not_found("artifact does not belong to this task"));
    }
    let text = state.app.objects.read_artifact_text(&artifact_id)?;
    Ok((
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, artifact.mime_type)],
        text,
    )
        .into_response())
}

async fn get_trace(
    State(state): State<Arc<ServerState>>,
    Path(task_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    state.app.task_store.load(&task_id)?;
    let entries = state.app.export_trace(&task_id)?;
    Ok(Json(json!({"trace": entries})))
}

async fn get_wiki_summary(
    State(state): State<Arc<ServerState>>,
    Path(task_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    state.app.task_store.load(&task_id)?;
    Ok(Json(json!({"summary": state.app.wiki.summary(&task_id)?})))
}

async fn verify_task(
    State(state): State<Arc<ServerState>>,
    Path(task_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let task = state.app.task_store.load(&task_id)?;
    let results = state
        .app
        .verifier
        .verify(&task_id, &task.acceptance_rules)?;
    let all_pass = results
        .iter()
        .all(|r| r.verdict == crate::contracts::Verdict::Pass);
    Ok(Json(json!({"results": results, "all_pass": all_pass})))
}

async fn run_demo(
    State(state): State<Arc<ServerState>>,
    Path(fixture): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let app = state.app.clone();
    let joined = tokio::task::spawn_blocking(move || -> Result<Value, SfaiError> {
        let spec = get_fixture(&fixture)?;
        let task = app.create_task(
            &spec.goal,
            spec.constraints.clone(),
            spec.acceptance_rules.clone(),
            Some(spec.budget.clone()),
        )?;
        if let Some(setup) = spec.setup {
            setup(&app, &task)?;
        }
        let mut model = (spec.build_model)();
        let mut observer = NoopObserver;
        let result = app.run_observed(model.as_mut(), &task.task_id, 50, &mut observer)?;
        Ok(json!({
            "task_id": result.task_id,
            "status": result.status.as_str(),
            "steps": result.steps,
            "cost": result.cost,
            "answer": result.answer,
            "open_issues": result.open_issues,
        }))
    })
    .await;
    match joined {
        Ok(Ok(value)) => Ok(Json(value)),
        Ok(Err(error)) => Err(ApiError::from(error)),
        Err(join_error) => Err(ApiError::internal(format!("demo join error: {join_error}"))),
    }
}

#[allow(dead_code)]
fn _assert_send_sync<T: Send + Sync>() {}

#[allow(dead_code)]
fn _app_is_send_sync() {
    _assert_send_sync::<App>();
}

// Keep StatusCode import used even if handlers change shape.
#[allow(dead_code)]
const _: StatusCode = StatusCode::OK;

// TaskStatus is part of the run result contract; keep the import explicit.
#[allow(dead_code)]
fn _task_status_marker() -> TaskStatus {
    TaskStatus::Active
}

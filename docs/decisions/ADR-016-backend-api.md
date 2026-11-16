# ADR-016: M7 backend HTTP API (`sfai serve`)

**Date:** 2026-11-16
**Status:** Accepted for v0.1 — revisit at multi-user/ACL work
**Context:** The frontend needs one engine behind both the CLI and the UI. This
ADR records the local HTTP API and the thread-safety work that made it
possible.

## Decisions

1. **axum + tokio multi-thread runtime.** `sfai serve` binds loopback
   (`127.0.0.1:8765` by default) and serves JSON + SSE. Handlers are async;
   kernel runs happen on `spawn_blocking` because the M0–M6 kernel is
   synchronous by design.
2. **App is `Send + Sync`.** `Database` became `Arc<Mutex<Connection>>`,
   approvals/registries/skill activations moved from `Rc`/`RefCell`/`Cell` to
   `Arc`/`Mutex`, and handler/ref-resolver trait objects gained `Send + Sync`
   bounds. SQLite access is serialized by the mutex (one writer at a time).
3. **Per-task run lock.** `ServerState.running` rejects duplicate
   `POST /api/tasks/:id/run` with `409`. Cancellation uses a per-run
   `Arc<AtomicBool>`; the kernel checks it at the top of every step and ends
   the task in `cancelled`.
4. **SSE is polling-based.** `/api/tasks/:id/stream` polls the append-only
   ledger every 500 ms and emits `ledger` events (or `heartbeat`), so no
   in-process pub/sub is required and a crashed client cannot leak handlers.
5. **Optional bearer token.** `SFAI_API_TOKEN` guards every `/api/*` route
   except `/api/health`. Without it the server is open — acceptable on
   loopback only; never bind to a public interface without a token.
6. **CORS is permissive** (`tower-http` `CorsLayer::permissive`) for local
   frontend development. This must be tightened before any non-loopback
   deployment.
7. **Demo endpoint.** `POST /api/demo/:fixture` creates the task from the
   fixture spec, applies its setup, and runs it — lets the UI show a working
   demo with one call.

## API shape

Errors are `{"error": "..."}` with 400/401/404/409/500. Task creation returns
`{"task": {...}}`; task reads return `{"task":..., "state":...}`; runs return
the `TaskRunResult` fields (`task_id`, `status`, `answer`, `steps`, `cost`,
`open_issues`).

## Deviations / limitations

- **Single SQLite writer.** The mutex serializes all DB work; concurrent UI
  reads queue behind a running write. Fine for local single-user use; revisit
  with a connection pool when multi-user lands.
- **SSE latency** is bounded by the 500 ms poll; no push channel yet.
- **Cancellation is cooperative**: it is observed between steps, not inside a
  running tool (tool timeouts handle the latter).
- **No ACL/tenant filtering** — M7 multi-user item remains open.
- **No rate limiting / request auth beyond the optional token.**

## Consequences

- Follow-up: connection pool, push-based events, ACL, rate limiting, bind
  guard for non-loopback, and tightening CORS.
- The desktop UI can be built against this API; the CLI keeps using the
  library directly.

# ADR-019: Real model providers (DeepSeek first; Qwen / MiMo later)

**Date:** 2026-11-16
**Status:** Accepted for v1 real-provider runs
**Context:** The product needs real models, and the paper track needs
real-model measurements. The first target is DeepSeek; Xiaomi MiMo and Qwen
follow. All are (or will be) OpenAI-compatible.

## Decisions

1. **Provider profiles.** `integrations/providers/profiles.rs` defines
   `deepseek`, `qwen`, `mimo`, `custom`. Each profile carries base URL, model,
   ordered API-key env vars, and a price table. Profiles are data, not code
   paths: one adapter serves all of them.
2. **Resolution order.** `--provider` (CLI) or `provider` (API request) wins;
   without it, a fully configured `custom` profile from
   `SFAI_MODEL_BASE_URL`/`SFAI_MODEL_NAME` is used; otherwise the run fails with
   an actionable error. `serve` may start without a provider (offline fixtures).
3. **One tool for the decision kernel.** Real providers get exactly one tool,
   `submit_action`, whose argument schema is the action-intent envelope. Models
   may also answer with a JSON intent in `content` (Markdown code fences are
   tolerated); plain text becomes a direct `respond`.
4. **Retries.** HTTP 429/5xx retry up to 3 attempts (0.5s/1s backoff), 180s
   timeout. Non-retryable transport errors surface as `ModelError::Provider`.
5. **Pricing is approximate and overridable.** Defaults (`deepseek` 0.27/1.10,
   `qwen` 0.40/1.20, `mimo` 0/0 per 1M tokens) are engineering placeholders;
   `--price-in`/`--price-out` or contract prices must be set before comparing
   cost numbers. Costs are stored per task in the trace/ledger.
6. **Per-request overrides.** `POST /api/tasks/{id}/run` accepts
   `provider`/`model`; `GET /api/providers` lists profiles plus whether each is
   configured (key/env), so the UI can render provider selection.
7. **Benchmark v1 is the same benchmark.** `sfai bench --provider ...` swaps the
   scripted model for the real adapter; suites, ablations, metrics and report
   format are unchanged, so v0 and v1 numbers are directly comparable.

## Limitations

- Live calls are not exercised in CI (no keys); tests use a local mock server
  for the HTTP path, tool-call normalization, content-JSON fallback and pricing.
- Provider prices and model names drift; overrides are mandatory for published
  cost comparisons.
- Reasoning models may bill reasoning tokens; trust the provider-reported
  usage for cost, and record `--price-out` accordingly.
- No streaming yet; runs are request/response with retries.

## Consequences

- Adding a provider = adding a profile (env/prices) — no kernel change.
- The paper must disclose provider, model version, price table, hardware, and
  number of seeds alongside benchmark results.

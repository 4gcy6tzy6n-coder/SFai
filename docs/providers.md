# Real model providers

`sfai` talks to any **OpenAI-compatible** endpoint. Built-in profiles cover
DeepSeek, Qwen (DashScope compatible mode), Xiaomi MiMo, and a generic
`custom` profile.

## Quick start (DeepSeek)

```bash
export DEEPSEEK_API_KEY=sk-...

sfai run --goal "Find three sources on sparse context activation and write a cited report." \
  --provider deepseek --model deepseek-chat

sfai serve --provider deepseek          # UI/API runs default to DeepSeek
sfai bench --provider deepseek --model deepseek-chat \
  --suite dataset --ablation none --size 3 --out evals/reports/v1-deepseek.json
```

## Profiles

| provider | API key env (fallback) | default base URL | default model |
|---|---|---|---|
| `deepseek` | `DEEPSEEK_API_KEY` | `https://api.deepseek.com/v1` | `deepseek-chat` (also try `deepseek-reasoner`) |
| `qwen` | `DASHSCOPE_API_KEY` (or `QWEN_API_KEY`) | `https://dashscope.aliyuncs.com/compatible-mode/v1` | `qwen-plus` |
| `mimo` | `XIAOMI_MIMO_API_KEY` (or `MIMO_API_KEY`) | `https://token-plan-cn.xiaomimimo.com/v1` (Token Plan) | `mimo-v2.6-pro` (or `mimo-v2.6-flash`) |
| `custom` | `SFAI_MODEL_API_KEY` | `SFAI_MODEL_BASE_URL` | `SFAI_MODEL_NAME` |

Overrides (any provider): `--model`, `--price-in`, `--price-out` (USD per 1M
tokens); env: `<PROVIDER>_BASE_URL`, `<PROVIDER>_MODEL`.

Pricing is **approximate** by default (`deepseek`: 0.27 / 1.10 USD per 1M;
`qwen`: 0.40 / 1.20; `mimo`: Token Plan is subscription-quota based; keep 0/0 and compare tokens). Pass `--price-in/--price-out`
or set your contract prices before comparing cost numbers.

## Behaviour

- One tool is exposed (`submit_action`); the model may also answer with a JSON
  intent in `content` (code fences tolerated).
- HTTP 429/5xx are retried up to 3 times (0.5s/1s backoff); 180s timeout.
- Usage is priced with the profile's table and recorded in the task trace; the
  benchmark reports it as `cost USD`, `cost/success USD`.
- `GET /api/providers` (when `sfai serve` is running) lists profiles and whether
  each is configured; `POST /api/tasks/{id}/run` accepts `provider`/`model`.

## Troubleshooting

- `provider ... is missing an API key`: export the env var listed above.
- MiMo Token Plan keys (`tp-`/`ttp-`) use `https://token-plan-cn.xiaomimimo.com/v1`;\n  other clusters: set `XIAOMI_MIMO_BASE_URL` (e.g. `token-plan-sgp` / `token-plan-ams`).\n  Pay-as-you-go `sk-` keys use `https://api.xiaomimimo.com/v1` instead.
- Reasoning-style models: some emit reasoning tokens; output-token cost reflects
  the provider's reported usage, so keep `--price-out` consistent with billing.

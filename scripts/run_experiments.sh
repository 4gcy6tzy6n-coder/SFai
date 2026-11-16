#!/usr/bin/env bash
# Run every experiment config whose provider key is available.
# Usage: ./scripts/run_experiments.sh [config-glob]
set -euo pipefail
cd "$(dirname "$0")/.."

BIN="${BIN:-target/release/sfai}"
STORAGE="${STORAGE:-experiments/.run/storage}"
mkdir -p experiments/results "$STORAGE"
cargo build --release

CONFIGS="${1:-experiments/configs/*.json}"
for cfg in $CONFIGS; do
  [ -f "$cfg" ] || continue
  name="$(python3 -c "import json;print(json.load(open('$cfg'))['name'])")"
  provider="$(python3 -c "import json;d=json.load(open('$cfg'));print(d.get('provider') or '')")"

  case "$provider" in
    deepseek) key=DEEPSEEK_API_KEY ;;
    qwen) key=DASHSCOPE_API_KEY ;;
    mimo) key=XIAOMI_MIMO_API_KEY ;;
    *) key="" ;;
  esac

  if [ -n "$key" ] && [ -z "${!key:-}" ]; then
    echo "skip $name ($key not set)"
    continue
  fi

  echo "==> running $name"
  "$BIN" bench --config "$cfg" --storage "$STORAGE" \
    --out "experiments/results/$name.json" \
    | tee "experiments/results/$name.md"
done

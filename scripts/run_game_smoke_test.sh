#!/usr/bin/env bash
# Boot the real game, start level 1, move Robbo, save PNG screenshots.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

export ROBBO_SMOKE_OUT="${ROBBO_SMOKE_OUT:-$ROOT/target/smoke}"
mkdir -p "$ROBBO_SMOKE_OUT"

run_smoke() {
  unset ARGV0
  if [[ "${ROBBO_SMOKE_MODE:-binary}" == "test" ]]; then
    cargo test -p robbo-app --test game_smoke -- --nocapture --test-threads=1
  else
    cargo run -p robbo-app -- --smoke-test
  fi
}

if command -v xvfb-run >/dev/null 2>&1; then
  echo "Running smoke test under xvfb-run …"
  xvfb-run -a -s "-screen 0 1280x720x24" bash -c "$(declare -f run_smoke); run_smoke"
else
  echo "xvfb-run not found — trying direct run (needs a display/GPU) …"
  run_smoke
fi

echo "Done. Screenshots in: $ROBBO_SMOKE_OUT"

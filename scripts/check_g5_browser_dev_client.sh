#!/bin/sh
# Manual evidence run, not part of ./dev check: the Wasm DevClient end to end in a browser page.
# Starts thrum-cook watch --serve, then wasm-bindgen-test-runner in interactive mode at
# http://127.0.0.1:8000; open that page in Chrome. Interactive mode reports E2E_READY, E2E_DONE
# and the test result in the page and its console only, so whoever watches the page:
#   - creates "$work/edit-now" after E2E_READY (this script then edits one source), and
#   - creates "$work/stop" once the page shows the result (or the time limit stops the run).
# Usage: sh scripts/check_g5_browser_dev_client.sh [seconds]
set -eu
limit=${1:-180}
repo=$(cd "$(dirname "$0")/.." && pwd)
work="$repo/tmp/g5-browser-client/page-$(date +%s)"
mkdir -p "$work"
PATH="$(dirname "$(rustup which --toolchain 1.99.1 rustc)"):$PATH"
export PATH
cd "$repo"
if lsof -nP -iTCP:8000 -sTCP:LISTEN >/dev/null 2>&1; then
  echo "port 8000 is in use; stop the previous runner first" >&2
  exit 1
fi
cargo build -p thrum-cook --offline --quiet
cook="$repo/target/debug/thrum-cook"
"$cook" demo "$work/src" >/dev/null
"$cook" watch "$work/src" "$work/store" --serve 127.0.0.1:0 >"$work/watch.log" 2>&1 &
watcher=$!
runner=""
stop() {
  # cargo starts wasm-bindgen-test-runner as its child; stop that too so port 8000 is released.
  if [ -n "$runner" ]; then
    pkill -P "$runner" 2>/dev/null || true
    kill "$runner" 2>/dev/null || true
  fi
  kill "$watcher" 2>/dev/null || true
}
trap stop EXIT
for _ in $(seq 1 100); do
  if grep -q '^serving' "$work/watch.log" && grep -q 'generation 1' "$work/watch.log"; then break; fi
  sleep 0.2
done
origin=$(sed -n 's/^serving //p' "$work/watch.log")
if [ -z "$origin" ] || ! grep -q 'generation 1' "$work/watch.log"; then
  echo "the dev server did not publish generation 1 within 20 s" >&2
  cat "$work/watch.log" >&2
  exit 1
fi
echo "server $origin"
echo "work $work"
THRUM_G5_DEV_SERVER="$origin"
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner
NO_HEADLESS=1
export THRUM_G5_DEV_SERVER CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER NO_HEADLESS
cargo test --target wasm32-unknown-unknown -p thrum-asset --test dev_server_browser --offline --no-run --quiet
cargo test --target wasm32-unknown-unknown -p thrum-asset --test dev_server_browser --offline >"$work/runner.log" 2>&1 &
runner=$!
echo "page http://127.0.0.1:8000"
elapsed=0
while [ "$elapsed" -lt "$limit" ] && [ ! -f "$work/stop" ]; do
  if [ -f "$work/edit-now" ]; then
    python3 -I -c "import sys;p=sys.argv[1];b=bytearray(open(p,'rb').read());b[-1]^=1;open(p,'wb').write(b)" "$work/src/cloud.tvol"
    rm "$work/edit-now"
    echo "edited cloud.tvol"
  fi
  if ! kill -0 "$runner" 2>/dev/null; then
    echo "the runner exited early" >&2
    break
  fi
  sleep 1
  elapsed=$((elapsed + 1))
done
[ "$elapsed" -lt "$limit" ] || echo "time limit reached" >&2
echo "--- runner log"
cat "$work/runner.log"
echo "--- watch log"
cat "$work/watch.log"

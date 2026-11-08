#!/bin/sh
# End-to-end in a browser page: thrum-cook watch --serve, then the Wasm DevClient served by
# wasm-bindgen-test-runner in interactive mode at http://127.0.0.1:8000. Open that page in Chrome;
# once it logs E2E_READY, edit one source (this script does it when the file
# "$work/edit-now" appears). Everything stops after the time limit. Usage: sh scripts/check_g5_browser_dev_client.sh [seconds]
set -eu
limit=${1:-180}
repo=$(cd "$(dirname "$0")/.." && pwd)
work="$repo/tmp/g5-browser-client/page-$(date +%s)"
mkdir -p "$work"
PATH="$(dirname "$(rustup which --toolchain 1.99.1 rustc)"):$PATH"
export PATH
cd "$repo"
cargo build -p thrum-cook --offline --quiet
cook="$repo/target/debug/thrum-cook"
"$cook" demo "$work/src" >/dev/null
"$cook" watch "$work/src" "$work/store" --serve 127.0.0.1:0 >"$work/watch.log" 2>&1 &
watcher=$!
runner=""
trap 'kill $watcher $runner 2>/dev/null || true' EXIT
for _ in $(seq 1 100); do
  if grep -q '^serving' "$work/watch.log" && grep -q 'generation 1' "$work/watch.log"; then break; fi
  sleep 0.2
done
origin=$(sed -n 's/^serving //p' "$work/watch.log")
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
while [ "$elapsed" -lt "$limit" ]; do
  if [ -f "$work/edit-now" ]; then
    python3 -I -c "import sys;p=sys.argv[1];b=bytearray(open(p,'rb').read());b[-1]^=1;open(p,'wb').write(b)" "$work/src/cloud.tvol"
    rm "$work/edit-now"
    echo "edited cloud.tvol"
  fi
  sleep 1
  elapsed=$((elapsed + 1))
done
echo "--- runner log"
cat "$work/runner.log"
echo "--- watch log"
cat "$work/watch.log"

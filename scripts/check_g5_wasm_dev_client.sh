#!/bin/sh
# End-to-end: thrum-cook watch --serve, then the Wasm DevClient under wasm-bindgen-test-runner (Node).
set -eu
repo=$(cd "$(dirname "$0")/.." && pwd)
work="$repo/tmp/g5-browser-client/run-$(date +%s)"
mkdir -p "$work"
PATH="$(dirname "$(rustup which --toolchain 1.99.1 rustc)"):$PATH"
export PATH
cd "$repo"
cargo build -p thrum-cook --offline --quiet
cook="$repo/target/debug/thrum-cook"
"$cook" demo "$work/src" >/dev/null
"$cook" watch "$work/src" "$work/store" --serve 127.0.0.1:0 >"$work/watch.log" 2>&1 &
watcher=$!
trap 'kill $watcher 2>/dev/null || true' EXIT
for _ in $(seq 1 100); do
  if grep -q '^serving' "$work/watch.log" && grep -q 'generation 1' "$work/watch.log"; then break; fi
  sleep 0.2
done
origin=$(sed -n 's/^serving //p' "$work/watch.log")
echo "server $origin"
THRUM_G5_DEV_SERVER="$origin"
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner
export THRUM_G5_DEV_SERVER CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER
cargo test --target wasm32-unknown-unknown -p thrum-asset --test dev_server_wasm --offline --no-run --quiet
# Edit one source after the client's first load (it starts within a second of the runner).
(
  sleep 6
  python3 -I -c "import sys;p=sys.argv[1];b=bytearray(open(p,'rb').read());b[-1]^=1;open(p,'wb').write(b)" "$work/src/cloud.tvol"
  echo "edited cloud.tvol"
) &
cargo test --target wasm32-unknown-unknown -p thrum-asset --test dev_server_wasm --offline 2>&1 | tee "$work/test.log"
echo "--- watch log"
cat "$work/watch.log"

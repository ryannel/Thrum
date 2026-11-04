#!/bin/sh
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd)
cd "$repo_root"
rustc_path=$(rustup which rustc --toolchain 1.99.1)
RUSTC="$rustc_path" rustup run 1.99.1 cargo build -p v1-scene --lib --release --target wasm32-unknown-unknown --locked --offline
build_root=${CARGO_TARGET_DIR:-target}
mkdir -p "$build_root/v1-scene-web"
if command -v wasm-bindgen >/dev/null 2>&1; then
  bindgen_bin=wasm-bindgen
else
  bindgen_bin="${CARGO_HOME:-$HOME/.cargo}/bin/wasm-bindgen"
fi
"$bindgen_bin" --target web --out-dir "$build_root/v1-scene-web" "$build_root/wasm32-unknown-unknown/release/v1_scene.wasm"
cp examples/v1-scene/web/index.html "$build_root/v1-scene-web/index.html"
printf 'Run: python3 -m http.server 8765 --bind 127.0.0.1 --directory "%s/v1-scene-web"\n' "$build_root"

#!/bin/sh
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd)
cd "$repo_root"
# `--dev` builds the development page into v1-game-web-dev, with the material hook reload
# (`?material=`), which compiles shaders synchronously. The default page cannot start it.
features=
web=v1-game-web
if [ "${1:-}" = "--dev" ]; then
  features="--features dev-material"
  web=v1-game-web-dev
elif [ -n "${1:-}" ]; then
  echo "usage: build.sh [--dev]" >&2
  exit 2
fi
rustc_path=$(rustup which rustc --toolchain 1.99.1)
# shellcheck disable=SC2086 # $features is empty or one flag and its value
RUSTC="$rustc_path" rustup run 1.99.1 cargo build -p v1-game --lib --release --target wasm32-unknown-unknown --locked --offline $features
build_root=${CARGO_TARGET_DIR:-target}
mkdir -p "$build_root/$web"
if command -v wasm-bindgen >/dev/null 2>&1; then
  bindgen_bin=wasm-bindgen
else
  bindgen_bin="${CARGO_HOME:-$HOME/.cargo}/bin/wasm-bindgen"
fi
"$bindgen_bin" --target web --out-dir "$build_root/$web" "$build_root/wasm32-unknown-unknown/release/v1_game.wasm"
cp examples/v1-game/web/index.html "$build_root/$web/index.html"
cp crates/thrum-controls/web/thrum-controls.js "$build_root/$web/thrum-controls.js"
cp examples/v1-game/params/*.params.json "$build_root/$web/"
if [ -n "$features" ]; then
  cp examples/v1-game/materials/*.material.wgsl "$build_root/$web/"
else
  rm -f "$build_root/$web/"*.material.wgsl
fi
printf 'Run: python3 -m http.server 8765 --bind 127.0.0.1 --directory "%s/%s"\n' "$build_root" "$web"

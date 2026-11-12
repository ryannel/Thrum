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
build_root=${CARGO_TARGET_DIR:-target}
# Each page compiles into its own Cargo target folder. Cargo's lock ends with the build, before
# wasm-bindgen reads the Wasm, so a shared release/v1_game.wasm would let a concurrent build of
# the other page swap in its Wasm: the hook could reach the default page, or leave the dev page.
cargo_target=$build_root
if [ -n "$features" ]; then
  cargo_target=$build_root/v1-game-dev-material
fi
rustc_path=$(rustup which rustc --toolchain 1.99.1)
# shellcheck disable=SC2086 # $features is empty or one flag and its value
RUSTC="$rustc_path" rustup run 1.99.1 cargo build -p v1-game --lib --release --target wasm32-unknown-unknown --locked --offline --target-dir "$cargo_target" $features
mkdir -p "$build_root/$web"
if command -v wasm-bindgen >/dev/null 2>&1; then
  bindgen_bin=wasm-bindgen
else
  bindgen_bin="${CARGO_HOME:-$HOME/.cargo}/bin/wasm-bindgen"
fi
"$bindgen_bin" --target web --out-dir "$build_root/$web" "$cargo_target/wasm32-unknown-unknown/release/v1_game.wasm"
cp examples/v1-game/web/index.html "$build_root/$web/index.html"
cp examples/v1-game/web/temporal.html "$build_root/$web/temporal.html"
cp crates/thrum-render/src/synthetic/temporal/witness.json "$build_root/$web/temporal-witness.json"
cp examples/v1-game/web/route.html "$build_root/$web/route.html"
cp examples/v1-game/web/capture.html "$build_root/$web/capture.html"
cp examples/v1-game/web/capture.mjs "$build_root/$web/capture.mjs"
cp examples/v1-game/routes/*.json "$build_root/$web/"
cp crates/thrum-controls/web/thrum-controls.js "$build_root/$web/thrum-controls.js"
cp examples/v1-game/params/*.params.json "$build_root/$web/"
if [ -n "$features" ]; then
  cp examples/v1-game/materials/*.material.wgsl "$build_root/$web/"
else
  rm -f "$build_root/$web/"*.material.wgsl
fi
printf 'Run: python3 -m http.server 8765 --bind 127.0.0.1 --directory "%s/%s"\n' "$build_root" "$web"

#!/bin/sh
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd)
cd "$repo_root"
# `--dev` builds the development page into v1-game-web-dev, with the material hook reload
# (`?material=`), which compiles shaders synchronously. The default page cannot start it.
mode=${1:-}
case "$mode" in
  "") web=v1-game-web; set -- --no-default-features ;;
  --proving) web=v1-game-web-proving; set -- --features proving ;;
  --dev) web=v1-game-web-dev; set -- --features dev-material ;;
  *) echo "usage: build.sh [--proving|--dev]" >&2; exit 2 ;;
esac
build_root=${CARGO_TARGET_DIR:-target}
# A qualification can retain its own page while continuing to share Cargo's build cache.
web_output=${THRUM_V1_WEB_OUTPUT:-"$build_root/$web"}
# Each page compiles into its own Cargo target folder. Cargo's lock ends with the build, before
# wasm-bindgen reads the Wasm, so a shared release/v1_game.wasm would let a concurrent build of
# the other page swap in its Wasm: the hook could reach the default page, or leave the dev page.
cargo_target=$build_root
case "$mode" in
  --proving) cargo_target=$build_root/v1-game-proving ;;
  --dev) cargo_target=$build_root/v1-game-dev-material ;;
esac
rustc_path=$(rustup which rustc --toolchain stable)
RUSTC="$rustc_path" rustup run stable cargo build -p v1-game --lib --release --target wasm32-unknown-unknown --locked --offline --target-dir "$cargo_target" "$@"
mkdir -p "$web_output"
if command -v wasm-bindgen >/dev/null 2>&1; then
  bindgen_bin=wasm-bindgen
else
  bindgen_bin="${CARGO_HOME:-$HOME/.cargo}/bin/wasm-bindgen"
fi
"$bindgen_bin" --target web --out-dir "$web_output" "$cargo_target/wasm32-unknown-unknown/release/v1_game.wasm"
page_variant=proving
if [ -z "$mode" ]; then
  page_variant=shipping
fi
# Both builds share the page sources. Emit only the blocks for the selected variant.
for page in index route; do
  awk -v variant="$page_variant" '
    /^[[:space:]]*\/\/ @proving$/ { section = "proving"; next }
    /^[[:space:]]*\/\/ @shipping$/ { section = "shipping"; next }
    /^[[:space:]]*\/\/ @end$/ { section = ""; next }
    !section || section == variant
  ' "examples/v1-game/web/$page.html" > "$web_output/$page.html"
done
if [ -n "$mode" ]; then
  cp examples/v1-game/web/temporal.html "$web_output/temporal.html"
  cp crates/thrum-app/web/interface-input.js "$web_output/interface-input.js"
  cp examples/v1-game/web/capture.html examples/v1-game/web/capture.mjs \
    examples/v1-game/web/capture_upload.mjs "$web_output/"
  cp examples/v1-game/routes/*.json "$web_output/"
  witness=$(mktemp "$web_output/.temporal-witness.XXXXXX")
  trap 'rm -f "$witness"' EXIT
  RUSTC="$rustc_path" rustup run stable cargo run --locked --offline --quiet \
    -p thrum-render --example temporal_witness --target-dir "$build_root" > "$witness"
  chmod a+r "$witness"
  mv "$witness" "$web_output/temporal-witness.json"
else
  rm -f "$web_output/temporal.html" "$web_output/temporal-witness.json" "$web_output/interface-input.js"
  rm -f "$web_output/capture.html" "$web_output/capture.mjs" "$web_output/capture_upload.mjs"
  for route in examples/v1-game/routes/*.json; do
    name=${route##*/}
    if [ "$name" != exit-flythrough.json ]; then
      rm -f "$web_output/$name"
    fi
  done
  cp examples/v1-game/routes/exit-flythrough.json "$web_output/"
fi
cp crates/thrum-controls/web/thrum-controls.js "$web_output/thrum-controls.js"
cp crates/thrum-app/web/host-page.js "$web_output/host-page.js"
cp examples/v1-game/params/*.params.json "$web_output/"
if [ "$mode" = "--dev" ]; then
  cp examples/v1-game/materials/*.material.wgsl "$web_output/"
else
  rm -f "$web_output/"*.material.wgsl
fi
printf 'Run: python3 -m http.server 8765 --bind 127.0.0.1 --directory "%s"\n' "$web_output"

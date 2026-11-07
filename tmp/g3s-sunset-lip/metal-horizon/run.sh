#!/bin/bash
set -euo pipefail
probe=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$probe/../../.." && pwd)
worktree=${THRUM_HORIZON_WORKTREE:-$root}
tests=crates/thrum-render/src/visibility/pyramid/tests.rs
dest=${tests%.rs}/horizon_probe.rs
cd "$worktree"
test -z "$(git status --porcelain -- "$tests" "$dest")"
mkdir -p "$(dirname "$dest")"
cp "$tests" "$probe/tests.rs.backup"
trap 'cp "$probe/tests.rs.backup" "$tests"; rm -f "$dest" "$probe/tests.rs.backup"; rmdir "$(dirname "$dest")" 2>/dev/null || true' EXIT
cp "$probe/horizon_probe.rs" "$dest"
printf '\nmod horizon_probe;\n' >> "$tests"
export THRUM_METAL_HORIZON=$probe
export CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-$root/target/g3s-stars-valley}
export RUSTC_WRAPPER=
cargo test --offline --locked -p thrum-render --lib metal_flat_horizon -- --nocapture

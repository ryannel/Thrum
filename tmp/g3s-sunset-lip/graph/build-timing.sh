#!/bin/bash
set -euo pipefail
p=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$p/../../.." && pwd)
cd "${THRUM_HORIZON_GRAPH_WORKTREE:-$root}"
synthetic=crates/thrum-render/src/synthetic.rs
capture=examples/v1-game/src/bin/window/capture.rs
scratch=$(mktemp -d "$p/timing-build.XXXXXX")
cp "$synthetic" "$scratch/synthetic.rs"
cp "$capture" "$scratch/capture.rs"
restore() {
    cp "$scratch/synthetic.rs" "$synthetic"
    cp "$scratch/capture.rs" "$capture"
    rm "$scratch/synthetic.rs" "$scratch/capture.rs"
    rmdir "$scratch"
}
trap restore EXIT
python3 - "$p" "$synthetic" "$capture" <<'PY'
from pathlib import Path
import sys
scratch, synthetic, capture = map(Path,sys.argv[1:])
s=synthetic.read_text()
anchor='        let horizon_enabled = shadow_enabled'
assert s.count(anchor)==1
synthetic.write_text(s.replace(anchor,
    '        let horizon_enabled = std::env::var_os("THRUM_PROBE_HORIZON_TAPS").is_none()\n            && shadow_enabled'))
s=capture.read_text()
anchor='    std::fs::create_dir_all(&output)'
assert s.count(anchor)==1
s=s.replace(anchor,'''    if let Some(row_file) = std::env::var_os("THRUM_PROBE_HORIZON_TIMING") {
        let content = content.as_ref().ok_or("timing route requires cooked content")?;
        return timed_route(&mut host, &device, Path::new(&row_file), || {
            let stats = content.lock().map_err(|_| "timing observations poisoned")?;
            if !stats.errors.is_empty() {
                return Err(format!("timing content errors: {:?}", stats.errors));
            }
            Ok((stats.drawn, stats.submitted_frames))
        });
    }
''' + anchor)
capture.write_text(s+'\n'+(scratch/'timing-fragment.rs').read_text())
PY
export RUSTC_WRAPPER=
export CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-$root/target/g3s-stars-valley}
cargo build --offline --locked --release -p v1-game --bin v1-game
cp "$CARGO_TARGET_DIR/release/v1-game" "$p/timing-game"

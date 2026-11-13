# Reproduce the 13 November M2 proof

Run from the repository root. The source checkpoint is `7a791b20dcd2c70b2537661b123c0d13978bd9f9`.
The retained assembly and bulk sources are in `tmp/mac-m2/current-20261113`. The executable is
`v1-game`, and the cooked store is `valley`. [Build map](build.json.gz), [retained masters](retained-masters.json),
[source-check result](check.json), [exact-baseline result](exact-baselines.json.gz).

## Inspect retained evidence without a new recording

Both witness commands must exit zero. The historical known-answer commands exit two because their retained
rows do not span the marked interval. Their three known misses remain required answers. The tests cover
missing/duplicate ordinals, zero/absent times, unfamiliar status, bad conversion, wrong PID, callback-time
substitution, truncated coverage, false image/control joins, and permission attribution.

```sh
python3 scripts/mac_m2_presented.py --input tmp/mac-m2/current-20261113/presented/off-traced/presented.xml --arm tmp/mac-m2/current-20261113/presented/off-traced --output tmp/mac-m2/current-20261113/presented/off-traced/witness.json
python3 scripts/mac_m2_presented.py --input tmp/mac-m2/current-20261113/presented/on-traced/presented.xml --arm tmp/mac-m2/current-20261113/presented/on-traced --output tmp/mac-m2/current-20261113/presented/on-traced/witness.json
python3 scripts/mac_m2_presented.py --input docs/evidence/PLATFORM-RUNTIME-MAC/m2-presented-off-traced.tsv.gz --arm docs/evidence/render/mac-m2-20261111/off-traced --output tmp/mac-m2/current-20261113/known-off.json
python3 scripts/mac_m2_presented.py --input docs/evidence/PLATFORM-RUNTIME-MAC/m2-presented-on-traced.tsv.gz --arm docs/evidence/render/mac-m2-20261111 --output tmp/mac-m2/current-20261113/known-on.json
python3 -m unittest discover -s scripts -p 'test_mac_m2*.py' -v
python3 docs/evidence/render/mac-m2-20261113/analyse.py --motion tmp/mac-m2/current-20261113/motion/session --window tmp/mac-m2/current-20261113/window --presented tmp/mac-m2/current-20261113/presented --output tmp/mac-m2/current-20261113/analysis.json
python3 docs/evidence/render/mac-m2-20261113/cadence.py --root tmp/mac-m2/current-20261113/presented --output tmp/mac-m2/current-20261113/cadence.json
```

The committed XML exports are gzip-compressed. To rerun the witness from committed files alone, expand
`presented.xml.gz` and `signposts.xml.gz` into a fresh directory and use that directory with the other retained
arm tables and window metadata. The witness resolves compressed TSVs automatically. `analyse.py` accepts the
committed compressed JSON/TSV records directly. `cadence.py` reads the expanded GPU/iostat records in the local
masters and handles the current `SceneGpuTimings` named-pass format. Its sum includes the graph's surface
present query, so the separately reported present query must not be added again.

The source decoder and package commands below were run on every original. They verify independent zstd/XOR
reconstruction against pre-encoding hashes, image/request/surface joins, and the CPU channel/row oracle.
The replacement arms explicitly retain their three retired readbacks as capture gaps.

```sh
python3 scripts/capture_v1_runtime.py --output tmp/mac-m2/current-20261113/motion/session --finalize
python3 scripts/package_mac_m2_window.py --output tmp/mac-m2/current-20261113/window/grid-replacement
python3 scripts/package_mac_m2_window.py --output tmp/mac-m2/current-20261113/window/valley-replacement
python3 scripts/package_mac_m2_window.py --output tmp/mac-m2/current-20261113/presented/on-untraced
python3 scripts/package_mac_m2_window.py --output tmp/mac-m2/current-20261113/presented/on-traced
```

## Recorded collection

The collector now requires an explicit `--identity` from freshly collected same-build window proofs for
presented arms. `--window-only` collects the untraced window proofs without entering the exhausted historical
diagnostic commission. The merged release was rebuilt with `cargo build --locked --offline --release -p v1-game
-p thrum-cook`, copied outside disposable target output, and its freshly cooked content was pinned before M2.1.
The permission-denied launcher collected the elapsed-time pan. Window qualification followed under `gpu`, using
the same binary/store and `--window-only`. The screenshot observer's read-only preflight returned granted under
its existing identity, while M2.1's four attributed preflights returned denied.

After inspecting the window proofs and reproducing M2.4, the builder recorded OFF first to establish M2.5.
Then ON/untraced and ON/traced completed the matched set. OFF also serves as M2.2's OFF arm, as the plan permits.
Both traces use `--presented-minimal --presented-deferred`, a 16-second recording limit and 75-second
finalization limit. Each export is bounded to 30 seconds. The exact commands, process timing and environments
are retained per arm. Both TOCs say Deferred. Each fresh export identifies only Built-In Display 1, main,
3456×2234, maximum refresh 120 Hz. The game window independently reports that panel and 1920×1080 BGRA output.
The zero-row kernel display tables are recorded in each arm's `row-counts.json`.

The durable counter is `tmp/mac-m2/presented-commission-20261111.json`. It now contains all six attempts,
with the first four preserved and the fresh OFF/ON recordings fifth/sixth. This milestone has no remaining
recording allowance. The saved [counter](recording-commission.json) and per-arm invocation files retain the
commission and settings. A new output directory does not renew it.

## Visual inspection and timing investigation

The agent inspected native full-resolution source selections, native crops, the offscreen contact sheet,
10 fps pan strips from the ON movies, actual-window screenshot pairs and complete browser playback at 1×
in a 1920×1200 viewport. The page's video element was 1920×1080. The five playback reports retain rate, duration,
wall time and decoder drops. Observations were sampled screenshots alongside complete playback and source
verification. Continuous physical-panel viewing is unmeasured. The ON/traced browser playback dropped six
frames while all 846 original source images verified. The other four playback reports show zero decoder drops.

The authored AppKit sweep maps elapsed seconds 4–6 to yaw 0–36 through the ordinary slider target/action.
The final accepted value is 35.999988556° after the normal float conversion. Marker emission is polled at the
next callback after each nominal boundary. The ON marker spans of 2.000409208 and 1.996285916 seconds retain that
polling jitter, and every surrounding image and normal input record is preserved. Initial/final submitted-image
holds exceed one second in both ON arms. The offscreen route separately proves the complete two-second elapsed
pan and endpoint holds under its own monotonic opportunity clock.

Every strict over-period OFF and ON interval appears in `cadence.json.gz`, including deviations smaller than
the conversion error. Four OFF long intervals span 16.666667, 20.832417, 12.500041 and 16.666708 ms. Their
present-call gaps are 8.238958, 8.178458, 9.023125 and 7.562292 ms. The graph-query sums at their destination
drawables are 3.262, 3.747, 3.313 and 4.040 ms. Several later host render-duration samples rise to 8–17 ms.
This lag is consistent with bounded presentation backpressure. It remains an inference. The measurements
do not isolate an instantaneous scheduling, driver or compositor cause.

ON/traced has four long OS intervals under capture, and the source-image gap at 569→570 is 23.815875 ms.
The destination host render duration is 17.042709 ms, while its named GPU-query sum is 4.034125 ms. Requested
and accepted yaw continue monotonically, and source bytes change across that gap. Neighboring ordinals, per-pass
queries, opportunity/target times, present calls and host load are retained. Host render duration includes waits.
Run-wide iostat samples include trace export/finalization, and the process CPU snapshot is lifetime CPU.
Background paired-sync simulator services appear in the OFF snapshot. These observations constrain causal
observer-cost attribution. No timing number is used as a performance gate.

## Retention and integration

Compressed originals, trace bundles, binary and cooked content remain locally under the current-run root.
`retained-masters.json` records hashes and sizes for the executable, compressed originals and trace members.
The motion manifest and per-arm archive receipts bind all source frames to their original hashes. These ignored
local masters have no declared durable backup. Git holds selected native images, full host-time movies,
source/build/content manifests, complete exported tables, analyses and check output. Preserve the masters and
both historical recording counters through independent review. Reconstructible PNG expansions may be removed
after retained-root verification and review. The builder's source checkpoint passed GPU-locked `./dev check`
and all registered exact baselines. The final item/check audit runs against the evidence commit before the
required `agent_loop.py merge --worktree .` integration.

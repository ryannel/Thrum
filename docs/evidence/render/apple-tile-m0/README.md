# APPLE-TILE M0 evidence, 11 November 2026

M0 proves the ordinary portable comparison path. The planner settled D0.2 on 11 November, assigning the
native copy/select fixture and its stateful twin to M10. The retained Mac/Chrome proof from `03bc878a`
covers the corrected M0 scope. All five items pass, and M0 is claimed for lead review. [The resumption audit](resumed-proof-audit.json) verifies the retained delivery
and records the intervening source changes. Renderer, shaders, V1 scene and capture runtime are unchanged.

## Reproduce the Mac proof

Build the real game with its existing diagnostic feature and cook the checked showcase source, then run the
single qualification command under the GPU lock. Use a fresh output directory and at least 16 GiB free.
Set `ORCHESTRATE_LOOP` to the installed `build/scripts/agent_loop.py` from the build-milestone skill.

```sh
cargo build --locked --offline --release -p v1-game --features draw-probe
cargo build --locked --offline --release -p thrum-cook
target/release/thrum-cook cook examples/v1-game/assets/showcase tmp/apple-tile-m0/valley
python3 "${ORCHESTRATE_LOOP:?Set ORCHESTRATE_LOOP}" lock gpu --worktree . -- python3 scripts/apple_tile_m0.py --binary target/release/v1-game --store tmp/apple-tile-m0/valley --output tmp/apple-tile-m0/qualification
```

The retained successful execution used `tmp/g3t-m0/tools/v1-game`, SHA-256
`743ecb920f32237a722d034fe5bf6d5b318cbb2bf908492c9c56b42793cf881f`, and
`tmp/g3t-m0/valley`. Its [qualification record](qualification.json.gz) pins the source revision and dirty
patch digest, every source hash, Rust 1.99.1/compiler, binary, unchanged shaders, cooked root, route,
Metal adapter, source format, extent, H=1, exact GPU exposure words and submitted sample/history identities.
[The command log](checks/qualification-2.log.gz) records success. Missing and mismatched cooked roots are
refused before capture. [Missing-root log](checks/refuse-missing-store.log.gz),
[mismatched-root log](checks/refuse-mismatch-store.log.gz).

The recorded linear manifests have a documented [row-stride metadata correction](linear-stride-erratum.json).
Their source bytes, eight-byte pixel format, decoder size and comparisons are correct. The retained manifests
preserve the original value and hash. The command now derives that field from the selected source format.

## Mac correctness and visual inspection

- Both portable fixed runs log all 361 routed ticks, including the authored cut. Five complete 1280×720
  checkpoints repeat exactly in RGBA8 and raw RGBA16Float, including alpha/half bits. Source/sample/exposure
  identities match. Full baselines and identity sidecars are in [native-baseline](native-baseline).
- The current tone-to-Rgba8Unorm-to-present path repeats exactly for Bgra8Unorm and Bgra8UnormSrgb at a
  1000×800 destination from a 1280×720 source. Full stored BGRA controls retain scaling and letterboxing.
  The surface copy is a separately labelled diagnostic observation after the ordinary presenter submission.
- Phase mismatch, cooked placement omission, mirror slot 5060 omission and omission of one nonzero weak
  light in the existing overflowing CPU cluster all fail exact comparison. At the mirror/light checkpoint,
  respectively 696,207, 650,623, 13,344 and 104,594 pixels change. Both overflow arms retain dropped references.
  Night-star omission changes 12,231 SDR pixels and 18,252 linear pixels. Each has a native-size mask/crop in
  [comparisons](comparisons), with reference/candidate content crops in [controls](controls).
- T2 settles full 1920×1080 Sun-side and night stills in both formats, with stars on/off controls. The shared
  realtime host driver records 964 route images over 8.036 s, source ticks 179–600 and the authored history cut,
  plus 481 day/night/day variant images over 4.011 s. Every image has its actual GPU exposure packet.
  Both sessions have zero tap/writer omissions. All lossless originals pass independent decoded SHA-256 checks.
  Full records, selected native frames and host-time 1× previews are in [motion](motion).
- Builder inspection covered full Sun-side/near/far valley, sphere and cooked rock, mirror/local-light cubes,
  their shadows, the cut neighbourhood and night stars/occlusion, plus native omission crops. The 1× route
  playback advances through the valley and the intended cut, then holds the final close view. The variant
  source contains the noon/night/noon switch and continuous camera pan. Movie encoding is a viewing aid.

The original full Mac SDR still predates this capture extension and is retained in
[prechange-mac](prechange-mac), including its exact post-extension comparison.
The linear extension then captured the ordinary renderer
before M1 changes. No WGSL arithmetic, fusion or tone quantization changed. This record establishes
same-target bytes and elapsed-host-time engine output. Panel cadence, physical traffic and display output
remain separately qualified in their owning milestones.

## Chrome

The original and repeat pre-change T2 fixed captures each contain 481 full 1280×720 submitted RGBA images.
Every decoded hash and deterministic sample/history identity repeats. A fresh default Wasm build with the
M0 changes also matches all 481 pre-change images exactly. [Current comparison](chrome-baseline/current-comparison.json),
[pre-change repeat comparison](chrome-baseline/repeat-comparison.json).
Compressed provenance/runtime records and five selected full frames are in [chrome-baseline](chrome-baseline).
The complete original sources remain in `tmp/g3t-m0/prechange-chrome-fixed/frames` through accepted T2 M1.

`python3 scripts/run_v1_showcase.py moving` also passes its named endpoint/no-history smoke on current source,
with two native/two actual Chrome runs, per-tick identity and checkpoint repeats. Its comparator and route are
preserved. This smoke has its original functional scope. The full T2 capture establishes submitted output.

## Diagnostics and checks

D0.3's named creation-counter tests run on this Metal device. They cover renderer/cooked creation sites,
thread/phase, generation clones, detached preparation, replacement and unscoped play rejection. Default
creation remains direct, with no diagnostic submit lock. Native-object producers and instrumentation are absent.
Live-object deltas remain distinct from creation events. [Earlier detailed log](creation.log.gz).

D0.4 retains the [Xcode traffic probe](../G3T-TRAFFIC-PROBE-2026-11-08.md), including its actual per-encoder
Bytes Read From / Written To Device Memory counters and units, one-replay/shared-load limits and unavailable
raw scratch. Uncaptured rows remain unmeasured. Logical attachment bytes retain their separate meaning.

The exact comparator accepts PNG, PPM, raw RGBA8/BGRA8 and raw RGBA16Float. It compares stored bytes without
float conversion or tolerance. Its caller establishes sample/exposure identity. Exit 0 means equality,
1 means changed pixels and 2 means refusal. It requires a new output directory.

```sh
python3 scripts/compare_v1_pixels.py --reference baseline.rgba16f --candidate repeat.rgba16f --encoding rgba16f --extent 1280 720 --output tmp/apple-tile-m0/new-comparison
```

The Python suite passes 17 tests. Scoped release Clippy, capture lifecycle/GPU channel/linear star-graph tests
pass. The first full check exposed an old graph assertion that omitted the new exposure-to-readback edge.
That capture-only assertion was corrected. The required milestone gate passes all three executable checks, including the full repository gate.
[Final check record](checks/milestone-check-final.json.gz) and [current moving smoke](checks/moving-current-run.json.gz)
retain their target results. Earlier failing assertion logs remain in [checks](checks).

The resumed milestone gate passes the current Metal creation command, actual Chrome moving-route command
and full repository check. [Resumed gate](checks/resumed-milestone-check.json.gz),
[current route record](checks/resumed-moving-run.json.gz). The corrected D0.2 scope is the basis for this claim.

## Retention

`tmp/g3t-m0/qualification-2` owns the selected complete lossless sources and all attempt metadata through lead
review. For reconstruction, copy a session's `manifest.json`, `runtime.json` and verified `frames.zst` into a
new task directory, then use the existing T2 `--finalize` there. The new directory avoids existing preview
files and preserves the captured manifest hashes.
The binary, cooker, cooked store and earlier pre-change baselines are retained there or in its parent.
The selected exact baselines, controls, provenance, crops and previews are versioned here.
[Retained-file hashes](retained-files.json) record the delivery. Local bulk sources have no remote backup.
Owned superseded captures and verified duplicate expansions were removed after replacement/source verification.
`tmp/g3t-m0/STORAGE.md` and cleanup manifests record ownership and deletions. The initial storage refusals and
first command's stale return-field failure remain recorded in the attempt history.

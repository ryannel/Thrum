# APPLE-TILE M0 partial evidence, 10 November 2026

Source `d5dcb89f`, pinned Rust 1.99.1, this M3 Max/mac-metal. Renderer source and shaders were unchanged.
M0 remains open. D0.1, D0.2 and D0.5 remain open.

- D0.3 passed `cargo test --locked --offline -p thrum-render --features creation-counters creation -- --nocapture`
  under the orchestrate GPU lock outside the sandbox. [Complete log](creation.log.gz) records 12 passed,
  zero failed and the intentionally ignored release measurement. The device helper asserts Metal on macOS.
  Production renderer/cooked creation-site coverage, cloned generation sites, detached preparation attribution,
  replacement isolation and rejected unscoped play creation passed. Pipeline events retain thread/phase;
  live-object net deltas are separately labelled and may be unattributed. Native producers remain absent.
  Source inspection of `runtime/creation_off.rs` confirms direct default wgpu creation calls and empty ledger/regions.
- D0.4 passed inspection of the retained [Xcode replay probe](../G3T-TRAFFIC-PROBE-2026-11-08.md).
  The counter is Bytes Read From / Written To Device Memory, per encoder, in bytes/KiB/MiB.
  Logical attachment bytes remain separate. One replay per trace, shared GPU/load, unavailable original scratch
  and unmeasured route/day/device rows are explicitly recorded. This inspection adds no replay or traffic result.
- T2 runtime capture refused before allocation with `less than declared 16 GiB free space`.
  [Failed preflight log](capture-preflight.log.gz) came from
  `python3 scripts/capture_v1_runtime.py --binary target/release/v1-game --output tmp/g3t-m0/preflight --mode realtime`.
  Initial available disk was 14 GiB, falling to 13 GiB during builds. No image or motion evidence was produced.
  Retaining the required pre-change target baselines remains pending, together with the M0 scene-linear extension,
  prepared variants, exact comparator/omission controls and full-resolution present/letterbox controls.

The required `python3 scripts/run_v1_showcase.py moving` passed under the GPU lock outside the sandbox.
[Complete result](moving-run.json.gz) pins native/Wasm builds, cooked root, route, checkpoint hashes,
per-tick identity and all negative controls. Both native and Chrome ran twice. This is the existing G7
endpoint/no-history route smoke. It leaves D0.5 open pending the full-resolution pre-change T2 baseline.
Local endpoint sources are in `tmp/g7-showcase/runs/moving-20261110-235303`, retained through lead review.
No submitted temporal or elapsed-time motion qualification is claimed by that command.


Validation before commit passed `./dev check-docs` (621 Markdown files, 2863 links, 30 module records).
A structural comparison against HEAD confirmed only M0 item status/evidence changed in the done-when file.
Integration runs the profile's complete `./dev check` under its lock.

# APPLE-TILE M0 partial evidence, 10–11 November 2026

M0 and D0.1/D0.2/D0.5 remain open. Renderer/shader source is unchanged from `d5dcb89f`.
The earlier checkpoint `f66d05f4` landed after the full profile check passed. Rust is pinned to 1.99.1.

- D0.3 passed the named creation command under the GPU lock outside the sandbox on this Mac.
  The device helper asserts Metal. [Complete log](creation.log.gz) records 12 passed, zero failed and the
  intentionally ignored release measurement. Coverage includes renderer/cooked pipeline sites, generation clones,
  detached attribution, replacement isolation and the unscoped play guard. Pipeline events retain thread/phase.
  Live-object net deltas remain separately labelled, including unattributed windows. Native producers are absent.
  Source inspection of `runtime/creation_off.rs` confirms direct default creation and empty ledger/regions.
- D0.4 passed inspection of the retained [Xcode probe](../G3T-TRAFFIC-PROBE-2026-11-08.md).
  It names Bytes Read From / Written To Device Memory per encoder, in bytes/KiB/MiB. Logical attachment bytes,
  one-replay/shared-load limits, unavailable scratch and unmeasured route/day/device rows remain explicit.
- The [capture preflight](capture-preflight.log.gz) refused with `less than declared 16 GiB free space`.
  The command was `python3 scripts/capture_v1_runtime.py --binary target/release/v1-game --output tmp/g3t-m0/preflight --mode realtime`.
  Initial free space was 14 GiB. After worktree build cleanup it was 15 GiB, and the preflight still refused.
- `python3 scripts/run_v1_showcase.py moving` passed under the GPU lock.
  [Complete result](moving-run.json.gz) pins builds, cooked root, route, hashes, 361 per-tick identities and
  negative controls across two native/two Chrome runs. This remains endpoint/no-history route smoke.
  Local sources remain in `tmp/g7-showcase/runs/moving-20261110-235303` through lead review.

The implemented comparator checks every stored byte in PNG, PPM, raw RGBA8 and raw RGBA16F, retaining formats
and exact extent/channel matching. RGBA16F is compared without float conversion. Results record file/pixel hashes.
Differences produce native-size binary masks/crops and changed byte/pixel counts. Callers establish target,
sample, exposure and history identity separately.

```sh
python3 scripts/compare_v1_pixels.py --reference baseline.png --candidate repeat.png --encoding png --output tmp/g3t-m0/new-comparison
```

Raw inputs require `--extent WIDTH HEIGHT`. Exit 0 means identical pixels, 1 means differences, 2 means refusal.
Outputs require a new directory. [All ten endpoint repeats](pixel-repeats.json.gz) passed for native/Chrome RGB
checkpoints. An [edited-source one-byte control](one-byte-comparison.json) failed at (100, 100), with the correct
[native-size crop](one-byte-difference-crop.png). The [capture suite](pixel-suite.log.gz) passed 16 tests including
alpha/half-float bit sensitivity, malformed input refusal and native difference coordinates.

Scene-linear tap integration, prepared variants, T2 baseline retention, actual renderer omission controls,
present/letterbox controls and full route/motion qualification remain pending. Documentation checks passed.
The done-when file retains only the earlier permitted D0.3/D0.4 status/evidence changes.

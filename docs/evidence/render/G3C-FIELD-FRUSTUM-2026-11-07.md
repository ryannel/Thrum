# G3c M1: frustum culling in the game's dense field

7 November 2026. Apple M3 Max/Metal, Rust 1.99.1, wgpu 30.0.1. Each section names the commit or Wasm build it
was gathered at. The first native rows and the first Chrome rows were gathered on branch `claude/g3c-gpu-culling`
on main `a346d60f`. The closing reruns come from the culling counts readback on branch `claude/g3c-stats-main`:
- the first ones on main `b8f230e7`, before root asked for counts to stop holding the render slot;
- the final ones at `a2cf00f3` on main `a3a973cc`, after that change. Its code equals the commit that adds this
  record; only this file differs.
Each section names which run it reports.

**Labelled contended:** other sessions shared the CPU and GPU throughout (load average 19–21), so p95 tails are
noisy.

## The field

`THRUM_SCENE=field` selects the V1 game's dense flythrough content (`examples/v1-game/src/field.rs`). It is built
only through public interfaces: the simulation seed, `SyntheticScene`, `SceneSource` and `FrameHost::new_with_setup`.

| Contents | Count | Detail |
| --- | ---: | --- |
| Ground slab | 1 | 840 m square |
| Walls | 512 | 20 × 6 × 1 m; they stand between prop rows |
| Props | 100,000 | Boxes of varied size and yaw on a 2.5 m lattice, about 790 m square; every twentieth one drifts at 0.8 m/s |
| **Total** | **100,513** | |

The four local peers control the last four props. A scripted camera flies a 250 m loop at 1–6 m altitude:
- **Snap turns:** within each 600-tick block, the camera looks backwards for 120 ticks.
- **Cut:** every 1,800 ticks it jumps a quarter of the way round the loop. Snap turns and cuts reset presentation
  history.
- **Near-plane crossings:** the path passes through walls and tall props.

## Correctness

`cargo test -p v1-game --release --test field_gpu`

- The test prepares two game hosts with the same field, one Direct and one Frustum. It drives both through the same
  ticks, waits for each frame to complete, and compares `capture_pixels` RGBA at 640×360.
- **The culled host really culls.** The test asserts each host's prepared mode, read from the renderer
  (`FrameHost::camera_culling`). Direct frames report no counts. A Frustum frame reports its read-back draw count
  when the count has settled by the time the frame retires. Counts never hold a frame. In the final full-path run,
  all 1,930 Frustum frames were counted. At its fewest, the Frustum host must draw more than zero and fewer than half of the active instances. A
  mutant that drops the mode between `GameSetup` and `SyntheticScene` fails the test before the first tick.
- **Default release run:** 79 sampled ticks to tick 1,930 (every 30th tick, plus each event and its neighbours).
  Byte-equal. Debug builds, as run by `./dev check`, stop at tick 460 after the first snap turn (about 65 s).
- **Full path, `THRUM_FIELD_FULL_PATH=1`:** every tick from 1 to 1,930 of the 3,600-tick loop. This covers three
  snap turns (each start and end) and the first quarter-loop cut at tick 1801. The next turn starts at tick 2101,
  outside the run. Byte-equal at all 1,930 ticks. In the final run (`a2cf00f3`), the Frustum host drew 21,745 of
  100,513 at its fewest, and no frame retired before its count. The log,
  `tmp/g3c-boundary/m1/full-path-m1close3.log`, records the commit, the variable and the 1,930 frames compared.
- **Mode survives resize and device replacement:** `cargo test -p thrum-render --lib culled_modes_survive`
  (f20a1d15) resizes the surface, then publishes a replacement device. Frustum stays selected, and every slot's
  report carries Frustum's read-back draw counts (closing rerun). The replacement's capture equals a Direct host on
  the same device. A mutant that
  drops the mode in `device_prepare_inputs` fails the test.
- **Counts never hold a frame:** `culling_counts_never_hold_render_slots_or_cross_a_replacement` (Metal).
  - It stalls both slots' count mappings. Two rounds of frames still retire, with no counts.
  - Once the stall is released, counts resume.
  - It then leaves real mappings pending on the old device and publishes a replacement. Only the new device's
    counts appear: (0, 0) for its empty scene, where an old count would read (1, 1).
  - The mutant that makes retirement wait on the count times out.
- **A late count never goes to the next frame on its slot:**
  `a_late_mapping_is_never_attributed_to_the_next_frame_on_its_slot` (Metal, at `aaf30165`).
  - Frame A (one active slot) retires before its count is read.
  - Frame B (empty prefix) reuses the slot without counting. A's count arrives while B runs, and B still reports
    none.
  - The next frame on the slot reports its own (0, 0).
  - The mutant that keeps A's metadata makes B report A's (1, 1).
- **Unit tests:** `cargo test -p v1-game --lib field` checks that the layout is deterministic, that the field spans
  far beyond the view, that walls stand halfway between prop lines, and that every turn and cut is flagged.

## Paired GPU cost on the game path

`cargo bench -p v1-game --bench field_culling`

- **Setup:** 1280×720 with Sun shadows. 1,800 ticks follow 60 warm-up ticks.
- **Pairing:** both hosts step every tick, in alternating order, and each frame completes before the next is driven.
- **Whole GPU:** all timed passes except tone. Forward includes the camera visibility pass.
- **Visibility:** on average 22.0% of instance centres are inside the frustum. This is a CPU proxy. The GPU's
  read-back draw count in Chrome, which tests bounds rather than centres, is about the same: p50 21,951 of 100,513
  (21.8%).

| Run | Whole GPU ms p50 (p95), Direct → Frustum | Forward ms p50 (p95), D → F | Paired F−D median |
| --- | --- | --- | ---: |
| 1 | 3.141 (3.289) → 2.932 (3.071) | 0.963 (1.099) → 0.740 (0.855) | −0.210 ms |
| 2 | 3.260 (11.072) → 2.935 (10.658) | 0.980 (3.710) → 0.738 (3.492) | −0.282 ms |
| 3 | 3.579 (8.978) → 3.362 (8.752) | 1.067 (2.185) → 0.846 (2.033) | −0.253 ms |
Runs 1 and 2 kept no raw output. Run 3 is the closing rerun, under heavier load (load average 37–42), with its
raw output in `tmp/g3c-boundary/m1/bench-m1close.txt`. All three runs move the same way by about the same amount.

**Decision:** without `THRUM_CULLING`, the field defaults to Frustum. The fully visible grid keeps Direct, as root's
Q1 asked. The decision is provisional, because these meshes are 12-triangle boxes. M3 repeats the rows on realistic
meshes.

## Chrome

`examples/v1-game/web/build.sh`, then `node scripts/check_v1_browser.mjs --case field` (Frustum) and
`--case field-direct`. Chrome on this Mac, WebGPU, 1280×720, 600 qualified samples each.

- **Runs 1 and 2:** both cases pass with 600 GPU-timed samples, at the display rate, with no overload, refused
  ticks or clamps.
- Runs alternated Direct, Frustum, Direct, Frustum. Chrome quantises timestamps to about 0.066 ms, so only p50 and
  p95 steps are meaningful.

| Run | Forward ms p50 (p95), Direct → Frustum | Far (Sun shadows) ms p50 |
| --- | --- | ---: |
| 1 | 0.786 (0.918) → 0.590 (0.655) | 0.852 both |
| 2 | 0.786 (0.918) → 0.590 (0.655) | 0.852 both |
| 3 | 0.918 (3.473) → 0.786 (2.556) | 0.918 both |
| 4 | 1.180 (4.588) → 0.852 (5.636) | 1.180 both |

- **Run 3** is the closing rerun, one page each, with the counts readback built in (Wasm `c995d441…` for the oracle
  build). Its raw reports are in `tmp/g3c-boundary/m1/chrome-m1close/`.
- **What the page reports:** each page names the prepared renderer's mode (`culling=Frustum` or
  `culling=Direct`). Frustum also reports its read-back draw counts:
  `culling_drawn_p50=21951 culling_drawn_p95=23033 culling_active=100513`.
- **Run 3** was taken under heavier contention (load about 57), and its tails are longer.
  - Both cases still pass.
  - The Frustum page recorded 7 overload events, 200 refused ticks and 229 shed views. The Direct page recorded
    0, 0 and 113.
  - One unpaired run cannot separate this from load. A paired check followed: the Frustum page with the counts
    readback against the same build with it removed, in ABBA order (load 27–31). All four runs had no overload
    and no refused ticks. Shed views were 80 and 86 with counts, and 63 and 89 without. Forward p50 was within one
    timestamp step. The raw reports are in `tmp/g3c-boundary/m1/counts-ab/`.
- **Run 4** is the final rerun at `a2cf00f3`, after counts stopped holding the render slot (Wasm `39eca5c1…`; load
  about 25). It is one page each and unpaired.
  - Both cases pass with no overload events and no refused ticks.
  - Shed views were 296 for Frustum and 229 for Direct. One unpaired run under load cannot separate culling from
    contention. The run 3 ABBA check above was the deciding test for whether counts cost frames, and in this code
    counts no longer hold a slot at all.
  - The Frustum page counted all 600 samples (`culling_counted=600/600`), with drawn p50 21,986 and p95 23,060 of
    100,513.
  - The raw reports are in `tmp/g3c-boundary/m1/chrome-m1close3/`.

**Pixel equality with Chrome Direct:** `node scripts/check_v1_browser.mjs --case field-oracle`, using the
caller-owned capture token (Q2c, d0f9cce4).
- **Setup:** two fresh pages, one Direct and one Frustum, are paused before their first frame by the same
  parameter file. Both set the same explicit camera at 16 views (eight yaws × two pitches).
- **Comparison:** each capture's committed tick, view and extent must match the other page's before the bytes
  are compared. The device generation is recorded but not compared, since each page has its own.
- **Mode check:** the oracle requires the Direct page to report `culling=Direct` with no counts, and the Frustum
  page to report `culling=Frustum` with fewer drawn than active. Otherwise a lost mode could compare Direct with
  Direct.
- **Result:** all 16 views byte-equal at `tick=0;extent=1280x720;generation=1`. This held five times:
  - on the Q2c revision, before it landed;
  - in root's run of the merged checker on main (no retained artifact);
  - on main `f20a1d15` plus the M2 Occlusion branch (Wasm SHA-256 `df972905…`);
  - on main `b8f230e7` plus the counts readback, with the mode check (Wasm `c995d441…`). The Frustum page reported
    drawing p50 20,800 of 100,513. The result is in `tmp/g3c-boundary/m1/chrome-m1close/field-oracle.json`.
  - At `a2cf00f3`, after counts stopped holding the render slot (Wasm `39eca5c1…`). The Frustum page reported drawing
    21,938 of 100,513. The result is in `tmp/g3c-boundary/m1/chrome-m1close3/field-oracle.json`.
- **Device loss:** on the Frustum page, after a real device destroy, `begin_capture` and an outstanding capture
  are both refused. After the restore button, a fresh capture returns the same tick under generation 2.

## Watched motion

- 567 Frustum frames: the 40 ticks either side of each event, encoded at 30 fps.
- The author reviewed contact sheets around the tick-301 snap turn and the tick-1801 cut. Walls hide most of the view
  just before the turn, and the field reappears whole after the turn and the cut. No pops at the view edges.
- **Independent review: pass** (`tmp/g3c-boundary/m1/review/R-M1-motion.md`). The reviewer screened all 560
  non-event frame pairs with a block-matched difference and inspected every flagged edge. Each abrupt change was a
  snap, a cut, the camera passing through geometry, or an object leaving at its prior speed. Its one borderline case,
  a box leaving the right edge at tick 1031, cannot be a culling pop: Frustum is byte-equal to Direct at that tick.

## Not covered

- **Live window:** attempts presented nothing while the screen was locked. A rerun at the close had no
  display-link callbacks (`m1/window-frustum.log`). The
  headless host above uses the same `FrameHost` path. So the title's mode and forward ms, and the native end-of-run
  report's new forward GPU ms and culling counts, are checked only by reading the code. The Chrome frame report
  is observed showing the mode, forward GPU ms and draw counts.
- **Uncontended numbers.**
- **Realistic meshes:** M3.
- **Occlusion:** M2.

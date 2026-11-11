# APPLE-TILE M2 review: The existing frame lifecycle runs through one split graph

- **Reviewer:** Claude Opus 5.5 (high), independent of the builder. Review date: 11 November 2026.
- **Commit judged:** `a6101020a94c50e386bc33ce7f41743a84937957`, in a clean checkout. From it I built:
  - release `v1-game` with `draw-probe,thrum-render/creation-counters` (SHA-256 `54bc5e92…`);
  - `thrum-cook`, and a fresh valley store, root `70d2baf8…e21d`, which matches `moving-v1.json`;
  - the iOS simulator app.
- **Attribution build:** for attribution only, I also built the parent `3870a021` (see
  [parent-3870a021-m2-matrix.json](M2-review/parent-3870a021-m2-matrix.json)). Its SHA-256 is `1b55ef38…`, which is
  byte-identical to the builder's evidence binary. It contains the M2 code but not
  APPLE-PRESENTATION M1 (`5258d963`).
- **Result: FAIL.** D2.1, D2.2, D2.4 and D2.5 pass. D2.3 fails with one Must Fix.
- **First review:** `fails` was 0, so there was no earlier review to follow up.

Captures and summaries are in [M2-review/](M2-review/). Bulk sources, such as T2 spools, OS videos and simulator
records, stay in my scratch folder `tmp/review-m2/`. Every GPU and window run held the orchestrate `gpu` lock.

## What I ran

| Run | Result |
|---|---|
| `scripts/apple_tile_m2.py` on `a6101020` | **Fails at the first exact comparison.** `portable-bgra8-mirror-light`: 316,181 of 800,000 letterbox pixels differ from the M0 baseline. [comparison](M2-review/a6101020-mirror-light-letterbox-comparison.json), [difference map](M2-review/a6101020-mirror-light-letterbox-diff.png), [manifest](M2-review/a6101020-m2-matrix.json) |
| `scripts/apple_tile_m1.py` (the M0 exact matrix) on `a6101020` | **Fails at the first comparison.** `portable-sdr-approach`: 472,377 pixels differ. [comparison](M2-review/a6101020-sdr-approach-comparison.json) |
| `scripts/apple_tile_m2.py` with the parent `3870a021` binary | Passes. The letterbox is exact for both recipes and both surface formats (4/4 comparisons, 0 changed pixels), and the realtime route and stall runs complete. [manifest](M2-review/parent-3870a021-m2-matrix.json) |
| Graph and executor unit tests (`graph_record`) | 5/5 pass |
| `./dev check` on `a6101020` | Exit 0. This includes the split tests, `lost_encoded_frame_abandons_staging_without_a_queue_submit` and the Metal test `metal_both_recipes_prepare_one_graph_and_reuse_scene_textures_on_surface_resize`. |
| `scripts/apple_tile_m1_sim.py` (iPhone 18 Pro simulator, iOS 27.2) | Passes. [route log](M2-review/sim-route.log), [final endpoint](M2-review/sim-m1-final.png) |
| My own OS-window observer on `a6101020`, both recipes | Covered settled sun, night and noon; the held camera; key response; resize to 760×472 and to 700×572 points, then restore; and same-PID surface replacement with OS video. [window-observations.json](M2-review/window-observations.json) |
| The same held-camera observer with the parent binary | All four held captures are byte-identical. [held-window-stability.json](M2-review/held-window-stability.json) |

**Window setup.** I ran the window observations attended, with these conditions:
- `CGPreflightScreenCaptureAccess()` returned true for my process. The console was on and not locked.
- My process was already AX-trusted, and no grant was requested or changed.
- The observer was `/usr/sbin/screencapture` on macOS 26.6.2 (25G83), at backing scale 2, on host clocks (`time_ns`
  and `monotonic_ns`).

**Edge conditions run.**
- mac-metal:
  - Low sun: yes, with the sunset-clear-sun state.
  - Fast camera: yes, with key turns and the qualifier's post-replacement pan.
  - Distance: partly, through route checkpoints on the parent only.
  - Storm: not run, because the valley scene has no storm state.
  - Full refresh: not run. Cadence is out of scope and owned by Mac M2/M3.
- ios-sim:
  - Background and foreground: yes.
  - Rotation: not run, because the route has no rotation step.

**Owner answers.** The three entries the builder quotes match `ACCEPTED.json` `milestones.M2.owner_answers` verbatim:
- D2.5 at 08:37:37Z;
- D2.3 at 08:37:44Z;
- D2.5 "Show it in Codex" at 10:16:52Z.

My D2.5 judgement rests on my own observations, not on the builder's Computer Use session.

## Must Fix

### MF1 `d23-exactness-broken-at-claimed-commit` (D2.3, verified)

- **What I did.** I ran the milestone's own reproduction commands, `scripts/apple_tile_m2.py` and
  `scripts/apple_tile_m1.py`, with a release binary built from the claimed commit `a6101020`.
- **What happened.**
  - The exact M0 comparisons fail on the first checkpoint each script tries:
    - the 1000×800 letterbox `mirror-light` image differs in 316,181 pixels;
    - the SDR `approach` image differs in 472,377 pixels.
  - [The difference map](M2-review/a6101020-mirror-light-letterbox-diff.png) shows the signature of a sub-pixel
    projection offset. The analytic sky is unchanged, and every rasterised surface (terrain, sphere, rock, cubes)
    differs.
  - The same script with the parent `3870a021` binary, which has the M2 code but not APPLE-PRESENTATION M1, passes
    4/4 exactly.
  - In the real window at the claimed commit, the camera is held and the world paused, yet consecutive OS captures
    differ in 874k–1.05M of 2.07M content pixels, with a maximum difference of 210 codes. The parent binary gives
    byte-identical held captures.
  - Evidence: [four held captures ×2](M2-review/a6101020-apple-tile-held-window-4-captures-x2.png), compared with
    [the parent](M2-review/parent-3870a021-portable-held-window-2-captures-x2.png), and
    [the numbers](M2-review/held-window-stability.json).
- **Why.** APPLE-PRESENTATION M1 (`5258d963`, 09:16) landed after M2's code commit (`0393bc45`, 07:42) and before
  M2's claim. It applies `.with_jitter_phase(0, [640, 360])` to every game view, including route captures, at
  `examples/v1-game/src/host/scene_source.rs:513`. `PreparedSynthetic::update_frame` (`crates/thrum-render/src/synthetic.rs:1776`)
  then rasterises every submitted frame with a jittered projection.
  - The builder's D2.3 and D2.5 evidence all uses binary `1b55ef38…`. My build of the pre-APPLE-PRESENTATION parent
    `3870a021` reproduces that hash exactly. The simulator identity records commit `2b9943bb`, which also predates
    `5258d963`.
  - The evidence README says "the retained final binary is unchanged". So the claimed commit was never qualified,
    and none of the builder's records detects the break.
  - APPLE-PRESENTATION's own M1 review already holds the root defect as its MF1
    (`jitter-presented-without-reconstruction`).
- **What should happen.** At the commit claimed for M2, same-target scene-linear and submitted SDR bytes are exact
  against M0 and the paired recipe (D2.3), and a held view is deterministic (the milestone bar).
  - The likely fix is APPLE-PRESENTATION's fix: the presented frame stays unjittered until a reconstructor consumes
    the jitter. M2's code itself is not at fault.
  - After that lands, rerun `apple_tile_m1.py` and `apple_tile_m2.py`, including the realtime cut and stall
    sequences, with a binary built from the commit being claimed. Record that commit's revision and clean diff
    alongside the binary hash in the manifests.
  - Do not refresh or widen the M0 baseline.
- **Proposed item** that would have caught it: "Every exact or visual proof in this milestone records the git revision,
  clean-diff hash and binary hash of the build it ran, and that revision is the claimed commit."

## Items

### D2.1 One graph per generation and variant, with late surface binding: pass

**Question.** Do graph and executor tests do three things?
- Reject scene work behind acquisition when it has no surface dependency.
- Require the surface binding only for phase two, and prove graph order.
- Keep the tone intermediate and leave arithmetic, formats and draw order unchanged.

**Answer.** Yes, on both sides of the boundary:
- **Graph side** (`graph.rs`, `compile_surface_variant` and `compile_late_surface`):
  - Each day, night and capture variant compiles once with the surface pass appended.
  - Every live scene pass gets an explicit edge to the present pass.
  - The post-acquire set must equal the surface consumer plus its descendants, and it must be a contiguous suffix of
    the original schedule. Otherwise compilation returns `GraphError::SurfacePhase`.
  - `split_tests.rs` rejects three cases: an unrelated pass declared post-acquire, a scene pass depending on the
    surface, and an empty post set.
- **Executor side** (`graph_record.rs`):
  - `execute_before_acquire` and `execute_after_acquire` slice the same prepared recorder list at
    `pre_acquire_passes`, so there is no per-frame lookup.
  - The executor test shows that phase one runs without a surface, phase two refuses without one, and the final order
    is [scene, present].
- **Host side:** `encode_scene_only` runs only the prefix. `finish_encoded` runs the suffix with an execute-time
  `LateSurface`.
- **Real Metal:** the split-graph test asserts, for both recipes and for capture on and off, that the suffix is exactly
  `PRESENT_PASS` and that "synthetic tone encode" sits before the boundary. It also asserts that scene textures are
  reused across a surface-only resize.
- **Code that did not change:** no `.wgsl` changed in `0393bc45`. The tone intermediate (Rgba8Unorm) and present
  shader are the same pass the standalone `SurfacePresenter` declares, now shared through `present::surface_pass`.
  APPLE-PRESENTATION later changed `synthetic_display.wgsl`, adding a motion-view branch that is off by default.
- **Capture readback:** it reads the scene output, not the drawable, so placing it before acquisition is consistent
  with the design.

### D2.2 Refusal, loss and replacement under the merged graph: pass

- **Healthy refusal.** `EncodedFrame::skip` and its `Drop` discard the recording and timing reservation, then flush
  staging with one empty submit. No scene is submitted and no history is committed, because `commit_frame` runs only
  in `finish_encoded`.
- **Loss before acquire.** `lost_encoded_frame_abandons_staging_without_a_queue_submit` (Metal) covers both cases in
  one test. The healthy skip makes +1 flush attempt. The loss makes 0 queue calls, leaves source and history unchanged
  and keeps the history reset pending.
- **Loss after acquire.** On the Mac this is `check_encoded_device` after `get_current_texture`
  (`render_actor.rs:317`), which calls `abandon_lost_device`. On iOS, `render.rs` checks `is_lost()` after
  `AfterAcquire`.
- **Successful submit.** The "missing binding degrades" test covers both the late and the direct path, and
  `./dev check` covers the replacement and recovery GPU tests.
- **Resize.** `resize_surface` now rebinds the merged graph per slot. `GraphTextures::rebind` refuses any descriptor,
  slot or storage change, and the Metal test proves that surface-only resize reuses the allocation.
- **Stale generations.** The replacement tests in `./dev check` pass, and so does my OS-observed same-PID replacement:
  generation 2→3, all 845/847 rows presented, maximum row gap 27 ms, one retirement with outstanding work retained
  (`stale-recycle Some((1, 1))`).

One coverage gap here is a Suggestion (S2).

### D2.3 Complete image and motion, exact against M0 and paired recipes: fail

**Inspection list:**
1. M0 exact SDR and scene-linear bytes on route checkpoints, both recipes.
2. Letterbox bytes for Bgra8Unorm and sRGB.
3. Paired-recipe exactness.
4. Sample and exposure identity.
5. Realtime route with cut, and realtime render stall at 1x.
6. Full frames and native crops of the Sun, stars, mirror and local lights.

**Results:**
- At `a6101020`, entries 1 and 2 fail (MF1).
- Entries 3–5 could not be completed at the claimed commit, because both scripts stop at the first exact failure.
- On the parent, entries 2 and 5 pass, but the parent is not the claimed commit.
- Entry 6, at the claimed commit:
  - the [OS-window states](M2-review/apple-tile-night-os-vs-t2.png) show complete content: Sun, stars, local lights,
    mirror and shadows;
  - the image is jittered, so it is not exact.

### D2.4 iOS lease and background race under merged execution: pass

I ran the simulator route on an owned iPhone 18 Pro (iOS 27.2) with the app built from `a6101020`:
- Hidden view: 120 denied, 0 acquired, 0 presented, while the world ticked.
- Replacement: epoch 2→4, same PID, no live detached windows.
- Stalled replacement: deferred 1, `stale_dropped` 1, then recovery.
- Raced present: the one designed in-section present after revocation, then the old window detaches on release.
- Background and foreground, twice, one of them during a render stall: the world paused and resumed at the same tick,
  the surface epoch advanced, and releases were deferred.

**Code.** `ios/render.rs` checks `lease.is_current(epoch)` after acquisition and before `encoded.submit`. A stale lease
gives `skip`, which flushes without a scene and keeps history uncommitted. Loss gives `abandon`. So no background
submit, old-generation publication or stale resume is possible on this path. The lease unit tests run in
`./dev check`.

**Limits.** This is function only, with no cooked draw or runtime-motion claim. Rotation was not exercised.

### D2.5 Real Mac game window, both recipes: pass (attended setup present)

**Inspection list:**
1. Settled Sun, night and noon against T2 `still` references at the same paused controls, with the proof overlay,
   stars and direct culling.
2. Sun disk and glow, sky and stars, terrain, local lights, mirror sphere, cubes and shadows.
3. Ordinary key input.
4. Resize: letterbox geometry in content pixels, for both letterbox and the taller pillar case, then restore.
5. Surface replacement with OS video and lifecycle logs.

**Results:**
1. **Settled states.** Sun, night and noon match their T2 references in content for both recipes: [portable
   noon](M2-review/portable-noon-os-vs-t2.png), [Apple tile sun](M2-review/apple-tile-sun-os-vs-t2.png), [Apple tile
   night](M2-review/apple-tile-night-os-vs-t2.png), [portable night](M2-review/portable-night-os-vs-t2.png). These
   are matched states, not the same submission.
2. **Content.** Everything on the list is present, including the Sun disk and stars at night. The night local-light
   pool is blown out in both the window and the reference. That look is inherited, not an M2 change.
3. **Keys.** E×4 and then Q×2, posted to the PID, turned the camera visibly in the OS video:
   [Apple tile](M2-review/apple-tile-key-response-os-contact.png), [portable](M2-review/portable-key-response-os-contact.png).
4. **Resize.** I measured the letterbox in content pixels:
   - 1520×880 gives 12/13 black rows around an 855-row image;
   - 1400×1080 gives 146/147 rows around a 787-row image (788 expected after rounding);
   - restoring gives a full 1920×1080 with no black rows.
   - Images: [Apple tile 1520×880](M2-review/apple-tile-window-resized.png), [Apple tile 1400×1080](M2-review/apple-tile-window-resized-narrow.png),
     [portable 1400×1080](M2-review/portable-window-resized-narrow.png); portable's 1520×880 numbers are in
     [window-observations.json](M2-review/window-observations.json).
5. **Replacement.** The same-PID surface replacement in OS video spans generation 2→3 with the scene intact, then the
   qualifier's pan to 36° follows: [portable](M2-review/portable-replacement-os-contact.png), [Apple
   tile](M2-review/apple-tile-replacement-os-contact.png).

**Judgement.** The changed surface path functions in the real window. The held window crawls, which breaks the bar's
"deterministic visible semantics". That crawl has the same root as MF1 and is recorded there, not counted twice.

## Read-through of changed code

I read every changed line of `0393bc45` in `graph.rs`, `graph_record.rs`, `present.rs`, `runtime.rs`,
`preparation.rs`, `synthetic.rs`, `camera_culling.rs`, `scene_graph.rs`, the new tests and `scripts/apple_tile_m2.py`.
I also read the evidence-only commits `23442056` and `a6101020`, and the APPLE-PRESENTATION code that landed in
between.

I found none of the signs of fake work: no hard-coded results, no test-only branches, no weakened M2 tests and no
silenced lints. The one `storage_tests.rs` change is a call rename. The M2 evidence binaries and images are captured
outputs, not computed stand-ins. The `camera_culling.rs` change routes the prepass depth ID through `target.ids`, so a
rebind keeps it current.

## Suggestions

- **S1 (verified) `rebind` misreports its error.** `GraphTextures::rebind` returns `GraphError::SurfacePhase` for a
  transient descriptor, slot or storage mismatch (`crates/thrum-render/src/graph.rs`, `rebind`). That mismatch is an
  allocation-compatibility failure, not a phase error. A distinct variant would make a failed surface-resize rebind
  diagnosable.
- **S2 (verified) no token-level loss-after-acquire test.** No token-level Mac test separates "loss after acquisition,
  before submit" from "loss before acquisition". Both reach `abandon_lost_device`. The Mac case relies on
  `render_actor.rs:317` plus the iOS route. A test that acquires a texture (or a stand-in), destroys the device and
  then abandons would make D2.2's separate cases explicit.
- **S3 (verified) evidence provenance.** The native T2 and matrix manifests record a binary hash but no git revision,
  while the simulator script records both. Adding `git rev-parse HEAD` and the diff hash to `apple_tile_m0/m1/m2.py`
  manifests would have exposed MF1 at claim time.
- **S4 (suspected) the claim-time rebase.** The milestone was claimed after rebasing onto another module's
  rendering change without rerunning its exact matrix. The build-milestone handoff should require rerunning exact and
  visual proofs whenever the rebase brings in rendering changes.

FAIL M2: 1 fail, 0 cant_tell, 0 outside the items

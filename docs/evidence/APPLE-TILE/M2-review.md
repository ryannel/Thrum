# APPLE-TILE M2 review: The existing frame lifecycle runs through one split graph

- **Reviewer:** Claude Opus 5.5 (high), independent of the builder. Review date: 11 November 2026. Round 2.
- **Commit judged:** `b74cae87a34e92a1db5095b20dd3f1ea0322377e`, in a clean checkout (`git status` clean apart from
  this review). From it I built:
  - release `v1-game` with `draw-probe,thrum-render/creation-counters`. Its SHA-256 is `8b417841…a8468`, the same as
    the builder's evidence binary. The only commits after the builder's matrix revision `9af8664f` change
    documentation and `scripts/*mac_m2*`, so its source is the source I built;
  - the iOS simulator app (`examples/v1-game-ios/ios/build-simulator.sh`).
- **Cooked store:** I reused the valley store I cooked in round 1 (`tmp/review-m2/valley`). Both scripts'
  `validate_store` accepted it against `moving-v1.json`.
- **Result: PASS.** All five items pass. Round 1's Must Fix is fixed and no new Must Fix was found.

Selected captures and summaries for this round are in [M2-review/r2/](M2-review/r2/). Round 1's files are in
[M2-review/](M2-review/). Bulk sources, such as T2 spools, OS videos and simulator records, stay in my scratch folder
`tmp/review-m2-r2/`. Every GPU, window and simulator run held the orchestrate `gpu` lock.

## Round 1 Must Fix

### MF1 `d23-exactness-broken-at-claimed-commit` (D2.3): fixed, verified

**Round 1.** At `a6101020`, APPLE-PRESENTATION M1 had made every game view rasterise with a jittered projection. As
a result:
- the exact M0 comparisons failed;
- a held window crawled;
- the builder's evidence came from an older binary.

**The fix.** It came from APPLE-PRESENTATION's own fix, `ae0cc8fa`:
- The game now calls `with_jitter_sequence`, which keeps the sample metadata without the raster jitter
  (`examples/v1-game/src/host/scene_source.rs`).
- `SyntheticView::raster_projection` returns the unjittered `view_proj` unless `with_jitter_phase` has been selected
  explicitly (`crates/thrum-render/src/synthetic/view.rs`).
- The tests were tightened, not weakened:
  - `threaded_pause_keeps_pixels_still…` now requires all 31 consecutive paused frames to be identical;
  - the recovery tests now compare against the single pre-loss frame instead of the closest of 32 jitter phases.

**My verification at `b74cae87`:**
- **`scripts/apple_tile_m1.py`:** 38/38 exact comparisons, 0 changed pixels
  ([summary](M2-review/r2/m1run-summary.json)). This covers:
  - both recipes;
  - SDR and scene-linear, at 5 route checkpoints;
  - the portable repeats;
  - Sun and night.

  The script also checks that each checkpoint's sample and exposure identity equals the retained M0 identity.
- **`scripts/apple_tile_m2.py`:** 4/4 exact letterbox comparisons, 0 changed pixels (both recipes, Bgra8Unorm and
  sRGB), plus complete realtime route and stall runs ([summary](M2-review/r2/m2run-summary.json)).
- **Provenance:** both manifests now record revision `b74cae87`, the empty-diff hash `e3b0c442…` and binary
  `8b417841…`. Round 1's Suggestion S3 is done.
- **Baselines:** the M0 baseline folder `docs/evidence/render/apple-tile-m0/native-baseline` was last changed in the M0
  commit `03bc878a`. It was not refreshed.
- **Held window:** with the camera held and the world paused, four consecutive OS captures per recipe are
  byte-identical, with 0 of 2,073,600 content pixels changed
  ([window-observations.json](M2-review/r2/window-observations.json), `held_vs_first`). In round 1 this was 874k–1.05M
  pixels.

## What I ran

| Run | Result |
|---|---|
| `scripts/apple_tile_m1.py` | Pass: 38/38 exact |
| `scripts/apple_tile_m2.py` | Pass: 4/4 exact letterbox, plus realtime route/cut and stall for both recipes |
| My own paired-recipe comparison over the m1 outputs, portable against Apple tile | 0 differing files: every SDR/scene-linear checkpoint's pixels, surface bytes and identity JSON, and the Sun/night spools ([paired-recipes.json](M2-review/r2/paired-recipes.json)) |
| T2 `still` references at `b74cae87`, Sun, night and noon, both recipes, proof overlay, stars, direct culling | Captured, `capture_complete` true |
| My OS-window observer, both recipes | Settled states; held camera; key response; resize to 760×472 and 700×572 points, then restore |
| Same-PID surface replacement with OS video, both recipes | Generation 2→3; every row presented |
| `scripts/apple_tile_m1_sim.py` (iPhone 18 Pro simulator, iOS 27.2) | Pass ([log](M2-review/r2/sim-route.log)) |
| `./dev check` | Exit 0. Includes the split-graph, executor, loss, recovery and paused-stability tests named below |

**Window setup (attended).**
- `CGPreflightScreenCaptureAccess()` returned true for my process, and `IOConsoleLocked` was false. No grant was
  requested or changed.
- The observer was `/usr/sbin/screencapture` on macOS 26.6.2 (25G83), at backing scale 2, on host clocks (`time_ns`
  and `monotonic_ns`).
- PIDs, window IDs, bounds and build hashes are in [window-observations.json](M2-review/r2/window-observations.json).

**Owner answers.** The builder cites `ACCEPTED.json` `milestones.M2.owner_answers`. The three entries there are:
- D2.5 at 08:37:37Z;
- D2.3 at 08:37:44Z;
- D2.5 "Show it in Codex" at 10:16:52Z.

They grant the attended setup (Screen Recording on, IINA allowed) and direct how the remaining checks are shown. My
D2.5 and D2.3 judgements rest on my own runs, which used that existing grant.

**Edge conditions run.**
- mac-metal:
  - Low sun: yes, the sunset-clear-sun state, in the window and in T2.
  - Fast camera: yes, the key turns, the route cut, the stall's elapsed-time catch-up, and the post-replacement pan.
  - Distance: yes, the route checkpoints out to the far terrain.
  - Storm: not run, because the valley scene has no storm state.
  - Full refresh: not judged. Cadence is out of scope and owned by Mac M2/M3.
- ios-sim:
  - Background and foreground: yes, twice, one of them during a render stall.
  - Rotation: not run, because the route has no rotation step.

## Items

### D2.1 One graph per generation and variant, with late surface binding: pass

**Question.** Do graph and executor tests do three things?
- Reject scene work behind acquisition when it has no surface dependency.
- Require the surface binding only for phase two, and prove graph order.
- Keep the tone intermediate and leave arithmetic, formats and draw order unchanged.

**Answer.** Yes. Round 1 read the M2 code in full. Since then:
- `graph.rs`, `graph_record.rs`, `present.rs`, `runtime.rs` and `thrum-app` are unchanged.
- The jitter gate and ATMOSPHERE M5's opt-in cloud shade both changed `synthetic.rs`:
  - The cloud shade appends its reads to the same per-variant scene graph (`scene_graph.rs`, `cloud.append_reads`).
  - It adds bindings only when `sun_optical_depth` is present, and that defaults to `None`, with
    `GameSetup::diagnostic_clouds` false.
  - So it does not move any scene work behind acquisition.
- These tests pass in my `./dev check`:
  - `split_tests::late_surface_requires_only_phase_two_binding_and_preserves_order`;
  - `split_tests::unrelated_scene_cannot_be_declared_post_acquire_or_depend_on_surface`;
  - the three `graph_record` binder and order tests;
  - the Metal test `split_graph::metal_both_recipes_prepare_one_graph_and_reuse_scene_textures_on_surface_resize`.
- The exact M0 matrix confirms that arithmetic, the tone intermediate and quantization are unchanged at this commit.

### D2.2 Refusal, loss and replacement under the merged graph: pass

- **Code:** the healthy-refusal, loss and successful-submit paths read in round 1 are unchanged.
- **Tests:** these pass in `./dev check`:
  - `lost_encoded_frame_abandons_staging_without_a_queue_submit`, which covers healthy skip (+1 flush) and loss (0
    queue calls, history reset pending);
  - the device-replacement tests (`gpu_device_replacement_abandons_slots_and_isolates_old_candidates_and_callbacks`
    and others);
  - `culled_modes_survive_surface_resize_and_device_replacement`;
  - the four `host::tests::recovery_gpu` tests. These are now stricter: consecutive replacement frames must be
    identical and within 1 code of the pre-loss frame.
- **Surface bytes and letterbox:** exact, from `apple_tile_m2.py`, as above.
- **My OS-observed same-PID replacement:**
  - portable: generation 2 ran 360/360 rows and generation 3 ran 476/476, all presented, with a maximum row gap of
    27.0 ms;
  - Apple tile: 360/360, then 485/485, with a maximum gap of 26.9 ms;
  - both runs had one retirement with outstanding work retained (`stale-recycle Some((1, 1))`), `surface_faults=0`
    and `device_losses=0`.

  Contact sheets: [portable](M2-review/r2/portable-replacement-contact.png),
  [Apple tile](M2-review/r2/apple-tile-replacement-contact.png).

Round 1's Suggestion S2, a token-level Mac test for loss after acquisition, is still open.

### D2.3 Complete image and motion, exact against M0 and paired recipes: pass

**Inspection list (written before looking):**
1. M0 exact SDR and scene-linear bytes on all route checkpoints, both recipes, with sample and exposure identity.
2. Letterbox bytes for Bgra8Unorm and sRGB, both recipes.
3. Paired-recipe exactness, portable against Apple tile, including identity.
4. The realtime route cut: history epoch change, intact content either side of the cut.
5. The realtime render stall at 1x elapsed time: deliberate skipped opportunities, then camera motion resuming at
   the elapsed-time position, not where it paused.
6. Full frames and native detail of the Sun disk and glow, the stars, the mirror, local lights, terrain and shadows.

**Results:**
1. **M0 exactness:** 38/38 exact. The retained identities match.
2. **Letterbox:** 4/4 exact.
3. **Paired recipes:** every checkpoint pixel file, surface file and identity file is byte-equal across the recipes
   ([paired-recipes.json](M2-review/r2/paired-recipes.json)). The OS-captured windows of the two recipes are also
   byte-identical in every state I captured (Sun, night, noon, held, after keys, narrow resize).
4. **Route cut.** Both recipes complete with:
   - 964/964 opportunities submitted;
   - 0 ring omissions and 0 dropped views;
   - history epochs 1→2 at image 524, with a maximum submit gap of 10.4–10.5 ms.

   The cut-adjacent full-resolution sources (Apple tile 797–800,
   [composite](M2-review/r2/apple-tile-cut-adjacent.png)) switch from the sphere-and-cubes view to the rock view with
   no blend, smear or missing geometry.
5. **Render stall.** Both recipes run 451/451 opportunities with one deliberate 30-opportunity skip at opportunity 247
   (`render-owner-before-ordinary-step`, about 251 ms) and a 259.3–259.4 ms submit gap. The accepted yaw jumps
   from 14.45° to 19.05° across the gap, which matches the elapsed time at the route's rate, then continues to 36.0°.
   So motion follows elapsed time and is not paused-and-resumed.

   The contact sheets ([Apple tile](M2-review/r2/apple-tile-render-stall-contact.png),
   [portable](M2-review/r2/portable-render-stall-contact.png)) show a steady pan with the scene intact throughout.

   I judged 1x continuity from the original-time records and the time-spaced contact sheets, not by watching the
   movie.
6. **Content.**
   - The settled OS window and the T2 `still` reference at `b74cae87` differ only in about 570 pixels, which are the
     window's rounded corners: [Apple tile Sun](M2-review/r2/apple-tile-sun.png),
     [Apple tile night](M2-review/r2/apple-tile-night.png), [portable noon](M2-review/r2/portable-noon.png),
     [portable night](M2-review/r2/portable-night.png).
   - The Sun disk and glow, the stars (including the cluster), terrain, shadows, the mirror sphere and the
     local-light cubes are all present.
   - The night local-light pool is still blown out. That look is inherited and is not an M2 change.

### D2.4 iOS lease and background race under merged execution: pass

- **Code:** `thrum-app` and the iOS shell are unchanged since round 1. The renderer changes (jitter gate, opt-in
  cloud) don't touch the lease path. I reran the route with the app built from `b74cae87`
  ([log](M2-review/r2/sim-route.log), [final endpoint](M2-review/r2/sim-final.png)).
- **Hidden view:** 120 denied, 0 acquired, 0 presented, with 120 world ticks.
- **Replacement:** epoch 2→4, same PID, 0 live detached windows.
- **Stalled replacement:** deferred=1, stale_dropped=1, then presents resumed.
- **Raced present:** exactly the one designed admitted present after revocation, then the old window detached after
  release.
- **Background and foreground, twice (once during a stall):** the world paused and resumed at the same tick (3267,
  then 3424), and the surface epoch advanced (8→10, then 10→12).
- **Limits:** this is function only. Rotation was not exercised.

### D2.5 Real Mac game window, both recipes: pass (attended setup present)

**Inspection list (written before looking):**
1. Settled Sun, night and noon against T2 `still` references at the same paused controls.
2. Sun, sky and stars, geometry, local lights and mirror.
3. A held camera is stable.
4. Ordinary key input.
5. Resize letterbox geometry in content pixels, then restore.
6. Surface replacement in OS video with lifecycle logs.

**Results:**
1. **Matched states:** they match, as listed under D2.3 entry 6. These are matched states, not the same submission.
2. **Content:** everything on the list is present.
3. **Held camera:** byte-stable on both recipes. This was round 1's crawl, now gone.
4. **Keys:** E×4 and then Q×2, posted to the PID, turn the camera visibly in the OS video
   ([portable](M2-review/r2/portable-key-contact.png)). The after-key frame differs from the held frame in 2.03M
   pixels.
5. **Resize:**
   - 1520×880 gives 12/13 black rows around an 855-row image (855 expected);
   - 1400×1080 gives 146/147 rows around a 787-row image (788 expected after rounding);
   - restoring gives a full 1920×1080 with no black rows;
   - the result is identical on both recipes ([Apple tile narrow](M2-review/r2/apple-tile-window-resized-narrow.png),
     [portable wide](M2-review/r2/portable-window-resized.png)).
6. **Replacement:** as listed under D2.2.

**Ask.** This item adds no reverse Mac dependency or panel claim. The ordering is unchanged since round 1:
- Mac M2 owns cadence;
- Mac M3 depends on this milestone by module order.

## Read-through of changed code

I read every source change between round 1's commit `a6101020` and `b74cae87` that touches the renderer or game:
- the jitter gate `ae0cc8fa` (`view.rs`, `synthetic.rs`, `stars.rs`, `contributed.rs`, `scene_source.rs` and the two
  host test files);
- ATMOSPHERE M5's cloud-shade plumbing (`synthetic.rs`, `scene_graph.rs`, `host.rs`, `scene_source.rs`);
- the M2 evidence commits `af69ba7d`, `9af8664f` and `b74cae87`.

The two `scripts/*mac_m2*` files belong to PLATFORM-RUNTIME-MAC.

I found none of the signs of fake work:
- The cloud shade is off by default, behind `diagnostic_clouds`, and is checked at prepare and update time.
- No test was weakened. The two recovery tests and the paused test now assert exact consecutive frames instead of
  "closest of 32 phases".
- The M0 baselines are untouched.
- The builder's done-when edits are only D2.3's `status` and `evidence` and the milestone's `state`.
- The builder's evidence binary hash reproduces from the claimed commit.

## Suggestions

- **S1 (verified, carried from round 1) `rebind` misreports its error.** `GraphTextures::rebind` still returns
  `GraphError::SurfacePhase` for a descriptor, slot or storage mismatch (`crates/thrum-render/src/graph.rs`, about
  lines 636 and 641). A distinct allocation-compatibility variant would make a failed resize rebind diagnosable.
- **S2 (verified, carried from round 1) no token-level loss-after-acquire test on the Mac.** It is covered only through
  `render_actor.rs` and the iOS route.
- **S3 (suspected) pacing counter.** For Mac M2 to look at:
  - In my replacement runs `late_target_evaluations` was 112 (portable) and 822 (Apple tile). In round 1 it was 0 and
    0 on the same script.
  - `thrum-app` is unchanged, and every row was still presented with gaps of 27 ms or less. So this is probably
    machine load during my session, not an M2 defect.
  - I did not judge cadence. Mac M2's capture-OFF cadence runs should confirm it.
- **S4 (verified) evidence wording.** `MF1-recheck.md` reports 960/712 route images and 32/30 skipped stall
  opportunities. My run gives 964/964 and 30/30. Realtime counts vary with load, so this is not a defect. The README
  could say that these counts are per-session observations, not fixed expectations.

PASS M2

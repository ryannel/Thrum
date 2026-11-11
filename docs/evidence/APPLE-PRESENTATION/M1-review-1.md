# APPLE-PRESENTATION M1 review

Reviewer: Claude Opus 5.5 (high), independent of the builder. 11 November 2026.
Commit judged: `40319eafb334754d9753706283e9655c94bf3eb6`, built from a clean checkout. Target: mac-metal (Apple M3 Max,
portable recipe). Release `v1-game` built with toolchain 1.99.1 (binary SHA-256 prefix `231598ce8836bfc2`). Showcase
store cooked from `examples/v1-game/assets/showcase`, root `70d2baf8…e21d`, which matches the routes.

**Result: FAIL.** All five items pass. One Must Fix lies outside the items: the shipped game now presents jittered
frames with no reconstructor, so a held camera shows every edge crawling. That worked before this milestone.

| Item | Verdict | Basis |
|---|---|---|
| D1.1 | pass | `./dev check` run by me (exit 0); layout/Halton test read; RENDER-SPEC and hash checked |
| D1.2 | pass | `./dev check`; Metal readback tests read; my colour-vs-motion cross-check |
| D1.3 | pass | `./dev check`; independent CPU projection in the test; atmosphere diff read |
| D1.4 | pass | All four cases traced through the code on both sides; Suggestions S2 and S3 |
| D1.5 | pass | My own fixed-time engine captures, watched as ordered sequences, with native crops and decoded values |

## Must Fix (outside the items)

### MF1 `jitter-presented-without-reconstruction`: the game's presented image jitters with no reconstructor (verified)

- **What I did.** I captured the valley route with the camera held and the display overlay off (`held-day-640.json`,
  640×360, the game's real render extent; and the same at 1080p), through the submitted engine tap
  (`scripts/apple_presentation_m1.py capture --overlay off`). Then I compared consecutive submitted frames.
- **What happened.** The camera doesn't move, yet consecutive frames differ on every silhouette, the skyline, the
  terrain ridges, the shadow edges and the ground texture. At 640×360, 950–1,900 pixels per frame pair change by more
  than 24 codes, with a maximum of 208. See:
  - [the difference image ×4](M1-review-1/held-640-consecutive-diff-x4.png);
  - [four consecutive frames of the sphere at native size ×4](M1-review-1/held-640-sphere-4-consecutive-frames-x4.png),
    where the silhouette and highlight step by about a pixel from frame to frame;
  - [the skyline over the same four frames](M1-review-1/held-640-skyline-4-consecutive-frames-x4.png).

  In the window, the 640×360 image is scaled up about 4.7× on the built-in panel. So a jitter step of up to about one
  render pixel (consecutive Halton offsets differ by up to 0.97 px) moves edges by several display pixels at the frame
  rate. That is visible edge crawl and shimmer on a held view.
- **Why.** `examples/v1-game/src/host/scene_source.rs:513` enables jitter on every game view
  (`.with_jitter_phase(0, [640, 360])`). `PreparedSynthetic::update_frame` (`crates/thrum-render/src/synthetic.rs:1776`)
  then rasterises with the jittered projection on every submitted frame. No reconstructor exists until M3, so nothing
  resolves the jitter. The builder changed two existing tests to absorb this:
  - `examples/v1-game/src/host/tests/lifecycle_gpu.rs:59` now waits a full 32-phase cycle before comparing paused
    frames;
  - `recovery_gpu.rs` (`closest_phase`) accepts the closest of 32 earlier frames.

  Before M1, a paused or held view gave identical frames, and those tests asserted that.
- **What should happen.** M1's bar judges motion "on the current portable output, before any reconstruction exists".
  Nothing in the design or plan asks for the presented image to be jittered before TAAU, and no shipped game presents
  jittered frames without temporal resolve. Fix this at the selection point, not cosmetically:
  - the presented frame stays unjittered until a reconstructor consumes the jitter (M3), or under an explicit
    temporal mode that tests and the M1 witnesses select;
  - the view record still carries the phase and offsets, and the phase still advances on submission.
- **Proposed item** that would have caught it: "With no temporal reconstructor active, the game's submitted frame is
  unjittered: with the world paused and the camera held, consecutive submitted frames are byte-identical on
  mac-metal, and the existing lifecycle and recovery tests compare consecutive frames exactly again."

## Suggestions

- **S1 The motion view saturates at ±8 px, and saturated motion uses the invalid colour (verified).**
  `synthetic_display.wgsl:101-105` maps `0.5 + v/16`, clamped, with blue as `|v|/16`. Saturated motion right and up
  with magnitude ≥ 16 px gives (1,0,1), the same magenta that marks invalid correspondence.
  - In my moving-route capture, up to 222,576 texels per frame are exactly (255,0,255), from fast near-ground
    translation. I can't tell from the view whether any are invalid. See the bottom-left of the later frames in
    [the moving sheet](M1-review-1/moving-motion-sheet.png).
  - In the 3°/tick pan, the whole frame clamps to −8 px. Centre-to-edge growth is invisible there; see
    [the fast pan sheet](M1-review-1/fast-lowsun-sheet.png).

  Use a distinct invalid colour that can't be reached otherwise (for example a checker pattern), and a log or
  selectable scale.
- **S2 Sky and stars ignore cuts and missing history (verified by code reading).** The bar asks for "marked invalid
  where there is no history rather than fabricated".
  - `atmosphere/gpu.rs:248` uses `submitted_sky.unwrap_or((*view, sun))`. So on the first frame, and after device
    replacement (a new `AtmospherePrepared`, while `inherit_submitted_view` copies only the synthetic view), the sky
    writes exactly zero motion.
  - On a camera cut, the sky and stars write a reprojection to the pre-cut camera.

  The frame's reset flag covers every reconstructor, and D1.4 names objects and the forward drawers, not the sky, so
  this is not a fail. M2's reactive work should still mark the sky and stars, or write the sentinel, when
  `history_reset` is set or there is no submitted sky. Carrying `submitted_sky` across replacement is the matching fix.
- **S3 The cooked volume invalidates everything it covers (suspected).** `cooked.wgsl` now writes `(65504,65504)`
  with the motion write mask `ALL` (`draw.rs:305`) wherever alpha > 0. The terrain behind a faint volume has valid
  history, yet loses it. This conflicts with the design's W3, where reactive equals the volume's alpha. The showcase
  route has no volume, so I saw no effect.
- **S4 Mip bias is a constant.** `view.rs` `pack_view` writes `-1.0` at byte 224. That is correct at native scale
  only. M5 (dynamic scale) must derive it from `log2(render/output) − 1`.
- **S5 Cut frames can't be captured.** The route tool refuses checkpoints within 4 ticks of a cut ("checkpoint …
  must be … 4 ticks after the origin and each cut"), and the builder's moving window ends at tick 196, before its cut
  at 200. So the cut onset was never seen in engine capture. The Metal test covers the sentinel on a cut for synthetic
  geometry. M3's D3.4 will need that window.
- **S6 Clusters and shadows use the unjittered view, but rasterisation is jittered (suspected, minor).** At a
  half-pixel scale this should not matter, but light-tile edges could misassign by one pixel.

## Items

### D1.1: pass

- I ran `./dev check` on the clean checkout. It exited 0, with 63 test groups OK, including
  `synthetic::view::temporal_tests::continuous_halton_values_and_exact_record_layout`.
- The test holds the expected values as hard-coded literals, independent of the implementation:
  - all 32 Halton(2,3) values across three cycles, with `assert_ne!(sample, [0.0; 2])`;
  - all 240 bytes of a packed record (jittered matrix at 0, previous at 64, unjittered at 160, jitter at 152/156,
    mip bias −1 at 224, phase at 228);
  - the Naga member offsets and span of `ViewRecord` (240) and `ViewAuxRecord` (144), and the subrectangle at aux
    128–144.
- Index 1..32 never gives (0,0), because the base-2 term is 0 only at index 1, where the base-3 term is −1/6.
- RENDER-SPEC §8 states the same layout. Its SHA-256 `a2e43a39…` matches all 11 ledger entries in `maturity.json`,
  and no old hash remains.
- The previous matrices come from the last submitted frame:
  - `commit_frame` is called only after `queue.submit` (`runtime.rs:1354`), and the capture path never calls it;
  - `pack_view(&self.view_sample, …)` packs `view_sample`, which `commit_frame` alone sets;
  - the abandoned-encoder case in `metal_sky_pan_…` asserts this, and the phase advances only there.

### D1.2: pass

- In `synthetic/motion/tests.rs`, the real `Rg16Float` motion attachment is copied to a buffer after the real
  forward pass, and every texel is decoded.
  - `metal_static_jitter_…` covers 35 jitter phases, failing on any nonzero bit. Its control moves the instance by
    0.1 and requires x < −1 px. Spawn and cut require the sentinel.
  - `metal_perspective_geometry_sky_and_stars_…` covers perspective geometry, sky and a star over 35 jittered phases,
    all exact zero.
  - Both passed in my run.
- The shader forms the clip delta before projection (`synthetic.wgsl` `vertex_at`/`motion_vector`) from
  `unjittered_view_proj`. The algebra is correct: `(c+d)/(w+dw) − c/w`.
- My independent check on real content: I matched 48×48 colour patches between consecutive submitted day-pan frames
  and compared the image displacement with the decoded motion view at the same pixel, over 16 frame pairs.
  - Sign and magnitude agree, for example 6.56 px at the centre against 6.5–7.4 px from the image.
  - The residual swings ±0.9 px with the Halton phase and averages 0.3 px over a partial cycle. That is the jitter
    difference, as expected for unjittered motion.

### D1.3: pass

- The expected sky motion is a CPU ray from the focal length and yaw, not the shader's inverse. At 5,5, 32,32 and
  58,58 the error is below 0.001 px (tolerance 0.05).
- The camera-still steps are each asserted: sidereal-only (star moves 0.586 px, sky exactly 0), solar 0.01 rad
  (disk −0.585 px) and solar 0.0001 rad (−0.00586 px against −0.00586 px).
- On the seam 4 ruling: the `sky.wgsl` diff only adds `infinity_motion` and the motion output, with radiance
  untouched. `stars.wgsl` adds motion, a `discard` where the weight is 0 (which adds nothing to colour), and
  `centre + jitter`. That last change is part of the jittered frame contract, not the atmosphere's look.
- On real content, during a pure-yaw pan the motion is continuous across the skyline: 0 codes of difference above
  against below at 8 of 9 columns, and 1 code (0.06 px) at the ninth. That means sky motion at infinity agrees with
  the geometry's projection. See [motion crop](M1-review-1/day-pan-t60-skyline-motion-crop-x2.png) and
  [colour crop](M1-review-1/day-pan-t60-skyline-colour-crop-x2.png).

### D1.4: pass

Where each mark is set and cleared:

- **Spawn.** `synthetic.wgsl:141-146`: `valid` requires a nonzero previous serial and a matching identity, generation
  and serial. Otherwise `motion_vector` returns `(65504,65504)` (line 571). It clears on the next frame, after
  `commit_frame` → `history.submitted()`.
- **Camera cut.** `history_gpu.rs:179` writes `read_serial = 0` for the reset sample, so every synthetic and
  contributed instance is invalid for one sample. Cooked uses `draw.rs:619`
  (`continued && !frame.history_reset`), and the history byte is rewritten on both paths. The Metal test asserts the
  cut sentinel.
- **Device replacement.** `runtime.rs:705` resets history on the replacement view, and the new `PreparedSynthetic`
  has fresh GPU history. `inherit_submitted_view` copies only CPU camera metadata and phase. A new cooked `Bound`
  starts `continued = false` (`draw.rs:612`).
- **Cooked asset swap.** The new `Bound` starts with `continued = false`. `lib.rs:931` sets it only after a frame
  with `history_advanced`, and captures reset it (line 945).
- **Can any forward drawer emit zero for geometry with no previous position?**
  - Synthetic and contribution draws share `vertex_at`.
  - Cooked direct and indirect share `cooked_vertex` and its fragment, with the sentinel.
  - The cooked volume always writes the sentinel (S3).

  None writes zero for invalid geometry. The sky and stars are outside the item's list; see S2.

### D1.5: pass

**Inspection list, written before looking:**

1. Whole-frame pan (noon and night): direction is history minus current, and magnitude grows from centre to the
   horizontal edges.
2. Skyline at native size during a pan: no band of zero or invalid motion where terrain meets the sky.
3. Near object against far terrain during translation: parallax, with a crisp boundary at the silhouette.
4. Star field at night, held and panning: stars carry the pan plus a small sidereal term, with no holes.
5. Moving object with the camera held: only the mover (and stars) nonzero, in the mover's direction.
6. Onset and stop of a pan: motion starts and stops on the right ticks, with no lingering.
7. Edge conditions: fast camera (3°/tick), low sun (18.2 h), distance (far ridges), a cut.

**What I captured** (all mine, from this commit; fixed-time submitted engine output, 60 samples/s; captures and PNGs
in `tmp/m1-review/cap/`):
- the builder's day, night and moving routes with the motion view;
- my [fast low-sun pan](M1-review-1/fast-lowsun.json), in motion and in colour;
- a night pitch-up and pan, then a cut ([pre](M1-review-1/night-pre.json), [post](M1-review-1/night-post.json));
- day pan colour;
- the held views at [640×360](M1-review-1/held-day-640.json) and 1080p.

**I watched motion** as ordered frame sequences (contact sheets at chosen ticks, consecutive native frames, and
values decoded per tick), not by playing video on a display.

1. **Pan.** On the noon pan, x is +6.6 px at the centre, rising to ≥ 7.6 px at the third points and clamped at 8 at
   the edges. Every sampled row shows the same, with y symmetric about the centre. The camera turns right, so the
   history lies to the right (positive x). The colour-patch match confirms this. Night is the same, plus the pitch
   term. Pass.
2. **Skyline.** Continuous within 0.06 px (see D1.3). Pass.
3. **Near and far.** At tick 150 of the moving route, the sphere has a crisp motion silhouette, distinct from the
   terrain behind it, and the motion changes smoothly with depth down the ground plane
   ([crop](M1-review-1/moving-t150-near-far-native-crop.png), [sheet](M1-review-1/moving-motion-sheet.png)). Pass.
4. **Stars.** In the night hold, about 22,600 star texels carry x 0.35–0.66 px and y +0.16 px of sidereal motion
   while the sky reads exactly 0 ([×20 amplified](M1-review-1/night-hold-stars-motion-crop-x4-amplified20.png)). In the
   pan, stars sit within a few codes of the sky's pan motion ([crop](M1-review-1/night-pan-stars-motion-crop-x4.png)).
   No holes. Pass.
5. **Mover.** In the held day view, the cube (3 m/s, moving right) reads x ≈ −2.4 px as a clean silhouette, and
   stationary terrain and sky read exactly 0 ([colour against motion, two ticks](M1-review-1/day-hold-mover-colour-vs-motion-t160-t204.png)).
   Pass.
6. **Onset and stop.**
   - Noon pan: the centre reads a constant 6.62 px through the pan, 3.29 px at tick 121 (the half-tick sample as the
     pan ends) and 0 from tick 122.
   - Fast pan: motion starts at tick 31 and stops at tick 62, matching keys 30 and 60 with the ½ phase.
   - No lingering. Pass.
7. **Edge conditions run:**
   - fast camera: direction is right, but the view saturates (S1);
   - low sun: the sky was correct, but the Sun disk was not in frame, so the solar term rests on the Metal test;
   - distance: far ridges continuous;
   - cut: frames from tick 64 onwards are correct; frames 60–63 can't be captured (S5).

   Not run: storm (the route format has no weather control), and full refresh (this item is fixed-time motion
   correctness; no cadence is claimed).

The view's ±8 px saturation and the invalid colour that saturation also reaches (S1) weaken the view as a tool. But
on every entry above, direction and growth were judged from unsaturated regions and decoded values, so the item's
statement holds.

## Builder evidence

I read it only after forming the verdicts above. It agrees with what I saw. One caveat: its D1.5 says magenta marks
invalid geometry, but in the moving route magenta is mostly saturation (S1). Its moving window stops before the cut
(S5).

## Checks run

- `./dev check` at `40319eaf`: exit 0 (log at `tmp/m1-review/dev-check.log`, not committed).
- Release build of `thrum-cook` and `v1-game`; `thrum-cook cook` gave the route's root.
- 11 engine captures, packaged by `scripts/apple_presentation_m1.py`. Each validated its submission, tick, phase,
  history and exposure identities.

FAIL M1: 0 fail, 0 cant_tell, 1 outside the items

# APPLE-PRESENTATION M1 review (round 2)

Reviewer: Claude Opus 5.5 (high), independent of the builder. 11 November 2026.
Commit judged: `e0956ee65b4222ccf2bda6860351f490d7c19155`, built from a clean checkout (every capture manifest records
the empty-diff hash `e3b0c442…`). Target: mac-metal (Apple M3 Max, portable recipe). Release `v1-game` built with
rustc 1.99.1, binary SHA-256 prefix `7be1636e00b70e3f`. Showcase store cooked by me from
`examples/v1-game/assets/showcase`, root `70d2baf8…e21d`, which matches every route.

**Result: PASS.** Round 1's only Must Fix, `jitter-presented-without-reconstruction`, is fixed. All five items pass
again on this commit. Nothing new rises to a Must Fix. Round 1's evidence and images stay in [M1-review/](M1-review/);
this round's are in [M1-review/r2/](M1-review/r2/).

| Item | Verdict | Basis |
|---|---|---|
| D1.1 | pass | `./dev check` run by me (exit 0); new selection test read; RENDER-SPEC text and hash checked |
| D1.2 | pass | `./dev check`; Metal readback test read, now with a colour positive control under explicit jitter |
| D1.3 | pass | `./dev check`; unchanged since round 1 except the star jitter gate, which is read |
| D1.4 | pass | Re-traced: the repair changes only which matrix is packed at offset 0; no mark moved |
| D1.5 | pass | My own fixed-time engine captures of this commit, watched as ordered sequences, with decoded values |

## Round 1 Must Fix

### `jitter-presented-without-reconstruction`: fixed (verified)

**The change.** `SyntheticView` now has two selectors:

- `with_jitter_sequence` keeps the Halton phase and offsets but rasterises unjittered;
- `with_jitter_phase` also turns on raster jitter, for a temporal consumer or an explicit witness.

`raster_projection()` (`crates/thrum-render/src/synthetic/view.rs:346`) chooses the matrix, and the following all use
that one choice:

- `pack_view` offset 0, which feeds geometry, the cooked draws and visibility;
- `contribution_frame`, for contribution culling (`contributed.rs:152`);
- the star raster offset (`stars.rs:243`).

The game selects `with_jitter_sequence` (`examples/v1-game/src/host/scene_source.rs:513`), and
`PreparedSynthetic::update_frame` uses `at_jitter_phase`, which keeps the view's selection
(`synthetic.rs:1779`). I grepped every use of `jittered_projection`, `raster_projection` and `with_jitter_*` in
`crates/` and `examples/`. No other raster path reads the jittered matrix. The sky was already unjittered, because
`pack_aux` inverts `view_proj`.

**What I did.** I repeated round 1's held-camera capture at 640×360 (the game's render extent, overlay off). I added
two held views: a night view pitched up 12° into the star field, and a low-sun view (18.2 h, facing west). I then
compared every consecutive pair of the 70 pairs in each, with my own comparison script, not the builder's.

| Held view, 70 consecutive pairs | Pixels > 24 codes per pair | Where they are | Round 1 |
|---|---|---|---|
| Day, noon | 70–96 | only the moving cube, its shadow and the sphere's shadow edge (bbox x 212–476, y 220–279) | 950–1,900 over every silhouette, skyline and texture |
| Night, stars | 2–25 | short horizontal star streaks from sidereal motion | not run |
| Low sun | 0 (peak delta 1 code) | everywhere, from solar time advancing | not run |

- In the [day change map](M1-review/r2/held-day-640-union-change-map.png), red marks any pixel that changed over
  all 70 pairs. Only the cube, the shadows and the sphere's shadow ring are red; the sphere silhouette, skyline,
  ridges and ground texture are not. The shadow movement is real: this route runs the world at about 288× solar time
  (12.013 → 12.106 h in 70 ticks), so the Sun moves visibly.
- Four consecutive native frames, ×4: the [sphere](M1-review/r2/held-day-640-sphere-4-consecutive-x4.png) and the
  [skyline](M1-review/r2/held-day-640-skyline-4-consecutive-x4.png) don't move. Compare round 1's
  [sphere](M1-review/held-640-sphere-4-consecutive-frames-x4.png).
- In the [night change map](M1-review/r2/held-night-640-union-change-map.png), stars trace consistent short horizontal
  streaks, which is sky rotation, not random crawl. Four consecutive native frames of the
  [star field ×4](M1-review/r2/held-night-640-stars-4-consecutive-x4.png) show steady stars.
- The [low-sun map](M1-review/r2/held-lowsun-640-union-change-map.png) changes everywhere, but by at most one code.

**Restored regressions.** I read the restored tests:

- `lifecycle_gpu.rs:67` now requires 31 consecutive paused frames to equal `before` byte for byte;
- `recovery_gpu.rs` drops `closest_phase` and requires the first replacement frame to be within one code of its
  single pre-loss frame, and a consecutive replacement frame to be byte-identical;
- `mixed_mesh_field_keeps_its_meshes_through_gpu_replacement` compares against its single `before` frame.

All of them ran and passed in my `./dev check`. Round 1's proposed item would now pass.

**The jittered path is still proved.** `metal_static_jitter_…` (`motion/tests.rs:120-169`) keeps the 35-phase
zero-motion test under explicit raster jitter. It adds a positive control: under that jitter the submitted colour must
change at least once. It then switches to `with_jitter_sequence` and asserts four things on each of 35 submissions:
the phase advances, the offsets are nonzero, offset 0 equals offset 160, and the colour is byte-identical.

## Must Fix

None.

## Suggestions

S1–S5 are carried over from round 1. Each was checked again on this commit.

- **S1 The motion view still saturates at ±8 px, and saturated motion reuses the invalid colour (verified again).**
  - In the moving route at tick 196, the bottom-left corner decodes as (255,0,255), the invalid magenta, but it comes
    from saturation (x ≥ 8, y ≤ −8). See the bottom row of
    [the moving sheet](M1-review/r2/moving-motion-sheet.png).
  - The 3°/tick pan clamps the whole frame to x = −8 ([fast low-sun sheet](M1-review/r2/fast-lowsun-motion-sheet.png)).
  - The fix is still a distinct invalid pattern and a selectable or logarithmic scale.
- **S2 Sky and stars ignore cuts and missing history.** `atmosphere/gpu.rs:248` is unchanged
  (`submitted_sky.unwrap_or((*view, sun))`). M2's reactive work should cover it.
- **S3 The cooked volume invalidates everything it covers (suspected).** Unchanged.
- **S4 Mip bias is a constant −1.** Unchanged. It is correct at native scale, but M5 must derive it.
- **S5 Cut frames can't be captured.** The route tool still refuses checkpoints within 4 ticks of a cut, so I again
  couldn't capture the cut onset. M3's D3.4 needs that window.
- **S6 is resolved for the shipped path.** Spatial output now uses the same unjittered matrix as clusters and shadows.
  The mismatch now exists only under explicit raster jitter, and M3 should recheck it there.
- **S7 (new; verified, probably harmless) Daytime stars write motion.** In the day pan after the camera stops (tick
  150, noon), about 24,000 texels are nonzero. They are the moving cube plus 3×3 star footprints across the sky, with
  x up to +0.6 px of sidereal motion, although the stars are invisible in daylight
  ([crop ×4](M1-review/r2/daypan-t150-sky-dot.png)). The sky gradient around them reads exactly 0.
  - A reconstructor would reproject those texels by under a pixel over smooth sky, which does no visible harm.
  - Still, `stars.wgsl` could also discard where the star's contribution is below one display code, so the motion
    target matches what's on screen. M2's reactive rule for stars should use the same threshold.
- **S8 (new) D1.1's wording against the repair.** D1.1 says the record carries "jittered and unjittered matrices". In
  the shipped spatial mode, offset 0 now holds the unjittered matrix. The jittered matrix is packed only when a
  temporal consumer selects it.
  - I pass D1.1 because round 1 asked for exactly this split, and because RENDER-SPEC now defines offset 0 as the
    selected raster matrix.
  - A planner should still reword D1.1 and the design's record list (DESIGN.md ~410) to "the selected raster matrix
    (jittered when a temporal consumer is active)", so the written item matches the contract.

## Items

### D1.1: pass

- My `./dev check` exited 0, with 63 test groups OK. That includes
  `temporal_tests::continuous_halton_values_and_exact_record_layout`, which is unchanged since round 1: the literal
  Halton values, `(0,0)` never occurring, all 240 bytes, and the Naga offsets.
- It also includes the new `spatial_output_retains_the_sequence_and_both_matrices`. Over 96 phases it checks that:
  - spatial and temporal records agree from byte 64 onwards;
  - spatial offset 0 equals offset 160, while temporal offset 0 differs;
  - the offsets are nonzero and the phase is `phase % 32`;
  - the selection survives `at_jitter_phase`.
- RENDER-SPEC §8 states the new selection rule, with the layout unchanged. Its SHA-256 `fcc5ed87…72ae` matches all 11
  `RENDER-SPEC.md` entries in `maturity.json`.
- Previous matrices still come from the last submitted frame: `commit_frame` runs after `queue.submit`, and the
  abandoned-view case still asserts this (`metal_sky_pan_…` printed `after abandoned view, first nonzero texels: []`).

### D1.2: pass

- The real `Rg16Float` attachment is read back after the forward pass, across 35 explicitly jittered phases, and every
  texel must be exactly zero.
- The moving control must reach x < −1 px, and the spawn must write the sentinel.
- The new colour control proves that the test really jitters. Without it, the selector change could have silently
  turned jitter off in the test as well.
- The shader still derives motion from `unjittered_view_proj` (`synthetic.wgsl:134,149`; `cooked.wgsl:30,41`).
- On real content, with no camera motion after the pan, sky and terrain decode as exactly 0, code 128
  (`day-pan` ticks 122–240).

### D1.3: pass

The sky and star motion code is unchanged since round 1. The only atmosphere-side change is that `stars.rs:243`
writes the raster jitter offset only under raster jitter, which still touches only the star's raster centre, not its
radiance or its motion. The Metal test passed in my run, matching the independent CPU projection: at 5,5 the GPU gave
1.3984 against 1.3988 px, the sidereal-only step gave 0.5859 against 0.5861, and the solar step gave −0.5854 against
−0.5858. All are well within 0.05 px.

### D1.4: pass

- The repair changes only the matrix at offset 0 and the culling matrix.
- The spawn, cut, device-replacement and cooked-swap marks traced in round 1 are untouched: `synthetic.wgsl`
  `valid`/sentinel, `history_gpu.rs` `read_serial = 0`, `draw.rs` `continued && !frame.history_reset`, and fresh
  `Bound` and `PreparedSynthetic` on replacement.
- The Metal test still asserts the spawn and cut sentinels and the replacement path (`motion/tests.rs:170-188`).
- No forward drawer gained a zero-motion path.

### D1.5: pass

**Inspection list, written before looking** (as in round 1, re-run on this build):

1. Whole-frame pan, noon and night: direction is history minus current, and magnitude grows from the centre to the
   edges.
2. Skyline during a pan: no band of zero or invalid motion.
3. Near object against far terrain during translation: parallax and a crisp silhouette.
4. Star field at night: stars carry the pan plus a small sidereal term, with no holes.
5. Moving object with the camera held: only the mover (and stars) nonzero.
6. Onset and stop of a pan: no lingering.
7. Edge conditions: fast camera, low sun, distance, a cut.

**What I captured** (all mine, from this commit; fixed-time submitted engine output, 60 samples/s, motion view on):

- the builder's day, night and moving routes (`docs/evidence/APPLE-PRESENTATION/m1/*-route.json`);
- round 1's [fast low-sun pan](M1-review/fast-lowsun.json).

Raw captures are in `tmp/m1-review-r2/cap/` (about 10 GB, not committed). I watched motion as ordered sequences:
contact sheets every 24 ticks ([day](M1-review/r2/day-pan-motion-sheet.png),
[night](M1-review/r2/night-pan-motion-sheet.png), [moving](M1-review/r2/moving-motion-sheet.png)), every 8 ticks for
the fast pan, and values decoded at a 7×5 grid per tick.

1. **Pan.** The noon pan turns right, so history lies to the right and x is positive. At tick 60, x is +6.62 at the
   centre, +7.06–7.18 at the third points and clamped at +8 at the edges, on every row. y is antisymmetric about the
   centre (+1.35 at the top-left, −1.41 at the top-right). The night pan shows the same. Pass.
2. **Skyline.** In a pure-yaw pan, the motion field is smooth across the horizon: no seam is visible in the sheets,
   and decoded rows don't step at the skyline. Pass.
3. **Near and far.** The moving route at tick 150 has a crisp sphere silhouette. Sky (teal), terrain (green) and the
   sphere are each distinct, and the motion changes smoothly down the ground plane
   ([native crop](M1-review/r2/moving-t150-near-far-native-crop.png)). Pass.
4. **Stars.** In the night route after the pan, 26,800 nonzero texels range from x 0.03 to 0.66 px, y up to 0.35
   px. These are the stars' sidereal term; the only negative x is the moving cube (−2.7 px). Sky texels read 0. No
   magenta. Pass.
5. **Mover.** With the camera held after the pan, the cube is the only geometry with nonzero motion: x −2.2 to −2.7
   px against a zero background. Pass.
6. **Onset and stop.** In the noon pan, the centre reads 6.62 px through the pan, 3.29 px at tick 121 (the half-tick
   sample) and 0 from tick 122. No lingering. Pass.
7. **Edge conditions run:**
   - fast camera: x clamped at −8 for a left turn, which is the correct direction, but saturated (S1);
   - low sun: the fast pan at 18.2 h, plus the held low-sun colour view under MF1;
   - distance: far ridges are continuous in the pan;
   - night: stars;
   - cut: not capturable (S5); the Metal test covers the cut sentinel.

   Not run: storm (the route format has no weather control), and full refresh (this item claims no cadence).

## Builder evidence

I read [the repair evidence](m1-mf1/README.md) after forming the verdicts above. Its held-view numbers (70–96 pixels
over 24 codes at 640×360; skyline, ground and sphere silhouette at 0) match my independent run exactly. The night and
low-sun held views are my additions.

## Checks run

- `./dev check` at `e0956ee6`: exit 0, 63 groups OK. The log is at `tmp/m1-review-r2/dev-check.log` (not committed).
  It includes every Metal motion test, `threaded_pause_keeps_pixels_still…`, and all four `recovery_gpu` tests.
- `cargo build --release -p v1-game -p thrum-cook`; `thrum-cook cook` gave the route's root.
- 7 engine captures (3 held with the overlay off, 4 with the motion view), each packaged and validated by
  `scripts/apple_presentation_m1.py package`: 71, 71, 71, 237, 237, 193 and 71 frames, all complete.
- My held-view comparison and crop scripts were throwaway, kept outside the repository. The routes I wrote are in
  [M1-review/r2/](M1-review/r2/): [held day](M1-review/r2/held-day-640.json),
  [held night](M1-review/r2/held-night-640.json) and [held low sun](M1-review/r2/held-lowsun-640.json).

PASS M1

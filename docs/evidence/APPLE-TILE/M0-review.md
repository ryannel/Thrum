# APPLE-TILE M0 review: A repeatable offscreen valley proves the comparison path

- **Reviewer:** Claude Opus 5.5 (high), independent of the builder. 11 November 2026.
- **Commit judged:** `c2b6e42e2102b79e5a7a0339c08283e23376ebcc`, a clean checkout. I built the release `v1-game` (with
  `draw-probe`) and `thrum-cook` from it, and cooked a fresh valley store.
- **Questions answered:** code correctness of the capture/comparison path, correctness of the controls, and visual
  inspection of stills and motion. No performance or cadence claim was made or judged.
- **Result:** all five items pass. There are no Must Fix findings. Four Suggestions are listed below.
- **First review:** `fails` was 0, so there was no earlier review to follow up.

Captures are in [M0-review/](M0-review/). Where my capture was byte-identical to a committed baseline, I link the
committed file rather than duplicating it.

## What I ran

All GPU runs held the `gpu` lock.

| Run | Result |
|---|---|
| `scripts/apple_tile_m0.py` with a missing store | `REFUSED missing cooked store`, exit 2 |
| `scripts/apple_tile_m0.py` with a different cooked store (`examples/v1-game/assets`) | `REFUSED cooked root differs from the route`, exit 2 |
| The documented M0 command from a fresh build and cook | `PASS M0 capture proofs`. The manifest records revision `c2b6e42e`, an empty dirty diff and rustc 1.99.1, which is the `rust-toolchain.toml` pin. |
| Fresh route checkpoints against the **committed** baselines in `native-baseline/` | **Byte-identical, with identical identity sidecars:** all 5 SDR `rgba8`, all 5 `rgba16f` scene-linear and all 5 1:1 surface BGRA. This includes the exposure GPU words, `history_epoch` and `submission_id`. |
| Fresh T2 stills against the committed baselines | Byte-identical: `sun-sdr`, `sun-linear`, `night-sdr`, `night-sdr-off`, `night-linear` and `night-linear-off`. |
| Fresh letterbox controls (1000×800 from a 1280×720 source) against the committed ones | Byte-identical for both Bgra8Unorm and Bgra8UnormSrgb. |
| Negative controls (pixels changed at the mirror-light checkpoint) | Phase 696,207; missing contribution 650,623; mirror slot 5060 13,344; weak light 104,594. Night stars changed 12,231 SDR pixels and 18,252 linear pixels. These match the builder's figures exactly. |
| Edge (mine): twilight preset as a T2 still, SDR and scene-linear, each captured twice | The repeats are identical in frame SHA-256 and exposure words. See [twilight still](M0-review/edge-twilight-1080p.png). |
| D0.3 check: `cargo test --locked --offline -p thrum-render --features creation-counters creation -- --nocapture` | 12 passed, plus 1 ignored. The ignored test is the release-only ledger cost measurement. |
| D0.5 check: `python3 scripts/run_v1_showcase.py moving` | PASS, from 2 native runs and 2 actual Chrome runs. There were no identity mismatches over 361 ticks, and all 5 checkpoints repeat on both targets. |
| Retained Chrome pre-change baseline (481 frames) | All 481 decoded frames match their recorded SHA-256 at the full 1280×720 size. The manifest hash matches `current-comparison.json`. |
| `python3 -m unittest test_t2_capture` | 17 tests OK. |

I did not run the full `./dev check`, because the builder's milestone gate covers it. I ran every item's named check
myself.

Edge conditions for mac-metal:
- **Low sun:** run. The route uses solar 17.3, the sun still is a sunset, and I added the twilight repeat.
- **Distance:** run, through the approach checkpoint and the far ridges in the stills.
- **Fast camera:** run. The route closes fast on the cubes before the cut.
- **Full refresh:** run as T2 realtime at a 120 Hz opportunity rate. Submit gaps were a median of 8.28 ms and a maximum
  of 12.4 ms over 964 images. This says nothing about panel cadence, and no cadence claim is made.
- **Storm:** not run. The product has no storm preset (`examples/v1-game/params/` has none), and no M0 item covers
  weather.

## Code read

I read all changed code in the milestone commits:
- `be252c0d` holds the D0.3 seam, read through `runtime/creation.rs`, `creation_off.rs` and the `creation_sites.rs`
  inventory test.
- `0a1ad2d1` adds the comparator and its tests.
- `03bc878a` adds the renderer capture source, readback/exposure copy, scene graph, submitted capture, the v1-game
  route/variant/overlay controls, `apple_tile_m0.py` and the `capture_v1_runtime.py` extensions.
- `f66d05f4` and `c2b6e42e` are evidence and records.

I looked for the signs of work that only looks finished and found none:
- **The comparator** compares raw stored bytes, with no float decode and no tolerance. It refuses layout or extent
  mismatches and validates PNG CRCs. It writes native-size masks and crops only when the inputs differ.
- **The fixture controls** (`THRUM_G3T_OVERLAY_CONTROL`, `THRUM_G3T_OMIT_PLACEMENT`, mirror omission) are compiled
  only under `cfg(all(feature = "draw-probe", not(wasm32)))`. They are off by default, and the script refuses a binary
  built without `draw-probe`.
- **The ordinary renderer** is unchanged when capture is SDR:
  - The scene attachment gains `COPY_SRC` only for `SceneLinear` targets.
  - The capture terminal pass adds only an exposure-buffer copy read, which explains the new `(0, 4)` dependency in
    the GPU graph test.
  - No WGSL changed in the milestone range.
- **Scene-linear capture** prepares replacement targets only after frames retire. It restores SDR in
  `scene_linear_capture_uses_the_ordinary_submission_and_can_restore_sdr`, which asserts identical SDR bytes after a
  linear session.
- **The stored baselines are real.** Rerunning from source reproduced them exactly, so they are not stale outputs
  committed in place of computed ones.
- **D0.3 coverage:**
  - Production `create_*_pipeline(` calls exist only in the two seam files.
  - The inventory test pins the raw-device escapes as an exact list, and its text match also catches calls made
    through `Deref`.
  - Default builds compile `DeviceSite` to direct wgpu calls, with no lock (`creation_off.rs`).

## Inspection list (written before looking)

I wrote this list before looking at any capture:
1. **Sun side:** sky gradient, sun disc and glow, and lit terrain at full 1920×1080.
2. **Far valley:** ridge silhouettes, aerial perspective, and the far terrain at the approach checkpoint.
3. **Near valley:** foreground ground texture and the near-plane checkpoint's close cube.
4. **Mirror and local light:** the mirrored cube (slot 5060), its shadow, the light pool from the overlay light, and
   cast shadows from the cubes and sphere.
5. **Cut:** the pre-cut and post-cut checkpoints, and frames on either side of the cut in the realtime sequence.
   Looking for ghosting or history carried across the cut.
6. **Night:** stars on and off, occlusion of stars by terrain at native size, and the dark landscape.
7. **Motion at 1×:** route continuity, tick monotonicity, the cut at its authored tick, the final hold, and the
   variant noon→night→noon switch with its pan.
8. **Present control:** the 1000×800 letterbox geometry and content in sRGB.
9. **Omission controls:** whether each diff is the intended omission, and not noise or a whole-frame shift. Phase is
   expected to shift the whole frame.

## Results per entry

The committed baseline images are linked because my captures are byte-identical to them.

1. **Sun side:** [sun still](../render/apple-tile-m0/native-baseline/sun-sdr/checkpoint-0.png). The sky gradient,
   sun disc, glow and lit terrain are coherent. At y≈670 there is a hard horizontal tonal step between two terrain
   bands. It is pre-existing content and stable (Suggestion S3).
2. **Far valley:** [approach](../render/apple-tile-m0/native-baseline/sdr-1/approach.png). The ridge silhouettes
   layer cleanly with aerial fade. Stable.
3. **Near valley:** [near-plane](../render/apple-tile-m0/native-baseline/sdr-1/near-plane.png). The close cube fills
   the view, and its edge and notch silhouette against the cube behind is correct. Stable.
4. **Mirror and local light:** [mirror-light](../render/apple-tile-m0/native-baseline/sdr-1/mirror-light.png) and
   [the omission stack](M0-review/mirror-omit-stack.png) (top: reference, middle: slot 5060 omitted, bottom: native
   difference crop). Omitting the slot removes the grey mirrored cube and its long shadow, and lights the face of the
   blue cube that it had shadowed. The light pool and the cube shadows are present and stable. The difference is the
   intended object, not noise.
5. **Cut:** [10 fps strip, 3.9–4.9 s](M0-review/route-cut-strip-10fps.png),
   [pre-cut](../render/apple-tile-m0/native-baseline/sdr-1/pre-cut.png) and
   [post-cut](../render/apple-tile-m0/native-baseline/sdr-1/post-cut.png).
   - The history epoch changes exactly once, at tick 380 (origin 180 + cut 200).
   - The first post-cut frame shows no ghost of the pre-cut view.
   - The foreground blurs heavily during the fast approach before the cut. I read this as camera motion blur, but it
     is strong (Suggestion S3).
6. **Night:** [native ridge crop](M0-review/night-ridge-native-crop.png) and
   [stars on](../render/apple-tile-m0/native-baseline/night-sdr/checkpoint-0.png) /
   [off](../render/apple-tile-m0/native-baseline/night-sdr-off/checkpoint-0.png). Stars stop cleanly at the terrain
   silhouette, with no stars drawn over terrain. The star-off control removes them. The night sky is a flat neutral
   grey (Suggestion S3).
7. **Motion at 1×:** [route at 4 fps](M0-review/route-motion-strip-4fps.png) and
   [variants at 4 fps](M0-review/variant-motion-strip-4fps.png). I sampled frames from the 1× host-time previews. I
   did not watch the video live.
   - **Route:** 964 images over ticks 179–600, monotonic, with a maximum step of 2 ticks. The camera advances through
     the valley, cuts once, then holds the final close view.
   - **Variants:** 481 images. Noon→night→noon switches at the two recorded `prepared-star-variant-switch` actions,
     with a continuous pan.
   - Every image carries an 8-word exposure packet.
8. **Present control:** [sRGB letterbox](M0-review/letterbox-srgb-mirror-light-1000x800.png). The content area is
   1000×562 and centred, with bars of about 119 px above and below. The content matches the 1280×720 source.
9. **Omission controls:**
   - Phase and contribution change the whole frame or large regions, as expected.
   - The weak-light control changes 104,594 pixels at mirror-light, with dropped references recorded in both arms.
   - The star omission is confined to the sky.
   - All are nonzero, with native-size masks in the qualification output.

## Items

### D0.1: pass
The single documented command, given a fresh build and cook, drives the real cooked game. It records:
- route samples for every tick, 361 ticks per fixed run;
- five fixed checkpoints, SDR and linear;
- the cut, the mirror and local-light overlay, and the prepared variant switch;
- T2 stills and realtime motion with full provenance.

It refuses missing and mismatched cooked roots before capture. Star-on, star-off and the switch are all present.
`run_v1_showcase.py`, the route and the G7 comparator are untouched in the milestone range, and the moving smoke
still passes. Every cited tool command is implemented and ran. The inspection list above passed against the bar's
stability terms.

### D0.2: pass
- The SDR, scene-linear and surface repeats are exact.
- The committed pre-change baselines, with identity sidecars (source, sample, exposure words, H=1, format and history),
  reproduce bit-for-bit from a fresh build of the claimed commit.
- Both SDR and sRGB full-frame present controls exist at a non-1:1 extent with letterbox.
- All five required negative controls fail exact comparison, with native-size masks.
- Realtime motion is accounted by host time and sample, not by movie index.
- There is no fusion, tolerance or float oracle.
- The native witness was correctly left to M10.

### D0.3: pass
The named check passes on this Metal device. Production coverage is enforced by the inventory test. Same-generation
clones (`a_cloned_record_shares_the_site…`) and detached preparation (`a_detached_preparation_overlapping_play…`)
are covered. Live-object deltas are documented and reported separately from creation events. There is no native-object
instrumentation, and default builds have no diagnostic lock.

### D0.4: pass
The probe names the actual counter (Xcode Profile › Counters › Memory, "Bytes Read From / Written To Device Memory")
and its KiB/MiB units. Attribution is per encoder, explicitly not per attachment. The limits are stated: one replay
per trace, a shared and loaded machine, and a ±1 MiB noise floor. The raw scratch (screenshots and TSVs) is disclosed
as uncommitted. No new physical-traffic result is claimed.

### D0.5: pass
The moving command passes, with actual Chrome twice and native twice. Per-tick identity matches across the source,
content and sample fields. Its scope is stated as compositor captures with no temporal claim. The 481-frame
full-resolution Chrome pre-change baseline exists and verifies frame by frame, and current Chrome matches it
(`chrome-baseline/current-comparison.json`).

## Findings

There are no Must Fix findings. The Suggestions follow, all verified.

- **S1. The retained bulk sources live only in the builder's worktree, and the README path is ambiguous.**
  - The README cites `tmp/g3t-m0/...` as the home of `qualification-2` (2.4 GB) and `prechange-chrome-fixed`
    (610 MB, the 481-frame Chrome baseline that M1 must compare against).
  - Those directories exist only at `/Users/ryannel/Workspace/thrum/.claude/worktrees/apple-tile-m0/tmp/g3t-m0/`.
    They are not in the main checkout's `tmp/`, and the README notes there is no remote backup.
  - Removing that worktree would delete the only full Chrome baseline.
  - Fix: before the worktree is cleaned, move or record the sources at an absolute, worktree-independent location
    under the storage rules.
- **S2. The weak-light control is a substitution, not a pure omission.**
  - In the overflow fixture, 34 co-located lights compete for a 32-entry cluster (`cluster.rs` `CAPACITY = 32`).
  - Zeroing light 31 (1000 cd) lets light 32 (250 cd) take its cluster slot. So `weak-off` compares 1000 cd with
    250 cd in that slot, not "with" against "without".
  - It still proves exact comparison catches a weak-light change in a saturated cluster, so the item holds. The
    README's wording "omission of one nonzero weak light" slightly overstates it. A future control could drop the
    light's reference while keeping the light in the list.
- **S3. Pre-existing look issues in the portable valley.** M0 doesn't change any of these, but the later visual
  milestones measured against this bar should know about them:
  - The night sky is flat neutral grey.
  - The sun still has a hard tonal step between two terrain bands (y≈670).
  - In the twilight still the sphere and rock have no visible contact shadow and read as floating.
  - Foreground ground blur during the fast pre-cut approach is strong.

  They are stable across repeats, so they don't fail M0's stability bar.
- **S4. The proof suite has no storm or weather case.** The project settings list storm as a mac-metal edge condition,
  but the V1 product has no storm preset. Future milestones that claim weather coverage need one added to the route or
  presets.

## Proposed item (none required)

No Must Fix was found outside the items, so no new item is needed. For S1, a later D-item could require that retained
baselines are recorded at a worktree-independent path and checked for existence by the M1 comparison command.

PASS M0

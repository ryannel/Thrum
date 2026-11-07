# G3s finite-Sun penumbra — 7 November 2026

The fixed valley camera no longer shows a one-frame foreground shadow jump as the Sun sets behind the ridge. The
two Sun cascades now filter a finite solar disk: a percentage-closer soft-shadow (PCSS) search for the nearest
upstream blocker, then a disk filter. A receiver-bending bound stops terrain creases from shadowing their own
receiver at low Sun. [Build plan](../../BUILD-PLAN.md), [first-light evidence](G3S-FIRST-LIGHT-2026-11-06.md).

![Top: main at solar frames 34 and 35. Middle: candidate at 34 and 35. Bottom, contrast ×3: the earlier PCSS
draft and the candidate at solar frame 30](g3s-first-light/solar-penumbra-compare.png)

[Candidate solar crossing, frames 20–40](g3s-first-light/solar-penumbra.mp4). These are native Metal offscreen
frames at 1280×720, 30 s of solar time apart, played at 5 fps. It is a sampled sequence, not live motion.

## What changed

- **Finite Sun.** The PCSS search and finite-disk filter come from the Codex work saved on
  `codex/wip-g3s-solar-penumbra`. The blocker search uses only upstream depths, and the plane offset is bounded by
  the nearest blocker gap.
- **Receiver bending.** At grazing Sun, a height deviation h above a planar receiver shows up as h/(n·l) along the
  light ray. So a crease or concave join a texel away reads as a blocker. One such case had the Sun 4.8° above a
  valley triangle and a join 0.6 m upstream rising 0.032 m/m. A continuous surface inside the filter footprint
  occludes only if it rises faster than the Sun's elevation above the receiver, tan(e) = 1/g, where g is the
  receiver depth slope. Depth within 0.9·tan(e) (capped at 0.25) × the tap's world offset of the plane now counts
  as the receiver's own surface. A fixed 0.05 slope fixed the patches but left dashed lines along steeper creases.
- **Taps.** 8 search taps and 16 disk taps. They score the same as 16/32 against ray truth, with no visible banding.

## Ground truth

A CPU ray tracer (`tmp/g3s-showcase/solar-penumbra-claude/probe.rs`) intersects the source `terrain.gltf`
triangles. It casts 64–512 rays across the 0.267° solar disk from each sampled pixel's visible receiver, using the
same camera and Sun model. It reproduces the handoff's visibility at pixel (640,600) for frames 33–38: 0.889,
0.707, 0.494, 0.283, 0.102, 0.

| Pixel (640,600), frames 32–38, SDR | Values |
| --- | --- |
| Main (`91f524b`) | 197 197 196 **93** 94 95 95 |
| Candidate | 197 188 177 149 129 122 95 |
| Ray truth mapped to lit/shadow levels | 197 185 166 145 124 105 95 |
| Shadows off | 197 197 196 196 195 195 194 |

The largest candidate step is 27 codes, at frame 37→38, where the truth step is 10. The conservative blocker
search lags a little as the last of the disk sets.

The table below scores 8-pixel-grid samples within the 500 m receiver reach (10,779 per solar frame) against a
shadow-off render of the same build. "False" means the ray truth is lit (>0.99) but the render is more than 3%
darker.

| Solar frame | Codex draft | Fixed 0.05 slope | Candidate |
| --- | --- | --- | --- |
| 22 | — | 2 | 0 |
| 26 | 72 | 1 | 0 |
| 30 | 135 | 20 | 0 |
| 31 | 84 | 19 | 0 |

Across solar frames 0–32, full-frame pixels darker than shadow-off by more than 3 codes fall from 1,305 (16/32
taps, fixed slope) to 49, with at most 8 in any one frame. On the yaw-0 views (sequence 02/03/20/21 and noon) the
false counts do not change. They are dominated by the sphere and rock, which the terrain-only probe omits.
Shadows beyond the 512 m receiver reach are absent in every build; at 1.7–6.4 km they cover about 20% of the
frame.

## Review and checks

- **Independent review.** Fresh-context reviewers saw only images and requirements.
  - The first compared main, the Codex draft and the fixed-slope candidate, labelled A/B/C. It ranked the
    candidate first: main fails the gradual crossing, and the draft shows visible streaks and stipple.
  - A second reviewer failed the fixed-slope build on a diagonal dotted line and a flickering dashed line. Those
    are the crease artifacts fixed above.
  - The final build's review is recorded in the handoff.
- **Tests.** The shipping GPU oracle (`synthetic/shadow/tests/penumbra.rs`) adds a concave grazing-join fixture
  that fails without receiver bending. Full `./dev check` passed on Metal. Chrome `sky-reflections` passed with
  Wasm SHA-256 `69e64cd8f511d29615d883c88c13600d7a6a03e5538c1e386403143c99ffa152`. That run uses the diagnostic
  scene only.
- **Cost.** Shared-GPU ABBA at 1080p, valley scene, load average 10–11, `--g3b-threaded-batch`. Scene pass-sum
  p50: baseline 1.71 ms, 16/32 taps 2.18 ms, 8/16 taps 1.97 ms (measured before the elevation bound, which adds
  only per-tap arithmetic). Forward+sky p50 rises from 0.39 to 0.51 ms. Raw
  logs are in `tmp/g3s-showcase/solar-penumbra-claude/bench/`.

## Open

- **Distant terrain.** Terrain beyond 512 m casts and receives no Sun shadow. The candidate is a heightfield
  horizon method.
- **Solar 77→78 step.** A 100-code foreground step appears even with shadows off, so it comes from the
  twilight/illumination path.
- **Sphere shadow edge.** It is serrated at noon at the 1024² near-cascade resolution.
- **Rock top.** A few dark texels remain at noon. These may be real crevice shadow.
- **Cost.** The +15% pass-sum is recorded as a baseline. Gathered fetches and a lit early-out are untried.
- **Still open as before:** the Chrome showcase scene, live motion, and iPhone.

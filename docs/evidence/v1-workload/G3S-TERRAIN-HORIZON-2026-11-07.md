# G3s terrain horizon shadows — 7 November 2026

Receivers past the Sun cascades' 512 m reach now take Sun visibility from a horizon marched over a top-down
caster height map. Before this, the distant valley at sunset stayed sunlit while ray truth had it in shadow. It
follows [the finite-Sun penumbra](G3S-SOLAR-PENUMBRA-2026-11-07.md) in [the build plan](../../BUILD-PLAN.md)'s
G3s item.

![Top: solar frame 30, main then candidate. Bottom: the 17.9 h sunset view, main then
candidate](g3s-first-light/terrain-horizon-compare.png)

[Candidate solar sequence, frames 14–44](g3s-first-light/terrain-horizon.mp4). These are native Metal offscreen
frames 30 s of solar time apart, played at 5 fps. It is a sampled sequence, not live motion.

## Method

- **Height map.** A third layer of the shadow depth array holds a top-down reverse-Z map of the highest caster
  surface. It covers 32 km around the camera at 1024² (32 m texels) and snaps to whole texels. The existing
  caster pipeline draws it, including cooked contributions, as the graph pass "synthetic Sun horizon heights".
- **Horizon.** Receivers beyond the cascade fade start march toward the Sun's azimuth. Up to 96 geometric
  steps span the nominal map width and stop at the footprint boundary. The march keeps the steepest elevation. Visibility is the
  circular-segment fraction of the 0.267° disk above that horizon. Inside the fade band it blends with the last
  cascade.
- **Early exits.** The tallest-caster bound can stop the march when the slope threshold is nonnegative.
  For a negative threshold its height falls with distance, so lower terrain can still cover part of the Sun.
  The upper-limb exit still stops once the disk is covered.

The Metal plateau oracle at a solar elevation of −0.1° failed on the old shader (visibility 1.0 against
0.3383) and passes with a 0.015 tolerance. It also checks fully lit and fully blocked receivers. A separate
Metal oracle exercises near/last blocker-window composition with different cascade depth spans.
The cooked valley preserved all 191 native frames and 34 Chrome images. At 1080p from 17.98 h, the reversed
pair under shared load 14.8–15.9 gave scene GPU p50/p95 7.71/9.42 and 8.43/10.68 ms before,
7.60/9.15 and 7.68/9.06 ms after. A concurrent first pair was highly variable. No speed claim.
Raw tests, runs and hashes are in `tmp/handback-review/horizon-bound*/`.
- **Why not a third cascade.** The terrain spans 36 × 36 km. A view-fitted cascade over that range would have
  about 30 m texels and would still need padding for kilometre blockers. The height map gives a physical
  finite-disk penumbra for terrain-on-terrain at any distance and reuses the caster pass.

## Ground truth

A CPU ray tracer intersects the source `terrain.gltf` triangles with a 64-ray Vogel disk per receiver. A debug
build writes Sun visibility as a red/green ratio, which cancels exposure. Mean absolute visibility error by
receiver distance:

| Solar frame | < 500 m | 0.5–2 km | 2–6 km | > 6 km |
| --- | --- | --- | --- | --- |
| 22, main | 0.000 | 0.57 | 0.94 | 1.00 |
| 22, candidate | 0.000 | 0.060 | 0.036 | 0.000 |
| 26, candidate | 0.000 | 0.062 | 0.004 | 0.000 |
| 30, candidate | 0.000 | 0.029 | 0.000 | 0.000 |
| 34, candidate | 0.049 | 0.000 | 0.000 | 0.000 |

Main rendered every distant receiver lit. With the candidate, no sample is off by more than 0.5. Two
iterations were rejected by this scoring:
- An 8 km radius missed a 939 m peak 12 km upstream.
- 40 steps spaced about 17% apart skipped that summit: 500–2,000 m mean error was 0.55 at frame 22. With 64
  steps it was 0.29.

## Checks

- **Tests.** A timing test keeps the height pass in the far-shadow bucket. The penumbra oracle runs with the
  horizon disabled. The fit test pins the near map unpadded.
- **Cost.** Shared-GPU ABBA at 1080p, valley scene, load average 2.2–2.7. Scene pass-sum p50 is 2.02 → 2.50 ms
  (+24%): forward+sky 0.54 → 0.75 ms and shadow 0.095 → 0.149 ms. Before the early exits it was 2.74 ms. Logs
  are in `tmp/g3s-showcase/solar-penumbra-claude/bench/`.
- **Repository and browser.** `./dev check` passed. The Chrome WebGPU `sky-reflections` case passed with Wasm
  SHA-256 `fb37319b49222dedba89633594fbaad2bb7452c998fc59e5ce7fcb7dc1c6877d`.

## Independent review

A fresh-context reviewer saw only the candidate frames and requirements. It found the ridge shadow climbing the
mid-ground smoothly, the floor dimming gradually, no popping, and plausible alpenglow. It raised two shadow items,
both checked against ray truth:
- **A brightness step about 60 m out (row 463).** It is identical on main. Truth visibility is uniform across it:
  0.48 above and below at frame 35. The source mesh has a Sun-facing rim there, with N·L 0.119 against 0.076 on
  the floor, so this is real geometry rather than a shadow seam.
- **A ragged distant peak edge with specks (sequence frame 21).** A 2-pixel truth grid over that peak gives a mean
  error of 0.063. Of the 130 samples off by more than 0.5, 121 lie within 4 pixels of a truth edge; the staircase
  partly follows the real silhouette and partly the 32 m texels. Nine interior specks remain (0.4%), six of them
  falsely dark. This is height-map self-occlusion on slopes near grazing.

The reviewer also noted 8-bit contour banding on the smooth floor gradient. That is output quantization, not
shadowing.

## Open

- **Edge placement and specks.** Distant edges place to about one 32 m texel and show rare near-grazing specks.
  A filtered horizon-angle map (below) is the place to fix both.

- **Cost.** The scalable form is a per-frame horizon-angle map: a coarse compute march over the height map,
  then one lookup per pixel. A max-height mip chain would also allow fewer steps.
- **Non-terrain casters.** Receivers above the terrain, such as objects past 512 m, are treated by height alone.
  Small distant casters beyond 512 m appear only through the 32 m height texels.
- **Map edge.** Occluders beyond 16 km of the camera are absent.

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
- **Near map unpadded.** The draft padded both maps by the Sun cone over the largest caster gap. At low Sun
  that gap reaches kilometres, which grew the near map from 40 m to about 130 m half-extent and faded the sphere's
  contact shadow over 20–25 pixels. The near map is now fitted to its receivers. Each receiver finds the largest
  blocker gap whose search and filter disk fit inside it. Nearer blockers use the near map, farther ones the
  padded last cascade, and the result is the minimum of the two.
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
false counts do not rise. They are dominated by the sphere and rock, which the terrain-only probe omits.
Shadows beyond the 512 m receiver reach are absent in every build; at 1.7–6.4 km they cover about 20% of the
frame.

The probe also ray-traces the sphere for contact. Over sequence 21's contact region (pixels 380–616 × 500–538),
mean error against truth mapped to lit/shadow levels is 1.04 codes on main, 2.6 with the padded near map and 0.93
now. The ridge-crossing pixel values and solar scores above are identical with and without the padding change.

### Whole-floor transition

An oversight review of the clip measured the far floor (about 35–55 m) shadowing after the near floor, with a
45-code drop at frames 37→38. A debug build that writes Sun visibility directly shows visibility is uniform
across the floor and tracks ray truth:

| Solar frame | 34 | 35 | 36 | 37 | 38 |
| --- | --- | --- | --- | --- | --- |
| Rendered visibility, every floor row | 0.83 | 0.51 | 0.29 | 0.15–0.22 | 0 |
| Ray truth | 0.69–0.70 | 0.48 | 0.25–0.28 | 0.09–0.11 | 0 |

The lit far floor sits at 247–252 SDR, on the tone curve's shoulder, so its display codes understate the early
darkening and show the final step as larger. The render lags truth by roughly 15 s of solar time. Shadow maps
measure the blocker on the ridge's Sun-facing side, so the blocker estimate is slightly late. That residual is
recorded rather than tuned.

The same review found a grey-blue, stair-stepped band along the floor's far edge in every frame. It was a
shading fault: a Sun-facing slope (n·l 0.128 in the source geometry) whose interpolated normal pointed behind
the view vector. The repair reflected that normal across V's tangent plane, which flips n·l negative for a
camera looking into the Sun. The normal is now lifted 0.01 onto V's visible side. Exact projection left n·v at
zero and produced black BRDF noise. The grazing GPU oracle was updated to the lifted normal.

## Review and checks

- **Independent review.** Fresh-context reviewers saw only images and requirements.
  - The first compared main, the Codex draft and the fixed-slope candidate, labelled A/B/C. It ranked the
    candidate first: main fails the gradual crossing, and the draft shows visible streaks and stipple.
  - A second reviewer failed the fixed-slope build on a diagonal dotted line and a flickering dashed line. Those
    are the crease artifacts fixed above.
  - A third reviewer failed an interim build on the sphere's low-Sun contact fade, fixed by the unpadded near map.
  - A fourth reviewer passed the final build on all five requirements: gradual darkening over frames 33–38 with
    a largest step of 13.5 foreground codes, no false shadowing, attached contact, no leaks and smooth penumbrae.
    It noted that the ground reaches full shadow at frame 38 while a thin sliver of disk remains. That matches
    ray truth, where the last few percent of the disk contribute little.
  - A fifth reviewer checked the floor-edge seam fix. It found no seam and no new noise. It failed the
    set on the 37→38 step (far-edge band 175 → 149 → 110 codes over frames 36–38), which is the measured
    visibility lag above. It also noted dark marks on the rock top at noon and isolated dark pixels along the
    far floor edge in the yaw-0 views. Both are present unchanged in the previous build.
- **Tests.** The shipping GPU oracle (`synthetic/shadow/tests/penumbra.rs`) adds a concave grazing-join fixture
  that fails without receiver bending. Full `./dev check` passed on Metal. Chrome `sky-reflections` passed with
  Wasm SHA-256 `bfcae7209bf56c63b70f13d92a6b822cea24c0aac54caa1a7a6e7695ff336e35`. That run uses the diagnostic
  scene only.
- **Cost.** Shared-GPU ABBA at 1080p, valley scene, load average 10–11, `--g3b-threaded-batch`. Scene pass-sum
  p50: baseline 1.71 ms, 16/32 taps 2.18 ms, 8/16 taps 1.97 ms. A later pair with the final build (unpadded near
  map, load average 7.5–7.9) measured 1.71 → 2.02 ms, with forward+sky 0.39 → 0.54 ms. Raw
  logs are in `tmp/g3s-showcase/solar-penumbra-claude/bench/`.

## Open

- **Distant terrain.** Resolved by [terrain horizon shadows](G3S-TERRAIN-HORIZON-2026-11-07.md).
- **Sky brightening at sunset.** Explained; it is not a radiance error. The capture meters incident light on a
  horizontal plane: sky irradiance plus direct Sun scaled by its elevation sine. The direct term vanishes first,
  so exposure rises. With a fixed camera (EV 13), the top-of-frame sky falls steadily from 145 codes at frame 14
  to 133 at 30, 112 at 50 and 72 at 80, and the floor falls with it. Whether sunsets should use a
  scene-weighted meter instead is a look choice for later.
- **Solar 77→78 step.** Fixed by [two-sided cooked casters](G3S-SUNSET-CASTERS-2026-11-07.md). It was a shadow
  leak.
- **Sphere shadow edge.** It is serrated at noon at the 1024² near-cascade resolution, as on main. A 1.5-texel
  minimum filter radius smoothed it but doubled the contact error, so it was not kept.
- **Two-map composition.** The near/last split has a fit test and capture truth scores, but no shipping GPU
  oracle fixture yet.
- **Rock top.** Dark marks on the Sun-facing top at noon. They may be acne or bad normals, not crevice
  shadow; score them with a probe that includes the rock.
- **Final-sliver lag.** Rendered visibility 0.15–0.22 at frame 37 against truth 0.10 makes the last step
  the largest. Check whether the cooked ridge silhouette differs from the source mesh before tuning the blocker
  estimate.
- **Far-edge specks.** Isolated pixels 20–35 codes dark along the far floor edge in the yaw-0 views.
- **Cost.** The +18% pass-sum is recorded as a baseline. Gathered fetches and a lit early-out are untried.
- **Not shadowing.** The review's most visible flaw is a staircase far-ridge silhouette (about 5 px steps) and
  faceted terrain normals under contrast boost. Both are terrain geometry or level of detail.
- **Still open as before:** the Chrome showcase scene, live motion, and iPhone.

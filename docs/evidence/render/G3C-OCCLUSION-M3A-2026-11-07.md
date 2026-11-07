# G3c M3a: Occlusion with a depth prepass and an ordered final draw

7 November 2026. Apple M3 Max/Metal, Rust 1.99.1, wgpu 30.0.1, Chrome on the same Mac. Branch `claude/g3c-m3a`:
- `49c22e58` replaces the M2 split with the depth prepass and the ordered final draw;
- `6846cb17` derives the depth tolerance and adds the full-footprint tie witness;
- `b2d6d1bc` adds the contribution, history and Chrome witnesses and the cost benches.

The base is main `45fc5d05`. Root approved the graph for the branch ("Root review of the M3a graph in 49c22e58").
After rebasing onto main, the same three commits are `3c2fbd3c`, `987e013b` and `05549900`; main's additions are
docs only. Every run here shared a heavily contended GPU
(load average 110–350). No row is a speedup claim.

**Status:** Occlusion runs in the game's field. It is byte-equal to Direct natively and in Chrome, and it hides
more than half of Frustum's draws on 99.3% of field ticks. On 12-triangle cubes it costs more than it saves, as the
design predicted (E1). The field default stays Frustum and the grid default Direct until M3c's realistic-mesh rows.

## The frame
Under `CullMode::Occlusion` the camera view runs:
1. `camera prepass list`: frustum ∧ last frame's visible bit, in slot order.
2. `camera depth prepass`: depth-only, into a private transient Depth32Float.
3. `camera depth pyramid`: min reduction (reverse-Z) of that depth.
4. `camera final cull`: every active slot is tested against the frustum, then the pyramid. It writes the final list
   in slot order and the next history bits from the same mask words.
5. `synthetic forward opaque`: unchanged. It draws the final list.

The forward pass draws in slot order into cleared depth, as Direct does, so equal-depth ties resolve as in Direct.
There is no contribution fallback.

## Correctness (native, Metal)
| Witness | Result |
| --- | --- |
| Prepass parity (`the_depth_prepass_writes_the_forward_depth_of_the_same_list`) | Prepass depth is byte-equal to forward depth for the same list, one- and two-sided; a one-unit bias mutant fails |
| Equal depth, both entering orders | `[0, 0]` pixels changed (M2: `[3216, 0]`) |
| Full-footprint ties, origin and far extent (`a_box_flush_with_a_prepass_wall_keeps_its_ties_near_and_far`) | No pixel changed; the box wins 5,300 of 11,236 face pixels on rounded ties at the far turned pose |
| Contribution (`a_blended_contribution_stays_on_top_while_occlusion_hides_a_slot`) | Final counts (2, 1, 1) against Frustum's (2, 2, 2); every frame is byte-equal to Direct with the veil, and the veil overlaps the cube on more than 1,000 px; drawing the contribution before the final list fails it |
| History (`stale_bits_after_a_swap_or_a_reveal_keep_direct_images`, `history_words_never_outlive_a_shrink_capture_or_contribution`) | Hidden, swap, held, back, reveal and cut frames are byte-equal to Direct; no stale word outlives a shrink |
| Field flythrough, full path (`field_gpu`, release) | 1,930 frames are byte-equal to Direct; Occlusion never draws more than Frustum and draws at most half of it on 1,917 of 1,930 ticks |

The depth tolerance is derived, not tuned (design, *Depth tolerance*). On cubes a zero-tolerance mutant also passes
the tie witness, because axis-aligned corners equal the vertices exactly and turned boxes have loose bounds. M3c
re-runs the witness with a mesh face on its bounds.

## Chrome
`node scripts/check_v1_browser.mjs --case field-oracle` (now three pages) captures the paused 100,513-instance field
at 16 views, 1280×720. Frustum and Occlusion are each byte-equal to Chrome Direct on all 16. Each view change leaves
Occlusion one frame of stale history. The 600-frame Occlusion qualification (`--case field-occlusion`) passes on the same Wasm.

## `@invariant` changes Direct
The forward vertex output gains `@invariant` so the depth-only pipeline computes the same position. Direct shares
that shader, so its images change:

| Comparison against the parent (`45fc5d05`) | Pixels differing |
| --- | --- |
| Metal, 10k grid, 320×180, 5 views × Sun on/off | 3 views equal; `inside` 10 px at 1 LSB; `partial` 1 px at 239 (a sub-pixel sliver whose visible face flips) |
| Metal, field at tick 901, 640×360 | 142 px at 1 LSB |
| Chrome, 16-view field oracle, 1280×720 | 9–102 px per view (≤0.011%); 1–11 px per view above 1 LSB |
| Chrome, the tip without `@invariant` | 16 of 16 views equal to the parent; Occlusion still byte-equal to Direct |

Direct and Frustum match each other in both trees. Frustum's pass names and order are unchanged with and without the
Sun.

**Root ruled A** (NOTES, "Root ruling on position invariance for M3a"):
- keep `@invariant`, since the WGSL guarantee is the prepass contract as vertex paths grow;
- keep the parity test and the image oracles alongside it;
- accept the sub-pixel Direct change as an explicit shader revision.

Retained in [g3c-m3a-invariance/](g3c-m3a-invariance/):
- the parent and tip images for the high-delta Metal grid views, the field at tick 901 and the Chrome view with the
  most pixels above 1 LSB;
- `direct-invariance-diff.json`, the one-time diff: every differing pixel, as `[x, y, parent, tip]`, for all 37
  compared images;
- three motion crops.

**In motion.** Native Direct along the route (567 frames at 640×360, parent against tip) differs on 0.064% of pixels
on average; 705 pixels in 220 frames differ by more than 16 levels. The motion crops (5× zoom, consecutive frames,
parent above tip) centre on the three frames with the most of them. They show 1-pixel shifts of silhouette edges
that move with the edge, with no isolated flicker or popping in either tree. G7 route against its existing
baseline (`scripts/run_v1_showcase.py check` on `05549900`, both targets, six stops): pass. The worst difference
from the checked-in baseline is 0.003, 0.004 and 0.005 against thresholds of 1.0, so no tracked baseline changes
(`tmp/g3c-boundary/m3a-g7-check.log`).

## Cost (contended)
`cargo bench -p thrum-render --bench occlusion_passes`: 1280×720, with Sun shadows. Frustum and Occlusion are
interleaved in ABBA blocks (60 measured), with history committed every frame. Each row gives medians from three runs
(`tmp/g3c-boundary/m3a-cost{1,2,3}.log`; run 1 misnamed the pyramid pass, so its pyramid reads 0). The behind-cover
scene is the one where the grazing test asserts that Occlusion hides at least 30% of 10k slots.

| Count | View | Run | Whole GPU F→O | Within-block O−F | F visibility | O prepass list | O depth prepass | O pyramid | O final cull | Forward F→O |
| ---: | --- | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| 10k | open | 1 | 0.971 → 1.086 | +0.124 | 0.022 | 0.021 | 0.009 | — | 0.025 | 0.794 → 0.791 |
| 10k | open | 2 | 1.924 → 3.751 | +1.143 | 0.128 | 0.260 | 0.019 | 0.335 | 0.074 | 1.107 → 1.080 |
| 10k | open | 3 | 2.425 → 2.966 | +0.560 | 0.137 | 0.182 | 0.023 | 0.339 | 0.071 | 1.151 → 1.040 |
| 10k | behind cover | 1 | 1.229 → 1.346 | +0.138 | 0.022 | 0.021 | 0.009 | — | 0.025 | 1.054 → 1.054 |
| 10k | behind cover | 2 | 2.717 → 2.740 | +0.272 | 0.278 | 0.150 | 0.020 | 0.298 | 0.054 | 1.517 → 1.282 |
| 10k | behind cover | 3 | 2.899 → 3.729 | +0.383 | 0.255 | 0.218 | 0.039 | 0.378 | 0.088 | 1.547 → 1.274 |
| 100k | open | 1 | 3.137 → 3.328 | +0.210 | 0.107 | 0.099 | 0.017 | — | 0.105 | 1.986 → 1.978 |
| 100k | open | 2 | 5.556 → 8.600 | +2.233 | 0.314 | 0.366 | 0.055 | 0.389 | 0.240 | 2.817 → 2.766 |
| 100k | open | 3 | 5.425 → 6.136 | +0.962 | 0.316 | 0.250 | 0.057 | 0.347 | 0.209 | 2.727 → 2.753 |
| 100k | behind cover | 1 | 2.305 → 2.491 | +0.230 | 0.107 | 0.099 | 0.018 | — | 0.090 | 1.169 → 1.151 |
| 100k | behind cover | 2 | 4.786 → 5.228 | +0.487 | 0.267 | 0.238 | 0.048 | 0.363 | 0.207 | 1.700 → 1.661 |
| 100k | behind cover | 3 | 4.010 → 5.626 | +2.077 | 0.244 | 0.342 | 0.058 | 0.370 | 0.187 | 1.578 → 1.696 |

Run 1 had the lightest contention (load about 230, against Frustum's visibility pass costing three times as much in
runs 2–3). Every run puts Occlusion above Frustum.

The culling passes add about 0.1–0.4 ms each. The pyramid (11 single-level dispatches on 1024×512) is the largest
of them under contention. Forward saves little on cubes. Behind cover the wall is slot 0, so Frustum's slot-ordered
forward already draws it first, and early depth rejects the hidden fragments. Only the vertex work of 12-triangle
boxes is left to save. This is the loss E1 predicted. The pyramid's per-level dispatches are the first candidate for
M4's native recipes.

Field route (`cargo bench -p v1-game --bench field_culling`; paired O−F per tick, split by whether Occlusion draws
more than half of Frustum, `m3a-fieldcost1.log`, load about 290):
- whole GPU p50, Direct → Frustum → Occlusion: 3.837 → 3.652 → 3.928 ms;
- paired O−F behind cover: 1,788 ticks, median +0.257 ms, p95 +3.316 ms;
- in the open: 12 ticks, median +0.183 ms.
The field report folds the culling passes into the forward bucket, so its forward column for Occlusion includes
them.

**Memory**, extra per render target over Frustum:
- the prepass depth, 3.69 MB at 1280×720 (8.29 MB at 1080p);
- the pyramid, 2.80 MB (5.59 MB at 1080p, on the 1024×1024 power-of-two floor);
- a second slot list, 0.40 MB at 100,513 slots;
- two history sides, 12.6 KB each.

## Motion
The full-path run saved 567 frames (640×360) per culled mode across seven stretches of the route. I reviewed them
as contact sheets: 24 across the path, 16 consecutive frames and a close pass through cubes. There are no holes,
missing cubes, shadow pops or flicker at silhouettes. All 567 Occlusion frames are byte-equal to Frustum's.

## Not covered
- Chrome does not run the native full path or the cube witnesses; its evidence is the oracle and the qualification.
- Device memory was not measured; the figures above are logical sizes.
- No quiet-GPU timing exists. A no-contention pair would change the magnitudes, not the direction.

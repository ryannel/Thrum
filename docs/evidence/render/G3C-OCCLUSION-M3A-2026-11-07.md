# G3c M3a: Occlusion with a depth prepass and an ordered final draw

7 November 2026. Apple M3 Max/Metal, Rust 1.99.1, wgpu 30.0.1, Chrome on the same Mac. Branch `claude/g3c-m3a-land2`,
rebased onto main `0c166280`. Hashes before the rebases are in brackets; main's commits since the first base
`45fc5d05` are docs only:
- `1cd2b4a9` replaces the M2 split with the depth prepass and the ordered final draw (`49c22e58`);
- `8c2004af` derives the depth tolerance and adds the full-footprint tie witness (`6846cb17`);
- `e3adf80a` adds the contribution, history and Chrome witnesses and the cost benches (`b2d6d1bc`, `05549900`);
- `9a5d459e` records this evidence first (`25d72238`);
- `aed91237` adds the GPU slack witnesses and tightens the Chrome and resize checks (`e73b2945`, review R-M3a).

Root approved the graph for the branch ("Root review of the M3a graph in 49c22e58"). Every run here shared a heavily
contended GPU (load average 60–350). No row is a speedup claim.

**Status:** Occlusion runs in the game's field. It is byte-equal to Direct natively and in Chrome, and it draws at
most half of Frustum's draws on 99.3% of field ticks. On 12-triangle cubes it costs more than it saves, as the
design predicted (E1). The field default stays Frustum and the grid default Direct until M3c's realistic-mesh rows.

**Provenance.** Each run names its tree in the first line of its log in `tmp/g3c-boundary/`:

| Run | Tree |
| --- | --- |
| Grid cost runs 1–3 (`bench-history/cost{1,2,3}.log`) | runs 1–2: main `9e811aa2` plus the bench fix (diff hash in each log); run 3: main `1344aec0` plus the final counted-frame bench, as landed |
| Withdrawn grid rows (`m3a-cost{1,2,3}.log`) | `6846cb17` plus the bench of `b2d6d1bc`, which never advanced history |
| Native full path (`m3a-fullpath.log`) | `6846cb17` plus the contribution and history witnesses; field code as at `b2d6d1bc` |
| Field cost (`m3a-fieldcost1.log`) | `b2d6d1bc` |
| Motion frames and diff (`m3a-motion-diff.log`) | renderer `6846cb17`; parent main `45fc5d05` |
| G7 route check (`m3a-g7-check.log`) | `05549900` |
| Mutants (`m3a-mutants.log`, `m3a-epsilon-mutants.log`) | `25d72238` plus the review fixes later committed as `e73b2945` |
| Chrome oracle and qualification (`m3a-chrome-final.log`) | `e73b2945`, clean tree, Wasm sha256 `abb322e1…` |
| `./dev check` | `m3a-devcheck6.log` at `b2d6d1bc`, `m3a-devcheck7.log` at `25d72238`, and the final tip's log posted to root |

Runs name the pre-rebase hashes. Renderer source is identical from `6846cb17` to `25d72238`; `e73b2945` changes
only a WGSL comment and tests, and each rebase changed only main's docs.

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
| Prepass parity (`the_depth_prepass_writes_the_forward_depth_of_the_same_list`) | Prepass depth is byte-equal to forward depth for the same list, one- and two-sided; a one-unit bias mutant fails (`m3a-mutants.log`) |
| Equal depth, both entering orders | `[0, 0]` pixels changed (M2: `[3216, 0]`) |
| Full-footprint ties, origin and far extent (`a_box_flush_with_a_prepass_wall_keeps_its_ties_near_and_far`) | No pixel changed; at the far turned pose the box covers 5,300 pixels in Direct and Occlusion changes none (`m3a-mutants.log`) |
| GPU slack at its boundary (`gpu_slack_matches_the_reference_at_its_boundary`, `the_final_cull_uses_the_full_clip_magnitude`) | The shader flips within two ulps of the reference's decision, with and without clip terms; `DEPTH_ULPS = 0`, `CLIP_GAMMA = 0`, a translation-free magnitude and a z-row-for-w magnitude each fail (`m3a-epsilon-mutants.log`) |
| Contribution (`a_blended_contribution_stays_on_top_while_occlusion_hides_a_slot`) | Final counts (2, 1, 1) against Frustum's (2, 2, 2); every frame is byte-equal to Direct with the veil, and the veil overlaps the cube on more than 1,000 px; drawing the contribution before the final list fails it (`m3a-mutants.log`) |
| History (`stale_bits_after_a_swap_or_a_reveal_keep_direct_images`, `history_words_never_outlive_a_shrink_or_capture`) | Hidden, swap, held, back, reveal and cut frames are byte-equal to Direct; no stale word outlives a shrink |
| Resize (`culled_modes_survive_surface_resize_and_device_replacement`) | After `resize_surface` and after device replacement, Occlusion's capture equals Direct's (a one-cube scene, so nothing is hidden) |
| Field flythrough, full path (`field_gpu`, release) | 1,930 frames are byte-equal to Direct; Occlusion never draws more than Frustum and draws at most half of it on 1,917 of 1,930 ticks |

The depth tolerance is derived, not tuned (design, *Depth tolerance*). On cubes a zero-tolerance mutant also passes
the tie witness, because axis-aligned corners equal the vertices exactly and turned boxes have loose bounds. The GPU
boundary tests pin the shader's slack to the derivation; M3c re-runs the tie witness with a mesh face on its bounds.

## Chrome
On `e73b2945` (Wasm `abb322e1…`, `m3a-chrome-final.log`):
- `node scripts/check_v1_browser.mjs --case field-oracle` runs three pages on the paused 100,513-instance field at 16
  views, 1280×720. Frustum and Occlusion are each byte-equal to Chrome Direct on all 16. Each view change leaves
  Occlusion one frame of stale history. At the last view Occlusion draws 1,180 against Frustum's 22,153, and the
  script asserts Occlusion draws fewer.
- `--case field-occlusion` is a 600-frame functional qualification, not an image comparison. The script asserts the
  page runs `culling=Occlusion` and that its drawn p50 (1,153) is below a fifth of the active count (100,513).

Chrome has no depth readback, so prepass parity there rests on `@invariant` plus the image oracle (root's ruling).

## `@invariant` changes Direct
The forward vertex output gains `@invariant` so the depth-only pipeline computes the same position. Direct shares
that shader, so its images change:

| Comparison against the parent (`45fc5d05`) | Pixels differing |
| --- | --- |
| Metal, 10k grid, 320×180, 5 views without the Sun and 4 with it (9 images) | 5 equal; in each Sun state, `inside` 10 px at 1 LSB and `partial` 1 px at 239 (a sub-pixel sliver whose visible face flips) |
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
- `direct-invariance-diff.json`, the one-time diff: every differing pixel, as `[x, y, parent, tip]`, for the 26
  compared images (9 Metal grid, 1 field, 16 Chrome);
- three motion crops.

**In motion.** Native Direct along the route (567 frames at 640×360, parent against tip) differs on 0.064% of pixels
on average; 705 pixels in 220 frames differ by more than 16 levels (`m3a-motion-diff.log`). The motion crops (5× zoom,
consecutive frames, parent above tip) centre on the three frames with the most of them. They show 1-pixel shifts of
silhouette edges that move with the edge, with no isolated flicker or popping in either tree. G7 route against its
existing baseline (`scripts/run_v1_showcase.py check`, both targets, six stops): pass. The worst difference from the
checked-in baseline is 0.003, 0.004 and 0.005 against thresholds of 1.0, so no tracked baseline changes
(`m3a-g7-check.log`).

## Cost (contended)
**Corrected 8 Nov.** The bench rows first recorded here came from a bench that called `commit_frame` without
`update_frame`. History never advanced, so the prepass list was always empty and the Occlusion arm did Frustum's work
(main review 2026-11-08, finding M1). Those rows (`m3a-cost{1,2,3}.log`) are withdrawn. The bench now updates and
commits each frame as a host does. It reads every frame's culling counts and asserts that Occlusion's prepass is
non-empty and that, behind cover, its final list hides at least 30% of Frustum's. Counts come from `PreparedSynthetic::submit_counted`, which records, submits and maps in
`FrameHost`'s order. A mutant without `update_frame`
fails that assertion on its first measured frame (`bench-history/mutant-no-update.log`).

`cargo bench -p thrum-render --bench occlusion_passes`: 1280×720, with Sun shadows. Frustum and Occlusion are
interleaved in ABBA blocks (60 measured). Rows are medians from three runs under load of about 210–340
(`bench-history/cost{1,2,3}.log`; run 3 is on the final counted-frame API). Run 1 was the most contended: Frustum's
visibility pass costs 3–20 times what it does in runs 2–3. Drawn counts are identical in all runs.

| Count | View | Run | Whole GPU F→O | Within-block O−F | F visibility | O prepass list | O depth prepass | O pyramid | O final cull | Forward F→O | Drawn F→O (O prepass) |
| ---: | --- | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- | --- |
| 10k | open | 1 | 6.031 → 7.975 | +2.162 | 0.427 | 0.367 | 0.371 | 1.012 | 0.247 | 3.101 → 2.411 | 10,000 → 10,000 (10,000) |
| 10k | open | 2 | 1.246 → 1.287 | +0.217 | 0.023 | 0.024 | 0.051 | 0.088 | 0.029 | 0.895 → 0.879 | 10,000 → 10,000 (10,000) |
| 10k | open | 3 | 1.009 → 1.187 | +0.141 | 0.022 | 0.022 | 0.049 | 0.080 | 0.027 | 0.802 → 0.804 | 10,000 → 10,000 (10,000) |
| 10k | behind cover | 1 | 7.836 → 10.727 | +3.283 | 0.466 | 0.442 | 0.178 | 1.294 | 0.313 | 4.137 → 2.832 | 2,142 → 78 (78) |
| 10k | behind cover | 2 | 1.441 → 1.641 | +0.181 | 0.024 | 0.029 | 0.015 | 0.085 | 0.026 | 1.096 → 1.093 | 2,142 → 78 (78) |
| 10k | behind cover | 3 | 1.415 → 1.566 | +0.123 | 0.023 | 0.023 | 0.015 | 0.080 | 0.025 | 1.060 → 1.052 | 2,142 → 78 (78) |
| 100k | open | 1 | 7.607 → 7.932 | +0.712 | 0.196 | 0.219 | 0.551 | 0.242 | 0.143 | 3.077 → 2.814 | 100,000 → 100,000 (100,000) |
| 100k | open | 2 | 3.783 → 4.738 | +0.623 | 0.136 | 0.138 | 0.336 | 0.092 | 0.103 | 2.256 → 2.279 | 100,000 → 100,000 (100,000) |
| 100k | open | 3 | 3.142 → 3.691 | +0.601 | 0.129 | 0.132 | 0.330 | 0.088 | 0.101 | 2.053 → 2.049 | 100,000 → 100,000 (100,000) |
| 100k | behind cover | 1 | 3.310 → 3.892 | +0.078 | 0.147 | 0.143 | 0.023 | 0.112 | 0.102 | 1.441 → 1.246 | 20,962 → 647 (647) |
| 100k | behind cover | 2 | 2.751 → 2.954 | +0.114 | 0.140 | 0.136 | 0.021 | 0.093 | 0.089 | 1.285 → 1.145 | 20,962 → 647 (647) |
| 100k | behind cover | 3 | 2.272 → 2.387 | +0.105 | 0.126 | 0.119 | 0.020 | 0.087 | 0.074 | 1.170 → 1.063 | 20,962 → 647 (647) |

Every row puts Occlusion above Frustum, including behind cover, where it draws 3–4% of Frustum's list. In the open,
nothing is hidden, and the prepass redraws the whole visible set: the depth prepass costs 0.05–0.55 ms there, against
0.015–0.18 ms behind cover. The open 100k row is the full cost of Occlusion when it saves nothing.

Per pass in run 2: prepass list 0.024–0.138 ms, depth prepass 0.015–0.336 ms, pyramid 0.085–0.093 ms (11
single-level dispatches on 1024×512) and final cull 0.026–0.103 ms. Run 1's passes are up to 1.3 ms under heavier
contention. The pyramid's per-level dispatches are still the first candidate for M4's native recipes.

Forward saves little on cubes. Behind cover in runs 2–3, drawing 78 boxes instead of 2,142 saves 0.003–0.008 ms, and drawing
647 instead of 20,962 saves 0.107–0.140 ms. Run 1's larger drops (1.3 and 0.2 ms) are within its contention spread. On this Apple
TBDR GPU, hidden-surface removal discards hidden opaque fragments whatever the draw order, so hidden boxes cost
Frustum little fragment work. What is left to save is the vertex and binning work of 12-triangle boxes.
This is the loss E1 predicted, now measured with occlusion culling.

Field route (`cargo bench -p v1-game --bench field_culling`; paired O−F per tick, split by whether Occlusion draws
more than half of Frustum, `m3a-fieldcost1.log`, load about 290). It is one run in rotating three-way order, not
ABBA; the plan amends the two-run rule for M3a and owes a second run before M3c's default decision:
- whole GPU p50, Direct → Frustum → Occlusion: 3.837 → 3.652 → 3.928 ms;
- paired O−F behind cover: 1,788 ticks, median +0.257 ms, p95 +3.316 ms;
- in the open: 12 ticks, median +0.183 ms.
The field report folds the culling passes into the forward bucket, so its forward column for Occlusion includes
them.

**Memory**, extra over Frustum. Each render target owns:
- the prepass depth, 3.69 MB at 1280×720 (8.29 MB at 1080p);
- the pyramid, 2.80 MB (5.59 MB at 1080p, on the 1024×1024 power-of-two floor);
- a second slot list, 0.40 MB at 100,513 slots.

The host keeps two targets in flight (`IN_FLIGHT = 2`), so per `FrameHost` that is about 13.8 MB at 1280×720 and
27.8 MB at 1080p. The two history sides, 12.6 KB each, are shared.

## Motion
The full-path run saved 567 frames (640×360) per culled mode across seven stretches of the route. I reviewed them
as contact sheets: 24 across the path, 16 consecutive frames and a close pass through cubes. There are no holes,
missing cubes, shadow pops or flicker at silhouettes. All 567 Occlusion frames are byte-equal to Frustum's.

## Not covered
- Chrome does not run the native full path, the cube witnesses or prepass parity; its evidence is the image oracle
  and the qualification, which compares no images.
- Footprint rounding in x and y is not in the derived ε. The integer mip rule gives at least half a pixel of margin,
  which the worst-case bound exceeds for w below about 1–4 m at the field's far extent. Cubes cannot show a failure.
  M3c widens the rectangle or proves the margin, with the mesh tie witness.
- The rasterizer's four-ulp interpolation term is assumed; no test on cubes witnesses it (M3c).
- The parity test runs one grazing scene near the origin. It does not cover the far origin, a capture's pending view
  or the orthographic fixture's reversed indices (M3c adds the far origin).
- The resize test's scene hides nothing; a resize mid-run rests on the render extent being fixed per host.
- Device memory was not measured; the figures above are logical sizes.
- No quiet-GPU timing exists, and the field route has one run. A no-contention pair would change the magnitudes, not
  the direction.

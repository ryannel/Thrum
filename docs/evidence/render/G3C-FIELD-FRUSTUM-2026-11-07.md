# G3c M1: frustum culling in the game's dense field

7 November 2026. Apple M3 Max/Metal, Rust 1.99.1, wgpu 30.0.1. Branch `claude/g3c-gpu-culling` on main
`a346d60f`.

**Labelled contended:** other sessions shared the CPU and GPU throughout (load average 19–21), so p95 tails are
noisy.

## The field

`THRUM_SCENE=field` selects the V1 game's dense flythrough content (`examples/v1-game/src/field.rs`). It is built
only through public interfaces: the simulation seed, `SyntheticScene`, `SceneSource` and `FrameHost::new_with_setup`.

| Contents | Count | Detail |
| --- | ---: | --- |
| Ground slab | 1 | 840 m square |
| Walls | 512 | 20 × 6 × 1 m; they stand between prop rows |
| Props | 100,000 | Boxes of varied size and yaw on a 2.5 m lattice, about 790 m square; every twentieth one drifts at 0.8 m/s |
| **Total** | **100,513** | |

The four local peers control the last four props. A scripted camera flies a 250 m loop at 1–6 m altitude:
- **Snap turns:** within each 600-tick block, the camera looks backwards for 120 ticks.
- **Cut:** every 1,800 ticks it jumps a quarter of the way round the loop. Snap turns and cuts reset presentation
  history.
- **Near-plane crossings:** the path passes through walls and tall props.

## Correctness

`cargo test -p v1-game --release --test field_gpu`

- The test prepares two game hosts with the same field, one Direct and one Frustum. It drives both through the same
  ticks, waits for each frame to complete, and compares `capture_pixels` RGBA at 640×360.
- **Default release run:** 79 sampled ticks to tick 1,930 (every 30th tick, plus each event and its neighbours).
  Byte-equal. Debug builds, as run by `./dev check`, stop at tick 460 after the first snap turn (about 65 s).
- **Full path, `THRUM_FIELD_FULL_PATH=1`:** every tick from 1 to 1,930, covering six snap-turn starts and ends,
  the quarter-loop cut and the turn after it. Byte-equal at all 1,930 ticks.
- **Unit tests:** `cargo test -p v1-game --lib field` checks that the layout is deterministic, that the field spans
  far beyond the view, that walls stand halfway between prop lines, and that every turn and cut is flagged.

## Paired GPU cost on the game path

`cargo bench -p v1-game --bench field_culling`

- **Setup:** 1280×720 with Sun shadows. 1,800 ticks follow 60 warm-up ticks.
- **Pairing:** both hosts step every tick, in alternating order, and each frame completes before the next is driven.
- **Whole GPU:** all timed passes except tone. Forward includes the camera visibility pass.
- **Visibility:** on average 22.0% of instance centres are inside the frustum.

| Run | Whole GPU ms p50 (p95), Direct → Frustum | Forward ms p50 (p95), D → F | Paired F−D median |
| --- | --- | --- | ---: |
| 1 | 3.141 (3.289) → 2.932 (3.071) | 0.963 (1.099) → 0.740 (0.855) | −0.210 ms |
| 2 | 3.260 (11.072) → 2.935 (10.658) | 0.980 (3.710) → 0.738 (3.492) | −0.282 ms |

**Decision:** without `THRUM_CULLING`, the field defaults to Frustum. The fully visible grid keeps Direct, as root's
Q1 asked. The decision is provisional, because these meshes are 12-triangle boxes. M3 repeats the rows on realistic
meshes.

## Watched motion

- 567 Frustum frames: the 40 ticks either side of each event, encoded at 30 fps.
- The author reviewed contact sheets around the tick-301 snap turn and the tick-1801 cut. Walls hide most of the view
  just before the turn, and the field reappears whole after the turn and the cut. No pops at the view edges.
- An independent review of the sequence is pending.

## Not covered

- **Live window:** an attempt presented nothing, because macOS reported the window occluded (screen locked). The
  headless host above uses the same `FrameHost` path.
- **Chrome.**
- **Visible-count readback:** needs a renderer seam.
- **Uncontended numbers.**
- **Realistic meshes:** M3.
- **Occlusion:** M2.

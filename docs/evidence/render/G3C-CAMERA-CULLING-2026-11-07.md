# G3c camera frustum culling, seam 1

7 November 2026, revision `d4cb6fd` (branch `claude/g3c-gpu-culling` on `c509337`), Apple M3 Max/Metal,
Rust 1.99.1, wgpu 30.0.1. **Labelled contended:** other sessions shared the CPU and GPU during
every run (load average 20–32), so tails and single within-block deltas are noisy. Treat these as
tripwire rows, not budgets.

## Correctness

`cargo test -p thrum-render --test visibility` and `--lib visibility` pass.

- The compaction matches a CPU reference: 10k instances, four perspective views and one
  orthographic view, varied presentation alpha. It covers active prefixes of 0, 1, 255, 256,
  257, 999, 1,000 and 3; the list stays in order, hidden slots are dropped, surely-visible slots
  are kept and surely-outside slots are dropped.
- Frustum and Direct are byte-equal RGBA at 320×180 for partial, full, empty and
  near-plane-crossing views, and with Sun shadows whose casters are off-screen. Of the 10k
  instances, 1/37 are hidden, 1/11 are mirrored (two-sided pipeline) and 1/3 are moving.
  The partial and near-plane views are checked to cull more than 1,000 instances and keep
  more than 100.
- The Frustum frame has a named `camera visibility` timestamp, and `scene_pass_count` sizes
  the batch.
- An agent reviewed the partial (with shadows) and near-plane frames by eye. The grid is
  clipped at the frustum edge, and the cubes beside the camera are cut by the near plane.

## Paired GPU cost

`cargo bench -p thrum-render --bench camera_culling`

The bench renders 1280×720 with Sun shadows, in 60 ABBA blocks (A = Direct, B = Frustum) after
10 warm-up blocks. The camera looks down the grid normal. "In view" is the share of instance
centres inside the frustum. Whole GPU is all timed passes except tone encode, p50 (p95).

| Count | View | In view | Whole GPU ms, Direct → Frustum | Forward ms, D → F | Visibility ms | Within-block F−D ms |
| ---: | --- | ---: | ---: | ---: | ---: | ---: |
| 10k | full | 100% | 0.956 (1.583) → 0.967 (1.858) | 0.784 → 0.787 | 0.021 | +0.012 |
| 10k | half | 50.8% | 1.315 (1.807) → 1.339 (2.171) | 1.163 → 1.151 | 0.021 | +0.031 |
| 10k | 5% | 4.8% | 1.027 (1.592) → 1.147 (1.892) | 0.872 → 0.846 | 0.021 | +0.060 |
| 100k | full | 100% | 3.022 (4.123) → 3.177 (4.641) | 1.962 → 2.003 | 0.129 | +0.253 |
| 100k | half | 50.5% | 5.885 (11.378) → 5.863 (12.598) | 3.885 → 3.453 | 0.292 | +1.125* |
| 100k | 5% | 5.0% | 2.649 (9.169) → 2.654 (8.549) | 1.548 → 1.211 | 0.139 | −0.216 |

\* A contention spike: the p95 is 2–3× that of the earlier run, and the within-block delta
disagrees with the medians. In an earlier run on the same fixture, before slot masks, this row
measured 4.088 → 4.115 ms whole GPU, 2.967 → 2.743 ms forward and −0.032 ms within-block.

Slot masks cut the visibility pass at 100k from 0.169/0.183/0.173 ms (full, half, 5%) to
0.129–0.139 ms (half excluded). Scatter no longer re-reads 240-byte instance records.

## Encode CPU and allocations

`cargo bench -p thrum-render --bench camera_culling_encode -- --sample-count 200`

The bench uses divan's allocation profiler on `PreparedSynthetic::encode` with shadows, without
submitting.

| Mode | 10k median | 100k median | Allocations per encode | Bytes |
| --- | ---: | ---: | ---: | ---: |
| Direct | 14.08 µs | 13.75 µs | 36 | 3.84 KB |
| Frustum | 14.35 µs | 14.34 µs | 40 | 4.08 KB |

Direct now prepares no culling pipelines, buffers, params uploads or view binding, so its frame is main's. Allocations don't depend on instance count. The four extra allocations are wgpu's recording of
the compute pass. There is no steady `write_buffer`: params are written only when the active
count changes.

## Reading

- Frustum compaction pays where vertex work dominates. At 100k and 5% visibility, forward drops
  by 0.34 ms and the whole frame by about 0.2 ms.
- At 10k, forward is raster-bound and culling only adds the pass (about 0.02 ms) plus
  indirect-draw overhead.
- At full visibility the pass is pure cost: about 0.13 ms at 100k. The Direct vertex entry has no
  visibility fetch, so Direct rows are unchanged main.
- Per root's Q1, the game default stays Direct until the workload's measured visibility mix
  favours Frustum.
- Not covered:
  - uncontended numbers;
  - the visible-window frame budget;
  - Chrome/WebGPU;
  - phones;
  - cooked (G6) contributions;
  - Hi-Z occlusion.

# M2 perspective and clustered-light diagnostic — 4 November 2026

**Scope.** The serial 10,000-entity/1,500-moving V1 Metal host now renders through a finite right-handed perspective camera with reverse-Z depth, a target-sized 128-byte view-aux uniform, view-dependent BRDF rays and conservative projected/log-depth local-light lists. The fixed orthographic view remains a GPU oracle. No cascade shadow, real platform surface, cooked content or visual acceptance is claimed.

**Correctness.** Unit tests check near→1/far→0 depth, inverse point round-trip, degenerate camera rejection, projected bounds under translated/rotated cameras and a near-plane crossing, and capacity overflow prioritizing a nearby strong light over 32 distant weak lights. Opt-in Apple M3 Max/Metal tests verify the old orthographic pixels, a perspective local-light receiver against a dark baseline, unchanged background, a resized target and stale-camera target rejection. The full local `./dev check`, Wasm target compile and iOS simulator target compile pass; only Metal ran pixels. The perspective image was inspected: the 10k grid is visible and regionally lit; its tiny repeated cubes are a synthetic diagnostic, not an art-quality result.

| Optimized 600-frame process run, 640×360 | Run 1 | Run 2 |
| --- | ---: | ---: |
| Committed ticks / catch-up frames / host calls over 16.67 ms | 600 / 0 / 0 | 600 / 0 / 0 |
| Dropped presentation / cluster references | 0 / 0 | 0 / 0 |
| GPU forward p50 / p95 | 0.150 / 0.178 ms | 0.148 / 0.171 ms |
| GPU display p50 / p95 | 0.067 / 0.084 ms | 0.066 / 0.079 ms |
| Host wall p50 / p95 / max | 2.403 / 4.496 / 10.781 ms | 2.293 / 3.405 / 10.655 ms |

The 600-frame host explicitly waits for GPU completion and timestamps; these figures are process diagnostics on this Mac, not device-frame qualification or a clean incremental projection-cost comparison. The host uploaded 172.8 MB of instance payload over each run. Current cluster tables are prepared once for static lights; live light/view extraction and update cost remain open. The next M2 slice is two cascaded Sun shadow maps with near/far receivers, an off-camera caster, graph/timestamp closure, visible split/bias checks and named-target cost.

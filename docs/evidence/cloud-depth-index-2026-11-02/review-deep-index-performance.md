# Independent ABBA deep-index performance decision

2026-11-02. Read the four actual saved raw runs; no GPU/browser job or candidate edits. **Recommendation: reject the index as the selected/default optimization, retain its exact source/proof/measurements as a rejected candidate, and restore the binary product without its extra table.** Correctness evidence remains useful; these runs do not establish a repeatable whole-graph speed gain.

## Matched scope and audit

Captures under `tmp/cloud-sky-2026-11-01/captures/`, in actual ABBA order:

- A1 indexed: `cost-1793591143779-fresh-source-1x.json`.
- B1 binary: `cost-1793591313058-fresh-source-1x.json`.
- B2 binary: `cost-1793591425676-fresh-source-1x.json`.
- A2 indexed: `cost-1793591535000-fresh-source-1x.json`.

All four share frozen build `3da8416b41ff33440ccd69271ec9efc46bde19c3f5dbb75e0cc97c6b83231c12` and identical complete file maps/recipe. Relevant pins: index JS9067b62a..., WGSL a887cb73..., probe0b7483ec..., preview JS a0824ac0.... Same recorded Apple Metal3 adapter, Chrome156, visible1280x720 canvas/DPR2; same cropped128 prepared field, Sun30, local camera, fitted model2, main+mirror enabled, ordinary inspection0, no direct-reference patches, no PNG/recorder output. Initial geometry/camera/fit/source controls match; query selection is the intended difference.

Each run has330 samples with exactly the first30 warmup, leaving300 measured frames. Every retained frame has source_updated=true and light_updated=true; source ages strictly increase and requests follow90+wall request time. Camera-relative-to-source offsets and quality controls are constant. Every retained whole GPU timestamp pair is finite/positive/ordered and reproduces its recorded elapsed value. All view-cap, direct fallback, invalid-footprint and unsupported counters are0. There are no hidden reused frames or absent mirror/main paths in the retained window.

This is wall-paced mature source90–approximately100s, not exactly paired per-frame source ages: retained end ages are100.131/99.692/99.660/99.995s. Workload and source windows are closely comparable, but not an immutable one-pose kernel benchmark. The files declare quiet sequential execution; this audit cannot independently establish external thermal/scheduling state.

## Whole-graph results

Milliseconds; median is the ordinary sample median. Other percentiles use nearest rank `ceil(p*N)-1`; no tail samples removed.

| Run | GPU p50 | GPU p95 | GPU p99 | GPU max | Completion cadence p50 /p95 | Queue-complete p50 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
|A1 indexed|15.172|20.972|23.003|27.918|31.0 /39.0|25.9|
|B1 binary|16.974|22.020|27.001|28.049|30.0 /36.9|25.2|
|B2 binary|16.679|21.955|23.855|24.052|30.0 /37.0|24.7|
|A2 indexed|17.891|22.020|23.921|28.049|31.1 /37.1|25.85|

Cadence uses299 consecutive retained completion intervals per run. Measured window completion rates are32.26/33.95/34.16/32.87Hz respectively; these are observer-loop completion rates, not displayed FPS. Cadence>33.333ms counts are83/57/51/59; no retained cadence interval exceeds50ms. GPU>20ms counts are21/48/45/56. CPU encode medians are0.3/0.1/0.1/0.1ms, another reason not to explain the run pattern as one isolated GPU kernel effect.

A1's GPU gain does not reproduce: A2 is slower than both binary medians and has the same p95 as B1. Both indexed runs have worse median completion cadence, and queue medians also favor binary. Pooling600 samples per mode yields indexed/binary GPU median16.056/16.974ms and mean16.776/17.110ms, but that aggregation hides the reversal and worse queue/cadence; it is not a stable benefit claim. No5ms admission: even every observed GPU minimum exceeds5ms in this local fixture.

Component timestamps are explicitly unsuitable for attribution here: all300 retained frames in every run have no admitted additive sum. Cross-pass component intervals are not consistently nonoverlapping/contained. I use only the actual whole pair; no sum of pass times, subtraction, Sun-kernel claim or final-view attribution follows.

## Work, memory and retention consequence

Both query paths still build and allocate the index table. Identical recorded primary allocations are653,376B source3-mip field,4,042,368B deep nodes/32B metadata/index,38,912B body+bank and264,224B shared fit GPU data, plus unchanged6,491,440B encoded source and other fixtures/resources. The index alone is2,230,272B. Fresh source/Sun/fit/ambient preparation remains active in both modes; this comparison does not remove table build work from the binary baseline.

Restoring the prior binary helper can certainly remove that table allocation/build by construction. The resulting byte saving is exact; its frame-time gain is not separately measured by these runs. A correct query accelerator with extra memory/build cost should not become selected solely from A1 or pooled medians when the end-to-end reversal and cadence do not support it. No broader GPU search is required to reject this bounded candidate.

Retain the exact executed indexed code, hashes, semantic/native parity evidence and all four raw measurements. Root reports identical age90/Sun30 native PNG SHA across prior cropped, indexed and binary versions; this performance audit has not independently watched further ages/Suns. Existing125-point parity and implementation proof establish only their finite correctness scope, not all rendering quality. The old scalar lateral-quality failure and full cloud/air/environment/family budgets remain open regardless of this optimization decision.

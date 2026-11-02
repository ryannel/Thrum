# Storm billowing and finite local breakup checkpoint

2 November2026. Source-mechanism checkpoint for CF02/P1b; **the cloud is still too regular and soft to accept as a finished storm**. The [canonical queue](../../design/modules/CLOUD-MIGRATION.md) remains authoritative. This is a standalone WebGPU source proof, not a Thrum runtime module.

## Reproduce

Exact [instrument](instrument/) optical build `9d84ac657834807f` is pinned by [identity](identity.json). Copy the instrument to a fresh tmp directory, run `node --test PATH/tests/*.test.mjs`, then `python3 PATH/serve.py --port 8147`. The active development server remains8146. Choose Cumulonimbus, pause, Grow from clear, and advance180/360/480 source seconds. Thermals default on; toggling them resets the fixture for legacy A/B. Full resolution and camera framing are explicit controls.

## Selected mechanism and research

The [execution card](instrument/billowing-proof.md) derives and bounds the source. [Thermal-chain research](https://www.pnnl.gov/publications/thermal-chains-and-entrainment-cumulus-updrafts-part-ii-analysis-idealized-simulations) motivates organised heads and local dry-air loss; [resolved dry thermal work](https://cimes.princeton.edu/publications/entrainment-resolved-dry-thermals) motivates expansion. These papers do not prescribe or validate our VFX coefficients. Nubis references in the [research registry](../../design/research/ATMOSPHERICS-TECHNIQUES.md) still support rendering lineage, not a disclosed native lifecycle.

Finite base pulses now share membership with persistent rolling controls. The CPU integrates heads from committed weather; GPU advection carries real q through summed streamfunction rolls. Occupied wet cores can condense; vacuum cannot. Local finite evaporation removes thin matter before conservative liquid-to-ice transfer. Main, light, receiver and independently reflected rays retain the shared coefficient source. No aloft head geometry or disk density is injected.

Independent mechanism review identified an overlap normalization that exactly cancelled axial falloff; it was removed. Dead packet windows no longer suppress flank loss. The source still produces stacked regular heads and can become column-like under sustained overlapping feed. That is adverse visual evidence, not a completed billowing gate.

## Evidence and scope

- [x] Four Node test files pass: weather/replay, reset intent, half/air references, analytic rolls/divergence, finite neck loss, bounded2400s packing, post-emitter head survival and motion-fixture validity. `./dev check` passes.
- [x] Actual1280×720 canvas clips at1x and16x retained in [observations](observations/), with exact build, fixed-fixture revisions, controls, camera, start/end source and submitted-image progress. First clip180.25→192.25s; second201.25→393.5s. Recorded at requested15fps VP9; not displayed FPS or a performance benchmark. Primary agent inspected decoded native frames at0/5/10s normal and0/3/6/9/11s accelerated, plus live source states; no claim of human video playback.
- [x] Clear-start180s shows distinct rising lobes. Continued source reaches a fed cap;710.5s observation has liquid2446.181/ice3450.412 proxy sums, top10287.5m and zero occupied cells on all six chart faces. This bounded observation does not override the earlier longer-run chart-edge failure.
- [ ] Finish matched480s source-off120/240s, topology and phase observations.
- [ ] Final storm morphology/motion acceptance; branching irregularity and crispness remain below target.
- [ ] Production full-sky/M2, point-light error, material renewal and regional exchange. Optional20MiB float32 velocity cache remains available but current Cb/head workload equivalence and performance need their own full-sky A/B; Cu oracle is not that qualification.

## Costs and limits

Normal Cb logical resources53.3MiB before lazy observer resources; bounded thermal tables add~.205MiB across24 upload slots plus zero bindings. Direct flow includes band-local head loops in all q passes and six carrier-neighbour evaluations. No per-frame source readback or CPU voxel traversal. Observer-only q readback adds10MiB GPU memory and~16.25MiB CPU arrays; topology/overlap traversal costs tens of milliseconds and is excluded from quiet measurements. Recorded video/diagnostics have separate overhead. Whole-GPU timestamp totals remain rejected because cross-pass timestamps overlap/reorder; no Thrum5ms/16ms budget pass is claimed.

A real long-background-return failure was observed: CPU weather advances beyond retained forcing and optics faults. New optical-epoch reseeding/chart anchoring is still open; this checkpoint does not claim recovery or uninterrupted q. Same-ID live Cu→Cb regridding, complete air/environment/TAA and full cloud families remain open.

# Exact Sun-depth index experiment — rejected for runtime cost

2 November2026. The [canonical Sun profile](../../design/modules/CLOUD-SUN-PROFILE.md) and [complete cloud design](../../design/modules/CLOUDS.md) own selection. **The candidate is mathematically/functionally useful but is rejected as the selected runtime path: there is no repeatable whole-graph speedup.** Binary lookup without the2,230,272B index table remains selected. No cloud-family, point-shadow, atmosphere or5ms budget gate is promoted.

The [independent proposal review](review-deep-index-proposal.md), [actual-endpoint argument](endpoint-index-comparison-note.md) and [implementation review](review-deep-index-implementation.md) cover the representation. Original uniform512integration intervals map to retained K48segments in packedu8 entries. Checked actual brackets and at most one correction precede interpolation; an unresolved finite proxy uses the old bounded binary search, while invalid bytes/nodes fail explicitly. Clear/two-node columns bypass index construction/lookup. Local deep buffers grow1,812,096B to4,042,368B; uniform full-workload tables would add168MiB and are not selected.

The [first actual GPU readback](actual-endpoint-index-validation-note.md) and [reproducible final validator](standalone-deep-index-validation-note.md) check all504,832defined entries across986active rows;3370two-node rows are unspecified and correctly ignored. The four old buffers are byte-identical. About18million proposal CPU bracket cases and2.035million actual-table boundary cases pass after at most one correction.125actual GPU queries match forced binary transmittance terms bitwise, with no errors, corrections or binary retries in that finite GPU sample. CPU cases exercise correction; no rare-branch GPU coverage is invented. Existing lateral point-T failures remain unchanged. Ten independent validator admission tests pass.

Actual native1280×720 [indexed](images/cu-1793590814552.png) and [binary](images/cu-1793590819595.png) PNGs are byte-identical to each other and the [earlier selected frame](../cloud-sun-depth-2026-11-02/images/cu-1793588264329.png), SHA2561d1271819b0e74dc9aee036b8113cafe508f42a3dc97bbf32533f2f7fed1444f. The root viewed the actual indexed image. This is one held90s/Sun30image, not new multi-Sun/lifecycle acceptance.

## Integrated ABBA result

[Independent performance audit](review-deep-index-performance.md) verifies one frozen build, held recipe/initial geometry and controls,30warmup+300fresh-source/light frames per run, and no concurrent GPU probes/recordings. The candidate table is allocated/built in BOTH query modes, deliberately isolating query choice; the binary comparison is not the table-free selected baseline.

| Run | Lookup | GPU p50/p95/p99/max,ms | Completed cadence p50/p95/p99/max,ms |
| --- | --- | --- | --- |
|A1|indexed|15.172/20.972/23.003/27.918|31.0/39.0/44.118/46.5|
|B1|binary|16.974/22.020/27.001/28.049|30.0/36.9/39.11/42.1|
|B2|binary|16.679/21.955/23.855/24.052|30.0/36.91/39.1/40.1|
|A2|indexed|17.891/22.020/23.923/28.049|31.1/37.01/39.118/42.1|

A2 reverses A1's apparent gain, and completion cadence favors binary in both pairs. Pooling runs hides that reversal. Reject retention as a proven optimization; no broader tuning campaign is justified here. This is not evidence that indices are universally slower. All modes fail5ms and use an M3-class local fixture, not the complete engine or required target matrix. The [overlapping pass-timing investigation](review-component-attribution.md) prevents false exclusive-kernel attribution; no component sum is admitted.

[Raw receipt index](compressed-receipts.json) pins deterministic gzip and exact original JSON hashes. [Final loaded identity](loaded-profile/identity-final.json) is3da8416b41ff33440ccd69271ec9efc46bde19c3f5dbb75e0cc97c6b83231c12; exact runtime source texts are retained by their SHA prefixes. The earlier [identity](loaded-profile/identity.json) pins the pre-count2fastpath correction. [Final GPU receipt](probes/deep-probe-1793591080135.json) reproduces all five earlier buffer hashes and bitwise point parity.

The selected viewer is restored to the prior combined-cache/whole-only timing path, removing the failed component instrumentation from its default workload. That returns a known diagnostic; no new performance gain is claimed from restoration. The frozen diagnostic sources remain in the [previous evidence](../cloud-sun-depth-2026-11-02/component-profile/identity.json).

A [uniform dense T lattice](dense-light-comparison-note.md) was independently rejected before GPU implementation:64/128/256depth planes all exceed the.005depth allocation on captured curves. This avoids trading away quality on a speed hypothesis. The next required work is complete nonrecursive air/fog/ambient/surface/reflection coupling, then new source families and complete skies.

Actual restored build6914a07e3996d231f77ff92dcee0a182aaa667b5aaeee61d4f3b1a63b449092d loads and renders successfully; the [restored receipt](restored-binary-frame.json) records binary lookup,1,812,096B deep buffers and no component-timing instrumentation. Its native PNG is byte-identical to the earlier selected frame. The scene is paused90s/Sun30/main+mirror at the live proof. No new sustained performance pass is inferred from the restoration frame.

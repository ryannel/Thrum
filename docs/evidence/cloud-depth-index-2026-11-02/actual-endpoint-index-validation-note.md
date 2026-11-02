# Actual GPU endpoint-index readback gate

2 November2026. **The finite index-readback gate passes.** This is equivalent lookup of the same compressed curves; it does not repair the earlier point/lateral error or establish a speedup. Root performed the GPU capture. This commission only read actual bytes and ran CPU comparisons, without a candidate/validator change or GPU/browser work.

Input receipt `captures/deep-probe-1793590778068.json`, SHA256 `78c7a567bcd1dd96e060be5df7cc06f1b9a3b9d49ac976fef3f10f73a67c7c94`. Baseline cropped receipt `captures/deep-probe-1793588276463.json`, SHA256 `76942987599d26b2d21a8ed4fb384e0f52a2b347b0a6a4fec859a46f1546eb40`.

Retained [JSON report](actual-endpoint-index-validation.json), SHA256 `276ee2181f58d8135e0fd94324cb6da53cb4222066df8e6e1df76e53ae809642`. Exact GPU helper/shader/Scene identities are copied from the receipt into the report. Actual selected profile is `thrum.deep-sun.tau-tube.k48.64x64.512.indexed.v2`, shader `f65db3aeb1d3642349d766090ca9a92910c8b3a314846a879d2bf36405c701bc`. Scene word43=82 explicitly selects indexed lookup; the probe records indexed path exercised and binary reference forced locally.

All binary payload lengths and SHA values were checked against bytes. The four existing payloads are **byte-identical** to the baseline:

| Payload | SHA256 |
| --- | --- |
|nodes|`0320671c8cf39ba1984e6ae1377153b50e43242c4952ffc8377a116d97a7e213`|
|metadata|`b717d6b600d713fe17598291223c672fad20d5d49894847999713aed70b6a6b0`|
|field|`4af579bc83e29c9c7acc8287a6328841255fe3602ba7035a0ddee395b4ab5523`|
|raw paths|`a092da43d5cf1366966149bc922ff7e90ffcf9a7612d370fa6a149f33050c7c0`|

Actual new index payload:2,230,272bytes, SHA256 `4546fd4a9a132e88b953de158ab59e4a00d3c119cda458634c423b94c3d49d5c`, shape66×66×128little-endianu32. Four interval bytes are extracted low-byte-first. All4356metadata rows are valid;986have count>2 and define table contents. The3370two-node rows are intentionally skipped. Their physical bytes are unspecified and were not inspected or treated as an admission failure.

For every defined row, actual stored anchor depths were matched bitwise against the original513float32 depths. The common grid reconstructs bitwise with L=416.91741943359375m, h=f32(L/512)=.8142918348312378m and d_i=f32(i*h). For each k=0…511 the independent expected class is the rightmost retained original endpoint index<=k, capped to the final segment. **All504,832actual packed bytes match these expected classes**, and each class addresses an actual valid node pair. All12,627anchors belonging to defined rows match original input endpoints.

A second check evaluates the actual table with NumPy float32 divide-then-power-of-two-multiply interval estimation. The2064nonterminal query depths include original boundaries, both nextafter neighbours, midpoints and supplied probe depths. Across986rows this gives2,035,104bracket cases. Maximum initial compressed-segment distance is1. Actual-node comparisons require1121one-step adjacent corrections and then yield zero final bracket failures. This covers the same half-open convention `lo<=depth<hi`; terminal branches are excluded. These are finite CPU arithmetic checks, not a uniform WGSL division/FMA certificate. The candidate's final bracket guard/bounded retry remains necessary.

Derived build accounting agrees with receipt metadata:986rows,504,832interval visits,126,208u32 stores,10,655anchor advances. These are derived counts, explicitly **not runtime hardware counters**.

All125captured GPU queries have valid indexed and forced-binary results, and their decoded float32 term bits are identical in both channels. Reported GPU binary retries=0 and adjacent corrections=0; all error/fallback counters are zero. Therefore those125GPU probes do **not** directly exercise rare correction/retry branches. CPU nextafter cases exercise the adjacent-correction mathematics. No malformed-table injection or forced-retry GPU result is claimed.

The existing independent `validate_deep_sun.validate_capture(...)` was rerun in memory on the new receipt without editing its code. Its compression, field/quadrature and lateral summary exactly matches the baseline: selected compression maximum.004990709/.004973868, manual arithmetic~1.04e-6/.675e-6, but lateral maximum.121507/.091745 and GPU-vs-fine-point maximum.121184/.093170. Equivalent lookup preserves these known filtered/point-accuracy limits. Do not promote them as point-T quality because the accelerator is exact.

The air commission was paused after four bounded CPU mapping/planet/ratio tests passed. No LUT payload was written. No new air, preview, deep shader or source implementation was made during this actual index gate. Quiet matched main/reflection and fresh/reused cost A/B remains root-owned next evidence; this report does not infer5ms or whole-graph performance.

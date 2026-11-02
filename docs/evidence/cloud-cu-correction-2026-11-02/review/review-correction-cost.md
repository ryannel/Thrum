# Independent correction checkpoint cost/probe audit

2 November2026. CPU-only audit of retained summary, cost330-frame receipt, three fit receipts, `scripts/cloud_sky_proof/analyse_checkpoint.py` and loaded comparison/probe code. No GPU work, concurrent GPU job, source/evidence edit or threshold change by this reviewer.

**Disposition:** the retained finite timing/probe claims recompute. The identified288-byte omission is corrected in the current analyser/summary; no remaining arithmetic/identity defect was found within the audited scope. Cloud cost remains FAIL, while complete engine/residency/source-factor/appearance qualification remains unavailable.

## Timing and workload

The cost input SHA is `9a858212fa48165ab6c72b7dac4599a7598043cfd953b2354ecac4b702b56b8f`, matching summary. There are330 rows:30 warmup and300 measured. Every measured row has fresh source and light preparation, zero invalid-view-path counters, nonzero ordered raw timestamps and exact agreement between raw nanosecond subtraction and saved GPU milliseconds. Warmup's first reused frame does not contaminate the measured selection.

Independent NumPy linear percentiles reproduce:

| Boundary, ms | p50 | p95 | p99 | max |
| --- | ---: | ---: | ---: | ---: |
| GPU bracket |11.96032|15.07328|17.89198336|19.00544|
| Queue/observer completion |19.70000|28.10500|30.40000|32.10000|
| CPU encoding |.10000|.40000|.40000|.50000|
| Complete-callback cadence |24.10000|32.90000|34.50400|41.00000|

GPU median/p95 are about2.39×/3.01× the5ms cloud allocation. The standalone p99 already exceeds16ms; this does not measure a complete engine or establish complete-browser-frame admission. Encoding is not total CPU work. Callback/queue observations include timestamp mapping and16-byte error readback, and are not presented latency or shader execution time.

Loaded code starts the timestamp in the first actual source compute pass (or first light pass/reuse marker when applicable). In these300 rows source/light are fresh, so it brackets source/mips, body/bank, prefix/ambient and the actual canvas render. The end compute consumes a texel from **that rendered texture** and ends its timestamp after the dependency; this is not an unrelated end marker. Marker/resource-transition costs are included. Pre-graph queue writes, post-bracket resolve/copy/readback, browser presentation/compositor and other game work are outside or distinct from that bracket. No per-render-begin timestamps or queue waits were relabelled as GPU execution.

Measured source ages are90.6966019…98.0202026s, strictly increasing. Requests equal90+wall elapsed; f32 source-time rounding is at most3.81e−6s. Source kind prepared, grid128, fitted model2,30-degree Sun, main+mirror and source extent remain fixed. Rate is1× despite the historical filename suffix. The top-level scene is prior240s context; per-row states are authoritative, as the current README explains. The camera rig is held relative to the advected source, not a literally stationary world camera. This~8s run is not sustained thermal qualification.

## Logical allocation audit

Current known GPU subtotal **18,194,516 bytes** recomputes:

- Packed guarded/mipped field:4,596,800+627,264+92,480=5,316,544B.
- Encoded source6,491,440B;66³RGBA16 prefix2,299,968B;8³RGBA16 ambient4,096B.
- Shared fit262,144+2,048+32=264,224B; body+bank2×128²×4=131,072B.
- Scene/errors/query buffers/marker minimum484B; field/mip/prepared uniforms288B.
- One1280×720×4 canvas texture3,686,400B.

The initial ledger omitted3×64B field uniforms,2×16B mip-dimension buffers and64B prepared-source uniform. Root corrected that288B and labels the result a known subtotal. The separate transient error readback is16B. Probe buffers/readbacks and replacement old/new overlap must be accounted for in their own phases, rather than pretending this is peak residency. Query/pipeline/driver/browser backing, swapchain multiplicity and process/physical/unified-memory residency are unmeasured.

CPU encoded bytes6,491,440 +metadata73,104 +fit recovery292,941 sum6,857,485B before parsed objects/compiler/observer overhead. They are logical CPU payloads, not a physical-memory certificate. The small subtotal cannot establish the512MiB complete-browser cap.

## Finite coefficient/body claims

I independently decoded the retained sparse Q1 density and reproduced the16 canonical512-midpoint columns over local y=[−2,514]m at source time90. The maximum difference from receipt canonical integrals is2.14e−14. An independently written double bilinear lookup of the pinned little-endian table/physical axes reproduces the following errors from actual saved GPU rows:

| Sun | LUT max | Centre A vs LUT(GPU T) | T vs canonical | A vs canonical T | A-map nonlinearity |
| --- | ---: | ---: | ---: | ---: | ---: |
|30°|2.260714e−7|1.380525e−7|5.775003e−4|1.598832e−6|.02574264|
|5°|2.260714e−7|3.771092e−8|5.775003e−4|4.711286e−7|.00363970|
|75°|2.260714e−7|1.149751e−7|5.775003e−4|3.965616e−6|.02021502|

Each receipt has104 LUT cases,16 columns and14 interpolation samples; finite-row flags and generation guards are valid. The first two columns satisfy the existing3e−5 implementation tolerance. Body/canonical and A-map errors remain **unqualified sampled approximation errors**, not failed LUT arithmetic and not invented quality passes. In particular the.02574 coefficient interpolation difference is much larger than upload arithmetic error and does not establish a uniform source-factor/radiance certificate. Shadow-depth, table-fit empirical error, unsampled positions/ages and final composed radiance need their separate witnesses.

## Identity and limits

All three receipt digests match summary. Cost build`e0425a51…` matches the retained comparison identity;16 available retained proof-text files match their recorded SHA values. Probe build`e18bdc27…` is a later distinct build: its shared source/field/fit/render component hashes match, while probe/UI/host recording changed. Do not call the entire builds identical. Probe source age90 differs from the timed90.7–98s interval.

The retained source hash is`999cd894…`; all rows use the same fit/profile/axes identities. Actual cost adapter evidence records Apple/metal-3 with empty device name; it cannot independently certify a named minimum device, native backend or mobile profile. The raw records cannot prove absence of every unrelated host GPU process; this reviewer performed no GPU work, and the benchmark's no-recorder/no-PNG boundary is explicit.

No new family/full-sky/engine acceptance follows from these calculations. The corrected light is an empirically checked production approximation, not an exact arbitrary-geometry energy or physical source-factor certificate. Preserve the failed source morphology result, original Q1 comparator, finite motion witnesses and current adverse cost gate as separate evidence.

# Independent offline check of actual 30-degree deep-Sun capture

2 November2026. **The actual stored-curve compression passes the selected finite checks; the64² lateral lookup fails its.008 allocation.** This capture does not qualify the whole light product, radiance, a cloud family, SKY-G or performance.

## Exact capture and command

Input [receipt](probes/deep-probe-1793583015463.json), SHA256 `31599dad01d3a8040f082449ca7c23126e98bd69aa704f3d4dfc5b637ffb3d5b`. Output [offline report](deep-validation-30.json), SHA256 `185f70ccf79f69bbfe6c83bc5ee69d904a442eaf5331261ba7b24f632322c426`.

Command actually run from the worktree root:

```sh
PYTHONDONTWRITEBYTECODE=1 /opt/homebrew/bin/python3.14 scripts/cloud_sky_proof/validate_deep_sun.py tmp/cloud-sky-2026-11-01/captures --receipt deep-probe-1793583015463.json --output tmp/cloud-sky-2026-11-01/deep-validation-30.json
```

The observed NumPy runtime is2.4.6. Default `python3` lacks NumPy; no dependency was installed. [Validator](../../../scripts/cloud_sky_proof/validate_deep_sun.py) SHA256 `9e8a37db2701e772151dc572d3c5311bdd71a637512f54fa51a0d45382195aa0`. [Focused tests](../../../scripts/cloud_sky_proof/test_validate_deep_sun.py) SHA256 `8e76a79f90e694404779a8741b8657b15ae6aac9ee4b4d4b6964781eeab5e659`. Seven CPU-only tests pass:

```sh
PYTHONDONTWRITEBYTECODE=1 /opt/homebrew/bin/python3.14 scripts/cloud_sky_proof/test_validate_deep_sun.py
```

Tests cover channel/guard/boundary decode, zero-extension and centre sampling, exact Scene bits and legacy-W padding exclusion, full-PL nonendpoint extrema and different valid anchors, incorrect tau filtering/reusing primary effective tau, four-tap weighting/invalid tap, whole synthetic receipt/SHA tampering, and explicit calibration admission without clamping. Synthetic tests do not stand for GPU evidence.

Receipt binary hashes were checked against actual bytes before interpretation:

| Payload | Exact layout | SHA256 |
| --- | --- | --- |
| nodes |66×66×48×2f32, depth/tau|`64958761904f479e0239d2ce2e8c3152d0b5a40a9906bfa1b19da248106c2657`|
| metadata |66×66×8f32|`42be8b4c301f08daf29920048b13c44567fd24ddf014c59d51c53fdb80b2aa0d`|
| packed field |34×130×130×4f16, x fastest|`278761a8b6a9482087289b993b95d87f0a58cf714d6885eee440b5fc1c0c039a`|
| actual paths |16×513×4f32, depth/tau/sigma/valid|`c3c118b07985c98b5ac9bd96e8ba98e0f43e1d6bc5b7fd542c941eb5082b5369`|

Exact192-byte Scene hash `b831302f419426dc058bbddb6a99a1b9cd2f6034c744947244e3fb7141bfc012` agrees with the receipt. Source clock90s, prepared Q1 static comparator,128³ logical field, source/light frame pinned in receipt. This is the explicitly commissioned **sigma_max=.125 diagnostic calibration**. Its3963 physical coefficient values above.1 originally triggered the validator's generic admission. The only data-dependent correction was an explicit named prepared-Q1.125 exception, retained in a test and report; no coefficient or quality threshold was clamped/changed. The report states `production_sigma_0_1_admitted=false`.

Compiled deep shader hash `5cfb5ba76ff231e921936bff3a0a3e235b520d2bce556ac6c63866864d60a11f`, profile `thrum.deep-sun.tau-tube.k48.64x64.512.v1`. Probe helper hash `664077b1fa43243e10edff2395afb68059154c79054e3e37891bac77afc77461`; probe path/query shader hashes `9c502dfef056650b8dfbb83c316ff9f338ce29ac19ffdb773549489b953d4dcb` / `27977c407132908e12690c54baff15c06d19c33688544ab0c74c5ee4b54dad94`. Receipt/report pin the other source/fit/build identities. These are captured identity pins, not a claim to have independently rebuilt the GPU program. Root performed the GPU capture; this commission performed CPU-only reads/calculation and changed no candidate, geometry, renderer or canonical file.

## Independent calculations

The oracle used the old `reference_packing.py` mathematical contract and the author-coordinated receipt/Scene contract. It did not derive its oracle from candidate deep JS/WGSL. It independently decodes little-endian half RGBA as consecutive logical Z groups, checks every guard/unused channel and active coefficient, then reconstructs a double trilinear zero-extended field. No hardware sampler is called.

It reconstructs U/V/W and extent/grid from the exact Scene u32 bit pattern, requires an orthonormal frame with W=-Sun, derives raw support half-extents from the halo-expanded source box and extends lateral half-widths by1/(1-4/64). Legacy Scene basis `.w` is ignored. Point lookup projects the point, obtains four actual curves, interpolates ordinary tau per curve, exponentiates primary/quarter terms separately, then weights terms. An invalid tap requests actual source integration; it is never averaged as clear.

For each selected GPU path, the actual recorded513 endpoint depth/tau values define the original PL curve. Stored GPU anchors may differ from the CPU cooker's anchors. The oracle merges both knot sets and evaluates each exponential difference at every piece endpoint and every analytic interior extremum; it does not merely compare shared knots or dense query samples. Thus the selected compression maxima below cover those entire finite PL domains, subject to stated float64/libm numerical limitations. The comparison allowance is.005 plus an explicitly reported2e-6 finite f32 tolerance; this tolerance is not a uniform backend certificate.

Separate field/source comparisons use double trilinear midpoint integration. Each actual512-sample GPU sigma/tau stream is compared with the same column's independently reconstructed field samples and cumulative tau. Point requests compare direct512 quadrature with4096 midpoint integration. Lateral error compares four **fine independently integrated column terms** against fine direct integration through the exact query point, before using compressed nodes. Column quadrature, compression-plus-stream arithmetic and GPU point arithmetic are reported separately.

## Results and the missed lateral failure

All4356 columns have valid structural metadata and ordered finite monotone active knots; none is marked unavailable. Maximum stored count29 of48. Actual counters:2,230,272 coefficient requests,2,233,041 endpoint visits,7,125 anchor builds. These are exact captured work counts, not timing claims. All124 captured point rows are available. Probe fallback, invalid-footprint and unsupported counters are all zero.

| Finite comparison | Max primary error | Max quarter-depth error | Interpretation |
| --- | ---: | ---: | --- |
|16actual stored curves vs actual original PL inputs|.0047754751|.0049849550|Within.005 compression allocation in these full-domain checks|
|Actual GPU query vs CPU manual four-curve lookup|.00000163755|.000000740823|Within stated2e-6 finite arithmetic tolerance|
|CPU direct512 vs4096 at query point|.000509949|.000236795|Below.005 source quadrature allocation on these finite witnesses|
|CPU fixed512 column-prefix vs fine column integration|.000895053|.000660314|Separate column quadrature witness|
|Compressed lookup vs independent CPU512 column terms|.003561703|.004690225|Includes compression plus stream/sampler/FP32 differences|
|Fine four-column terms vs fine direct query-point terms|**.308569098**|**.120653567**|**Fails.008 lateral allocation**|
|Actual GPU terms vs fine direct query-point terms|**.308116855**|**.120520953**|**Fails proposed.02 total-query target**|

The max selected GPU stream-versus-CPU-field transformed error is about.000244646/.000137067 across the reported rows; max pointwise sigma discrepancy about.000208103m^-1. These are separate sampler/position/arithmetic observations. The result does not assign them wholly to quadrature or claim a diagnosis of hardware precision. Float16 storage error against the unsampled canonical source was not measured by this readback-only commission; its.002 allocation remains unqualified.

Worst lateral witness: world point `(177.5328826904297,153.1774139404297,230.17752075195312)m`, query kind `between-four-columns`. Four taps `(27,41),(28,41),(27,42),(28,42)` have nearly equal.25 weights. Fine column taus are approximately`.369864,1.127773,.000509,.529770`, while actual-point tau is`1.0725533`. The fine averaged primary/quarter terms are`.6507029,.8854555`; actual-point terms are`.3421338,.7648020`. GPU terms`.6502507,.8853229` closely implement the averaged-column approximation. This is a lateral reconstruction miss, not a failed depth compressor or an exp-of-mean-tau bug.

A targeted reference convergence check at that same witness used2048/4096/8192/16384 midpoint samples. Point tau converges`1.07252528,1.07255334,1.07255498,1.07255486`; the averaged-column terms remain approximately`.650703,.885456`. At16384 the lateral error is still about`.308570,.120654`. Thus this large finite miss is robust to the tested reference subdivision. The worst direct-quadrature witness's fine primary term moves only about1.84e-6 from4096 to16384 samples, far smaller than the lateral failure. These targeted convergence checks do not establish a uniform bound.

The captured frame reports source_updated=false/light_updated=true and GPU15.007744ms/queue35.1ms, but this singleton includes the root's stated graph/timing scope and is **not** a benchmark or5ms pass. No motion, native appearance, two-light, full-sky, air/fog or independent-reflection quality follows from this scalar capture.

## Required consequence

Retain the successful selected curve compression evidence and the failed lateral witness independently. Current zero-fallback validity means the producer checks schema/depth capacity but does not enforce its lateral quality allocation. The64² map may not be admitted for these point queries without a conservative footprint/error guard plus refinement/direct evaluation or another verified representation. Which replacement to build is root-owned; this validator does not increase column counts, alter geometry, relax thresholds or hide the failure behind the valid compression flag.

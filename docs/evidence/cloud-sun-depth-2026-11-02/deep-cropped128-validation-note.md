# Actual cropped128 deep-Sun comparison

2 November2026. **The crop preserves the actual decoded level-zero field and improves the pinned lateral witness; it does not meet the lateral or total-query allocation.** No candidate, source, shader, validator or tolerance was changed for this comparison. No GPU/browser action was performed by this commission.

## Inputs and reproducibility

New actual capture: `captures/deep-probe-1793588276463.json`, SHA256 `76942987599d26b2d21a8ed4fb384e0f52a2b347b0a6a4fec859a46f1546eb40`. Its four binary payloads are individually SHA-pinned in that receipt and checked by the validator. The source is the same pinned Q1.125 calibration at90s. This remains outside production sigma<=.1 admission.

Old full128 actual capture: `captures/deep-probe-1793583015463.json`, SHA256 `31599dad01d3a8040f082449ca7c23126e98bd69aa704f3d4dfc5b637ffb3d5b`. Old offline report: `deep-validation-30.json`, SHA256 `185f70ccf79f69bbfe6c83bc5ee69d904a442eaf5331261ba7b24f632322c426`.

New [offline report](deep-validation-cropped128-30.json), SHA256 `a906ac90ef32079b7e884f05d9e040668650da570747fd22991da5207da7df86`.

The **unchanged** independent validator SHA256 is `9e8a37db2701e772151dc572d3c5311bdd71a637512f54fa51a0d45382195aa0`. Exact command from worktree root:

```sh
PYTHONDONTWRITEBYTECODE=1 /opt/homebrew/bin/python3.14 scripts/cloud_sky_proof/validate_deep_sun.py tmp/cloud-sky-2026-11-01/captures --receipt deep-probe-1793588276463.json --output tmp/cloud-sky-2026-11-01/deep-validation-cropped128-30.json
```

The new receipt pins actual deep shader `28a2d350315c2b6860135e679f36669e510984e35ad0fc946bf02cf54c95c666`, exact Scene bytes `9fce19aaa3c680a1f6c26cea51125d2f9fd84227ef0218bcb176f3fc9c7a171e`, and complete helper/compiled-probe/source/fit identities. Compression tolerance remains.005, finite f32 comparison allowance2e-6, quadrature/lateral/storage allocations.005/.008/.002. The finite reference remains4096 midpoint intervals; there is no new backend or uniform certificate.

## Preserved field and actual projection

The cropped logical allocation has dimensions64×48×76, master-grid minimum32,12,28 and exclusive maximum96,60,104. Physical world minimum128,48,112m and extent256,192,304m preserve4m cell centres. Original master bounds remain0–512m. Independent decoding of both actual binary field readbacks found:

`crop == full[28:104,12:60,32:96]` coefficient-for-coefficient, maximum absolute difference0.

Both contain45,438 positive decoded level-zero coefficients. Since the equal crop contains every positive old coefficient, the discarded old area is zero. This check covers level zero only; it does not stand for the separate all-three-mip GPU comparison. The prior support note's proposal to retain a full allocation was an implementation detail; this actual equivalent aligned allocation preserves the canonical field and is compatible with the support proof.

New actual U/V projection pitches are6.508856/5.897800m, versus11.488653/13.192145m previously. These are derived from the aligned cropped allocation plus its reconstruction halo, slightly looser than the exact nonzero-support projection in the support receipt. Source coefficients have not been tuned to improve the optical result.

## Separate finite error results

Both receipts structurally admit all4356 columns, with no unavailable query in their sampled sets. Old/new maximum retained knot counts are29/31. The new column count and source-request count remain4356 and2,230,272; endpoint visits rise2,233,041→2,240,927 and anchor builds7,125→15,011 because more projected columns intersect occupied content. These are captured counts, not a timing estimate.

| Maximum finite check | Old primary /quarter | Cropped primary /quarter |
| --- | --- | --- |
|16selected actual stored curves vs original PL inputs|.004775475 /.004984955|.004990709 /.004973868|
|GPU vs manual four-curve arithmetic|1.638e-6 /.741e-6|1.043e-6 /.675e-6|
|Direct512 vs4096 at point|.000509949 /.000236795|.000058804 /.000090770|
|Column512-prefix vs fine column integration|.000895053 /.000660314|.000747045 /.000364073|
|Compressed lookup vs CPU512 column terms|.003561703 /.004690225|.004562659 /.004879723|
|Fine four-column vs fine actual-point lateral error|**.308569098 /.120653567**|**.121507428 /.091744840**|
|Actual GPU vs fine actual-point terms|**.308116855 /.120520953**|**.121184040 /.093170434**|

Selected compression passes in both cases. Direct/column quadrature finite witnesses are below.005; GPU/manual arithmetic is within the unchanged2e-6 allowance. Lateral error still greatly exceeds.008, and total actual point error exceeds.02. All fallback/invalid-footprint/unsupported counters remain zero. The validity mechanism still misses these quality failures.

These old/new maxima use different cache-dependent probe selections (124vs125rows); they are not a matched-point quality comparison. The explicit pinned material witness below is the matched comparison. Primary and quarter maxima can also occur at different points. New primary lateral maximum is at217.195648,118.461777,217.705246m; new quarter maximum is at279.519836,200.184097,238.144699m.

## Matched original worst witness

Both captures include exact float32 world point177.5328826904297,153.1774139404297,230.17752075195312m. New kind is `pinned-material-witness`.

| Matched point | Old primary /quarter | Cropped primary /quarter |
| --- | --- | --- |
|Lateral-only error|.308569098 /.120653567|.115402433 /.044679831|
|Actual GPU vs fine point error|.308116855 /.120520953|.115077609 /.044693888|
|Actual GPU terms|.650250673 /.885322928|.457210898 /.809495568|

The fine point terms remain about.3421333/.7648017, as required by preserved source coefficients. The crop changes the projected columns/taps and reduces the light leak substantially. It does not pass either error allocation.

A targeted4096/8192/16384 midpoint convergence check on the new pinned witness gives point tau1.072554883/1.072554993/1.072554875. The independent fine weighted-column terms converge around.4575354/.8094813; lateral errors remain.1154021/.0446797. This finite failure is not explained by the reference midpoint count. The comparison does not establish uniform source-quadrature, lateral or storage bounds; float16 storage vs continuous canonical source remains unmeasured here.

The captured new frame reports source_updated=false and light_updated=false, using an existing prepared generation. Its11.993088ms GPU bracket/18.5ms queue singleton cannot be compared as a rebuild benchmark with the old fresh-light capture. No motion, lighting appearance, reflection, performance or whole-profile admission follows.

Retain the equivalent crop and successful compression evidence separately from the remaining failed lateral witness. Further admission needs a measured representation/quality guard and actual refinement/direct fallback; a valid curve count alone is insufficient. This commission does not select or implement that next change.

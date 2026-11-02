# Closure: prepared crop observer and finite GPU receipts

2026-11-02. Follow-up to `review-prepared-crop.md`. No GPU job, video inspection, image comparison, candidate/canonical edit or commit by this reviewer. Read corrected code and independently audited the four actual JSON receipts below, without changing any raw record.

## Findings

**CROP-R1 closed.** Actual `prepared-field-probes.js` SHA256 is `58b8ac0edafd18bee91a9a585245327336fd5f9b36e76d2797c2f31e7eee3277`. `withCaptureScopes` now requires the necessary scope/loss/queue API, pushes validation and OOM scopes before private allocation, tracks successful pushes, drains observer queue completion, pops all successfully pushed scopes, checks device loss and rejects before returning a receipt when these checks fail. Work exceptions also throw after cleanup rather than becoming a passing receipt. Capture error details remain separate from comparison results. Nonzero logical-codeword counts are reported without inventing a positive-content requirement at genuinely empty endpoints.

I independently extracted/executed the actual scope helper with CPU-only mocks: healthy work returned a scoped receipt; injected validation, OOM, second-push, queue completion, pop, device-loss and work failures all threw; every successful push was popped. This tests JavaScript control flow, not real GPU error delivery. Actual healthy receipts below report null validation/OOM/loss/queue/push errors, no pop failures and scope pass=true. The original all-zero-on-failed-field-build false-PASS path is therefore no longer silently admitted by this helper.

**CROP-R2 closed by actual code inspection.** `fit-probes.js` SHA256 `0f87c580079fefe00455118614adba8d49fb1b256a3270b4f2fe4a6119842776` now selects `bounds=sourceObject.fieldBounds??recipe.bounds`. Canonical CPU X/Z columns and Y-halo pitch/integration therefore use the current source-local crop, while reported world positions use the matching translated state. For128, first X centre is130m and Y pitch4m; for256, first X centre129m and Y pitch2m. This corrects the former master512m/cropped-count mismatch. No corrected body-quality GPU measurement is asserted by this closure; these four receipts test fields and canonical parity, not ScatterFit body error.

## Actual finite receipts

All are in `tmp/cloud-sky-2026-11-01/captures/`, and share frozen host build `66498a81aa44a1e525a945e686ff8000e74e24f5326b5ee13399dfc944d903a4` with an identical file map.

| Receipt | Nominal side / committed age | Matching full/crop logical nonzero counts, levels0/1/2 |
| --- | --- | --- |
|prepared-field-1793586430251.json|128 /90s|45438 /6931 /1156|
|prepared-field-1793586435485.json|128 /40s|14253 /3459 /803|
|prepared-field-1793586440202.json|128 /210.3699951171875s (float32 request210.37)|3300 /832 /245|
|prepared-field-1793586510291.json|256 /90s|443432 /64602 /9195|

Every receipt is pass=true with all3 mip comparisons reporting zero logical/codeword mismatch, crop/full guard contamination, full exterior nonzero and infinity/NaN. I recursively checked all parsed numeric values for finiteness: no nonfinite values. These are nonempty field comparisons, not only equal zero allocations. At128 all generic-versus-specialized full textures additionally have identical complete readback SHA256 at every level, with zero mismatches and matching positive counts. At256 no128-only specialization is enabled; it is full-generic versus crop-generic.

For128, crop dimensions64/48/76 and source-local min128/48/112m retain world pitches4/8/16m at levels0/1/2, with offsets32/12/28,16/6/14,8/3/7. For256, crop dimensions128/88/156 and min128/56/104m retain pitches2/4/8m and offsets64/28/52,32/14/26,16/7/13. Cropped extents remain256/192/304m and256/176/312m respectively. Full extents stay512cube. These values agree with the independently reviewed dyadic world/mip construction, not recompression of the crop into the old512m extent.

All48 canonical rows per receipt pass GPU allocation bit parity and committed float32-time/validity checks. Output-buffer hashes agree across roles. Age90 includes2 positive queried positions, age40 includes1; age210.37's48 queried positions are all zero, so its arbitrary-point sample does not establish nonzero decoder behavior by itself. That receipt's fields/mips are nevertheless nonzero as tabulated. Max reported CPU/GPU canonical discrepancy is3.7106813e-9 at90,1.6942809e-10 at40, and0 at the sampled late positions. These CPU discrepancies remain reported empirical values, not a newly selected quality threshold.

## Identities and optical scope

All four raw `source_hashes` agree with the frozen host map and reviewed operators:

- prepared producer `40a62b37fe55f31086416322f9665703441c4ca97619bfff6e74cc4c043aef78`;
- comparison helper `58b8ac0edafd18bee91a9a585245327336fd5f9b36e76d2797c2f31e7eee3277`;
- ScalarField `013b09879461b36002800897a4a50d549e229fb32cffe3c7df5ef21715a60bf1`;
- source decoder `8dc9c7636ca21fe6f588b2fd373def6c891a6f186b3e008cdfec16c2b6499865`.

Canonical generated source shader is allocation-independent (`bbae1be71b9f3a3562f7989e01a7cc0e8e03808d9e37736bf626f0b8d09389bd`). Generic field shader is `eb1e0c3703b603bf28e21631f5166e53a32abd943654b883aacee0aa682ac1b7`;128-specialized shader is `adb1bd01bcb8d8823d25dafc746168b02baf55c2933967f1d24e17e8532f58a7`. These actual generated shader identities distinguish producer specialization while retaining the same arbitrary-position operator.

The original manifest/density/support receipt/tool hashes remain pinned and equal across receipts. Sigma calibration is explicitly .125 per metre, calibration_only=true and production_source_admission=false. Master geometry bounds remain512m; crop changes allocation, not the optical length scale or sigma. There is no tau-query/closure-quality test in these prepared-field receipts: they do not establish deep compression, lateral .008/total .02 query admission, direct-reference image agreement, native frame equivalence or performance. Constant sigma/length controls and shared field codewords are necessary groundwork only.

The receipt host map pins preview JS `5d8354d7a57691a81c37c02a2e7c227280fab2a4fe0d8a5cd412a1060aa82f5d` and preview WGSL `197185242415bcc30a98b3b4a4b543b6c7335ba21f2fd93c1c453138bdf51882`. Current preview JS subsequently changed for new identity/UI work; these receipts must continue to be attributed to their frozen executed build, not current disk. Historical deep-probe identity collision is a separate record limitation; no old record was rewritten and no future regenerated deep comparison is assumed here.

Both crop review defects are closed at the stated code/finite-evidence scope. Exact full/cropped field/mip and128 producer equivalence is demonstrated at these finite ages; all-time crop-support safety continues to depend on the independent pinned complete-knot support proof. Ray midpoint phase/cache-domain/native appearance and complete family/engine/runtime gates remain separate.

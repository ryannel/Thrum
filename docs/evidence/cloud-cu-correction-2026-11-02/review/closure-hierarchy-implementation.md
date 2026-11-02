# Hierarchy implementation — scoped finding closure

2 November2026. Independently inspected the actual source/adapter/resource corrections and reran focused CPU tests.
No GPU, implementation/canonical edit or commit. This closes numerical/lifecycle findings while the separate native
source appearance verdict remains **failed**, with the selected prepared-master fallback unchanged.

## HIER-R1 — closed

CuSourceField.create now owns partial creation in try/catch and destroys initialized members before rethrowing.
ScalarField.create independently tracks textures/uniforms immediately as they allocate, so a failure before that
nested async factory returns is also cleaned. ScalarField.destroy clears its registry, avoiding double retirement.
This resolves the original unreachable partial candidate as well as the nested factory case identified in follow-up.

Reran test_resource_lifecycle.mjs: all **five** rejected partial creations release every allocated object exactly
once. The additional late-source-shader case first permits the nested field to succeed, then rejects the source
compile after its own buffers exist. Thus it directly tests the original outer failure, rather than testing only
the nested field twice. No actual device-loss/GPU failure was injected or claimed.

## HIER-R2 — closed

validateHierarchyFrame now checks actual typed frame/schema, supported grid, brick dimensions/stride, sufficient
metadata layout, finite optical parameters, expected source/grid/bounds/time/level and logical pool limits. It scans
contiguous prefix/count records, local caps, strictly ordered unique IDs below logical live, full-witness membership
and actual packed node/erosion-gradient admission. Unused physical pool tails remain permitted; stale tail IDs cannot
become logical contributors. Essential shape/schema/length/cap checks are mirrored in WGSL without a full list scan
per source point.

The builder performs structural admission. The actual CuSourceField.prepare upload additionally validates against
independent recipe/time/grid/bounds/maxLevel and recipe-derived [time,sigma,w,base] parameters before writes. Cached
point evaluation explicitly requires immutable admitted array contents; cached scalar/view keys are not advertised
as detecting arbitrary in-place index mutation. Explicit validation rescans, and changed headers reject on the cheap
path. This is an appropriate bounded contract for the current serialized proof caller.

Reran test_hierarchy.mjs successfully:13496source/index/brute comparisons,2556full-brick checks and5856birth checks,
plus malformed grid/stride/length/prefix/count/order/witness, stale logical-ID in padded pools, source-profile,
packed-gradient, expected-generation and cached-header negatives. The exact former bx=15 counterexample now rejects;
the valid query retains0.002618281430486036.

## Inspected identities and remaining limits

- convective-hierarchy.js: d3d124cb3cfd42ed93405826f1abded8042c696bb4fd276340f140d4f3d39e3c;
- test_hierarchy.mjs: d9d4924fd86624b95c18128741f63d43c4ae63a05bff1872a704d05e36f16a39;
- cu-source.js: 8e6bdddd6c4ad1a09acfd6de32cf85dcafe31e64d8329280b6b50fe1478f2722;
- scalar-field.js: 16d8430d170213ec50f0dae56b44a8bc19d2566b14d1ca759f6a1cc8959170c8;
- test_resource_lifecycle.mjs: 91ab81d7f1a15b1b842058af73daf975463c61dbe77ffc27dda6acbf079300b9.

No carrying blocker remains for these two findings. WGSL guard changes still require their actual GPU compatibility
run; CPU/string checks do not claim one. The .01m numeric guard's complete finite-profile qualification, motion,
independent optical consumers and runtime cost remain separate. The passing numerical slice does not override
review-hierarchy-appearance.md: the actual geometry still reads as smooth bodies/buttons/mechanical joins and has
not been accepted as CF01 or as a general cloud source.

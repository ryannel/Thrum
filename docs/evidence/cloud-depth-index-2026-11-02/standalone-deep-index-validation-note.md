# Standalone actual-index validator handoff

2 November2026. The independent one-off index-readback checks are now reproducible in [validate_deep_index.py](../../../scripts/cloud_sky_proof/validate_deep_index.py), SHA256 `2897c7c00e67507d0f179478f206f8564f0d703ca1abbfa65f889550fb68f14a`. No candidate imports, shader edits, source cook, browser or GPU calls. Code was written during the quiet A/B; execution began only after root explicitly reported timing complete.

[Ten focused synthetic tests](../../../scripts/cloud_sky_proof/test_validate_deep_index.py), SHA256 `f36712c9c2aa0103f96126421bcb0fefb004f2c431ae34bf96d7b77cb3c93ab1`, pass. They cover malformed SHA/shape, a rehashed wrong defined class, a rehashed out-of-range segment byte, unspecified-row acceptance, index-selector absence, false reported GPU parity vs decoded bits, legitimate recorded nonparity, invalid lookup diagnostics and malformed admitted node count. The small fixture deliberately puts255 in every unspecified row: those bytes must remain unread. These tests establish admission behavior, not GPU quality.

Commands actually run after quiet timing completed:

```sh
PYTHONDONTWRITEBYTECODE=1 /opt/homebrew/bin/python3.14 scripts/cloud_sky_proof/test_validate_deep_index.py
PYTHONDONTWRITEBYTECODE=1 /opt/homebrew/bin/python3.14 scripts/cloud_sky_proof/validate_deep_index.py tmp/cloud-sky-2026-11-01/captures --receipt deep-probe-1793591080135.json --baseline deep-probe-1793588276463.json --output tmp/cloud-sky-2026-11-01/deep-index-validation-final.json
PYTHONDONTWRITEBYTECODE=1 /opt/homebrew/bin/python3.14 scripts/cloud_sky_proof/validate_deep_index.py tmp/cloud-sky-2026-11-01/captures --receipt deep-probe-1793591080135.json --baseline deep-probe-1793590778068.json --output tmp/cloud-sky-2026-11-01/deep-index-validation-vs-first-indexed.json
```

Final capture1793591080135 SHA256 `698ec2e8d84e170bcf756dae674096504e4d95ec284ed1fe428f82c0b58912a9`, shader `a887cb736999c87a8cdcf1df291bdcdc9363d9658b5ad3fe82c582a469da5f81`. [Final vs cropped-baseline report](deep-index-validation-final.json) SHA256 `7959a707b1667a3cf0c9e7854c877452dc5d80ffee45aae005ffcea34586d591`; [final vs first-indexed report](deep-index-validation-vs-first-indexed.json) SHA256 `e602041ff0ae1eb78c0af1a60f2a3758e695bb0d261956a3ae20b9c6431e8376`.

Both finite gates pass. Nodes/metadata/field/paths are byte-identical to cropped baseline1793588276463. All **five** payloads, including index, are byte-identical to indexed capture1793590778068. Index SHA is `4546fd4a9a132e88b953de158ab59e4a00d3c119cda458634c423b94c3d49d5c`.

The CLI checks receipt schema, exact descriptor dimensions/formats, every required binary's actual SHA/length and exact Scene hash/grid. All4356metadata rows are inspected. All admitted active nodes are finite/monotone with exact original endpoints. The actual513depth grid reconstructs bitwise as f32(i*f32(L/512)). Only986valid count>2rows define index bytes;3370two-node rows and any invalid rows remain unread. All504,832defined packed entries match the independently derived original-input-interval segment classes, including logical segment-range checks.

There are2,035,104actual-table/float32 boundary cases, including nextafter neighbours, with maximum proxy distance1,1121CPU adjacent corrections and zero final bracket failures. All125captured GPU query pairs are valid and bitwise equal to forced binary lookup, with the actual indexed selector enabled. GPU correction/retry/error counters are zero. These125points therefore do not exercise the GPU's rare correction/retry branches; the nextafter correction result is CPU-model evidence. Metadata-derived work agrees with the receipt and is labelled derived, not a runtime hardware counter.

Exit0 means the finite index gate passed. Exit1 means the capture is structurally admitted but indexed parity/validity evidence is unavailable or fails; exit2 identifies malformed receipt/data. With an indexed baseline, all five buffers are compared; with the earlier baseline, its four existing payloads are compared. No quality threshold or source calibration was changed. This remains exact lookup-equivalence instrumentation; prior lateral/point-T error and performance/native/motion evidence are separate.

Air was left at its already completed CPU note/reference/tests. No further air cook or LUT payload was produced during these checks. Existing helpers, preview and prior review notes were left unchanged.

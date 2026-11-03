# Registry structure before complete-candidate identity checking

3 November 2026. This is the first code checkpoint of the [operation-wide render closure sequence](../../design/research/F3-RENDER-GLOBAL-CLOSURE-SEQUENCE-2026-11-03.md). It changes complete-candidate validation order, not source admission or the public registry-only API.

`prepare_staged_registry` now checks project, schema descriptors, carried history and registry consistency without an early UUID collision pass. `validate_staged_candidate` uses that preparation, then validates declared records. `validate_staged_registry` retains its exported behavior by running the original registry-ID collision check after preparation. A complete candidate consequently does not allocate the registry-only collision set before its later combined ID pass.

The focused fixture duplicates project and history IDs while declaring a malformed scene, for both scene@1 and scene@2. Complete-candidate validation reports the scene structure error; registry-only validation reports the duplicate. Once the scene is structurally valid, the complete candidate reports the duplicate through the existing record pass. A descriptor-order fixture checks that registry consistency still precedes record parsing.

`RUSTC_WRAPPER= cargo test --locked --offline -q -p thrum-source registry_stage` passed: 16 passed, 11 ignored, none failed. `RUSTC_WRAPPER= ./dev check` passed. Independent read-only review found no issue in the extraction. No paired timing was taken; avoiding a redundant set is a representation observation, not a measured speedup.

This checkpoint does not complete the required-reference capacity barrier, operation-wide canonical UUID collision order, typed edge resolution, deferred bounds, scene@3 admission, F4 selected-pin projection or combined resource and latency qualification. The existing edge capacity can still interrupt record parsing.

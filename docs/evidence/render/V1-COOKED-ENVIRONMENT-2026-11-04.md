# V1 accepted constant-light A/B diagnostic

4 November 2026. Source code: local commit `fd9505a7319a0a613eabd0a5a9276cc6c753cf86` on `codex/v1-cooked-environment`, Rust 1.99.1, locked dependencies. Workload: the existing 10,000-entity, 1,500-moving V1 scene, 640×360 offscreen on Apple M3 Max/Metal. This is a native diagnostic, not a live edit watcher or platform frame-budget result.

| Check | Result |
| --- | --- |
| Accepted source → cook | Two real F3 roots select one scene@2, authored environment@1 and light identity@1. Changed Sun colour changes F4 revision; an unrelated admitted file preserves product revision and artifact bytes. Missing scene environment refuses. |
| Artifact | Checked content hash and product key round-trip; changed bytes and a rehashed NaN numeric payload refuse. The F3 tolerance-boundary colour also remains valid through product and renderer checks. |
| Metal A/B/C | A and B produce different pixels at the same committed tick after prepared publication. Tick advances once on each subsequent drive; B's completed frame carries B's exact revision. C sets Sun lux to zero, removes both shadow passes and yields a clean timestamp batch where supported. |
| Boundaries | Host-owned prepared candidates cannot publish on another host. Publication retries while old packets are live; it refreshes all instance rows before swapping targets and retains the old view if preparation fails before publication. The browser library compiles without native source/cook dependencies. |

`cargo test -p v1-scene --test cooked_environment -- --ignored --nocapture` passed on Metal. Focused cook/render tests, strict Clippy and `./dev check` passed. `cargo check -p v1-scene --lib --target wasm32-unknown-unknown --locked --offline` passed with the pinned compiler. A final read-only review reported no findings after corrections to the Wasm dependency, F3 numeric tolerance, foreign-host guard and zero-Sun timestamp path.

The existing optimized Divan assembled-frame bench was rerun with 100 samples per case and GPU completion inside each sample. It still uses the *diagnostic* light, so it tests the shared shader/ABI path but not cooked photometric steady cost. Numbers count Rust allocations on the benchmark thread only.

| Synthetic frame | Median wall | Median allocator calls | Median allocated payload |
| --- | ---: | ---: | ---: |
| Four passes with timestamps | 1.229 ms | 141 | 26.37 KB |
| Four passes plus synthetic surface pass | 1.271 ms | 155 | 27.47 KB |
| Four passes without timestamps | 1.030 ms | 80 | 14.39 KB |

The previous surface checkpoint recorded 1.284/1.272 ms for the first two rows with the same allocation counts. These runs have 10–12 ms slow samples and do not isolate a shader speed difference or qualify pacing. The cooked A/B/C preparation and publication were outside this bench.

The A/B test did not force a submitted A packet to remain held during B preparation, inject a GPU preparation fault, or inspect shadow movement after changing Sun direction. The existing synthetic frame benchmark does not measure cold cook/prepare, replacement peak or the photometric steady scene. Browser and iPhone runtime, visible window, active file watching, automatic background scheduling and sustained device costs remain open. The design and next gates are in [the decision](../../design/decisions/V1-COOKED-ENVIRONMENT.md).

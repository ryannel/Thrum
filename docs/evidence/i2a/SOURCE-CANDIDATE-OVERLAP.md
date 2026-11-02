# All-type source candidate overlap cost

2 November 2026. **Local provisional-source performance and lineage proof.** A prior validated candidate remains live while the same 4,354-file tree is restaged, a replacement candidate is validated, and both source-carried histories are compared. The replacement has all 256 game types, two records per type, 3,584 scenes, 100,000 history fields, 1,024 type versions and 65,536 required scene references. Exact source revisions match; the history comparison accepts. The test does not authenticate an admitted prior root or exercise SQLite, editor application, cook or a device.

Run with Rust 1.99.1 release tests on macOS 26.6.2 ARM64. Source file SHA-256: `faed530ccc74d26e0ffe41129040184f4943bf79393e9673f108fab0d96357ea`; release test binary SHA-256: `e71b1f3e3ed04128950c2c7c61d286668d632e91308025198f17ecd33a5af405`. Reproduce with `RUSTC_WRAPPER= cargo test -p thrum-source --release measure_dense_all_type_candidate_overlap -- --ignored --nocapture`. Each row below is a fresh test-binary process; peak resident bytes come from macOS `wait4` and include fixture creation, both stages and both candidates. Timers exclude the initial fixture and first validation; `restage` copies and rechecks the unchanged declared tree, while `validate + lineage` includes the second candidate validation, history comparison and manifest revision.

| Run | Restage | Validate + lineage | Whole-process peak RSS |
| --- | ---: | ---: | ---: |
| 1 | 868.8 ms | 242.4 ms | 100,401,152 B |
| 2 | 888.3 ms | 239.5 ms | 99,614,720 B |
| 3 | 863.3 ms | 242.5 ms | 98,140,160 B |

Each stage has 4,354 files and 21,820,298 copied bytes. Compared with the [single-candidate process peaks](SOURCE-ALL-TYPE-EDGE-COST.md) of about 45.6–47.1 MB, retaining two full checked candidates substantially increases RSS. This is a deliberately pessimistic editor overlap: a production admission can retain only the authenticated predecessor history/root rather than the entire old validated record graph. The unchanged full restage costs roughly 0.86–0.89 s on this local filesystem, exceeding the second semantic check. A future optimized path can reuse verified immutable blobs, but it must still check complete membership and mutable-file drift before selecting a new source revision; a hash cache alone is not provenance.

These timings are neither an edit-to-ready latency nor a peak allocation bound. The next complete path needs configurable SQLite main/WAL/free-space planning with actual-fault recovery, collector-enforced pins and a production source API before F4 can cook the admitted bytes. Measure the live store/editor overlap and small-edit path there; do not infer them from this stress fixture.

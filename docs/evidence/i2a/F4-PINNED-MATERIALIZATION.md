# Pinned source as temporary native build inputs

2 November 2026. **Implemented first-profile temporary materialization; no
compiler or ready build is claimed.**

`thrum-cook::materialize_pinned_source` takes only an accepted source read pin
and a new absolute destination under a trusted private parent. Relative paths
refuse so later CWD changes cannot redirect cleanup. It
streams every authenticated manifest file through the production source-store
verifier into that new directory. It owns the directory until explicit close
or drop, and removes partial output on an ordinary copy failure. If cleanup
fails, the error carries the surviving path. The API does not read working
files, reuse an existing directory, or publish these bytes as a product.
The caller supplies a scratch plan. Preflight rounds file lengths to the
observed allocation granularity, includes per-file and distinct-directory
metadata estimates using sorted adjacent paths, and checks the per-tree target and observed volume free
headroom. It does not reserve blocks or account for other concurrent writers;
actual write errors remain recoverable failures.

A production-store test admits a three-file tree, damages the checkout scene,
materializes the old accepted pin and compares the scene output with the
original admitted bytes. It checks manifest count and copied-byte sum, absence
of generated `.engine` files, explicit and drop cleanup, and refusal to overwrite
an existing destination and sentinel. A relative destination and deliberately tiny scratch target
refuses before creating a directory. The lower-level source-store tests
separately cover corrupted pinned chunks and output-writer failures; this
materialization test does not inject a partial-copy fault.

One local macOS ARM64 release probe held one pin and warmed 20 copies, then
timed 200 new-directory materializations per run, excluding subsequent cleanup.
Three runs of the small three-file fixture gave p50 **338–347 µs**, p95
**404–421 µs**, p99 **436–480 µs** and maxima **456–515 µs**. Each run wrote
ordinary files, created their directories and verified SQLite source reads.
These samples exclude source staging/admission, pin acquisition, cleanup,
compiler startup/work, product publication, process RSS and target-device
variation. Reproduce with `RUSTC_WRAPPER= CARGO_TARGET_DIR=<owned-target>
rustup run 1.99.1 cargo test -p thrum-cook --release
measure_small_pinned_source_materialization -- --ignored --nocapture`.

The entire admitted tree is copied in this first profile. A future T2 builder
must select and verify its exact code/toolchain/dependency inputs, keep compiler
writes away from the copied source, retain and verify executable/product bytes,
and publish a trusted target manifest before a `BuildRevision` or run can be
called ready. Large trees, concurrent same-user interference with a nonprivate
destination, physical write faults and end-to-end edit-to-ready latency remain
unqualified. The parent [build-plan task](../../BUILD-PLAN.md) stays open.

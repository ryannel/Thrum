# First headless scene cook from accepted source

2 November 2026. **Implemented narrow F4 content product, not a ready game build.**

`thrum-cook::cook_entry_scene` receives only a `thrum-source::PinnedSource`. It reads
`project.json` and the selected entry scene through verified source-store reads,
decodes the fixed first-profile records, and returns an immutable caption/scene product
with its accepted-root provenance. A domain-separated product digest includes the
headless target, cooker format and the two direct input hashes. This keeps
`SourceRevision`, `SceneProductRevision` and a future `BuildRevision` distinct.

The integration test stages and admits A, changes the mutable scene file to invalid
bytes, then cooks A from its pin and obtains its original caption. It admits B with a
new caption and observes a new product identity. It admits C with an unrelated file
and obtains the same product identity with a different source root. It admits a valid
two-entry D, which this one-entry cooker refuses; the held A product is still usable.
Finally it pins historical A and reproduces A's product identity. The test runs
against the production native SQLite source store, not a provisional stage mock.

The record buffers are bounded by the existing 16 MiB source-record limit. The
product holds a caption, root, target and digest; no generic JSON tree or second
whole-source copy is retained. The entry lookup currently scans the authenticated
manifest once per cook.

One local macOS ARM64 release probe held the accepted source pin, warmed 100 cooks,
then timed 1,000 successive complete small-scene cooks per run, including both
verified SQLite file copies and fixed-record parsing. Across three separate runs,
p50 was 15 µs, p95 16 µs, p99 19 µs and maximum 43–52 µs. These are observations under local scheduling,
not a target budget or improvement claim. The probe does not include stage,
admission, pin acquisition, binary build, publication, run startup or RSS.
Reproduce with `RUSTC_WRAPPER= CARGO_TARGET_DIR=<owned-target> rustup run 1.99.1
cargo test -p thrum-cook --release measure_pinned_small_scene_cook -- --ignored
--nocapture` from the repository worktree.

No durable product publication, load-time product hash verification,
game-code/toolchain binding, target-ready `BuildRevision`, fresh run, private save,
multiplayer match or end-to-end edit latency is claimed. The parent [build-plan
task](../../BUILD-PLAN.md) remains open.

# First native compile from an admitted headless workspace

2 November 2026. **Opt-in native path proof, not a target-ready build.**

The bundled `i1-mobile-game` now has a CPU-only `headless` executable. It calls
the same `game::Rules`, `HeadlessHost`, authority, replica and receipt cursor as
the phone example. The entry queues one tap, advances one real host callback,
requires one committed replica counter and one result/event, and exits with a
stable success line. The ordinary library keeps `game` private; this entry does
not create a separate gameplay implementation.

The ignored `thrum-cook` qualification copies the current local workspace's
root Cargo manifest/lockfile/toolchain declaration and every workspace
package's manifest and `src` tree into a candidate fixture. That copied tree
is the input to explicit source admission; the compiler never receives a
checkout path. A source A pin cooks the scene, checks `build/headless.json`, and
materializes the accepted files. The test changes the working rule file after
admission, then uses locked offline Cargo with a separate target directory to
build A. It compares every materialized path and SHA-256 before and after
Cargo, hashes the exact binary, and executes it. A later source B with invalid
Rust is admitted independently. Its compile fails without a B executable;
the A binary hash and execution remain valid while B stays the selected source.

Command, from the repository root with the pinned toolchain and local registry
cache available:

```sh
RUSTC_WRAPPER= CARGO_TARGET_DIR=/Users/ryannel/Workspace/thrum/target \
  rustup run 1.99.1 cargo test --locked --offline -p thrum-cook \
  admitted_workspace_builds_headless_game_and_failed_edit_keeps_prior_binary \
  -- --ignored --nocapture
```

Local macOS ARM64 run: 53 admitted files, 1,204,606 copied bytes; the separate
debug Cargo build took 1,163 ms and produced a 2,306,184-byte binary. These
are one warm-registry, cold-target observations, not a throughput or
cross-machine claim. Both A execution and B failure/retention assertions
passed. `./dev check` also passed; its normal suite intentionally skips this
native compile qualification.

`--offline` stops Cargo registry/network resolution, but it is not an OS
network sandbox for arbitrary build scripts or proc macros. The test has not
closed the selected dependency graph against every admitted file and verified
external dependency root, recorded compiler/target/flags identities, or
published a durable binary/product manifest. A successful process exit and
binary hash therefore do not form a trusted `BuildRevision`. The next T2 step
is the isolated, attested compiler adapter; F4 then publishes and reloads the
verified target closure before a fresh world may start.

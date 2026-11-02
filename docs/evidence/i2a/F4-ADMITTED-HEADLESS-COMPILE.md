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

Local macOS ARM64 plain run: 53 admitted files, 1,204,606 copied bytes; the
separate debug Cargo build took 1,163 ms and produced a 2,306,184-byte binary.
These are one warm-registry, cold-target observations, not a throughput or
cross-machine claim. Both A execution and B failure/retention assertions
passed. `./dev check` also passed; its normal suite intentionally skips this
native compile qualification.

The complete local A/B qualification, including a new private Cargo home
prepared from A's materialized pin, uses:

```sh
THRUM_PYTHON=python3.11 THRUM_TEST_NATIVE_SANDBOX=1 \
  THRUM_TEST_PREPARE_CARGO_HOME=1 RUSTC_WRAPPER= \
  CARGO_TARGET_DIR=/Users/ryannel/Workspace/thrum/target \
  rustup run 1.99.1 cargo test --locked --offline -p thrum-cook \
  admitted_workspace_builds_headless_game_and_failed_edit_keeps_prior_binary \
  -- --ignored --nocapture
```

It needs a populated local Cargo registry and permission to apply the macOS
sandbox. The private home is owned beside the test project and removed after
the test.

With `THRUM_TEST_NATIVE_SANDBOX=1`, the same ignored test wraps each Cargo
build in a local macOS `sandbox-exec` profile. The profile defaults to deny,
explicitly denies network, permits writes only in the separate output tree,
and denies reads of the mutable checkout. It permits broad reads elsewhere so
Cargo can use the installed Rust toolchain and registry cache. Before each
compile, the test proves a checkout `Cargo.toml` read and an input-tree write
are refused. The local sandboxed A/B run passed: 53 files, 1,207,105 bytes,
1,056 ms for A's separate debug build and the same 2,306,184-byte binary
size. These are process-level observations for one host and fixture.

The later sandbox profile grants read data for the materialized input,
configured Cargo home, installed toolchain and named macOS system roots;
metadata reads are broader. The test clears the child environment, selects
the exact installed Cargo/Rustc path, supplies only the Cargo home, target,
PATH, temp directory and fixed flags, and uses a distinct output tree. With
`THRUM_TEST_CARGO_HOME` pointing at the private cache prepared in the
[cache experiment](F4-PRIVATE-CARGO-CACHE.md), the local A/B run passed: 53
admitted files, 1,213,846 bytes, 1,455 ms for A's debug build and a
2,345,736-byte executable. A checkout source read, shared Cargo config read
and materialized input write were refused. Exact source and private-cache
path/hash inventories were unchanged after both builds, while admitted B's
invalid Rust failed and A's binary still verified and ran. That earlier
manually prepared private cache had 318 files, 28,523,480 bytes and local
canonical path/length/file-hash tree digest
`1f5efe1d26eaf48c6e07c83e4b3745497452242daff8c7eeed83909842c8238d`.
This is one host observation, not a build-time target or a general cache size.

The structured Cargo transcript from the fresh A output tree reported five
compiler artifacts: `hashbrown 0.17.1`, `thrum-core`, `thrum-mobile-shell`,
and the bundled game's library and headless binary. The test checks the
reported source/manifest paths against admitted input or private Cargo home,
artifact paths against the fresh output tree, absence of build-script and
proc-macro events, and a sorted Cargo-ID or admitted-relative-path/version,
target, kind and feature digest. The observed unit digest is
`47042cc82aa994ad4383eba23b10891b66c296a9b1a87acd2a12c0bf5834b4c1`. This is
an observed selected graph, not a complete file-read trace or compiler
invocation attestation. The latest run used 53 admitted files (1,221,172 B),
1,357 ms for A's debug build and a 2,345,736 B executable; these are one
warm-cache local observation, not a performance guarantee.

The sandboxed run improves the process boundary; it does not make a trusted
builder. The prepared cache's index records were copied from mutable host
state and its tree digest is diagnostic rather than a release-verified root.
Broad system reads remain; general selected-unit and build-script/proc-macro
inputs, exact compiler/dependency identities and the output manifest are not
closed. A successful process exit and binary hash therefore do not form a
trusted `BuildRevision`. The next T2 step is a closed, attested compiler
adapter; F4 then publishes and reloads the verified target closure before a
fresh world may start.

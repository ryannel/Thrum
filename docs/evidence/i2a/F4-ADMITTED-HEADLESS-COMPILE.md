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
invocation attestation. The latest run used 53 admitted files (1,225,519 B),
1,103 ms for A's debug build and a 2,345,736 B executable; these are one
warm-cache local observation, not a performance guarantee.

The subsequent opt-in A build audits all five rustc `.d` files in the fresh
`debug/deps` output. It bounds each file to 1 MiB, parses the first dependency
rule including escaped spaces, requires its target to be the `.d` file, and
requires every reported source to be a regular file under the admitted input
or private Cargo root after canonicalization. The local run passed with 53
admitted files (1,232,354 B) and a 1,745 ms fresh-output build. The normalized
diagnostic digest was
`5d5280794e73179b41cac152647f99ed928af85894311aca88f2f74c09a60be8`.
An independent small macOS fixture supplies five dep-info rules, then proves
that an outside-root source, malformed first rule and missing unit each refuse.
The [Cargo dep-info reference](https://doc.rust-lang.org/cargo/reference/build-cache.html#dep-info-files)
describes these as source dependencies for rebuilds. This audit is after
execution and covers reported source paths only; it does not attest every
compiler file read, binary dependency, environment input or the trustworthiness
of the compiler that wrote the `.d` files.

The sandboxed run improves the process boundary; it does not make a trusted
builder. The prepared cache's index records were copied from mutable host
state and its tree digest is diagnostic rather than a release-verified root.
The local macOS profile was then narrowed further: data reads no longer grant
all of `/private/var`, `/var`, `/opt` or `/Applications`; it grants the selected
developer tree (here `/Applications/Xcode.app/Contents/Developer`), named
system roots, admitted input, private cache, toolchain and output. A denial
probe against the test project's
`Cargo.toml` outside the materialized input passed. With the default `cc`
linker, two otherwise passing A builds took 7,406 and 7,436 ms. Local macOS
sandbox logs showed `xcrun` and `xcodebuild` repeatedly trying to read denied
temporary cache files. Setting Cargo's host-target linker to Xcode Clang and
`SDKROOT` to the selected MacOSX SDK avoided those cache lookups; two narrow
sandbox A builds took 1,039 and 1,031 ms. Resolving the selected developer
root with `xcode-select` or `DEVELOPER_DIR` and its linker/SDK with `xcrun`
under explicit `--sdk macosx` before sandbox entry produced a 1,026 ms A build.
The Cargo linker variable is keyed from the selected Rustc host triple rather
than the test process architecture. These are local cold-output-tree
observations, not a latency guarantee. The direct linker and SDK are selected
by path but their complete bytes and loaded dependency closure are not yet
authenticated.

The read grant was narrowed once more to the resolved Clang toolchain root
and MacOSX SDK root, retaining the selected Rust toolchain and named system
roots. Two passing A builds took 1,572 and 1,103 ms; the latter is near the
direct-linker run with the broader developer grant. A separate sorted
path/type/length/SHA-256 inventory experiment measured 394 files, 1.03 GB
logical bytes and 0.59 seconds for the installed Rust toolchain; 5,730 files,
2.88 GB and 5.95 seconds for the selected Xcode toolchain; and 33,507 files,
0.95 GB and 5.66 seconds for the SDK. Symlink targets and directory paths
were included in those candidate digests. The whole developer tree would
cover roughly 117,000 files and took 26.5 seconds to inventory. These are
one-host cold observations, not verified release roots or edit-time costs.

The local macOS data-read profile now names `/System/Library`,
`/System/Cryptexes`, `/usr/lib`, `/usr/bin`, `/Library/Preferences`,
`/private/etc` and `/dev` instead of granting all of `/System`, `/usr` and
`/Library`. The opt-in admitted A/B test passed with this profile on 2 November:
private cache preparation took 1,308 ms, pre-A reuse verification 159 ms,
and A's fresh-output build 1,095 ms for 53 admitted files (1,228,273 bytes).
The same sandbox command successfully read `/private/etc/hosts`, checking
that the denial probes were not merely failing to launch. Denial probes then
used existing files at `/usr/share/misc/ascii` and
`/System/Applications/Calculator.app/Contents/Info.plist` and confirmed the
sandbox refused those reads. The test requires at least one
such existing system-file probe on the host. This is a narrower local data-read
boundary; global metadata reads and the named system roots remain broad, and
there is no complete compiler file-read trace or release root authentication.

An exploratory removal of the global metadata grant did **not** pass the
admitted A/B build and was reverted. Cargo first refused creation under the
temporary work root. Canonicalizing Cargo's output path and granting metadata
for the observed temporary and Rust toolchain ancestors let compilation reach
the native link, but Clang then reported no developer tools. The local macOS
sandbox log showed denied metadata probes for `/Applications` and for
`libxcrun.dylib` under the selected Xcode developer tree and Command Line
Tools tree. The linker diagnostic said `xcode-select: ... No developer tools
were found, requesting install` and `clang: error: linker command failed`.
These observations point to an Xcode support/discovery dependency outside the
selected Clang toolchain and SDK roots. They do not prove that both probed
`libxcrun.dylib` files were loaded. The attempted profile was not retained;
these denials are one-host evidence, not a complete read trace.
The trusted adapter must explicitly select and verify any required Xcode
support bytes and close path-ancestor metadata before removing the global
grant or claiming a complete input closure.

Broad system reads remain; general selected-unit and build-script/proc-macro
inputs, exact compiler/dependency identities and the output manifest are not
closed. A successful process exit and binary hash therefore do not form a
trusted `BuildRevision`. The next T2 step is a closed, attested compiler
adapter; F4 then publishes and reloads the verified target closure before a
fresh world may start.

The headless executable added a second Cargo target to the bundled game.
The iOS and Android packaging scripts now request `--lib` explicitly, matching
their use of only `libi1_mobile_game.a` and `libi1_mobile_game.so`. This keeps
mobile builds from compiling or linking an unused desktop executable.
The pinned Rust 1.99.1 iOS simulator `--lib` Cargo build completed locally in
1.76 seconds with the script's selected Rustc on `PATH`; Android packaging was
syntax checked but not cross-built in this worktree.

# First headless build from admitted source

2 November 2026. **Selected narrow native build profile; implementation and
qualification progress are recorded separately.** This resolves the first
source-to-build handoff under [C01, C05 and C10](CONTRACTS.md), the
[F4 family design](modules/WORLD-CONTENT.md), [T2 delivery](modules/TOOLS-DELIVERY.md)
and the [foundation source protocol](FOUNDATION-SPEC.md). It does not claim a
ready executable, private save or shipping target.

## Inputs and identity

The first headless game uses ordinary Rust code linked into a native executable.
The accepted source tree contains a fixed `build/headless.json` metadata file,
its Cargo manifest and lockfile, a pinned Rust toolchain declaration, the
selected rule-source file, a project/entry scene, and every code/metadata file
needed by the chosen local package graph. F3 accepts the exact file tree; F4
reads the build declaration only from a durable `PinnedSource`. A malformed
build declaration may coexist with a valid admitted source root but cannot
produce a ready build. The declaration is tooling metadata, not a second
authored scene or a game-rule authority.

The version-one declaration has exactly these fields:

```json
{"format":1,"cargoManifest":"Cargo.toml","cargoLock":"Cargo.lock","rustToolchain":"rust-toolchain.toml","package":"small-game","ruleSource":"game/rules.rs","entryScene":"3a5ef644-7225-4770-9f4d-e1d9c0a7d515"}
```

Paths use F3 portable relative syntax and must exist in the accepted manifest.
The entry scene must match the cooked product from the same accepted root.
Duplicate/unknown keys, extra bytes, unsupported format, missing inputs and
target mismatch refuse. This bounded metadata check does not parse Cargo,
attest a compiler, prove the rule file was compiled, or decide which files
Cargo/build scripts will read. A trusted T2 builder must inspect the complete
package/dependency closure inside the pinned tree and fail on any escape or
undeclared external source before invoking a compiler. The first profile has
no F3 external dependency roots; an engine SDK supplied outside this tree
would need a separately verified package root before it could be called closed.

`SourceRevision` identifies the entire admitted tree. `SceneProductRevision`
identifies the direct content inputs and target and can be reused across roots.
The eventual `BuildRevision` is the digest of a canonical target manifest
binding accepted `SourceRevision`, product byte hashes, exact executable byte
hash, selected code/schema/provider/protocol closure, target/profile, compiler
and toolchain identity, lock/dependency hashes, and the declared build recipe.
It is never inferred from source acceptance or from a successful compiler exit
alone. A ready build requires every retained artifact to verify against that
manifest and its source closure; old ready manifests and running worlds remain
separately pinned when a later build fails.

## Builder, publication and run

F4 streams verified source bytes into a new private temporary input tree. T2
selects an isolated native compiler adapter with network disabled, pinned
toolchain/dependency cache, controlled environment and a distinct output tree;
build scripts and proc macros are code execution and need that isolation. The
input tree must not become the compiler's writable target directory. T2 records
compiler identity, target triple, flags and exact output bytes, then verifies
the binary and product hashes after the process exits. A content-addressed
artifact store publishes a complete manifest only after all required bytes
and closure checks succeed. A failed compile, missing input, corrupt output,
stale source selection or interrupted publication leaves the prior ready build
identifiable and the newly accepted source separately available.

The first compiler profile should prepare one private Cargo dependency root
from release-verified registry index records and lockfile-checksummed archives,
then reuse that immutable root across edits with the same lock/toolchain key.
Cargo's resolver may need index records for lockfile packages outside the
selected host compile; a package list from filtered workspace metadata is not
the selected build-unit graph. The [local private-cache experiment](../evidence/i2a/F4-PRIVATE-CARGO-CACHE.md)
found 212 distinct registry index names and 37 cached archives (8.7 MB of
archive bytes) sufficient for this workspace's host build, while only one
registry crate compiled. This is a fixture observation, not a general minimum.
T2 must verify the exact cache root, selected units and paths the compiler
actually reads before calling any build ready. Per-edit work should reuse an
unchanged verified dependency root; copying the entire shared registry for
each edit is outside the selected cost model.

W1/F1 construct a fresh bounded headless world only from a target-ready pinned
build, with fresh `SessionId` and `WorldEpoch`. The existing `GameRules`,
authority and replica path handles local and multiplayer actions; the server
pins the ready gameplay build for a match, and a source edit belongs to a
future match. A local substantive edit stops/restarts a disposable run after
the new build is ready. The first low-peak transition may retire the old world
only after verifying the replacement closure and recording an explicit
loading state; it does not promise seamless live transfer. P7 binds a private
checkpoint to the exact source/build tuple and never rebases it silently.

## Cost and proof queue

The initial cold path reads the admitted tree once into temporary files,
performs Cargo dependency/compile work, cooks content, hashes output bytes and
prepares a new world. Memory includes the old run, source pin/SQLite scratch,
temporary compiler memory, retained product/binary bytes and replacement
world preparation. These costs are outside world callbacks but still count in
edit-to-ready and transition peaks. F4's direct scene product key permits
reuse when unrelated source files change. A compiler cache may reuse only
under a complete input/toolchain/target key and must reverify retained bytes;
timing only a cache hit is not an edit-to-ready measurement.

Proof order:

1. Parse the bounded build declaration from an accepted pin and refuse a
   missing/invalid/incompatible declaration without altering the last product
   or run. This is a metadata gate, not build readiness.
2. Qualify a trusted native compiler adapter against a complete pinned local
   Rust workspace, with no checkout read, network or input-tree writes. Record
   exact compiler/dependency/output provenance and failed-compile recovery.
   The [first native compile proof](../evidence/i2a/F4-ADMITTED-HEADLESS-COMPILE.md)
   exercises the actual game and failed-edit retention from a pinned workspace.
   An opt-in macOS sandbox run denies network, checkout reads and input-tree
   writes, but still permits broad filesystem reads and inherits Cargo build
   configuration. It does not close arbitrary build-script or proc-macro
   inputs, attest the compiler/dependency closure, or make the adapter trusted.
3. Publish and reload a verified target manifest and binary/product closure;
   corrupt/missing bytes and interrupted publication refuse, while the prior
   ready build remains available.
4. Start a fresh world through the existing rule/authority/replica callback,
   edit and admit B, build/restart B, and prove a failed C leaves B's run and
   C's accepted source distinct. Measure the complete edit-to-ready path and
   transition memory, then bind a private checkpoint/restore to the exact
   tuple. Real network transport and device qualification remain separate.

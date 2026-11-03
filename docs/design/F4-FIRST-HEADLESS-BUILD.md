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

The [prepared native environment design](T2-PREPARED-NATIVE-ENVIRONMENT.md)
owns the first macOS compiler selection, generated sandbox and invocation
binding. Its current metadata-isolation fixture is a candidate only: the
installed tool and SDK roots are still mutable host locations, and an
unreviewed cold hash does not retain them through compilation. F4 may publish
a ready manifest only after T2 proves that selected bytes and runtime reads
remain closed for the whole build.

The opt-in A/B native fixture now retains an **unready candidate output** in
its own directory after the successful A build. A compact canonical JSON
report names the accepted source generation and `SourceRevision`, the distinct
`SceneProductRevision`, fixed headless target, receipt count and exact byte
length/SHA-256 for the cooked scene, executable, raw rustc receipts, Cargo
transcript, receipt-verifier result, exact rustc receipt-wrapper executable,
supervisor request/report, admitted build
declaration, private-cache candidate manifest, sandbox policy and observed
environment selection. Its SHA-256 is held outside the report. Reload refuses
changed report or file bytes, missing/extra files, mismatched source/product
association, a cross-wired environment/cache record and a receipt set whose
raw digest does not match the verifier summary. The A/B fixture deletes both
the materialized A input and original Cargo output, reloads and executes A,
then reopens the accepted source in a fresh test process, checks the candidate
against the recooked expected product and executes A again. Invalid B leaves
that candidate intact. This format is local
diagnostic evidence, not `BuildRevision` or a publication API. It does not
authenticate original inputs, make installed tools immutable, trace implicit
compiler reads or prove ready-root publication.

The macOS fixture must not modify a retained executable inode after running
it, even when a corruption test restores identical bytes: macOS can keep a
stale kernel code-signature cache and kill a later launch. Binary and wrapper
tamper tests therefore use disposable candidate copies with distinct inodes;
the selected A artifact remains untouched. The [native signature fault
proof](../evidence/i2a/T2-RETAINED-MACOS-SIGNATURE-2026-11-03.md) records the
earlier failure and two corrected A/B runs. These extra copies are fixture
work and are not part of candidate publication cost.

The opt-in candidate now uses one exclusively owned, same-volume staging
directory. Each copied file and the report are synced after final bytes and
metadata are written; the completed bundle is verified, its directory is
synced, then it is renamed into a previously absent candidate name and the
parent directory is synced before success is reported. A process killed before
rename leaves a staging directory that readers ignore; an ordinary failure
removes its own stage. An existing candidate is never replaced. If the parent
sync fails, a visible new directory may exist but no caller has received a
successful selection. This is an **unready local candidate** with
process-crash atomic visibility, not a ready-root transaction, power-loss
durability certification or publication API. The selected ready root must
later be committed separately after complete T2 closure, using an independent
digest and retained reader pins; a stale stage cannot become a ready root.
The extra work is one staging/parent metadata sync and rename beyond the
existing streamed copy and verification. It runs per build, never per world
update; failed edits leave the previous runnable candidate intact.

The first compiler profile should prepare one private Cargo dependency root
from release-verified registry index records and lockfile-checksummed archives,
then reuse that immutable root only when the admitted Cargo manifest set,
lockfile, target, fixed build recipe and selected tool executables match.
An already locked package can become a headless dependency after a manifest
edit without changing `Cargo.lock`; a cache pruned for the old graph is then
insufficient. Ordinary Rust code edits leave this selection identity intact.
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
An [offline full-workspace vendor experiment](../evidence/i2a/F4-VENDOR-FEASIBILITY.md)
produced 229 package directories and 425 MB of local disk use. Cargo's
vendored checksums protect against accidental changes, but do not themselves
establish release provenance. The first profile therefore keeps the smaller
private-cache candidate while a reviewed release manifest or equivalent trust
root for exact dependency and toolchain bytes remains an explicit readiness
gate.
The [first-profile release trust proposal](T2-NATIVE-RELEASE-TRUST.md) makes
the origin and manifest choice reviewable before a local root can be promoted.

The first [native preparation tool](../../scripts/prepare_headless_cargo_home.py)
now copies and SHA-checks every lockfile archive and exact index records from
a local host cache before Cargo resolves in the private home. It prunes
unselected archives, rechecks offline resolution and records a
candidate tree digest. Its index origin is still the mutable host cache, so
the digest identifies observed bytes but does not authenticate their release.
The candidate manifest binds a separate cache reuse key to those selection
inputs, the pinned toolchain/build declarations, and the exact Cargo/Rustc
executable lengths and hashes. Local paths remain in the candidate manifest
and are checked separately, so the key itself is portable. A verifier checks the caller-selected
target and executable files against that candidate before a later admitted-source
build reuses the pruned cache. It also rehashes the complete private cache tree
against the candidate digest and refuses new ambient Cargo config or implicit
`build.rs` files. This is a candidate-selection guard, not a complete compiler
input attestation or a substitute for source immutability during compilation.
The local per-edit rehash is an explicit latency cost; a future authenticated,
immutable dependency root may use a cheaper verified retention mechanism only
after proving the same byte-integrity guarantee.
The first [release-input matcher](../evidence/i2a/F4-RELEASE-INPUT-MATCHER.md)
can compare that portable selection, cache root and selected Rust/linker/SDK
root bytes to a caller-selected manifest. It cannot establish that the
manifest was reviewed or that all compiler reads and outputs are closed.
On macOS, both metadata passes run in a deny-network sandbox with source,
private cache, toolchain and required system read grants; an external path
dependency refuses before its manifest can be read. Other build hosts need
their own qualified metadata and compiler isolation adapter.
The isolated compiler can reuse that root without granting the shared cache.
Keep this cache outside the source project and F3's `.engine` temporary budget;
source admission must see neither cache files nor an undeclared cache manifest.
The first sandboxed fixture now checks Cargo's structured compiler-artifact
stream against five expected package targets. Every reported manifest/source
path belongs to admitted source or the private cache, every reported artifact
belongs to the fresh output tree, and build-script/proc-macro messages refuse.
This observes one selected graph after execution; it does not authorize a
general package graph, attest exact compiler invocations or prove all files
read by rustc. A trusted adapter must close those inputs before publication.
The first macOS invocation candidate uses Cargo's stable `RUSTC_WRAPPER` hook
to write bounded receipts for the selected rustc path, raw ordered arguments,
working directory, child environment and exit. A strict verifier reconciles
successful compile receipts with the fresh Cargo artifact transcript, checks
each named `--extern` against an artifact reported for that dependency, and
refuses unsupported paths and hooks. The first pinned Cargo profile also
accepts only its observed version/configuration probe arguments from the
admitted source root. The compiler's Cargo crate/manifest environment must
agree with Cargo's target and source package. The verifier requires the observed
emission sets and reconciles crate types with Cargo's target metadata; the
debug codegen flags and recorded Xcode SDK/Clang layout must match the narrow
profile. This remains an observed graph after execution. The wrapper does not
observe rustc's implicit file reads, loaded
libraries, linker subprocess reads or the full macOS system boundary, and its
own executable still needs release selection. It runs per rustc process, not
per engine tick or source-file scan; cold tool/dependency inventory stays
outside the edit path. See the [local receipt proof](../evidence/i2a/F4-RUSTC-RECEIPTS.md).
The cache preparer also walks normal/build metadata edges from the headless
package and refuses custom-build/proc-macro targets in that possible set before
compilation. Cargo metadata unions workspace feature use: this walk includes
`foldhash`, while the selected compile transcript does not. It is deliberately
an overapproximation and cannot be used alone to prune the cache to exact
build units or define the final build identity.
The candidate manifest also records the SHA-256 and byte lengths of the
selected Cargo and Rustc executable files and rechecks them after metadata;
the A/B fixture compares those recorded digests to the installed files and
checks them again after both compiles. Hashing is cold preparation work, not
per-tick or per-edit work. These two files do not constitute the toolchain:
Rustc loads libraries and standard crates, and native linking uses system SDK
inputs. The trusted adapter must bind their exact release closure as well.
On the local macOS ARM64 qualification, the compiler sandbox can omit broad
temporary-directory and application reads when it resolves the selected
developer root, Clang linker and MacOSX SDK before entering the sandbox,
grants the selected linker toolchain and SDK roots, and passes the selected
paths to Cargo. Leaving the default
`cc` shim under that
narrow policy causes repeated denied `xcrun`/Xcode cache reads and an observed
roughly fivefold cold-build slowdown. The direct linker path avoids those
mutable cache reads and is faster in the paired local fixture. It remains a
host-specific candidate until the selected Clang, SDK, loaded libraries and
their invocation are verified as one release/toolchain closure.
A sorted path/type/length/content-hash inventory of this host's Rust toolchain
took about 0.6 seconds for 394 files and 1.03 GB of logical bytes. The selected
Xcode linker toolchain and SDK took about 5.95 and 5.66 seconds respectively
for 2.88 GB and 0.95 GB of logical bytes. These are cold release-preparation
candidates, not per-edit verification costs or authenticated roots; the
system libraries still need a closed provenance policy.
The [cold inventory helper](../../scripts/inventory_headless_tool_roots.py)
now defines a candidate digest over three named roots: installed Rust,
selected native linker toolchain and selected SDK. It streams each regular
file, records directory membership and symlink target bytes, and refuses
symlinks escaping a selected root. It is separate from cache preparation so
its 4.86 GB logical read is not repeated for each candidate source or cache.
The first [local inventory proof](../evidence/i2a/F4-TOOL-ROOT-INVENTORY.md)
does not authenticate a release or close the remaining system-library reads.

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
   writes. A later local run uses a private Cargo home, restricted source and
   cache reads, and a cleared compiler environment. Broad system read roots,
   unproved index completeness, unreviewed release inputs and unclosed selected
   build units remain. It does not attest the compiler/dependency closure or
   make the adapter trusted.
   The [narrow metadata proof](../evidence/i2a/F4-NARROW-METADATA.md)
   removes global metadata permission for the local A/B compile and denies
   unrelated file and directory metadata; its Xcode support paths and host
   runtime bytes still need release and retention qualification.
3. Publish and reload a verified target manifest and binary/product closure;
   corrupt/missing bytes and interrupted publication refuse, while the prior
   ready build remains available.
4. Start a fresh world through the existing rule/authority/replica callback,
   edit and admit B, build/restart B, and prove a failed C leaves B's run and
   C's accepted source distinct. Measure the complete edit-to-ready path and
   transition memory, then bind a private checkpoint/restore to the exact
   tuple. Real network transport and device qualification remain separate.

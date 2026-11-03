# Prepared native compiler environment

3 November 2026. **Selected first macOS ARM64 development adapter design;
implementation and qualification are separate.** This refines the
[first headless build](F4-FIRST-HEADLESS-BUILD.md), [release-input policy](T2-NATIVE-RELEASE-TRUST.md)
and [C01/C05/C10 contracts](CONTRACTS.md). It does not make an executable or
`BuildRevision` ready.

## Owner and required outcome

T2 owns one prepared environment selection for native compiler execution. F3
still owns accepted source bytes and history; F4 cooks products, binds a target
build and retains its artifacts. A game build declaration may select its Cargo
package and scene, but cannot select a compiler, trust manifest, sandbox policy
or ambient host configuration. A running world and multiplayer match pin a
future ready build under C01/C10; neither reads a candidate environment as a
second gameplay authority.

The adapter must make one exact, reviewable choice of release inputs, tool and
dependency roots, compiler wrapper, macOS support/runtime policy, sandbox
permissions and launch recipe. The same selection must generate the sandbox
and be checked against compiler receipts and retained output provenance.
Changing any selected byte or permission must refuse before a new ready root
can be published. A digest alone cannot retain host-installed bytes against a
concurrent updater. The candidate keeps `buildReady:false` until the selected
roots are held immutable for the whole build or an equivalent closed mechanism
has been proved. A later source B failure must leave A's retained build and
run identity intact.

## Candidate representation and lifecycle

Use two immutable layers rather than a new engine-wide build framework:

1. **Cold environment selection.** A canonical, versioned T2 record binds the
   caller-selected release manifest digest; private Cargo cache root; selected
   Rust, Xcode linker and SDK root identities; exact Cargo/Rustc/linker/wrapper
   executable hashes; selected Xcode support files; macOS host-runtime policy;
   and the fixed recipe and sandbox-policy format. A candidate may record an
   unreviewed manifest or host policy only with `buildReady:false`. The record
   is created outside admitted game source and is not selected by a game.
2. **Per-build invocation.** Bind the cold selection to one accepted F3 root,
   cooked product hash, newly materialized input tree and fresh output tree.
   Generate the deny-default sandbox policy and complete Cargo environment
   from this binding. The command, policy bytes, wrapper copy and later receipt
   verifier consume the same binding identity. Neither linker nor SDK is
   rediscovered inside the sandbox. A new source edit changes this invocation,
   not the cold tool/dependency selection when its dependency key is unchanged.

The preparer owns and retains the selected record while compiler children run.
It refuses aliases, absent or changed roots, ambient Cargo config, executable
hooks and unsupported discovery before launch. The per-build owner waits for
the whole process group, bounds elapsed time and diagnostics, then checks all
receipts and output bytes before releasing temporary inputs. No partial or
failed candidate can advance a ready root. A later ready publisher must retain
the executable and product blobs with a canonical manifest, reload and verify
them, then atomically select the new `BuildRevision`; it must preserve prior
ready roots and their readers during replacement.

For this first profile, the existing private cache and selected tool roots are
inventoried by streaming bounded buffers. The private cache is already copied
into an owned directory; the installed Rust/Xcode/SDK roots are still mutable
host locations. A cold inventory of those locations is a candidate selection
check, not an immutable byte lease. A ready adapter must select an owned,
no-writer snapshot (including support files), or prove an equivalent retention
mechanism that prevents substitution by its compiler children and cooperating
jobs both before launch and while child processes run. Rehashing the live roots
just before or after launch alone does not close the interval between those
checks. The first macOS development profile trusts the local user account; it
does not defend against a deliberately hostile same-user process. The existing
Cargo origin and Rust
component receipts remain adjacent evidence, not a substitute for a reviewed
preparation job. The current wrapper and [receipt verifier](../../scripts/verify_rustc_receipts.py)
observe invocation inputs and Cargo-reported units; they do not trace implicit
compiler or linker reads. A verified retained root can safely cover all bytes
within a deliberately permitted subtree, but the policy must name every
permitted subtree and account for path-ancestor metadata and discovery. It
must not silently restore global metadata access to make a build pass.

The first [APFS clone candidate](../evidence/i2a/T2-APFS-CLONE-CANDIDATE.md)
uses held source descriptors and explicit same-volume `fclonefileat` calls,
then hashes the entire cloned root against an independently supplied exact
inventory. Final digest equality can establish the selected **byte set** even
if the installed tree changed while files were copied; it does not establish
one instantaneous source snapshot or prevent a later write to the clone. The
current caller uses unreviewed local digests. An opt-in A/B qualification now
launches Cargo/Rustc and Clang/SDK from the clones; explicit sandbox controls
deny reads and metadata from the corresponding installed tool files. The
private-cache metadata phase now uses the selected cloned Cargo/Rustc in the
[later A/B qualification](../evidence/i2a/T2-SELECTED-METADATA-TOOLS-2026-11-03.md).
Xcode discovery and selected support files still use installed paths before
the metadata sandbox, and same-user writes to the clones remain possible.
The [later metadata sandbox reduction](../evidence/i2a/T2-METADATA-SANDBOX-READS-2026-11-03.md)
also limits its file data to selected source/cache/tool roots and named macOS
runtime paths, with metadata reads inside those subtrees and on their path
ancestors. It preserves broad
process permission and selected system-library/cryptex reads; it does not
prove complete implicit-read closure.
The compiler sandbox itself now denies installed support-file data and
metadata reads in both installed-root and retained-clone A/B fixtures; direct
selected Cargo/Rustc/Clang does not need those exceptions in this bounded
build. This does not remove their earlier preparation dependency or close
other system reads. Apple's
[file-flag rules](https://developer.apple.com/library/archive/documentation/FileManagement/Conceptual/FileSystemProgrammingGuide/FileSystemDetails/FileSystemDetails.html)
permit an owner to clear `UF_IMMUTABLE`, so that flag alone cannot exclude a
hostile same-user clone writer.
A ready preparer must bind the expected digest
to the reviewed release manifest and hold the selected clone under a no-writer
lifecycle through compiler shutdown and artifact publication.
The [selected retained-root boundary](research/T2-MACOS-RETAINED-ROOT-TRUST-OPTIONS-2026-11-03.md)
trusts the local user for this development profile. Owner-clearable file flags
remain an accident barrier, and the preparer must still prove one held
no-writer lifecycle for its selected bytes.

### Retained-root owner lifecycle target

For this development profile, cold preparation creates each content-identified
clone under a checked private owner directory. A permanent, user-immutable
per-selection lock file outside the clone excludes cooperating preparation
while it is made and sealed; the lock inode is never replaced. The preparer
removes write and special mode bits without removing execute bits, applies the
owner-clearable `UF_IMMUTABLE` flag to files and directories from leaves to root, and verifies
the exact inventory and a separately versioned seal policy before publishing a
small canonical selection record. Relative symlinks are checked without
following them; sealing their parent directory protects the entry. The root
directory is sealed last, so an interrupted preparation cannot appear complete
merely because some children were sealed. A partial clone or record is
quarantined and never selected.

A build reopens the selected record and root under a shared lock, checks their
identity and seal, and retains the lock and root descriptor through metadata,
compiler shutdown, receipt reconciliation and output verification. Several
builds may read the same sealed selection concurrently. A lock downgrade is
not treated as atomic: acquisition starts again and revalidates after obtaining
the shared lock. The lock descriptor must not be inherited by compiler
children. Published roots are append-only in this profile: a cooperating job
never unseals, renames or deletes one, even after a lease owner dies. This
keeps selected pathnames stable for surviving children. Garbage collection is
a separate future operation requiring proof that no compiler child still uses
the root. Recovery under the exclusive lock refuses partial state and may
quarantine only an owned identity; it cannot relabel that state as published.
This lifecycle still requires explicit selected support,
dependency and runtime records; sealing the three tool roots alone does not
close those inputs.

The lock coordinates only Thrum jobs. The flags and read-only modes block
accidental writes, including compiler writes that escaped a sandbox rule, but
the trusted local user can deliberately clear them. The release job and human
review must bind the selected record to an actual reviewed manifest before a
ready build can use it. Cold sealing and full hashing may cost seconds and are
outside the edit path; a warm build reuses its held read lease rather than
rescanning gigabytes on every edit. A fresh process must revalidate cold state
before relying on the record, including after an interrupted preparation.
Measure lock wait, seal walk, physical allocation and build overlap separately
before claiming a performance benefit.
The [first held-root candidate](../evidence/i2a/T2-HELD-TOOL-ROOT-CANDIDATE-2026-11-03.md)
proves this lifecycle one caller-selected APFS clone at a time, including
local full Rust, Xcode linker and SDK cold preparations. It has not held the
complete reviewed selection through a real Cargo job. A later
[admitted A/B qualification](../evidence/i2a/T2-HELD-NATIVE-AB-2026-11-03.md)
did hold those three sealed roots through private-cache preparation, real
Cargo A/B, receipt checks and output verification, then rehashed them before
owned scratch cleanup. It did not hold the cache, support or host-runtime
inputs in that selection. The one-host cold Xcode
pair took 33.45 seconds to clone, seal, fsync and rehash, separate from 13.18
seconds to inventory the installed sources. This belongs to release
preparation, not a game edit or world tick.

### Private dependency cache retention target

The current pruned private Cargo home is produced by a write-capable cold
metadata step, then rehashed before every A/B reuse. The local 2,923-file,
67.8-MB integrity pass costs roughly 150–230 ms per edit. The compiler sandbox
already denies writes to `CARGO_HOME`, but metadata preparation requires its
fresh writable home. Seal only **after** metadata resolution, archive pruning,
the second offline resolution and exact private-root inventory have completed.
A dependency-selection change creates a new private home; a code-only edit may
reuse the held selection.

The [cold retained-cache candidate](../evidence/i2a/T2-HELD-CARGO-CACHE-CANDIDATE-2026-11-03.md)
now binds the two digest domains and a sealed manifest under a held APFS root
lease. Five synthetic macOS fixtures cover extra-directory drift, manifest
drift, interrupted publication, concurrent readers and lease release after a
sidecar-close error. It has not yet been used by real Cargo or measured with a
full private cache.

Retain one cache under the same private release owner and append-only lifecycle
as the tool roots. Bind three distinct identities in its canonical record: the
existing `thrum.cargo-home.v1` digest used by the cache manifest and origin
receipts, a whole-tree digest that includes directory membership and links,
and the exact private-cache manifest bytes. Neither digest may be silently
substituted for the other. The cold preparer compares both before and after
sealing, and the compiler's `CARGO_HOME` must resolve only to that selected
sealed path. A held shared lease spans cache reuse checks, Cargo shutdown,
receipt verification and output retention. An unsealed cache still takes the
current complete byte rehash path.

The warm fast path can skip repeated full-cache and tool-executable hashing
only when its T2 owner holds matching cache and tool-root leases and validates
the admitted dependency-selection key. A Boolean caller claim or a subprocess
that verifies then exits cannot confer that lease on a later compiler process.
The existing origin comparisons remain local candidate evidence until a
reviewed release manifest and preparation job bind their acquisition. Prove a
real metadata preparation followed by an A/B build from the sealed home.
The [pinned Cargo bookkeeping probe](../evidence/i2a/T2-HELD-NATIVE-AB-2026-11-03.md#pinned-cargo-cache-bookkeeping-probe)
passed with `.global-cache` and `.package-cache-mutate` absent and cache writes
denied during unsealed A/B compilation; this establishes the compiler's
read-only behavior for that fixture, not a held cache. Prove
mutation/extra-directory refusal, independent reopen, interrupted preparation
and multiple concurrent read leases before removing the per-edit rehash.
Measure cold clone/seal and warm selection-key, cache, tool-hash and total
edit-to-ready costs separately; retain the full rehash on other platforms
until their own lifecycle is qualified.

## macOS runtime boundary

The local profile still permits named system-library and configuration
subtrees. The initial global file-metadata grant has been
removed in the local candidate, but metadata and data reads remain available
within those selected subtrees. `/Library/Preferences` and broad
`/private/etc` are now denied in the compiler fixture. A bounded refusal
diagnostic identified one LibreSSL read of `/private/etc/ssl/openssl.cnf`;
the profile grants and hashes only that file for this need. It also denies
broad `/usr/bin` and `/dev` data reads in the bounded installed and cloned
A/B fixtures, retaining `/dev/null` and `/dev/urandom` explicitly. The
system-library and cryptex reads remain broad. `dyld` can resolve libraries
through the OS shared cache; visible library paths alone are not a complete
runtime-byte inventory. The first ready build therefore needs a human-reviewed
host-runtime trust policy and exact selected OS/runtime evidence alongside the
official-HTTPS tool/dependency manifest. Until then the candidate may measure
and narrow permissions, but may not turn a sandbox success into readiness.

The first isolation proof generates additional ancestor metadata grants only
for selected input/output/cache/tool/support/runtime paths. It selects Xcode
`libxcrun.dylib` support paths for reads, denies existing unrelated file and
directory metadata requests while allowing selected controls, and runs A/B
under the generated policy. A probe of the Command Line Tools tree is not
proof that its bytes were loaded, and the support files are not yet selected
by immutable hashes. Unsupported Xcode discovery must fail rather than widen
the read grant. The selected release matcher and this policy generator must
consume one cold environment identity before publication is possible.

The [first local metadata proof](../evidence/i2a/F4-NARROW-METADATA.md) passed
the A/B fixture with ancestor grants and explicit Xcode support paths. The
[subsequent grant reduction](../evidence/i2a/T2-COMPILER-SUPPORT-GRANTS-2026-11-03.md)
removes those compiler-time exceptions. It does not select immutable support
bytes for metadata preparation, close system reads or join the release
matcher to the compiler launch, so the candidate remains unready.

## Cost and proof queue

Cold root verification should be outside the edit path only after immutable
root retention is proved. Until then the candidate may cache an inventory for
experiments, but cannot claim that reuse is trusted. The observed
Rust/linker/SDK inventory reads about 4.86 GB of logical bytes and took roughly
11–15 seconds on one Mac; a release preparation job must measure snapshot
creation and additional support/runtime bytes. A local
[APFS clone feasibility probe](research/T2-MACOS-APFS-TOOL-ROOT-RETENTION-2026-11-03.md)
copied the Rust root in 0.056 seconds and the selected Xcode linker/SDK roots
in 8.398 seconds with matching inventories and working cloned compilers.
Whole-tree atomic selection, an owned no-writer lifecycle and selected support
files remain unproved. The current per-edit private-cache integrity pass reads
67.8 MB and took roughly 150–230 ms. Keep that cost until an equivalent sealed
retention guarantee is proved; metadata-only checks cannot silently replace
byte integrity. Policy generation is proportional to selected roots and
ancestors, not source-file count. Receipt work is per rustc process, never per
engine tick. Count compiler output hashing, publication fsyncs, replacement
overlap and process RSS in the assembled edit-to-ready path rather than only
the compiler subprocess.

Prove in order:

1. Prepare a canonical candidate selection and generate its sandbox and
   fixed launch recipe. A/B must consume that one selection. A changed
   wrapper, root membership, support byte, policy or recipe refuses. Mutate a
   support/library byte after preparation and during compilation: a ready
   mechanism must prevent either mutation from changing the selected inputs.
2. Remove global metadata permission. Positive selected reads and negative
   existing-file/directory probes must pass under the generated policy; A
   compiles, invalid B fails, and A still executes the same authority/replica
   game. This local candidate proof has passed; keep it unready while
   host-runtime reads remain broad.
3. Bound Cargo stdout/stderr, time and descendant lifetime; audit successful,
   failed and interrupted receipts. The local [failed-B audit](../evidence/i2a/F4-RUSTC-RECEIPTS.md)
   now reconciles one ordinary failed compile with successful units and Cargo's
   failed finish, while the supervisor checks bounded stderr separately.
   A [fault-report slice](../evidence/i2a/F4-NATIVE-SUPERVISOR-CANDIDATE.md)
   now writes an unready, bounded observed-receipt inventory after a caught
   SIGTERM or timeout. A real pinned Cargo process with the actual receipt
   wrapper and a controlled sleeping compiler shim exercised the SIGTERM path;
   it does not prove all invocations were captured.
   Interrupted and timed-out receipt closure remains open. An overproducing or hung child must not
   retain an input tree or make adapter-held diagnostic memory unbounded.
   Measure compiler-job peak RSS separately; this profile has no hard process
   memory ceiling. The [withdrawn Rust collector experiment](../evidence/i2a/F4-BOUNDED-NATIVE-BUILD.md)
   identifies interruption, escaped-descendant and inherited-stdin faults
   that the next supervisor proof must reject. A [Python supervisor candidate](../evidence/i2a/F4-NATIVE-SUPERVISOR-CANDIDATE.md)
   now bounds the local A/B build and detects owner loss, but cannot yet
   contain an escaped session or provide a release job identity. The
   [macOS job-containment analysis](research/T2-MACOS-COMPILER-JOB-CONTAINMENT-2026-11-03.md)
   records why process-group cleanup does not close that lifetime. The user
   selected the reviewed-compiler-family assumption for this first development
   profile: reviewed Rust/Xcode helpers must remain in the build group, and an
   observed escape is a release fault. This is an explicit trust assumption,
   not OS transitive containment; successful, failed, timed-out and interrupted
   same-group paths plus retained receipt closure still need qualification.
4. Bind a candidate output report to F3 source root, F4 product bytes,
   environment selection, complete receipt set and streamed executable hash.
   The [local retained-output proof](../evidence/i2a/F4-RETAINED-NATIVE-CANDIDATE.md)
   copies the real A build into a separate candidate directory, then deletes
   its materialized source and original Cargo output before checking the
   executable and every retained file against an externally held report hash.
   A fresh process reopens the accepted F3 root, derives the expected scene
   product and runs the retained A executable after those temporary trees are
   gone.
   Corruption, missing receipts, cross-wired selection and invalid B refuse.
   This is a fixture-only candidate; release review, immutable selected roots,
   full compiler/system-read closure and durable ready publication remain open.
5. Select/review exact release and host-runtime inputs, close all permitted
   compiler/system reads, then publish and reload a durable ready manifest.
   Measure cold preparation, warm edit-to-ready tail and peak RSS separately.

The unchecked trusted-adapter item in the [build plan](../BUILD-PLAN.md)
remains the owner until these gates pass. Rendering, environment, water and
cloud providers consume only a ready source/product/build boundary; this T2
selection does not add a renderer or alter their scene contracts.

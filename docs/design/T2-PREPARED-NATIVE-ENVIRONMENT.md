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
immutable snapshot (including support files), or prove an equivalent retention
mechanism that prevents substitution both before launch and while child
processes run. Rehashing the live roots just before or after launch alone does
not close the interval between those checks. The existing Cargo origin and Rust
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
The compiler sandbox itself now denies installed support-file data and
metadata reads in both installed-root and retained-clone A/B fixtures; direct
selected Cargo/Rustc/Clang does not need those exceptions in this bounded
build. This does not remove their earlier preparation dependency or close
other system reads. Apple's
[file-flag rules](https://developer.apple.com/library/archive/documentation/FileManagement/Conceptual/FileSystemProgrammingGuide/FileSystemDetails/FileSystemDetails.html)
permit an owner to clear `UF_IMMUTABLE`, so that flag alone cannot provide the
required protection from a same-user clone writer.
A ready preparer must bind the expected digest
to the reviewed release manifest and hold the selected clone under a no-writer
lifecycle through compiler shutdown and artifact publication.
The [retained-root trust analysis](research/T2-MACOS-RETAINED-ROOT-TRUST-OPTIONS-2026-11-03.md)
keeps the same-user threat boundary open: owner-clearable file flags are an
accident barrier, not a strict exclusion of a hostile same-user process.

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
   records why process-group cleanup does not close that lifetime, the
   OS-owned job proof target and a weaker trusted-tool fallback that would
   require an explicit design choice.
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

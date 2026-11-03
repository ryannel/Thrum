# Retained native tool-root trust boundary

3 November 2026. **Selected first-profile boundary: trusted local user.** The
user selected option A for the first macOS headless development build. This
refines the [prepared macOS environment](../T2-PREPARED-NATIVE-ENVIRONMENT.md)
and the user-selected [reviewed exact-hash release policy](../T2-NATIVE-RELEASE-TRUST.md).
It does not make any current candidate `BuildRevision` ready.

## Confirmed boundary

F3's admitted source and F4's cooked product are already distinct from T2's
compiler inputs. T2 must retain one selected byte set from cold preparation
through Cargo metadata, compilation, receipt verification and artifact
publication. The initial APFS helper clones files into an owned destination
and verifies the whole-root inventory, but its clone is writable afterward.
A later [held-root candidate](../../evidence/i2a/T2-HELD-TOOL-ROOT-CANDIDATE-2026-11-03.md)
seals one caller-selected clone at a time and opens shared read leases; it has
not yet been used by the real Cargo A/B consumer. That older compiler proof
granted reads from unsealed clones and denied corresponding installed files.
Metadata preparation and selected Xcode support/runtime inputs are still
outside the complete held selection. No source or multiplayer authority may
come from a mutable candidate environment.

Apple describes APFS clones as copy-on-write same-volume copies, not a
whole-tree transaction ([APFS overview](https://developer.apple.com/documentation/foundation/about-apple-file-system)).
The per-file helper therefore compares the completed root with one
independently supplied digest rather than claiming an instantaneous
installed-tree snapshot. The current digest is a local candidate; a release
preparer must bind it to the reviewed manifest before readiness. Apple
also states that `UF_IMMUTABLE` can be cleared by the file owner
([file flags](https://developer.apple.com/library/archive/documentation/FileManagement/Conceptual/FileSystemProgrammingGuide/FileSystemDetails/FileSystemDetails.html),
[chflags guide](https://developer.apple.com/library/archive/documentation/OpenSource/Conceptual/ShellScripting/ShellScriptSecurity/ShellScriptSecurity.html)).
Read-only POSIX modes or user immutable flags prevent accidental writes but
cannot exclude a deliberately hostile process running as that same owner.

## First-profile options

| Option | Mechanism and trust boundary | Cost and limitation |
| --- | --- | --- |
| **A. Trusted local user** | The first development profile trusts processes with the builder's user identity. A private `0700` release owner creates exact-hash APFS clones under an absent name, installs a reviewed manifest, applies read-only/user-immutable flags as an accident barrier, keeps its owner/selection alive through all child shutdown and output verification, and permits the compiler sandbox only selected reads. Any observed changed byte or ownership failure refuses; no product is promoted. | Fits an ordinary local development build and avoids privileged setup. It cannot claim protection from a same-user process that deliberately clears flags or swaps a path during use. The policy must say this explicitly. |
| **B. Protected owner** | A distinct privileged account or genuinely read-only mounted release image owns the selected byte set and grants the builder read access only. The release preparer must prove mount/owner lifetime, no writable aliases, digest match, and child teardown before unmount/replacement. | Stronger same-user isolation, but adds privileged installation, update and recovery machinery. APFS cloning alone does not supply it. No such helper or verified image exists in this repository. |
| **C. Candidate only** | Keep cloned roots as unready local qualifications until B is implemented or another enforceable mechanism is found. | Preserves the strictest interpretation, but blocks a target-ready local build even after source, compiler and output closure are otherwise proven. |

Option A is the selected **first local development** boundary. It is still a substantial implementation
gate: one owner must bind reviewed release hashes, private dependency cache,
selected Xcode support and runtime records, sandbox policy and compiler
receipts; deny writes from its compiler children; prevent cooperating Thrum
jobs from replacing the root; and keep the old candidate on every failure.
Hashing before and after alone does not certify an interval against hostile
same-user ABA. The exact byte set must be selected once and held for the
whole job. Process-group cleanup and the separate compiler-job containment
decision remain open.

The cold 4.86-GB logical inventory and APFS clone work belong to release
preparation, not world updates. One local held-root probe measured 33.45
seconds to clone, seal, fsync and rehash the Xcode linker/SDK pair after
13.18 seconds of installed-source inventory. This is single-host cold work,
not a per-edit cost. The selected private Cargo cache is about
67.8 MB logical and its current per-edit integrity check took roughly
150–230 ms in local samples. Retaining old and new release roots overlaps
physical storage and must be measured with actual volume use; APFS clone
logical byte counts are not physical reservation. None of these samples
justify a throughput or latency guarantee.

The selected profile trusts the local user account against deliberate
same-user mutation. Read-only modes and owner-clearable flags remain accident
barriers; this profile makes no adversarial same-user isolation claim. Option B
is a later stronger tier if that threat boundary becomes required. Keep
`buildReady:false` until an actual reviewed release manifest, complete
retained no-writer lifecycle through real compiler work, compiler/system-read
closure, job containment and ready publication are qualified.

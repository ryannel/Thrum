# Release trust for the first native headless builder

Status: **selected policy**, 2 November 2026. The user selected a reviewed
exact-hash manifest from official HTTPS origins for the first CPU-only macOS
ARM64 development profile. This does not approve a particular manifest or mark
the current local compiler qualification ready. The [shared contracts](CONTRACTS.md) still
requires a target BuildRevision distinct from accepted source, cooked product
and active world or match identity.

## What has to be trusted

The first native adapter consumes an admitted Rust workspace, lockfile,
registry metadata and archives, Cargo/Rustc and their loaded toolchain files,
Clang, the selected SDK, required system libraries, a fixed recipe and a
controlled environment. Its executable and cooked scene product are derived
outputs. The [local tool-root inventory](../evidence/i2a/F4-TOOL-ROOT-INVENTORY.md)
and [private-cache proof](../evidence/i2a/F4-PRIVATE-CARGO-CACHE.md) identify
candidate bytes and reject local drift; they do not establish where the first
bytes came from. The [vendor experiment](../evidence/i2a/F4-VENDOR-FEASIBILITY.md)
shows that moving to a full-workspace directory source would grow the local
dependency tree to 425 MB without solving its origin.

Cargo's [`--locked`](https://doc.rust-lang.org/cargo/commands/cargo.html)
checks that dependency resolution does not change the lockfile, and
[`--offline`](https://doc.rust-lang.org/cargo/commands/cargo.html) prevents
network use. Neither authenticates a host's existing index cache. Cargo's
[source replacement guide](https://doc.rust-lang.org/cargo/reference/source-replacement.html)
says vendored checksums are for accidental modification, not security.
[Rustup's security guide](https://rust-lang.github.io/rustup/security.html)
says rustup uses HTTPS but does not validate download signatures. Therefore a
successful offline build and local SHA-256 inventory cannot alone be called a
release-verified toolchain/dependency root.

## Selected first profile

Use a **reviewed release input manifest**, produced once by a controlled
release preparation job, as the project's trust anchor. The release job fetches
the pinned Rust distribution, selected Apple toolchain/SDK and exact lockfile
packages through their official HTTPS channels, records source URLs and
versions, verifies archive checksums from the reviewed lockfile, and freezes
exact content digests for each selected input root. A human reviews and
approves the manifest in the repository before it can select a build. The
release job's image and installer provenance are part of that review.

The native adapter must then:

1. Match admitted source, declaration, target, recipe and selected package
   graph to the reviewed manifest. A manifest or lockfile edit is a new
   release-input review, even if source code alone could reuse a cache.
2. Check the exact private dependency root and selected tool/SDK roots before
   using them. Keep cold root verification outside each edit; per-edit work
   checks an immutable retained root and the admitted selection key. The
   current local cache rehash is about 150–230 ms per edit and stays until an
   equivalent sealed-root mechanism is proved.
3. Run the compiler under the selected no-network, no-input-write sandbox;
   close all selected build units, loaded tools/system reads and executable
   hooks before publication. A sandboxed success alone does not close this.
4. Hash binary and cooked product bytes, publish a canonical target manifest
   only after full closure verification, then reload it before reporting a
   target-ready BuildRevision. A failed or interrupted build retains the last
   ready revision; an admitted newer source remains a separate state.

This policy trusts the project's reviewed release manifest, its preparation
job and official HTTPS distribution. It does **not** claim that rustup or Cargo
provide publisher signatures. Signing the project's release manifest can be
added later without changing BuildRevision semantics. The first profile stays
macOS ARM64; other platforms need their own selected adapter and evidence.

### First exact-input matcher

The first [release-input verifier](../../scripts/verify_headless_release_inputs.py)
accepts a caller-selected, canonical JSON manifest outside the admitted game
source and private Cargo home. The trusted engine release loader, not game
source or this verifier, must choose that manifest. The version-one manifest
has exactly `format`, `profile`, `target`, `selectionKey`, `lockSha256`,
`dependencyRoot`, `toolRoots` and `originEvidence`. Its profile is
`headless-macos-arm64-dev-v1`, target `aarch64-apple-darwin`, and SHA-256 values
use lowercase hexadecimal. `dependencyRoot` records the prepared private
cache's digest, file count and logical bytes. `toolRoots` records the existing
descriptor-relative inventory for named Rust, linker and SDK roots. The
selection key covers admitted Cargo manifest membership and bytes, lockfile,
toolchain/build declarations, fixed recipe, target and Cargo/Rustc executable
byte identities. It excludes local executable paths so one reviewed manifest
can match the same installed bytes on another Mac; the local private-cache
candidate still checks its selected paths explicitly.

`originEvidence` binds one adjacent canonical JSON file by filename and
SHA-256. That record names the Rust, Apple linker, Apple SDK and crates.io
sources with version strings and official HTTPS URLs. The matcher checks the
record's bytes, shape and official hostnames. It cannot prove that those URLs
actually supplied the installed bytes; the controlled preparation job and
human release review own that provenance claim. No real reviewed manifest is
committed yet. The matcher reports an exact root match with `buildReady:false`
because selected compiler reads, system libraries, output provenance and
publication remain open.

Cold verification streams the selected Rust/linker/SDK roots once per release
selection, using the [measured candidate inventory](../evidence/i2a/F4-TOOL-ROOT-INVENTORY.md)
of about 4.86 GB logical reads and 11 seconds on this host. The dependency
cache's 67.8 MB local rehash remains in the per-edit path until a sealed-root
mechanism proves equivalent retention. Do not charge the cold tool scan to
every world update or silently omit it from first selection latency.

The first [Rust component archive verifier](../../scripts/verify_rust_component_archive.py)
accepts caller-supplied channel manifest, SHA-256 sidecar, component archive
and installed toolchain. It checks the selected official HTTPS URL/hash pair,
then streams the checksum-verified archive once and compares every file in its
installer manifest with the installed component. It accepts the selected
ustar file/directory subset plus short GNU long-name records, and refuses
other archive extensions before their metadata can be buffered. XZ dictionary
memory, decompressed bytes, member count and file sizes are bounded.
Installed paths are read through held directory descriptors without following
aliases. It reports `buildReady:false`.
It does not fetch the inputs or attest their HTTPS transport, make the
installed tree immutable, verify other components, or replace human review of
the release preparation job and complete tool/SDK/dependency closure. The
[local Cargo/Rustc/host-standard-library component proof](../evidence/i2a/T2-RELEASE-ORIGIN-CANDIDATE.md)
is one host's partial origin candidate.

The cold [crates.io archive comparator](../../scripts/verify_crates_io_archive_origin.py)
streams each archive listed by a selected private-cache candidate from its
official HTTPS CDN URL, refuses redirects, and compares fetched bytes with the
candidate, private cache and selected lockfile checksum. The complete private
cache is inventoried before and after. Its [37-archive local proof](../evidence/i2a/T2-CRATES-ORIGIN-CANDIDATE.md)
is still an origin candidate: cached sparse-index records, extracted source,
selected build-unit closure and the release preparation environment are not
authenticated by archive matching. It also reports `buildReady:false`.

The separate [sparse-index comparator](../../scripts/verify_crates_io_index_origin.py)
parses only the pinned Cargo 1.99.1 cache layout and compares every cached
version JSON line with the corresponding official `index.crates.io` HTTPS
record. The [212-name local comparison](../evidence/i2a/T2-CRATES-INDEX-ORIGIN-CANDIDATE.md)
matched 9,203 cached versions and the exact registry configuration. It proves
origin for those recorded bytes; it does not prove the cache includes every
upstream version, extracted source is archive-identical, or a particular
selected build and environment are fully closed. It reports
`buildReady:false`.

The cold [crate extraction comparator](../../scripts/verify_crate_extraction.py)
compares each selected locked archive's regular tar payload with the matching
private Cargo source tree, including complete file and directory membership.
The [37-package local pass](../evidence/i2a/T2-CRATE-EXTRACTION-CANDIDATE.md)
matched 2,634 extracted files against the same private-root digest used by the
archive and index origin candidates. It closes that local byte link, while
selected build-unit and compiler/system reads, release review and target
publication remain open. Its report also keeps `buildReady:false`.

## Considered stronger alternative

Require cryptographic publisher signatures or a project-signed mirror for
every toolchain, SDK, registry index and archive before the first ready build.
That would require a signing/mirroring operation, key ownership, rotation and
recovery, and provenance for Apple system components. It would delay the first
native adapter and does not remove the need for exact compiler-read closure or
artifact publication. Choose this only if the first development build must
meet that stronger supply-chain guarantee.

The signed-mirror option was not selected for the first development profile.
It remains available as a later release/security tier without changing the
first profile's exact BuildRevision identity or source/authority contracts.

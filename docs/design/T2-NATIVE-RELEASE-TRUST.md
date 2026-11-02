# Proposed release trust for the first native headless builder

Status: **decision requested**, 2 November 2026. This is a T2/F4 release
input policy proposal for the CPU-only macOS ARM64 profile. It does not mark
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

## Recommended first profile

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

## Stronger alternative

Require cryptographic publisher signatures or a project-signed mirror for
every toolchain, SDK, registry index and archive before the first ready build.
That would require a signing/mirroring operation, key ownership, rotation and
recovery, and provenance for Apple system components. It would delay the first
native adapter and does not remove the need for exact compiler-read closure or
artifact publication. Choose this only if the first development build must
meet that stronger supply-chain guarantee.

**Decision needed:** whether the reviewed release input manifest and official
HTTPS origins are sufficient for the first native development profile, or
publisher/project signatures are mandatory from its first ready BuildRevision.

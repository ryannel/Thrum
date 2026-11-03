# Narrow macOS compiler metadata candidate

3 November 2026. **One local ARM64 A/B proof; `buildReady:false`.** This tests
the first isolation step in the [prepared native environment design](../../design/T2-PREPARED-NATIVE-ENVIRONMENT.md).
It does not select a trusted release, retain immutable tool roots or publish a
target build.

The opt-in `thrum-cook` fixture now builds its deny-default `sandbox-exec`
profile from the canonical materialized input, private Cargo home, Rust
toolchain, selected linker toolchain, SDK, fresh output and named system roots.
Additional metadata-only grants are limited to ancestors needed to traverse
those paths; the earlier selected data-read subtrees retain their read
permissions, including metadata. It selects the active
Xcode developer tree's `libxcrun.dylib` and, when installed, the Command Line
Tools copy as explicit file reads. An absent or aliased selected support file
refuses. These are path selections, not yet immutable byte identities.

The fixture runs an allowed `/private/etc/hosts` data and metadata control,
then requires an existing `/usr/share` or `/System/Applications` file to fail
both data and metadata reads, and `/usr/share` directory metadata to fail. It
also checks checkout and input-write denials.
The first narrowed run reached rustc but failed to execute the copied receipt
wrapper: its `/var/folders` spelling differed from the sandbox's canonical
`/private/var/folders` selection. The fixture now canonicalizes both input and
output before forming the Cargo command, wrapper path, target path and receipt
verifier arguments. This makes the selected path spelling consistent at the
compiler boundary.

With the pinned Rust 1.99.1 toolchain, local private-cache preparation and
receipt wrapper, the sandboxed A/B run passed. A admitted 54 files and the
fresh Cargo compile took 1,491 ms, then 1,341 ms after adding the directory
metadata probe; these are uncontrolled correctness runs. The verifier
matched five successful compiles and three probes; the selected Cargo-unit and
dep-info digests remained the known first-profile values. Invalid B failed,
created no executable and left A's binary hash and execution intact. The
source and private-cache inventories were unchanged. Reproduce from this
repository with the command in [the receipt proof](F4-RUSTC-RECEIPTS.md#qualification).
The test needs macOS permission to apply `sandbox-exec` and an installed
Xcode/Command Line Tools layout.

The remaining allowed system subtrees are broad, and neither the Xcode support
files nor host-runtime bytes are selected by a reviewed release manifest.
Cold hashes of mutable installed tool roots do not hold them unchanged during
a build. The first-profile compiler adapter still needs immutable retention,
complete selected read closure, bounded Cargo process/output handling and a
retained target manifest before a ready `BuildRevision` can exist.

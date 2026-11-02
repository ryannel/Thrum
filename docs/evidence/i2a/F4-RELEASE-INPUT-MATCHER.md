# Candidate first-profile release-input matcher

2 November 2026. The user selected the [reviewed exact-hash manifest
policy](../../design/T2-NATIVE-RELEASE-TRUST.md) for the first macOS ARM64
headless development build. This proof checks a caller-selected manifest
against local bytes. No actual release manifest has been reviewed or installed,
and no target-ready BuildRevision is claimed.

The [matcher](../../../scripts/verify_headless_release_inputs.py) accepts a
canonical, bounded version-one manifest selected outside game source and the
private Cargo home. Duplicate/unknown fields, noncanonical JSON, wrong target,
malformed digests and missing or changed adjacent origin evidence refuse. Small
manifest and origin files are opened through held parent-directory descriptors
with no-follow/nonblocking flags and checked for replacement around reads. The
format-three origin record names exact official HTTPS hosts and versions and
hash-binds adjacent canonical Cargo archive, index and extraction reports plus
Cargo, Rustc and host Rust standard-library component reports. The matcher
checks the Cargo reports against the selected lockfile, archive list, target
and private-root digest. The three Cargo format-two reports share a portable
`selectionSha256` over target, lockfile, private-root identity and exact
archive/index selections. A receipt from a different same-count selection
therefore refuses, even if the proof and origin-record hashes are consistently
updated. Hostname and receipt checking do not establish
acquisition history or attest that the reports ran in a controlled job. The
caller must be a trusted engine
release loader that selects a repository-reviewed manifest and retained
acquisition record.

Each format-three Rust component report now lists the installed payload files
it matched against its checksum-verified archive and the three installer
metadata files it read: the installed channel record, components list and
per-component manifest. The matcher requires component, target, one official
channel-manifest digest, sorted unique path/digest/size rows, consistent payload
totals and matching shared metadata witnesses. The installed channel-record
hash remains separate because rustup rewrites that file. It collects those
selected file identities during the same cold scan
that computes the full Rust tool-root digest, then requires an exact match.
This binds the receipts to the selected root without another multi-gigabyte
scan. It does not cover unlisted installed components or establish that a
reviewed release job produced the receipts.

A local warm-host Rust-root scan over the installed 1.99.1 toolchain selected
the 146 Cargo/Rustc/host-standard-library payload paths and five unique
installer-metadata paths from rustup's component
manifests. It read the same 394 files and 1,030,074,151 logical bytes as the
prior full inventory, reproduced root SHA-256
`00d56ad367ace4cdbae132ce52f66a0cb49ef7f7b432dc1fd493421b947ae368`,
returned 151 witnesses and took 0.413 seconds. This measures the one-pass
witness path on this cache state; it is not a release-preparation proof or a
latency guarantee for a cold host.

The matcher compares the admitted source's path-independent cache selection
key, lockfile hash, complete private cache digest and the descriptor-relative
Rust, linker and SDK root inventories to that manifest. Selected Cargo, Rustc
and linker files must reside within the respective roots. The local private
cache candidate separately binds the exact Cargo/Rustc paths and rehashes the
cache before reuse. The candidate format moved to version two so its portable
selection key cannot be mistaken for the earlier path-dependent key. A local
admitted A/B sandbox compile still passed with the new key and kept the same
2,923-file, 67,792,387-byte private root digest. Its A cache preparation took
1,480 ms, root reuse check 162 ms and compilation 1,566 ms in that run; these
are warm-host observations, not a build-time guarantee.

Ten small POSIX fixtures check the CLI and matching selection with code-only reuse;
changed dependency, Cargo manifest, origin record and SDK bytes; duplicate
or unknown manifest fields; wrong selected target; an internally inconsistent
lock hash; a Unicode installation path; a manifest-file symlink swap; and an
unofficial HTTPS origin even when its bytes are deliberately rehashed into the manifest.
The proof fixtures refuse changed proof bytes and correctly rehashed reports
naming a wrong private root, archive URL or same-count index selection. A
consistently rehashed Rust receipt with a wrong installed payload or metadata
digest also refuses at the selected root. These fixtures
use synthetic tiny roots rather than installed release inputs. The separate
[tool-root inventory proof](F4-TOOL-ROOT-INVENTORY.md) measured the installed
three-root cold scan at 11.0 seconds over about 4.86 GB logical bytes. This
matcher has not yet run against a reviewed full-size release manifest. The
selected compiler unit/read closure, broad system libraries, retained binary
and product manifest, atomic publication and reload remain unqualified.

A full-size **unreviewed** local match then copied the six workspace package
manifests and source trees plus root Cargo/lock/toolchain files from checkout
`ec7f366` into isolated system scratch, added the fixed headless declaration,
and prepared the private cache there. The first attempt under worktree scratch
correctly refused the checkout's ancestor `.cargo/config.toml`; moving the
owned source to system scratch matched the admitted build fixture's isolation.
The cache root was again 2,923 files, 67,792,387 logical bytes and SHA-256
`58c57419b000f0dbb2037792932f0191abecceae9e14e77e4b384fb06811225b`.
An ephemeral candidate manifest used the previously inventoried installed
Rust 1.99.1, Xcode 27.2 linker and macOS SDK root digests, with a synthetic
format-one origin record containing official URL/version labels. This was run
before the later proof-receipt requirements. The cold matcher read
those roots and returned `matchedInputRoots:true`, `buildReady:false`,
manifest SHA-256
`dd7dc25a0b39e054abd3ab903980ed432ec4e03ac0d03aa73aad7c95c699b3ab`
and selection key
`85aae01bd75251560ed2314627a79ebfa07e57162c1566f9c978b72abd7a96a5`
in 15.417 seconds. This is one cold host observation, including the private
cache rehash. The copied source was **not** admitted through F3, and the
ephemeral origin record was **not** a reviewed acquisition record. Bulk scratch
was removed after measurement. These limits prevent promoting the result to
release-input verification or a ready build.

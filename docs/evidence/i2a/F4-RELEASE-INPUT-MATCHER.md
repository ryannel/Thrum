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
origin record names exact official HTTPS hosts and versions; hostname checking
does not establish acquisition history. The caller must be a trusted engine
release loader that selects a repository-reviewed manifest and retained
acquisition record.

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

Five small POSIX fixtures check the CLI and matching selection with code-only reuse;
changed dependency, Cargo manifest, origin record and SDK bytes; duplicate
or unknown manifest fields; wrong selected target; an internally inconsistent
lock hash; a Unicode installation path; a manifest-file symlink swap; and an
unofficial HTTPS origin even when its bytes are deliberately rehashed into the manifest. They
use synthetic tiny roots rather than installed release inputs. The separate
[tool-root inventory proof](F4-TOOL-ROOT-INVENTORY.md) measured the installed
three-root cold scan at 11.0 seconds over about 4.86 GB logical bytes. This
matcher has not yet run against a reviewed full-size release manifest. The
selected compiler unit/read closure, broad system libraries, retained binary
and product manifest, atomic publication and reload remain unqualified.

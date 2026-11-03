# macOS compiler system-read grant reduction — 3 November 2026

The first CPU-only compiler sandbox no longer grants data reads across
`/usr/bin` or `/dev`. It retains exact device reads for `/dev/null` and
`/dev/urandom`; the selected output path and explicit `/dev/null` write
exception remain. The profile grants the exact observed LibreSSL config file
from the [prior reduction](T2-PREFERENCES-GRANT-REDUCTION-2026-11-03.md),
with neither broad `/private/etc` nor `/Library/Preferences` reads.

Before each compiler launch, the opt-in fixture requires failed data and
metadata probes against an existing `/usr/bin/uname`, failed data read of
`/dev/zero`, successful `/dev/urandom` byte read, and the earlier selected
tool/source/system positive and negative controls. Its test utilities still
execute under `allow process*`; that process permission does not grant their
arbitrary file data reads. The policy is generated from one selected tool
environment for both A and B.

Two separate macOS ARM64 qualifications passed:

| Compiler roots | A compiler result | Invalid B and retained A |
| --- | --- | --- |
| Installed Rust/Xcode | 5 compiles, 3 probes; 3,736 ms fresh-output stage | 3 dependency compiles, 1 failed game compile, 3 probes; A reloads and runs |
| Exact-inventory APFS clones | 5 compiles, 3 probes; 8,066 ms fresh-output stage | Same failed-B shape; A reloads and runs |

The clone run used local candidate root digests, not a reviewed release
manifest. Its owned scratch was inspected and removed afterward. Times are
uncontrolled observations and do not compare policy or clone performance.
The remaining broad host data-read roots include `/System/Library`,
`/System/Cryptexes` and `/usr/lib`. This proof does not trace all implicit
compiler/library reads, retain immutable host bytes, qualify a host-runtime
trust policy or publish a ready build.

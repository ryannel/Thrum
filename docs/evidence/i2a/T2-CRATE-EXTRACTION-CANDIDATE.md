# Private crate extraction byte candidate

2 November 2026. **Local input-preparation proof, not a reviewed release
manifest or ready build.** The [selected archive-origin comparison](T2-CRATES-ORIGIN-CANDIDATE.md)
and [sparse-index comparison](T2-CRATES-INDEX-ORIGIN-CANDIDATE.md) identify the
same 2,923-file private Cargo root by digest. This pass checks the extracted
source files inside that root against its locked `.crate` archives.

The [extraction comparator](../../../scripts/verify_crate_extraction.py)
checks the candidate root and lockfile, then streams each selected archive
through a bounded gzip reader and strict tar parser. It refuses archive paths
outside the package root, unsupported members, duplicate names, extra or
missing extracted files/directories, changed bytes and an unexpected Cargo
extraction marker. Complete private-root inventories before and after must
match. Four synthetic fixtures cover success, changed/extra extracted files,
an altered `.cargo-ok` marker and a nested empty directory with an explicit
package-root directory entry.

On an isolated copy of the earlier admitted-source fixture, a fresh locked
offline cache preparation again produced private-root SHA-256
`58c57419b000f0dbb2037792932f0191abecceae9e14e77e4b384fb06811225b`.
All 2,634 extracted regular files across 37 packages matched the archive
membership, length and SHA-256; their total unpacked byte count was
40,300,544. The local comparison took about 0.57 seconds including two full
private-root inventories. Its report says `buildReady:false`.

This links the local extracted source bytes to the same archive identities
checked against official HTTPS origins. It does not prove that Cargo's selected
build-unit graph or every compiler/system read is closed, that the release
preparation environment has been reviewed, or that a binary and cooked product
were retained under a verified target manifest.

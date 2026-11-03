# Selected crates.io archive origin candidate

2 November 2026. **Local release-preparation evidence, not a reviewed release
manifest or ready build.** The [first native release trust policy](../../design/T2-NATIVE-RELEASE-TRUST.md)
requires official HTTPS origins and exact hashes for the selected private Cargo
dependency root.

The [private cache preparer](../../../scripts/prepare_headless_cargo_home.py)
was rerun on an isolated copy of the earlier admitted-source fixture, outside
the checkout's ambient `.cargo` configuration. Its locked offline metadata
step produced a version-two candidate with 37 selected registry archives,
8,664,545 archive bytes, and the same 2,923-file private-root digest previously
observed:

`58c57419b000f0dbb2037792932f0191abecceae9e14e77e4b384fb06811225b`

The [archive origin comparator](../../../scripts/verify_crates_io_archive_origin.py)
first checks that candidate against the complete private-root inventory and
the fixture's `Cargo.lock`. For each of its 37 selected archives, it streams
the private bytes and then the corresponding
`https://static.crates.io/crates/<name>/<name>-<version>.crate` URL over HTTPS,
with redirects refused. Every private and fetched length/SHA-256 matched the
candidate and lockfile. A second complete private-root inventory matched after
the downloads. The comparator reports `buildReady:false` and a deterministic
list of the checked URLs and identities. Three synthetic tests cover a valid
comparison, changed official or private bytes, and a changed lockfile or
unsupported candidate selection. The real 37-archive run also exercised the
TLS and HTTP-status path; no redirect was observed.

The [Cargo registry-index reference](https://doc.rust-lang.org/cargo/reference/registry-index.html)
describes the registry's HTTPS archive download endpoint. This local run is an
origin candidate for the listed archives. It does not authenticate the copied
sparse-index records or extracted source tree, prove that the cache's selected
archive list is a complete compiler input graph, or review the release
preparation environment and manifest. The exact compiler/system read closure,
retained binary provenance and target-ready BuildRevision remain open.

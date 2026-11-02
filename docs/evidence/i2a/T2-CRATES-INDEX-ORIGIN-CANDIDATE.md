# Selected Cargo sparse-index origin candidate

2 November 2026. **Local release-preparation evidence, not a reviewed release
manifest or ready build.** This extends the [selected archive origin candidate](T2-CRATES-ORIGIN-CANDIDATE.md)
to the copied sparse-index records used by one private Cargo cache.

The [Cargo registry-index reference](https://doc.rust-lang.org/cargo/reference/registry-index.html)
defines the `index.crates.io` sparse endpoint and its JSON line records.
[Cargo's versioned cache source](https://doc.rust-lang.org/beta/nightly-rustc/src/cargo/sources/registry/index/cache.rs.html)
describes the version-three local cache layout used by the pinned Cargo 1.99.1.
The [index origin comparator](../../../scripts/verify_crates_io_index_origin.py)
refuses other cache layouts, validates each cached name/version pair, and
compares the original cached JSON line bytes with that version's official HTTPS
index line. It permits newly published upstream versions that the older local
cache does not contain; it does not claim the cached set is complete.

I reran the locked offline [private cache preparer](../../../scripts/prepare_headless_cargo_home.py)
on an isolated copy of the earlier admitted-source fixture. Its 2,923-file
private-root digest was again
`58c57419b000f0dbb2037792932f0191abecceae9e14e77e4b384fb06811225b`.
The comparator checked all 212 selected cached index names and matched 9,203
cached version lines with official HTTPS index responses, plus the exact
registry `config.json` bytes. Complete private-root inventories before and
after matched. Three synthetic tests cover a valid comparison, changed
official or private bytes, malformed cache identities and duplicate official
version refusal. The real run reports `buildReady:false`.

This establishes local origin evidence for the records present in that cache,
not completeness of upstream versions at a reviewed release instant. It does
not prove extracted-source bytes came from the matched archives or close the
selected compiler unit, environment, tool/system reads and binary publication.
The release preparation environment and exact input manifest still require
review before a ready BuildRevision can be claimed.

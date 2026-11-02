# Private Cargo cache feasibility for the first native builder

2 November 2026. **Local design experiment; no production adapter or ready
build.** The [first headless build design](../../design/F4-FIRST-HEADLESS-BUILD.md)
requires a closed dependency root. This experiment asked whether the bundled
host game needs the Mac's entire shared Cargo registry to compile offline.

The current workspace lock has 235 package records, including 229 registry
records and 212 distinct registry crate names. Host-filtered `cargo metadata`
listed 43 packages, 37 from the registry, because it still includes workspace
and development resolution outside the selected host compile. The selected
`cargo tree -p i1-mobile-game -e normal,build --target
aarch64-apple-darwin` showed `i1-mobile-game`, `thrum-core`,
`thrum-mobile-shell` and `hashbrown 0.17.1` with `inline-more`; its actual
compile output likewise built only `hashbrown` from the registry.

A private `CARGO_HOME` was assembled in ignored task scratch from the 212
corresponding sparse-index cache entries and the 37 host-filtered registry
archives. Each archive was rehashed and compared with its `Cargo.lock`
checksum before copy. The private cache used about 18 MB for index records,
8.3 MB for archives and 1.1 MB for extracted source after the build. The
archive byte sum was 8,664,545. The shared registry was about 920 MB. A
fresh separate Cargo target directory compiled the existing game successfully
with `--locked --offline` and this private home in 1.26 seconds on the local
macOS ARM64 host. A cache with only the selected `hashbrown` archive failed
resolution while requesting `foldhash`; a cache with only the 37 filtered
index entries failed while requesting `pollster`. This is why the first
profile cannot infer all resolver inputs from the selected compile tree alone.

A narrow-read macOS sandbox experiment then used the private cache and allowed
the checkout source, exact installed toolchain, required system roots and a
separate writable output tree. Cargo needed explicit root/ancestor metadata
reads and macOS developer-selection metadata to complete linking; with those
grants, the host build passed. This sandbox profile still permitted reads of
the whole shared `~/.cargo` and broad system roots, so the successful private
`CARGO_HOME` build and the sandboxed run do not together prove exclusive
private-cache reads. This was still a mutable-checkout experiment, not a
compile from an accepted pin, and the copied sparse-index records were not
independently authenticated. It does not prove arbitrary build-script,
proc-macro, target, toolchain or cross-platform closure. The next T2 adapter
must use a verified dependency-root manifest, controlled environment and a
materialized accepted source, and must retain precise artifact provenance.

The later [admitted-source compile qualification](F4-ADMITTED-HEADLESS-COMPILE.md)
used this private cache as the only permitted Cargo home within a narrowed
macOS sandbox. Its read grants excluded the shared `~/.cargo` data path, and
the test verified the private cache stayed unchanged. This closes the earlier
shared-cache-read ambiguity for that one A/B fixture; cache index provenance
and general selected build-unit closure remain open.

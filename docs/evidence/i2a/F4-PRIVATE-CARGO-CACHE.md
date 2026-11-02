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

The first reusable preparation step is
[`scripts/prepare_headless_cargo_home.py`](../../../scripts/prepare_headless_cargo_home.py).
It requires a new destination outside the materialized source tree, a host
registry cache and exact Cargo/Rustc paths. Python 3.11 `tomllib` decodes the
version-four lockfile. The tool copies all lockfile crate-name index records
and SHA-checks all 229 lockfile archives before invoking Cargo. It refuses
ambient `.cargo` config and replacement sources, resolves the workspace
offline only in the private home. On macOS both metadata passes run under a
deny-network sandbox with read data limited to the materialized source,
private cache, selected toolchain and named system roots. A local path
dependency outside the source pin failed with `Operation not permitted`
before its manifest could be read, and the partial private cache was removed.
The preparer removes archives outside the host-filtered
resolution, rechecks that resolution, removes Cargo's path-dependent
`.global-cache` usage database, and records a path/length/content-hash tree
digest in a separate candidate manifest. It cleans its newly created tree
on a reported failure. Cargo metadata output is drained through bounded
in-memory pipes with a 32 MiB JSON and 1 MiB diagnostic limit and a 120-second
timeout. The first five Python 3.11 fixtures prove checksum-fault cleanup,
source-config refusal, lock-version path-escape refusal, live output-limit
termination and refusal when a descendant keeps metadata pipes open. This
manifest is an observation of local cache bytes, not release authentication.

Repeated local preparations produced identical private roots after omitting
`.global-cache`: 2,923 files, 67,792,387 bytes, SHA-256
`58c57419b000f0dbb2037792932f0191abecceae9e14e77e4b384fb06811225b`.
The final all-lockfile-archive preparer took 1,058 ms in the admitted A/B
fixture; this is one warm host-cache observation. It copied 53,177,502
archive bytes before pruning to 37 host-filtered archives. The larger final
root than the earlier manual 27 MB cache is expected: private metadata
resolution extracts those archives. Physical peak disk use during preparation
was not measured. The opt-in A/B qualification prepared
this cache directly from A's accepted materialized source and compiled with
it in the sandbox. A first placement inside F3's `.engine`
directory made B's admission refuse its 64 MiB temporary planning target;
placing the cache inside the project root left an undeclared manifest. The
working fixture owns the cache beside the project, outside F3's inventory,
and removes it after both builds. This separation is part of the selected
builder layout, not a reason to weaken F3 admission checks.

The preparer now records a conservative normal/build dependency walk from the
headless package and refuses packages carrying custom-build or proc-macro
targets before compilation. Six Python fixtures cover this walk as well as
the earlier failure boundaries. The admitted A fixture's metadata walk
reports five possible packages: the game, `thrum-core`, `thrum-mobile-shell`,
`hashbrown` and `foldhash`. Cargo's actual build transcript reports five
artifacts from the first four packages, with no `foldhash` artifact. Metadata
has unioned a `hashbrown` feature used by other workspace targets. The
possible-package record is a conservative preflight, not a selected-unit
graph; no additional archive or extracted-source pruning follows from it.

The candidate manifest now includes the exact Cargo and Rustc executable
lengths and SHA-256 digests. The preparer hashes both files before metadata
and again after the two private resolutions; the admitted A/B qualification
compares the manifest with the selected files and rehashes them after B's
failed compile. On this macOS ARM64 host the executable digests were Cargo
`6d576d8058f8ca643e8fce55b64ee3c614ba64c9da2e536bae843e6a2e1f3951`
and Rustc
`1ba7bb8e273bd64125f4048d2ecce530e0a6470e565ab1b3aecf0b82b532f7b1`.
The latest private-cache preparation took 1,573 ms, including these new
streaming hashes. This observation does not bind Rustc's loaded libraries,
standard crates, system linker or SDK, so it is not a release toolchain root.

The pruned cache now carries a separate reuse key. It hashes the complete
admitted `Cargo.toml` path/content set, `Cargo.lock`, pinned toolchain and
headless build declarations, host target, fixed offline/locked build recipe,
and the selected Cargo/Rustc executable paths, lengths and hashes. This corrects a reuse
hole: an already locked crate can become a headless dependency after a manifest
edit without changing the lockfile, while the old pruned cache lacks its
archive. The verification command requires the caller's target and exact
Cargo/Rustc files, checks their current bytes, then checks the key before reuse for
B. It also refuses ambient Cargo configuration and implicit `build.rs` files
that could change execution without a manifest edit. Nine Python 3.11 checks
cover code-only reuse, manifest/target key changes, command-line refusal on
manifest edits, a wrong caller target or changed tool bytes, and new
config/build-script refusal alongside the prior
failure boundaries, including a SIGINT probe that confirms the metadata child
is reaped before candidate cleanup. The opt-in admitted A/B macOS sandbox qualification passed
with this guard: A preparation took 1,420 ms, A compile 1,049 ms, and the
private root remained 2,923 files with the same SHA-256 above. This verifies
candidate cache selection for one local profile; it does not authenticate the
registry index or close all compiler reads.
In a later paired local A/B run, the verifier took 42 and 44 ms respectively,
including Python startup and rehashing both selected executables. Cache
preparation took 1,386 ms; A compilation took 1,449 ms in that run. These
single warm-host timings are observations, not latency guarantees or a claim
that full dependency-root revalidation has been measured.

The verifier now streams the full private-cache tree again and compares its
path/length/content digest with the preparation manifest before each build.
A fixture changes one cached archive byte and confirms reuse refusal; code-only
source edits still reuse the unchanged tree. In the local admitted A/B sandbox
run this raised verifier time to 195 and 232 ms, including the tool hashes and
Python startup, over 2,923 files and 67,792,387 logical bytes. Preparation
took 1,408 ms and A compilation 1,011 ms. This catches candidate cache drift
before the compiler starts, but the local host cache's original index records
remain unauthenticated. A production adapter must use a verified immutable
root or race-resistant reads with the same integrity semantics.

The inventory now walks directories through no-follow file descriptors,
opens regular files with no-follow/nonblocking flags, checks file and directory
identities around reads, and sorts records by path components to preserve the
existing digest. A FIFO replacement refuses promptly; a controlled parent
directory swap to a symlink refuses before reading the outside tree. Eleven
Python fixtures pass. The private-root digest stayed unchanged, and the latest
paired A/B qualification measured 150 and 186 ms for the reuse checks. This
remains local candidate evidence, not a general hostile-filesystem or
release-authentication proof.

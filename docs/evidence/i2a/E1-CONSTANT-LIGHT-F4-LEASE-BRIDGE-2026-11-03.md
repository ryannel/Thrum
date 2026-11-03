# Provisional constant-light F3 → F4 lease bridge — 3 November 2026

The [selected F4 product](../../design/modules/ENVIRONMENT.md#selected-first-f4-constant-light-product-and-binding)
has an isolated internal bridge proof using the checkpointed provisional
`VerifiedSourceLease`, staged candidate and F3 typed decoder. The cook reads three
exact leased records, follows entry scene → schema-1 source → light identity,
normalizes/serializes 160 bytes, verifies local product bytes and binds exact
provisional SourceRevision and stable IDs. Mutable checkouts are poisoned before
B cooks; A survives B publication after its original stage/candidate are dropped.
This does **not** establish admitted-source authority, persistent F4 cache/product
publication, a ready build, renderer behavior, visual quality or device performance.

Three production boundaries remain. The proof-local `SelectionRecipe` derives
IDs/definition hashes from the F3 staged candidate before publication and checks
its exact revision against the lease; that checkpoint's lease exposes selection/per-ID
reads, without a verified-manifest/typed-selection accessor. F3 currently retains
its full environment numeric projection only under `cfg(test)`, so this module
runs inside an exact-source scratch crate copy through an appended test module.
A real F3→F4 API must retain/expose validated numeric values and stable selected
scene/source/light identities from one verified lease, sharing its manifest/index
once per build. Finally, `cached_product` proves local verification/reuse of A's
bytes under B's key; it is not a persistent cache lookup or product publication.
CookKey/product-manifest/binding encodings remain explicitly **proof-local**.

Ten selected Rust tests pass in debug and release: nine retained codec tests plus
the joined matrix. Its 17 actual leased closures cover ten cosmetic edits/content
changes with key/product reuse and a changed exact-source binding; six semantic
edits with distinct keys; a competing unselected source/non-entry scene; stale
selection/recipe/binding refusal; shared clone/final-reader pin release; failed new
candidates preserving selected A; and same-length stored-source corruption refusing
A while current B stays exact. A separate Python stdlib oracle independently
rehashes every retained staged file/manifest, follows the links and matches every
complete 160-byte payload, BlobHash, CookKey, product revision and binding digest.
Locked offline Clippy with warnings denied and rustfmt checks pass. Other crate
tests were compiled but filtered; repository integration checks remain separate.

One sequential release process per workload ran on MacBook Pro / Apple M3 Max /
36 GB / macOS 26.6.2 / Rust 1.99.1, `aarch64-apple-darwin`. “First” means first
application lease/decode after publication, with filesystem caches unflushed.
Warm acquisition/join has 31 samples; shared-lease join and retained-record decode
have 101. The selected light values and payload are identical across source-tree sizes.

| Local phase | 3 records / 5 files | 4,094 records / 4,096 files |
| --- | ---: | ---: |
| F3 staging, ms | 1.340 | 670.068 |
| F3 validation/revision/selection, ms | 0.134 | 79.591 |
| Provisional publication, ms | 0.853 | 207.907 |
| First manifest verification/index acquisition, ms | 0.135 | 10.003 |
| Warm acquisition median / p95 / max, ms | 0.109 / 0.153 / 0.170 | 9.955 / 10.338 / 10.641 |
| First three-read/decode/cook/hash/bind, µs | 67.333 | 108.792 |
| Warm join median / p95 / max, µs | 50.959 / 67.625 / 69.375 | 94.583 / 119.833 / 122.291 |
| Join sharing one lease/index median / p95 / max, µs | 46.875 / 52.417 / 63.542 | 45.625 / 46.958 / 188.083 |
| Decode three retained records median / p95 / max, µs | 2.709 / 2.792 / 3.334 | 2.542 / 2.625 / 3.167 |
| Rust child process high-water RSS, bytes | 9,945,088 | 25,362,432 |

The measured shared-lease path acquires one manifest/index and shares it through
lease clones; each cook performs three indexed record reads with no full-manifest
iteration. The separate warm-acquisition diagnostic deliberately reacquires it.
A future build orchestrator must preserve this sharing; this proof implements no
build orchestrator. Same-process standalone codec probes (500,000 iterations) measured cook 21.568/21.421, encode 10.926/10.858,
product decode 13.344/13.259 and 160-byte SHA-256 64.979/65.262 ns/op for the
small/large process respectively. These narrow function samples and the joined
measurements have different scopes; this is no improvement or engine-latency claim.

With A/B simultaneously retained, unique source bytes are 2,099/509,383; their
canonical source manifests total 2,394/1,810,622 bytes; leased selected-record Vec
capacities total 1,888 bytes; payloads total 320 bytes; product manifests/bindings
add 512 Rust layout bytes. RSS includes fixture creation, both stages, shared
indexes, SQLite, test harness and both revisions; it neither attributes the peak
to overlap nor provides allocation counts. Store chunk payloads total
4,493/2,320,005 bytes and store logical file lengths 646,656/10,476,976 bytes
at the overlap point.
These are local bounds for this fixture, not retained process budgets.

The real store refused a 4,096-record/4,098-file trial at its 4,096-file profile
limit; the final large workload respects that limit. `/usr/bin/time -l` could not
query the sandboxed clockrate sysctl, so the retained RSS comes from one direct
Rust child's macOS `getrusage`, excluding its Python parent. No OS-cold IO, sustained
edit latency, allocator counts or device qualification ran.

The [retained package](E1-CONSTANT-LIGHT-F4-LEASE-BRIDGE-PROOF-2026-11-03.tar.gz)
(205,297 bytes; SHA-256
`713c8df6a4c18c9c8f83ac0c6bb47142bf68ace5a75a8c71527d5e52e46874d8`) includes exact source hashes, original
codec, both oracles, fixtures, selected outputs, loaded-binary identity, all checked
commands and compact preliminary refusals. Repository HEAD was
`1c35b5300ba0714ce6346dc94d66bc7978441c25` plus the exact snapshot hashes in
`source-identity.json` (store source SHA-256
`f9355227dc36ce39b5b6ae6e56d14115fbc3ab41dc12e234f6cff8b06430cd44`);
no production files were changed by this proof. Independent review found and
corrected a stale scratch snapshot, then hardened replay to check the exact
`lib.rs` suffix and source file membership; negative mutation probes pass. The
selected tests, oracle and measurements were rerun against the committed
checkpoint. Reproduce after extraction
into owned scratch with `python3 run_proof.py --measure`. Package ID:
`thrum.e1.constant-light.f3-f4-lease-bridge-proof.v1`.

Later main commits introduced an accepted-source store and changed the source
API. This archive remains exact evidence for checkpoint `1c35b53`; it does not
exercise the later `NativeSourceStore`/`PinnedSource` path. The production F4
handoff requires a new proof against that accepted-source API.

All 269 retained files were compared byte-for-byte with the selected scratch
files before owned build/store cleanup; the archive carries its own inventory.
The extracted archive inventory, source-snapshot verifier and independent oracle
also passed. `RUSTC_WRAPPER= ./dev check-docs` passed after promotion. Only this
proof’s owned build/store/replay outputs were removed after retained-root verification.

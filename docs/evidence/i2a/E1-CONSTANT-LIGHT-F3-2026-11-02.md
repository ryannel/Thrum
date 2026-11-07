# Provisional F3 constant-light source closure — 2 November 2026

This is a bounded native staged-source proof for the first R0 environment
profile. It does not establish a cooked F4 product, an admitted E1 build,
renderer behavior, visual quality, device performance or the authored-rain
profile. The selected source and version contract is in
[FOUNDATION-SPEC](../../design/FOUNDATION-SPEC.md#first-engine-owned-environment-records--selected-next-f3-extension).

## Fixture and checked path

The tests in `crates/thrum-source/src/registry_stage.rs` construct a declared
project tree with a schema-2 entry scene, `thrum.environment.authored` schema 1
`constant-light-v1`, `thrum.light.identity` schema 1, and an existing `game.*`
record. F3 captures exact file bytes into immutable staging, rehashes them,
dispatches by checked type/version, applies direct bounded parsers, validates
the complete declared UUID namespace and typed scene→source→light links, then
derives a provisional manifest revision. Three SDK-owned built-in descriptors
carry fixed type/version semantics and domain-separated definition hashes.
The baseline scene and game-descriptor paths remain supported. Built-in-only
projects need no game descriptor; declared unselected environment content is
valid without activating a Sun.

The test matrix covers valid mixed and built-in-only closure; unknown engine
type, schema and profile refusal; wrong-type and unresolved targets; collision
against project, history, schema, scene, game and built-in IDs; the actual
65,536 required-edge boundary; exact-byte revision changes from known-field
and formatting edits; exact opaque value bytes in unchanged staged records;
and the prior staged reader remaining valid after a rejected new candidate.
Direct parser tests cover duplicate keys, documentation, finite and normalized
light quantities, the exact unsupported set, and opaque byte/depth/node/string
bounds. A non-entry scene binding remains declared inactive content while its
typed reference still resolves.

A separate test-only SQLite interaction carries a built-in-only candidate
through the existing root transaction twice, changing raw Sun illuminance.
After reopening it verifies both complete manifests and every listed blob,
compares both environment blobs byte-for-byte with their immutable staged
copies, and checks raw opaque JSON, current/historical idempotent retries and
token conflict. Invalid negative illuminance and wrong-type scene/source/light
links refuse before root admission; generation 2 and its outcomes remain.
This exercises the existing test store, not a production admitted-source API.
Another focused test pins admitted revision A, advances the root and mutable
checkout to B, drops the provisional candidates, reopens the store and reads
A's environment via the pinned manifest revision. It then changes one stored
blob chunk without changing its length: the verified read refuses the hash
mismatch, releases A's pin and leaves B exact and current. This proves the
test pin/read interaction, not a production lease lifecycle or collector.

## Reproduction and limits

Run `RUSTC_WRAPPER= ./dev check` from the repository root. On the local pinned
Rust 1.99.1 toolchain this passed repository checks, rustfmt, Clippy and the
workspace test suite; `thrum-source` reported **121 passed, 33 ignored**. The
independent code review found a whole-record discriminator traversal that
needed F3 depth/node bounds; the final proof includes its correction and a
production staged-path boundary test. Source identity: repository HEAD
`05184410ccdee3494c9558822f98306c4081f277` plus uncommitted SHA-256
`environment_json.rs=1ff7bee5f9d89fe4b3bd3fede1afe64fc66089b4dfc6076dec5b206ad162ddef`,
`registry_stage.rs=dce42689de21f6e4fdec479777d1b46da6fe750e27c64dddbcc39610f10e70c3`,
`game_json.rs=59d7b88fb59b6a6d55cdb377da4eabab5f9ce52e6264e34822d3c00bf4bb12b7`,
`sqlite_store.rs=cac40d674cefc132464d5b9d870cac4632f013b44c8336666680cda5ef157f7f`.
No end-to-end F3 source editor operation,
F4 cook/build binding, production environment root admission, process
memory or latency qualification ran. The opaque neighbor test edits a source
string and proves staged retention; it is not an editor round-trip proof.

The existing release-mode 4,096-record provisional-candidate probe was run
serially against an archive of HEAD and the modified source on the same Apple
M3 Max host. Three warmed old/new pairs measured **81.98/80.24**,
**77.84/78.26** and **76.32/78.30 ms** respectively from validation through
provisional revision, after staging. The medians were 77.84 and 78.26 ms;
these local runs do not establish a cost improvement or regression. The fixture
has 4,095 tiny scenes and one game record, so it does not measure the new
constant-light source at scale, combined peak memory or edit-to-ready latency.
The command was `RUSTC_WRAPPER= rustup run 1.99.1 cargo test --locked --offline
--release -p thrum-source measure_4096_record_candidate_revision -- --ignored
--nocapture`; the archived HEAD used an isolated Cargo target directory.

Schema 1's meaning is constant light only. The proposed rain source is schema
2 and remains refused here; fixture-only receiver schemas are also refused.
The later F4 product hash must include the selected built-in definition hashes
and typed dependencies before this source can drive a rendered game.

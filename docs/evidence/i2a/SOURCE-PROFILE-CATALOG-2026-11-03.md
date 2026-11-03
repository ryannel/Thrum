# First-source capacity catalogue and initial typed refusals — 3 November 2026

The first F3 content limits were spread across the inventory, manifest,
project/record parsers and accepted SQLite adapter. This checkpoint exposes a
read-only `thrum_source::first_source_profile()` slice whose entries take
their values from those enforcing owners. The query performs no allocation or
I/O and enters neither `SourceRevision` nor a cook/product key. Disk planning
targets, storage transaction allowances and F4 target support are separate.

## Distinct scopes and composition

The inventory permits at most 100,000 nonexcluded files **and directories**,
including empty directories. A canonical manifest separately permits 100,000
**files**, and the accepted adapter permits at most 64 MiB of encoded canonical
manifest bytes. One source file is bounded at 16 GiB; one editable JSON record
at 16 MiB. These limits can conflict: a tree may satisfy each individual file
bound but exceed encoded-manifest bytes, and directories consume inventory
capacity without becoming manifest file entries.

`core.project` schema 2 lists at most 4,096 scene/game/built-in records, 256 schema
descriptors and 4,096 entry-scene IDs, with entry IDs also bounded by listed
records. Project, history and descriptor files sit outside the 4,096 record
count, so the structural file subtotal can reach 4,354 before code and binary
masters. The candidate emits at most 65,536 required-reference summaries;
that is a count of emitted references, not necessarily distinct graph edges.
The catalogue also names generic JSON, schema, history, game opaque and
authored-environment extension bounds with their own units and scopes.

The current F4 `cook_entry_scene` supports a narrower target subset: exactly
one entry scene and schema-1 `core.scene`. F3 source admission must remain
independent of that cook support. An accepted source can still fail the target
build without changing its source identity.

## Refusal evidence

`InventoryError`, `ManifestError`, `StageError`, `StagedCandidateError` and
`SourceStoreError` now expose
`limit_exceeded()` where the boundary knows a specific dimension. Inventory
reports a lower bound from either declared files or early-stop traversal;
manifest errors retain exact observed counts or lengths. The accepted
canonical-manifest guard reports its exact encoded length and 64 MiB allowance.
The project parser reports a lower bound when records, schemas or entry-scene
IDs reach their first extra element, before decoding that element's value.
The staged candidate reports a lower bound when the processed required
references first exceed 65,536, including a built-in environment reference.
Other syntax/semantic failures remain their owning errors. The public
`ObservedQuantity::AtLeast` type represents early-stop observations; no
parser manufactures an exact count from its serde error text.
The typed detail is one quantity component of the C12 diagnostic envelope;
the caller must still supply owner, stage, selected profile, outcome and a
supported next action.
The generic strict JSON parser now returns the exact input byte length when
the 16 MiB record bound is exceeded, and a lower bound when it stops on the
first excess value or container level. Duplicate keys, malformed syntax and
trailing input remain syntax refusals without a capacity detail. Focused tests
cover array and object value counts, depth, bytes and non-capacity failures.
An exploratory optimized million-value dense-array parser probe took
9.42–9.71 ms in three runs before this detail and 9.50–10.03 ms in three
interleaved runs afterward on this host. The small difference is not a
representative throughput or tail qualification; the excess-token probe is
entered only after the bound is reached.
Direct `core.schema` and `core.registry-history` parsers also report exact
record bytes and the first excess descriptor, permanent-field or type-version
value token. The bound probe rejects an excess container without visiting its
children; malformed excess tokens remain syntax failures. Registry-stage,
lineage and accepted-store errors preserve the nested typed detail. Tests
cover all three arrays, their distinct dimensions, malformed extra values and
non-capacity syntax.
`RUSTC_WRAPPER= ./dev check` passed after design-hash refresh: repository
checks, 112 Python tests, workspace Clippy and Rust tests, including 197
passing `thrum-source` unit tests (46 opt-in tests ignored). The 64 MiB
accepted-manifest threshold is tested through its shared guard; a full public
admission fixture above that encoded size remains unrun.

The catalogue does **not** establish that all simultaneous maxima fit memory,
that every parser refusal is typed, that a wider profile works, or that a
consumer can use every F3-valid candidate. Schema symbol/text bounds and game
and environment parser limit propagation remain open,
as do structured disk estimate/observation receipts and full F3→F4 workload
qualification.

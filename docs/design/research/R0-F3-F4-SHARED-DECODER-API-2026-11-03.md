# R0 shared source decoder and projection API — candidate

3 November 2026. This is an implementor-facing interface candidate for the
[render-source grammar](R0-F3-RENDER-SOURCE-GRAMMAR-2026-11-03.md) and
[F3 work policy](F3-RENDER-IDENTITY-WORK-POLICY-2026-11-03.md). It does not
activate `core.scene@3` or `thrum.render.scene@1`. The current F3 registry
stage, F4 scene cook and `PinnedSource` API do not implement this interface.

## Shared structural preparation, then closure

`thrum-source` owns one versioned strict grammar and immutable built-in
descriptor for `thrum.render.scene@1`. F3 and F4 select different
projections from the same decoded meaning. Structural preparation must finish
for every declared record before F3 checks the complete UUID namespace;
a root-local decoder cannot reject a dangling or wrong-kind edge early and
thereby hide a later global duplicate. A candidate Rust shape is:

```rust
pub fn check_render_record(bytes: &[u8])
    -> Result<CheckedRenderRecord<'_>, RenderHeaderError>;

pub fn prepare_render_v1<P: RenderProjection>(
    checked: CheckedRenderRecord<'_>,
    work: &mut RenderDecodeWork,
    projection: &mut P,
) -> Result<PreparedRender, RenderDecodeError<P::Error>>;

pub fn check_render_edge(
    edge: &RenderEdge,
    namespace: &CompleteNamespace,
) -> Result<ResolvedRenderEdge, RenderClosureError>;

pub fn finish_render_v1(
    prepared: &[PreparedRender],
    checked: &CheckedEdgeReceipt,
) -> Result<RenderClosureReceipt, RenderClosureError>;

pub trait RenderProjection {
    type Error;
    fn emit(&mut self, event: RenderEvent<'_>) -> Result<(), Self::Error>;
}

pub enum RenderDecodeError<E> {
    Source(RenderSourceError),
    Pressure(ResourcePressure),
    Projection(E),
}
```

`CheckedRenderRecord<'_>` is an opaque borrow of the original bytes plus the
already checked `record_header` result. F3 dispatch constructs it while
classifying the record; F4 obtains it from a `thrum-source` wrapper after its
accepted-pin copy succeeds. Its private constructor prevents a caller from
pairing a header for one byte slice with another. Neither consumer repeats
the header walk inside `prepare_render_v1`. Exact Rust names can change; the
two phases, ownership and failure distinctions cannot. A private compiled
[envelope slice](../../evidence/render/R0-F3-RENDER-ENVELOPE-2026-11-03.md)
now proves the byte-bound token and fixed-count borrowed collection handoff;
it is not wired into dispatch or the remaining typed grammar. Structural
preparation checks the complete typed record
grammar and per-object numeric rules. `RenderEvent<'_>` has closed typed
variants for root/embedded identities, **edge declarations** and locally
checked camera/mesh/instance/material/texture/local-light objects. It does
not resolve an edge. UUIDs are canonical binary octets and numbers are finite
decoded f64 values. Mesh position/UV/triangle slices and decoded texture bytes
borrow one object-scoped scratch lifetime; a projection copies any data it
retains. `PreparedRender` temporarily owns root ID, descriptor, counts,
compact mesh extrema, texture formats, deferred instance TRS/mesh facts and
canonical locators, without a full geometry/texel body. F3's operation sinks
retain IDs and edges across all declared records for the global pass. A
`RenderClosureReceipt` is issued only after that pass and deferred closure.
**All events and prepared facts remain provisional until the complete
operation succeeds.** On source, pressure, projection, duplicate, edge or
deferred-predicate failure, drop every provisional projection and publish no
candidate/product.

Reuse the existing `environment_json::record_header` whole-record 16 MiB,
one-million-node and 64-depth **no-tree walk** at F3 dispatch. It already
checks the envelope and traverses nested fields without retaining a value
tree; adding a second shape-only prewalk would spend another linear parse
without a new invariant. The selected render decoder must consume raw object
slices through one strict typed grammar after that pass, and reconcile the
header's generic error with its own typed diagnostic/pressure boundary. F4
needs an equivalent checked entry when decoding the selected accepted pin;
the API should reuse the same pass rather than repeat it. Decode JSON escapes
before checking keys, UUIDs and enum values;
duplicate **decoded** keys refuse. Prefer a streaming collection visitor
with per-object scratch to an array-wide `Vec<&RawValue>` index; if the latter
is selected for simplicity, its peak and latency must be measured and
charged. No `serde_json::Value` tree, F4-only parser or retained F3 copy of
the mesh/texture body is permitted. Root/object field order is immaterial:
the fixture puts `fields` before root `id/type`, instances before meshes,
triangles before UVs and base64 before texture dimensions. Borrowed raw slices
may defer fields until prerequisites are known. For semantically invalid but
syntactically complete objects with valid IDs, select the first structural
error by canonical collection/UUID/field order, not callback encounter order;
the parser may retain only the smallest error locator while visiting all
object slices. An object with missing/invalid UUID uses its collection and
source array ordinal as a fallback locator; reordering such malformed
anonymous objects can change which structural error is first. Within one
object, field checks use fixed schema field order after decoded duplicate-key
checks. Syntax failure reports the parser's byte offset rather than a
reorder-invariant semantic locator. Exact per-vertex radius,
triangle and quaternion checks belong to shared structural preparation.
Within one collection, valid-ID object errors sort before anonymous object
errors; valid IDs sort by UUID, then schema field. For equal UUID and field,
borrowed raw object bytes break the tie before source ordinal, so swapping
distinct same-ID objects does not change the selected error. Anonymous
objects sort by source ordinal, then field. Formatting an already-invalid
same-ID object can change that final raw-byte tie; no semantic key is issued
for invalid source. A streaming scanner retains one lowest structural error
and continues checking later objects, but stops projecting valid objects
after the first structural error. Earlier projection output is provisional
and discarded if any error wins. Resource-pressure projection refusal may
stop early as validation incomplete, with no semantic source result.
F4 target-support refusal runs after source structural/global closure and
must not hide a later source error.
The private scanner's generic callback error is only a mechanism; its F3/F4
production wrapper must allow early abort solely for operational incomplete
or failure classes. Target Unsupported cannot be passed as that callback
error. Unknown-field diagnostic text also remains parser-owned scratch until
the winning error is selected; account or measure the rejected-object churn
before claiming a complete decoder memory tier.
Instance transformed-box checks need the referenced mesh's extrema and run
**after** typed edges in shared closure; texture-slot transfer compatibility
is also an edge check. F4-specific representation and target support follow
source validity and cannot redefine it.

## Consumer handoff

| Phase | Owned input and work | Output and release |
| --- | --- | --- |
| F3 declared validation | Authenticated provisional staged bytes for every declared render root, including unselected roots. An operation-scoped charged work account starts before inventory/capture. | The F3 projection emits identity and edge declarations to temporary charged sinks; compact mesh extrema/instance/texture facts remain only through closure. Retain top-level summary/counts after success. |
| F3 global closure | One complete namespace of project, history, schema, scene, game, built-in and embedded IDs, and all actual typed edges. | Complete structural parse, one deterministic collision pass, then typed/same-root/slot-format edges, then deferred transformed bounds. Release chunks and merge scratch after closure; no `EmbeddedIdentities` validity cap. Pressure leaves admission incomplete and same-token retryable. |
| F4 selected cook | One accepted `PinnedSource` and its authenticated manifest. Follow the selected entry scene's `render.source` and unchanged `environment.source` edges. `copy_file_to` can write a partial prefix, so decode only after its successful full length/hash return. | Run the same structural preparation and local closure on the selected root. The accepted F3 pin attests the complete tree; F4 rechecks selected local closure before feeding product projections. Exact build binding names admitted source and selected closure. Discard all partial work on failure. |
| V2/V3/V4 | Verified F4 product bytes, W1 committed/presentation sample, selected target and recipe. | Packet and representation certificates are per-view; unchanged exact CPU mesh facts and semantic products can be reused under their keys and leases. |

F3's complete-candidate order is: registry structure; all declared-record
structure by canonical path; one global duplicate-UUID pass; all typed,
same-root and slot-format edges by canonical source locator; then deferred
cross-object predicates by canonical locator. A missing mesh therefore
reports its edge error before a bound needing that mesh. Each phase reports
its first deterministic error. Structural parsing may append to temporary
sinks but never resolves edges based on parse order. F3's coordinator calls
`check_render_edge` while traversing the **operation-wide sorted edge stream**,
including non-render edges. Only after every edge succeeds does it issue an
unforgeable `CheckedEdgeReceipt` and call `finish_render_v1` over all prepared
render roots. F4 uses the same helpers on its one selected root, after its
accepted pin attests global source closure. No per-root bound may run while
another root's edge remains unchecked. `finish_render_v1` chooses the first
deferred error by canonical locator across its complete input, independent
of the supplied root order.

The current `validate_staged_registry` checks registry UUIDs in an early
`HashSet`, while `validate_staged_records` later sorts top-level IDs and
resolves type-specific references. Complete-candidate validation must replace
those split decisions with the ordered phases above. Define one canonical
path/object/field locator and total identity key `(UUID, path, kind, owner,
locator)` for top-level and embedded entries; use stable embedded UUID/field
locators instead of reorder-sensitive array ordinals where available. The
first equal-UUID pair by this key is reported independent of chunk size.
The exported registry-only validator's behavior change needs reconciliation
with callers/tests; retaining its early duplicate refusal in the complete
path preserves the precedence defect. The saved fixture has **16 graph edges
including the project entry-scene reference**, whereas current
`StagedRecords::required_edges` capacity accounting excludes that project
entry edge. Under the existing capacity unit, this fixture would emit **15
charged required-reference summaries**. Containment spends zero. A change
to that unit needs a separate profile/test migration.

Host `ValidationIncomplete(ResourcePressure)` is distinct from semantic
source refusal and filesystem/WAL `SourceStoreErrorKind::SpacePressure`. The
operation account crosses stage → validation → admission through explicit
ownership, release and transfer. A strict charged-byte promise needs a
known-layout precharge protocol for every allocation it covers; ordinary
`Vec::try_reserve_exact`, serde `String`/`Vec` growth or post-allocation
measurement cannot prove it. It does not bound process RSS or unrelated
F4/T2 memory unless they join the same owner. Pressure must not be stored as
a terminal validation rejection for `(epoch,key)`.

## Smallest implementation sequence and proof

1. Add the **inactive** shared structural decoder, closure code and immutable
   descriptor to `thrum-source`; test the [complete candidate
   fixture](../../evidence/render/R0-AUTHORED-FIXTURE-2026-11-03.md),
   escaped/reordered equivalents, canonical structural-error selection for
   valid IDs, exact numeric oracles and aborting
   projections with in-memory sinks. The [private transformed-box
   predicate](../../evidence/render/R0-F3-TRANSFORMED-BOUND-PRODUCTION-2026-11-03.md)
   is compiled but has no decoder caller yet. Keep admission dispatch disabled.
2. Integrate scene schema 3 and the render root into F3 staged parsing.
   Keep `core.scene@1/@2` semantics and existing constant-light descriptors
   intact. Add an F4 scene@3 environment projection before activation: the
   current constant-light selected-pin cooker accepts only scene@2. Replace
   split collision/edge checks, validate unselected roots,
   and prove deterministic path/field diagnostics under reorderings and
   three-way cross-chunk collisions.
3. Add the host operation account and distinct pressure propagation through
   capture, validation and admission. Inject failure at every charged phase;
   verify old-root readability and exact same-token retry. Measure charged
   high-water, RSS and edit-to-admitted latency separately with overlapping
   pins and T2/F4 work before selecting a host allowance.
4. Have F4 use the same decoder on a verified selected pin, cook/reopen
   products and bind exact source/build identity. Test A held while B edits,
   failed B preservation, formatting-only semantic reuse, target unsupported
   refusal and target-device mask/normal/shadow evidence. Only then activate
   an R0-ready source/build profile.

Source validation, F4 target support, build readiness and device quality are
separate results throughout this sequence.

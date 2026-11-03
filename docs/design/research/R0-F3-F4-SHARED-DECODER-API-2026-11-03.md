# R0 shared source decoder and projection API — candidate

3 November 2026. This is an implementor-facing interface candidate for the
[render-source grammar](R0-F3-RENDER-SOURCE-GRAMMAR-2026-11-03.md) and
[F3 work policy](F3-RENDER-IDENTITY-WORK-POLICY-2026-11-03.md). It does not
activate `core.scene@3` or `thrum.render.scene@1`. The current F3 registry
stage, F4 scene cook and `PinnedSource` API do not implement this interface.

## One decoder, object-scoped outputs

`thrum-source` owns one versioned strict decoder and immutable built-in
descriptor for `thrum.render.scene@1`. F3 and F4 select different
projections from the same decoded meaning. A candidate Rust shape is:

```rust
pub fn decode_render_v1<P: RenderProjection>(
    bytes: &[u8],
    work: &mut RenderDecodeWork,
    projection: &mut P,
) -> Result<RenderSummary, RenderDecodeError<P::Error>>;

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

The exact Rust names can change with implementation, but these ownership and
failure distinctions cannot. `RenderEvent<'_>` has closed typed variants for
the root, embedded identity, typed edge and checked camera/mesh/instance/
material/texture/local-light object. UUIDs are canonical binary octets;
numbers are finite decoded f64 values. Mesh position/UV/triangle slices and
decoded texture bytes borrow only one object-scoped scratch lifetime. A
projection must copy any data it retains. A completed `RenderSummary` owns
only root ID, descriptor identity, counts and checked closure metadata; it
does not retain geometry, texels or identity chunks. All events are
**provisional** until complete decode succeeds. On any source, pressure or
projection failure, the caller drops its partial projection and publishes no
candidate/product.

The parser performs the whole-record 16 MiB/one-million-node/64-depth bounded
no-tree prewalk and then consumes raw object slices through one strict typed
grammar. Decode JSON escapes before checking keys, UUIDs and enum values;
duplicate **decoded** keys refuse. Prefer a streaming collection visitor
with per-object scratch to an array-wide `Vec<&RawValue>` index; if the latter
is selected for simplicity, its peak and latency must be measured and
charged. No `serde_json::Value` tree, F4-only parser or retained F3 copy of
the mesh/texture body is permitted. The exact triangle, radius, quaternion
and transformed-box predicates are part of the shared decoder. F4-specific
representation and target support are separate after source validity.

## Consumer handoff

| Phase | Owned input and work | Output and release |
| --- | --- | --- |
| F3 declared validation | Authenticated provisional staged bytes for every declared render root, including unselected roots. An operation-scoped charged work account starts before inventory/capture. | The F3 projection emits identity and edge events to temporary charged sinks; checked mesh extrema remain only until instance bound closure. It retains top-level summary and checked counts. |
| F3 global closure | One complete namespace of project, history, schema, scene, game, built-in and embedded IDs, and all actual typed edges. | Complete structural parse, then one deterministic collision pass, then typed/same-root edge resolution. Release chunks and merge scratch after closure; no `EmbeddedIdentities` validity cap. A pressure result leaves admission incomplete and same-token retryable. |
| F4 selected cook | One accepted `PinnedSource` and its authenticated manifest. Follow the selected entry scene's `render.source` and unchanged `environment.source` edges. `copy_file_to` can write a partial prefix, so decode only after its successful full length/hash return. | One F4 projection may feed multiple immutable products from one checked decode. Exact build binding names the admitted source and selected closure; semantic product keys use relevant decoded meaning. Discard all partial work on copy/decode/cook failure. |
| V2/V3/V4 | Verified F4 product bytes, W1 committed/presentation sample, selected target and recipe. | Packet and representation certificates are per-view; unchanged exact CPU mesh facts and semantic products can be reused under their keys and leases. |

The current `validate_staged_registry` checks registry UUIDs in an early
`HashSet`, while `validate_staged_records` later sorts top-level IDs and
resolves type-specific references. The render extension must replace those
split collision decisions with one complete namespace pass. Preserve a
fixed error precedence: registry structure, declared-record structure,
global duplicate, then typed edge. Define one canonical path/object/field
locator for top-level and embedded entries and the same first-pair order
independent of chunk size. The exported registry-only validator's behavior
change must be explicitly reconciled with its callers/tests.

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

1. Add the standalone decoder/API and immutable descriptor to
   `thrum-source`; test it on the [complete candidate
   fixture](../../evidence/render/R0-AUTHORED-FIXTURE-2026-11-03.md) and
   independent exact numeric oracles. It can exist without admitting the
   new schema. Freeze the transformed-box exact predicate only after its
   width, all-fallback and repeated-instance cost proof.
2. Integrate scene schema 3 and the render root into F3 staged parsing.
   Keep `core.scene@1/@2` semantics and existing constant-light descriptors
   intact. Replace split collision/edge checks, validate unselected roots,
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

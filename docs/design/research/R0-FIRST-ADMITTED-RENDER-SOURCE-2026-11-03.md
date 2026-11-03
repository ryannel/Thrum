# First admitted R0 render source and certificate handoff — proposal

3 November 2026. **Proposed for cross-owner review.** This is the smallest
lasting source shape that can take the selected R0 opaque/masked, Sun-plus-four-
local-light fixture from one accepted F3 pin through authenticated F4 products
to a numerical certificate. It is not an adopted schema, a ready build or a
renderer. The governing contracts remain [Foundation](../FOUNDATION-SPEC.md),
[C08/C09](../CONTRACTS.md) and [R0](../RENDER-SPEC.md#8-r0-first-rendered-game-contract-and-proof).
The [flat-face source-sign oracle](../../evidence/render/R0-PLANAR-SOURCE-SIGN-ORACLE-2026-11-03.md)
and [raw-integer Metal diagnostic](../../evidence/render/R0-EXACT-REPRESENTED-DOT-METAL-2026-11-03.md)
motivate the split between source and representation evidence; neither is
source-to-shader conformance.

## Keep one authority per value

| Value | Authoritative source | Derived consumer |
| --- | --- | --- |
| Active environment and Sun identity | Entry `core.scene` `environment.source` and existing `thrum.environment.authored@1` → Sun-only `thrum.light.identity@1` | F4 constant-light product; one W1/V3 light registry |
| Four local lights | Stable IDs and physical descriptors in the new render source, owned by W1 | Same W1/V3 registry; no E1 driver and no renderer-owned rival registry |
| Camera pose/projection and authored fixed exposure | Render source camera | V2 view packet and V4 display metadata |
| Storage scale, target and display/shadow operator versions | Render preparation recipe | F4 product/ABI keys and V1/V4 preparation; no second exposure value |
| Texture transfer and texels | Typed texture content | F4 texture product; the one material sampler owns sampling policy |
| World geometry and material state | Render source data and one committed W1 endpoint | V2 packet and V3 shading; V1 only prepares/submits |

Do not change the fixed Sun meaning of `thrum.light.identity@1`. The current
Foundation sentence about an incoming E1 driver applies to that Sun identity,
not every possible W1 light. `core.scene@2` remains valid; a new version can
add a typed `render.source` required edge while preserving its
`environment.source` activation. The render root must **not** carry another
environment selector. A candidate form is `core.scene@3 →
thrum.render.scene@1`. This naming/versioning needs F3 and W1 agreement before
it becomes a built-in definition.

The first `thrum.render.scene` record is one strict, versioned root with
bounded typed nested camera, mesh, instance, material, texture and local-light
entries. Embedded stable IDs enter F3's complete declared UUID namespace and
typed edge checks; they are not private array indices. One root is enough
because these entries share the same first scene activation and ownership.
Separate public record types follow only when independent sharing, authoring
or lifecycle needs are demonstrated. First-profile geometry and small texture
masters can be inlined with exact decoded values; later external immutable
masters require an explicit typed file-reference form and revision. The source
format does not encode the three-quads fixture or its roughness/illuminance
values as universal schema constraints.

Source numbers mean exact represented values after the pinned finite-binary64
JSON decoder. The oracle's hexadecimal notation is evidence notation, not
source syntax. F3 validates the complete source graph, unknown fields,
required UUID/type edges, finite numbers, dimensions, indices, winding and
declared material/texture semantics. SourceRevision still hashes exact bytes;
semantic cook keys normalize negative zero and hash only relevant decoded
values plus built-in definition and evaluator versions. No second F4 parser or
mutable checkout read is allowed.

Normal-map and two-sided fields can have source meaning without a prepared
first-profile product. A required normal map, smooth shading normal, reflected
or animated transform returns typed unsupported until its basis, filtering,
source-error and shader path are proved. An absent map binds the neutral typed
texture but the flat-normal shader path must not evaluate a nonexistent
tangent basis. The geometric normal is derived from winding; an independently
authored flat-normal vector cannot override the source plane predicate.

## Cook and certificate identity

Use one accepted `PinnedSource` and one verified manifest/index per build.
From it, F4 resolves the selected entry scene and its environment once, then
cooks immutable mesh/vertex/index, material/default/texture, camera, local
light, existing 160-byte Sun/ambient/background, shader and pass products.
Each product records a domain-separated semantic cook key, actual byte hash,
typed dependencies and target/evaluator versions. An authenticated manifest
binds keys to bytes; a separate build binding names the exact admitted source
revision, selected scene and complete product set. A product hash alone is
not a ready `BuildRevision`. Reopen and verify product bytes independently
before exposure; a failed B leaves A's selected build and leases intact.

Two numerical evidence keys prevent needless recooking:

1. **Physical source predicate:** exact decoded geometry/winding, material
   sidedness/map mode, relevant physical transforms, camera pose and Sun/local
   source facts. For each flat face, use `N=(P1−P0)×(P2−P0)`, `d=N·P0`,
   `N·Sun`, `N·Camera−d`, and `N·LocalPosition−d`. This certificate is
   independent of a view-relative origin. `P0..P2` are physical world-space
   positions after the first profile's identity transform; source mesh-local
   positions cannot be substituted after a later transform. It says which
   real source side is positive; it does not provide shader cosine magnitude.
2. **Representation and range:** the physical certificate plus actual F4
   product bytes, normalization/packing versions, chosen origin/projection,
   interpolation and shader-expression/error-model revisions. It bounds the
   source-to-represented operands, sign/magnitude error, half-vector, local
   distances, material/texture extrema, every dangerous shader intermediate
   and half-float storage. An origin or target change invalidates this second
   certificate without recomputing unchanged exact physical predicates.

The representation certificate may be prepared per view for this small static
fixture. It cannot be assumed cheap for a 250k-triangle scene or arbitrary
camera motion. A value whose interval crosses a hemisphere, emitter-radius,
normalization or storage precondition returns typed unsupported before the
dangerous operation. The exact represented-dot GPU diagnostic cannot recover
opposite source signs that were narrowed to identical inputs, and its sign
result alone cannot justify a Smith ratio. A general mapped-normal or grazing
path needs a separate, measured evaluator.
F4 can reuse a physical predicate only while the source geometry, physical
pose, material normal mode and camera/light facts named by that key remain
unchanged. V2 must select or refresh view-dependent representation evidence
for each committed camera/world endpoint and origin; a stale certificate
cannot follow a moved pose merely because the source revision is unchanged.

## First authored proof content, not a source-format limit

A candidate six-triangle content set uses a rear +Z opaque receiver at `z=0`,
a masked fence at `z=1` and an offscreen opaque caster at `z=2`. Place the
camera at `(0,0,5)` looking −Z with a 60° vertical perspective, 16:9,
near/far `0.1/20` and 1280×720 output. Use receiver-to-source Sun
`(0.6,0,0.8)` at 100,000 lux, four 1,000-cd point lights at
`(±1,±1,3)` with `0.1 m` radius, 1,000-lux ambient, black background,
storage scale `1/1024`, fixed exposure `1/10000`, bounded emission and actual
roughness at least 0.32. Material cards should vary dielectric/metal,
base/ORM/emission while the mask has exact cutoff 0.5. The offscreen caster
must demonstrably shadow a visible receiver region; source coordinates,
shadow receiver volume/bias and exact texels are still to be pinned.

These values are **provisional fixture inputs**, not engine-wide lux,
roughness, resolution or disk/memory limits. The public 0.045 minimum
perceptual roughness remains supported by the material contract. A separate
low-illuminance variant should exercise it. Preparation must derive actual
receiver-to-local distance minima, cooked material/light extrema and
pre-exposure/half ranges from the saved source, including packing error;
`z` separation alone is not a complete shader proof. The current
[arithmetic study](R0-ARITHMETIC-ENVELOPE-2026-11-03.md) supplies an
illustrative real bound, not these actual cooked extrema.

## Cost and rejection-oriented proof order

The first small source has at most six triangles and five direct lights, so
the basic flat predicate pass is at most 36 triangle × light/view checks
before sharing. Texture extrema are computed once per changed texture, and
one source pin/index is shared across products. Record decoded/product and
certificate bytes, arbitrary-precision scratch, old/new overlap and edit
latency. At 1280×720, one full-screen shaded layer under five direct lights
is 4.608 million fragment/light pairs. It is not an upper bound when
overlapping surfaces, masked coverage or backend depth behavior cause more
fragment shading; measure actual shaded-fragment counts and overdraw. The
six-triangle CPU preparation cost says nothing about that GPU multiplier. Do
not turn the isolated integer compute timing into a fragment fallback budget.

1. Resolve the new scene binding and local-light authority with F3, W1, E1,
   F4, V2/V3 and V1/V4 owners; pin exact field meanings and scope without
   rewriting existing built-in hashes in place.
2. Prove strict typed F3 parse/closure on a complete A source, including
   opaque and masked content. Reject duplicate/unused UUID collisions,
   wrong-type edges, rival Sun selectors and malformed geometry/texture or
   map references. Keep scene@1/@2 meanings intact. A well-formed normal map
   remains valid source even when the first render preparation cannot use it.
3. Cook from one accepted pin, reload exact product bytes and a complete
   binding, then edit checkout to B. Prove A remains readable/runnable;
   relevant edits invalidate only dependent products, while formatting-only
   edits change source binding but can reuse semantic products. F4 refuses a
   present normal map, smooth normal or other unsupported first-profile
   requirement before publishing a ready product.
4. Prove physical and representation certificates against independent exact
   and interval oracles. Kill stale camera/light/origin/packing/normal-mode
   mutants. Refuse inside-radius, near-zero length, ambiguous sign/magnitude,
   overflowing intermediate, unsupported half packing and mixed-product
   revisions before publication.
5. Only then bind the actual WGSL/ABI and source products to V1–V4. Check
   mask/depth/shadow/forward agreement, BRDF and colour cards, offscreen
   caster, moving native-resolution output, held two-credit/resize/device-loss
   lifetimes, and sustained whole-frame CPU/GPU/physical-memory cost.

The first decision is source shape and local-light authority. Numerical
certificate algorithms, exact content bytes, operator-level bounds and
the target shader remain open; this proposal is not a developer-ready
implementation instruction.

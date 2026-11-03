# R0 render-source grammar and F3/F4 handoff — candidate

3 November 2026. This candidate refines the [first admitted render-source
handoff](R0-FIRST-ADMITTED-RENDER-SOURCE-2026-11-03.md). It is a proposed
built-in source meaning, not an implemented schema or a ready render product.
F3, W1, F4, V2/V3 and V4 must agree on these exact meanings before assigning
definition hashes. Existing `core.scene@1/@2` and the constant-light source
retain their current meanings.

## Envelope, identity and activation

The selected first form is a `core.scene@3` file with its existing optional
`scene.label` and `environment.source` fields plus required `render.source`.
`render.source` is a required typed reference to exactly one declared
`thrum.render.scene@1` record. It does not select an environment. The entry
scene's `environment.source` remains the only active environment selector;
the first R0 profile requires exactly one entry scene, a supported constant
light source and a render source. Unselected scene and render records remain
valid declared content whose required references still close. F4 follows the
selected scene edge from one admitted pin, never searches the checkout or
chooses a render root by filename.

The render record has the normal strict source envelope (`id`, `type`,
`schema`, `fields`, optional bounded `documentation`). Its fields are
`cameras`, `active_camera`, `meshes`, `instances`, `materials`, `textures` and
`local_lights`. Arrays have canonical UUID identity independent of order;
order only has the explicitly stated meaning for mesh vertices/triangles and
texture bytes. Each embedded object has one canonical RFC 4122 UUID `id`.
Semantic cook keys and deterministic product enumeration sort identity-keyed
entries by UUID; reordering those JSON arrays alone cannot change their
meaning or force a different product. Mesh vertex, triangle and texture-byte
order remain inputs to their respective products.
The active camera, instance mesh/material, and material texture slots are
typed UUID edges. The selected first profile requires one active perspective
camera and exactly four point local lights; those counts are a **recipe
requirement**, not generic `thrum.render.scene@1` syntax.
Every schema-1 embedded edge targets an identity in its owning render root.
Cross-root sharing needs a later explicit source contract; F4 can therefore
decode one selected root without searching unrelated render records.

F3 validates every embedded ID against the complete project UUID namespace,
including top-level records, project/history/schema IDs and other embedded
source identities. It checks typed edges even in unselected render records.
The existing registry stage currently indexes only top-level IDs and exposes
only the environment-to-Sun built-in edge; implementation needs a separate
bounded typed embedded summary and edge resolver. Embedded entries are not
separate files and do not enter the accepted manifest's file record-identity
column. Aggregate required-edge accounting includes their references.
The common 16 MiB record-byte, 1,000,000 JSON-value-node and 64-level limits
apply before typed allocation. No new arbitrary mesh, light or texture-count
limit is introduced solely for this schema; the first render recipe declares
its own finite preparation capacity and refuses over-capacity content before
publication. Parser peak memory and adversarial near-limit latency still need
measurement.
Unreferenced embedded IDs can outnumber required edges across many legal
records, but the first design does **not** add a fixed `EmbeddedIdentities`
source-capacity dimension. The [candidate F3 identity-work
policy](F3-RENDER-IDENTITY-WORK-POLICY-2026-11-03.md) uses a selected
operational allowance for charged live work in the native host, with a
separate retryable pressure result when validation cannot finish. Source
meaning and source validity remain independent of that host allowance.
Containment does not spend a `RequiredReferences` edge. The account,
complete allocation audit and combined stage/pin measurements must be
proved before this schema becomes an active admission profile; a bounded
external sort is a separately owned future path.
The [isolated embedded-index microprobe](../../evidence/render/R0-F3-EMBEDDED-INDEX-PROBE-2026-11-03.md)
shows this dimension is independent: four bounded source-shaped roots can
carry 332,016 embedded IDs with only four edges. Its generic JSON parser
dominates peak memory, so it does not yet justify a numeric capacity.
An [isolated strict typed-decoder prototype](../../evidence/render/R0-F3-TYPED-DECODER-PROTOTYPE-2026-11-03.md)
reduces that fixture's measured peak by releasing per-object scratch and
preserves escaped-string semantics, but it is outside F3 and does not measure
combined stage/pin work or select a host work allowance.

## Field meanings

The exact spellings below are candidate field IDs. Before implementation, the
same names must be frozen in the built-in descriptor and exercised by a
complete admitted-source fixture. Unknown keys, duplicate keys, wrong enum
values, unresolved/wrong-type references and nonfinite numbers refuse source
admission. An absent optional map means the typed neutral binding; a present
but unresolved map is never an absence.

| Embedded kind | Required content and source meaning |
| --- | --- |
| Camera | `id`, authored initial `position_m` f64[3], `orientation_xyzw` f64[4], `vertical_fov_rad` f64, `near_m`/`far_m` f64 and positive dimensionless `exposure`. Right-handed Y-up, camera forward −Z. The perspective angle is strictly between 0 and π and `0 < near < far`. The current viewport determines aspect at extraction, so resize does not edit source. |
| Mesh | `id`, ordered f64 `positions_m` triples, matching ordered f64 `uv0` pairs, and ordered u32 `triangles` index triples. Indices refer only to this mesh. Index order defines front-facing winding in the right-handed world; either winding is valid. UV `(0,0)` addresses the top-left image edge. |
| Instance | `id`, required `mesh` and `material` references, authored initial `translation_m` f64[3], `orientation_xyzw` f64[4], `scale` f64[3] and `two_sided` material policy through the material reference. W1 owns its committed runtime transform. The first R0 product supports identity static instance transform only; valid other TRS receives F4 `Unsupported` until transformed geometry, parity and normal certificates are proved. |
| Material | Required `id`, `alpha_mode` (`opaque` or `mask`), `two_sided` boolean and one `sampler`; `alpha_cutoff` is required for mask and forbidden for opaque. Optional `base_factor_rgba` linear dimensionless f64[4], `emissive_radiance_factor_cd_m2` linear Rec.709 f64[3], `metallic_factor`, `roughness_factor` and `occlusion_strength` take the native factor defaults and composition in RENDER-SPEC §2. Optional typed `base_texture`, `orm_texture`, `emissive_texture` references share `uv0` and that sampler. Base and alpha factors, metallic, roughness, occlusion strength and cutoff lie in `[0,1]`; emissive factor is nonnegative. Schema 1 has no authored normal-map or smooth-normal field; F4 binds the flat-normal ABI default and native normal scale `1`. |
| Texture | `id`, positive integer `width` and `height`, `format` (`rgba8-srgb` for base/emissive or `rgba8-linear` for data), and canonical padded RFC 4648 base64 `rgba8` bytes. Exactly `4×width×height` decoded bytes are present in upper-left-origin row-major order, with no row padding or mip levels. Validate checked dimension multiplication and encoded/decoded length before allocation. |
| Point local light | `id`, authored initial `position_m` f64[3], nonnegative `luminous_intensity_cd` f64, nonnegative f64[3] `unit_luminance_linear_rec709` with `|Y(c)-1|≤1e-9`, and nonnegative `radius_m` f64. W1 owns its identity and committed position; R0 supports a static light at its authored position, and V3 applies inverse-square illumination outside the declared emitter radius. |

The one material sampler has explicit `min_filter`/`mag_filter` values
`nearest|linear`, `address_u`/`address_v` values
`clamp-to-edge|repeat|mirror-repeat`, and no mip choice in this one-level
source. Sample coordinates use normalized edge coordinates: `(0,0)` is the
top-left edge and the center of texel `(i,j)` is
`((i+1/2)/width,(j+1/2)/height)`. F4 makes the same sampler/texture product
available to depth, shadow and forward roles; V3 evaluates the same UV,
decoded alpha and cutoff in all three. sRGB RGB channels decode to linear
before filtering; alpha and data maps filter linearly in their stored unit
interval. A texture reference's slot determines its permitted transfer
format. A material's single sampler and `uv0` policy avoid hidden per-slot
differences; imported content with incompatible independent samplers/UV sets
must be converted losslessly or explicitly refused.
For the first R0 masked product, F4 requires equal `min_filter` and
`mag_filter`, so pass-specific derivatives cannot select different filtering
kernels. A valid source with unequal filters is F4 `Unsupported` for a masked
material until an explicit shared alpha-sampling path is proved. Equal filters
are necessary, not proof of bit-identical target samples: WebGPU leaves
precise filtering behavior implementation-dependent. Qualify cutoff-boundary
coverage on each target; a target that cannot meet the shared-alpha contract
needs an explicit common sampling evaluator or a typed refusal. The ordinary
texture path must also prove linear-light filtering of sRGB RGB, or use an
explicit decode/filter product with the same source meaning.
[WebGPU sampler behavior](https://gpuweb.github.io/gpuweb/#samplers),
[glTF texture-coordinate origin](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#images).

The source camera and local-light poses are *authored initial values*, not a
second runtime authority. W1 owns committed transforms. V2 derives the
camera and instance presentation poses from one coherent W1 endpoint and
interpolation sample; V3 uses the corresponding light poses. An origin shift
changes representation evidence, not physical source signs. The first R0
profile admits only static lights/instances, while the camera may move. Its
actual presented camera pose, not just the authored initial pose, keys the
view-dependent predicate certificate. Source projection/exposure stay authored;
viewport aspect and view-relative origin are derived per packet.

## Numeric validation and refusal boundary

JSON numbers decode through F3's pinned finite-binary64 decoder. Their exact
decoded values, including subnormal values, are source meaning; semantic
keys normalize negative zero, whereas `SourceRevision` still hashes exact
file bytes. Decode JSON string escapes before validating UUIDs, enum values
and field names; equivalent escaped/plain spellings have the same semantic
meaning, while duplicate decoded keys refuse. F3 checks dimensions,
domains, canonical UUIDs, typed closure and
the Foundation v1 authoring extent. For a mesh triangle, require three
distinct indices and at least one nonzero component of the **exact dyadic**
cross product `(P1−P0)×(P2−P0)` formed from decoded f64 vertices. An ordinary
rounded f64 cross product alone may classify a real but near-degenerate
triangle as zero. Its order determines winding; there is no arbitrary
minimum source area. F4 can return typed `Unsupported` when a structurally
and exactly valid triangle cannot fit the first profile's representation,
normal, shadow or shading error envelope. F3 source validity does not promise
that a target device can draw it.

F3 validates camera and instance quaternions from their exact decoded-f64
dyadic components. Let `Q=x²+y²+z²+w²` and let `ε=1/10⁹` be an exact
rational. Require `(1−ε)²≤Q≤(1+ε)²`; this also excludes zero. This rule
does not inherit the host-dependent binary64 `hypot` threshold of the
existing constant-light source. The source rotation is the exact proper
rotation represented by `q/√Q`; its matrix uses the standard quadratic
quaternion numerator divided by `Q`, so F3's bound check needs no square
root. Quaternion sign and nonzero scalar-equivalent encodings yield the same
source rotation. For right-handed column vectors in XYZW order, the rows of
the numerator `N(q)` are
`(w²+x²−y²−z², 2(xy−wz), 2(xz+wy))`,
`(2(xy+wz), w²−x²+y²−z², 2(yz−wx))` and
`(2(xz−wy), 2(yz+wx), w²−x²−y²+z²)`; `R=N/Q`.
F4 normalizes the authored initial quaternion once with a
versioned finite-precision evaluator, canonicalizes sign (`w>0`, or at
`w=0`, first nonzero x/y/z positive) and supplies the W1 initial pose.
The F4 representation certificate must bound its deviation from the exact
source rotation; an evaluator change revises the product key. The first R0
instance product accepts only zero translation,
unit scale, and a normalized quaternion with exactly zero x/y/z and positive
w; an authored negative zero is equivalent to zero. It does not substitute
an approximate identity transform. Static zero scale may remain valid
authored content under Foundation, but the first R0 product refuses singular
and nonidentity transforms. F4 also refuses animated/reflected instances,
unsupported two-sided shading or any source it cannot certify before
publishing a ready binding. An attempted normal-map or smooth-normal source
field is an F3 unknown-field refusal in schema 1; adding its tangent-space
and interpolation meaning requires a new source schema. No substitute
normal or omitted map is silently used.

The Foundation v1 world-coordinate extent of ±`10^6` m applies to initial
camera/light/instance positions and transformed bounds; each mesh vertex's
local radius measured from its local origin has exact decoded-f64 squared
norm at most `10^6` m². A rounded binary64 `hypot` may accept a point just
outside that bound; use a certified fast comparison with exact fallback.
For this schema, form each mesh's tight closed local axis-aligned admission
box from componentwise extrema of **all** decoded positions, including
unreferenced vertices. An empty mesh has empty `uv0` and `triangles` and uses
the local origin point as its admission box. An instance's transformed
admission bound is the entire affine image
`{t + R(q) diag(scale) p : p∈local box}`. Every coordinate of that bound
must remain in the closed `[-10^6,10^6]` m world interval; equality admits.
This is a conservative **box** source rule: a rotated empty corner may be
outside the world extent even when every actual transformed mesh vertex is
inside. It is not a claim that the rejected vertex geometry crossed the
boundary. For example, local vertices `(1000,0,0)`, `(0,1000,0)` and
`(-1000,0,0)`, an approximate Z rotation encoded by
`q=(0,0,0.6,0.8)` and `t_y=999000` place actual vertices below the upper
world bound, while the local-box corner `(1000,1000,0)` transforms above it.
The authored box rule rejects that instance deliberately.

F3 retains six f64 extrema per mesh while checking instances; for each
world axis, the signs of the exact `R_ij·scale_j` coefficients select the
local extrema for its lower and upper support. Compare the resulting exact
rational bounds against ±`10^6` using a certified fast interval and exact
fallback. An independent eight-corner rational oracle must agree, including
at equality, one decoded step across the boundary, negative/zero scale and
quaternion sign changes. This is `O(mesh vertices + instances)` for source
structure before the bounded exact-arithmetic cost, rather than rescanning
every vertex per instance. The triangle predicate's 36-limb width cannot be
reused without a separate bound proof: quaternion products, unrestricted
finite scales and subnormals have a wider exponent span. Measure worst-case
all-fallback instances and combined F3 memory before activation. These are
existing Foundation-envelope constraints with a schema-local bound shape,
not new R0 fixture limits. A valid source can still be unrepresentable in
the first R0 device profile.

The exact-dyadic triangle rule needs a bounded adaptive implementation: a
proved fast filter may accept clear cases, with a fixed-width/exact fallback
for cancellation. Qualify worst-case all-fallback 16 MiB records, parser peak,
and whether the representation certificate can retain exact source vertices
without duplicating the full JSON tree. “No second F4 parser” means one
versioned typed decoder reused on authenticated pinned bytes, not retaining
every decoded mesh in F3 memory until cook. F3 retains compact checked
identity/edge summaries; F4 reopens authenticated bytes and uses that same
decoder. Product keys include the built-in definition hash,
material evaluator revision and relevant exact values. Formatting-only edits
change the exact source binding but may reuse semantic products. Target or
view changes can invalidate a representation certificate without recooking
immutable geometry.
An [isolated exact-triangle probe](../../evidence/render/R0-F3-EXACT-TRIANGLE-PROBE-2026-11-03.md)
now supplies a 36-limb width argument under the Foundation local extent,
independent rational signs for 8,009 cases, and a predicate-only 39,000-case
all-fallback timing. It does not replace the actual typed parser, combined
peak measurement or target/compiler verification of the interval filter.

The minimal F3 extension is a `StagedScene.render_source` typed edge and a
`StagedRender` top-level path/ID/definition summary that emits compact
embedded identities and typed edges to account-charged sinks. Parse every
declared root, including unselected ones. A global sorted/merged UUID view
includes project/history/schema IDs, top-level records and emitted embedded
IDs; reject duplicates and resolve type plus same-root ownership for every
render edge. Check the existing required-edge capacity before appending;
failure to reserve identity work is a retryable incomplete validation, not a
new source-dimension refusal. Source errors name source path/ID, field and
target; pressure receipts name the owner, stage and charged bytes. Only the
top-level render record enters the accepted manifest's record identity. The
decoder may project into an F3 summary sink and release geometry/texel
scratch per object; F4 can project the same grammar into cook inputs from
verified pinned bytes. Measure both peak forms, including simultaneous
old/new pins, before selecting the host profile.

## Required implementation proof

1. Admit one complete `core.scene@3` source with Sun-only environment and
   opaque/masked render content. Keep `@1/@2` fixtures valid. Reject wrong
   typed refs, collisions across top-level and embedded IDs, unselected
   dangling edges, duplicate keys and invalid base64/size arithmetic.
2. Use independent exact-dyadic and f64 decoding oracles for ordinary,
   near-degenerate and all-fallback triangles; test both windings and extrema
   of the v1 world envelope. Check exact quaternion norm and transformed
   local-box support against an independent eight-corner rational oracle,
   including conservative empty-corner rejection, all-fallback work and
   shared-mesh many-instance cost. Record admission latency and peak resident
   bytes.
3. Cook all products from the same accepted pin; verify reopened product bytes
   and exact build binding. Hold A while editing/admitting B and prove no
   mixed source/product revision. Formatting-only B may reuse semantic bytes.
4. Refuse valid-but-unsupported transform, light-radius or numeric
   envelope cases at F4 before publication. Validate mask, sampler and alpha
   agreement across depth/shadow/forward on the actual target.
5. Run moving camera and resize through W1→V2→V3/V4 with coherent presentation
   pose, viewport-derived aspect, source-sign and representation evidence.
   Prove a stale pose/origin/material/product mutant refuses. Measure complete
   frame, overdraw and memory high-water with the named device.

The immediate next decision is the minimal typed F3 summary/sink API and a
measured exact-predicate implementation. Until those are pinned and implemented,
this is a reviewable contract candidate, not a source profile that tooling
should accept.

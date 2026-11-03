# Renderer implementor dogfood — 2 November 2026

## Result and authority

The current canonical design is sufficient to choose R0's lighting/material law, shader language,
identity/readiness and ownership. It is not yet an executable rendering interface. An implementer
still has to specify view reconstruction data, shadow allocation/filtering, concrete display policy,
pipeline descriptors, transparent storage/overflow and R3's optical support. This walkthrough supplies
one proposed bounded implementation model for those choices and separates unresolved wider requirements.

This document was written by attempting to build the saved design on paper, following the repository
engine-module-design skill. It reads the **current working** [CONTRACTS](../CONTRACTS.md),
[RENDER-SPEC](../RENDER-SPEC.md), [BASELINE-SPEC](../BASELINE-SPEC.md),
[composition](../COMPOSITION.md), [water optics](../modules/WATER-OPTICS.md),
[render research](RENDER-RESEARCH-2026-11-02.md) and
[environment source study](ENVIRONMENT-SOURCE-RESEARCH-2026-11-02.md).
Canonical specifications retain authority; recommendations here have not been integrated into them.
All types, shaders, passes and runtime commands described below are proposals, **not implemented SDK
APIs or new crates**. No renderer was built, no Broadsides files changed, and no runtime/visual/device
evidence was generated. This is a self-dogfood exercise, not an independent implementation review.

The honest outcome is: **R0 can be specified without a new physical model; R1 needs the concrete
view/history/resource protocol below; R3 can use a declared narrow optical profile, while broader
water/transparent/media combinations retain explicit core design gaps.** A narrow proof must not
be promoted into those wider capabilities.

## First authored inputs and cooked outputs

The first source project uses ordinary F3 files and stable authored IDs. No renderer configuration
comes from editor-only state or a filesystem watcher. A run pins one admitted source/build closure.

| Product | Required source fields | F4 validation and cooked result | Actual first consumer |
| --- | --- | --- | --- |
| Render recipe | Profile/version; target feature tier; view IDs; material/geometry roots; source bindings; shadow/display/AA policies; declared camera/content envelope; finite limits | Resolve explicit dependencies and permitted fallbacks; finite pass/pipeline descriptor list with inclusion causes; resource-envelope report | F2 admission, then V1 prepare |
| Mesh part | Stable mesh/part ID; triangle-list indices; finite positions in metres; normals; UV; optional tangent; bounds; declared sidedness | Checked counts/index bounds/nondegeneracy; import transform once; tangent validation or selected cook; source-correspondent LODs/coverage if present; vertex/index products and material ID | V2 scene/extraction, V3 depth/shadow/forward |
| Material | Closure/version; alpha mode/cutoff; base/emissive input colour profiles; four slot references; factors/normal scale/sidedness | Exact RENDER-SPEC closure; reject unmapped required extensions; neutral defaults only for optional absent slots; identical mask/cutoff dependencies across roles | V3 material/pass closure |
| Camera | Stable camera ID; perspective or orthographic projection; finite transform; viewport; clipping range; render cadence; output rectangle; fixed exposure/storage scale; AA provider | Reject zero viewport/invalid near-far/projection; separate two same-profile cameras; describe sample/history retention | P2 → V2/V4 |
| Lights | Stable IDs; directional/point/spot geometry; unit-luminance RGB colour; lux/candela; transform/direction; finite range/emitter radius; shadow/media participation | Reject invalid units/intensity/range; normalize source direction once; derive bounded list capacities; declare one source/attenuation partition | W1/E1 → V3 registry |
| Environment | R0 constant ambient; R1/R3 HDRI with colour profile/no Sun disk; source-model IDs/support/valid intervals; fog coefficients; cloud tau/wind binding | Cook diffuse convolution/specular mip source from same HDRI; validate source dependencies and coefficient/support bounds; no implicit weather simulator | E1 → V3, with thermal/gameplay remaining separate |
| Terrain/shore | One source geometry revision, cell/region IDs, datum; declared render/collision/bed/placement approximation | Required collider/bed/render/placement products cross-reference source revision and error; optical rigid proxy material/cutout dependencies complete | W2/P3/E1/V2 |
| Water optical profile | Physical domain/endpoint IDs; common surface evaluator; material index/absorption/scattering; support/closed boundaries; ray/event/visit limits; reflection/incident-light policy | Validate against WATER-OPTICS; cook regional rigid geometry/material BVH and coverage; diagnose unsupported geometry rather than omit it | E1/V2 → V3 |

For the **first R0 fixture**, use one static geometry interface for all objects (instanced indexed
triangle lists, including single instances), two material classes (opaque/mask), a directional Sun
and four unshadowed local lights, one Sun map, fixed authored ambient, fixed exposure, no air/water/
animation/transparent surfaces. Masked geometry includes a fence with holes and an object behind it.
The camera can be switched between perspective and orthographic without changing world/light units.
The content remains within the R0 64-entity/250k-visible-triangle limits. The
selected spatial R0 profile does not prepare or retain unused velocity roles or
require a motion attachment without an active consumer; a later temporal
profile selects those roles separately under [RENDER-SPEC §8](../RENDER-SPEC.md#8-r0-first-rendered-game-contract-and-proof).

Recommendation agreed with the environment-source owner: author the exact R0 field
`ambient_irradiance_lux: [f32;3] = [1000,1000,1000]`. It is hemispherical incident diffuse-fill
irradiance in linear Rec.709 photometric RGB, isotropic so identical for every surface normal.
Evaluate Lambert fill as `(1-metallic)*base*ambient_irradiance_lux/pi`; the source does not perform
another radiance-to-irradiance conversion. This is an isotropic diffuse approximation, with no
invented environment specular. Author separate terminal background radiance `[0,0,0] cd/m²` for R0;
ambient irradiance does not provide background pixels. An R0 material requiring offscreen specular illumination must instead
declare a constant-radiance/environment provider and its prefilter product. This avoids treating an
untyped “ambient colour” as both irradiance and radiance.

## Rust-facing handoffs that an implementer can actually pack

Names here describe boundaries; they do not expose hecs, wgpu handles or private solver arrays. Use
existing F3/C01 identity types where available. A hash means the F4 product digest, not an unchecked
path. Collections below carry independently validated counts/byte lengths; an owned handle retains a
lease, not an unbounded borrowed snapshot reference.

```rust
// Conceptual proposal: ordinary Rust structs/enums, not compilable declarations.
RenderEpochs {
    world: WorldEpoch, source: SourceRevision, build: BuildRevision,
    presentation: Generation, device: DeviceGeneration,
}
PresentationSample {
    tick_lo: Tick, tick_hi: Tick, alpha: f64, sample_serial: u64,
    // Time is derived from tick/timebase; source evaluators receive the exact sample.
}
ViewRequest {
    camera: CameraId, view_generation: Generation, profile: ProfileId,
    projection: ProjectionDescriptor, viewport_px: [u32; 2], output_rect_px: RectU32,
    camera_pose_world: Pose64, sample: PresentationSample,
    render_this_callback: bool, cut_generation: Generation,
    exposure_scale: f32, storage_scale: f32, aa: ReconstructionPolicy,
}
PriorViewSample {
    sample: PresentationSample, camera_pose_world: Pose64,
    projection: ProjectionDescriptor, viewport_px: [u32; 2],
    source_parameters: LeasedEvaluatorRevisions, epochs: RenderEpochs,
    origin_epoch: OriginEpoch, storage_scale: f32,
}
RenderPacket {
    epochs: RenderEpochs, packet_serial: u64, render_origin_world_m: [f64; 3],
    origin_epoch: OriginEpoch, committed_endpoints: SnapshotIdentityPair,
    scene_changes: OwnedInstanceChanges, geometry_materials: ProductLeases,
    source_projection: OwnedEnvironmentProjection, views: OwnedViewPackets,
    // Separate CPU extraction and GPU submission leases; no world-reader lifetime leak.
}
DrawInstance {
    identity: InstanceKey, mesh_part: ProductId, material: MaterialId,
    current_world: Matrix64, prior_world: Option<Matrix64>,
    bounds_world: Bounds64, deformation: DeformationHandle,
    correspondence: CorrespondenceStatus,
}
CorrespondenceStatus = Valid { evaluator_revision, prior_sample }
                    | Invalid { reason: CutOrChangedOrUnavailable };
```

The coordinator creates `ViewRequest`; V2 resolves its prior sample from the image actually accepted
for **that camera/view generation**, not from `profile`, global frame parity or tick n−1. Two cameras
may share profile/layout/pipelines while retaining different prior times. V4 records a history commit
only after that provider's output has been encoded/submitted successfully; a failed or skipped view
does not advance its history sample. Surface acquisition failure may still permit an offscreen view
to commit only when that view explicitly consumes the completed offscreen image; OS display is not
the universal history clock. Completion submission ID and content validity remain separate facts.

The common render origin is a W1/W2/F4 region-derived representation origin, stable over the initial
bounded fixture, not a camera-driven physical rebase. R0–R3's bounded active geometry and both cameras
share it when all admitted errors fit. Per-view camera transforms are computed in f64 relative to
that origin before f32 packing. A wider camera separation requiring another packet/origin pays separate
derived data explicitly. Changing render origin resets histories initially. It does not move physical
bodies or source phase.

```rust
LightSource {
    id: LightId, source_revision: Revision, sample: PresentationSample,
    geometry: Directional { toward_source_world: UnitVec3, lux: f32 }
            | PointOrSpot { position_world_m: [f64;3], candela: f32,
                            radius_m: f32, support_m: f32, cone: Option<Cone> },
    colour_y1: LinearRec709, applied_terms: AttenuationSet,
    casts_geometry_shadow: bool, participates_in_media: bool,
}
SourceProjection {
    provider: ProviderId, model: Revision, source: Revision, support: Bounds64,
    valid_interval: WorldTimeInterval, sample: PresentationSample,
    role: AuthoritativeOrPrescribedOrPresentation,
    sun_before_transport: LightSource, wind: FieldProduct,
    cloud_optical_depth: FieldProductOrExplicitClear,
    air: ExponentialAir, fog: BoundedHomogeneousVolumes,
}
MediumSegmentQuery {
    origin_world_m: [f64;3], direction_world: UnitVec3, range_m: Range64,
    sample: PresentationSample, source_revisions: RevisionSet,
    active_media: MediumIdSet, purpose: CameraOrSourceRay,
}
OpticalQueryResult = Hit { distance_m, primitive, material, oriented_media, error }
                  | CoveredMiss { support, range }
                  | Incomplete { visit_or_event_limit }
                  | UncoveredExit | UnsupportedGeometry | StaleProduct;
```

`toward_source_world` always points from receiver to source; propagation is its negative. V3 rejects
an already-cloud/air-attenuated Sun supplied to a path that owns those same terms. A clear provider
returns a valid optical depth of zero; unavailable/stale is not zero. V3's physical-light evaluation
does not supply E1 heat by dividing lux by 683. Source, wind, terrain exposure, P3 contact and water
queries remain CPU/headless or analytically reproducible under C09; GPU projections are derived copies.

## Finite closure and the missing GPU fields

Keep RENDER-SPEC ABI revision 1's `ViewRecord=160 B`, `MaterialRecord=64 B`, `ObjectRecord=192 B`
and four binding groups. Pack scalar offsets explicitly into zeroed bytes and compare Naga reflection
with the layout table; do not `transmute`. Validate textures, offsets, dynamic alignment, vertex IO and
all pass subsets before V1 preparation. A geometry schema/packing change increments its ABI/product key.

The implementor cannot reconstruct rays and depth from the existing ViewRecord alone. **Recommendation:**
retain it and add an explicitly versioned group-0 auxiliary uniform binding rather than silently
reinterpreting its unused stride bytes. A proposed `ViewAuxRecord` is 128 B:

| Offset | Data | Why it is needed |
| --- | --- | --- |
| 0–63 | `inverse_view_proj` unjittered mat4 | Reconstruct a current perspective or orthographic ray/hit |
| 64–79 | `(width,height,inv_width,inv_height)` vec4 | Pixel conventions, post filters and motion conversion |
| 80–95 | `(near_m,far_m,view_height_scale,storage_scale_ratio)` vec4 | Finite depth envelope, orthographic projection and valid pre-exposure conversion |
| 96–111 | `(prior_time_hi,prior_time_lo,current_time_hi,current_time_lo)` vec4 | Bounded split-time presentation data; exact CPU ticks/revisions stay in packet |
| 112–127 | `(camera_kind,history_valid,view_flags,source_projection_index)` vec4<u32> | Explicit interpretation and validity; no WGSL bool ABI |

Two f32 time parts do not grant arbitrary-time accuracy: source evaluators should upload reduced wave
phases/evaluator parameters for the actual sample pair, and declare error. Matrices are column-major;
right-handed world Y-up, camera looks along −Z, WebGPU clip depth `[0,1]`. Select reverse-Z with finite
near/far, clear 0 and greater compare. Perspective/orthographic builders and inverses have round-trip
point/ray fixtures. Clip jitter changes raster sampling; motion uses unjittered transforms. Previous
object and view transforms are expressed in the same current packet origin or marked invalid.

Recommended initial bindings are: group 0 view plus auxiliary; group 1 scene uniform, packed scene
tables, shadows/environment/source textures and pass-specific optical data; group 2 material uniform,
four texture slots and sampler; group 3 object/skin/geometry tables. `SceneTables` is one checked
u32 storage arena with aligned typed ranges for light records, cluster offsets/indices and source
records; it avoids one binding per light or a storage-buffer explosion. Optical BVH/triangle/material
tables use one separate arena. Group-3 objects, optional skins and vertex-pulling geometry are three
more storage buffers: five storage buffers total in a pass needing all of them, before pass-local
event storage. Validate each exact layout against queried stage limits, including event-buffer bindings;
split a pass rather than assume all optional buffers can coexist. Post passes use exact subsets, not a
mandatory giant scene layout. Target compressed textures are separate cooked variants with declared
feature support; uncompressed fallback is a real included product with its own bytes.

The exact R0 descriptor inventory is **12 proposed tuples**: two alpha classes × depth/shadow/forward/
velocity = eight, plus tone/display-input, FXAA, UI and final output. All use one geometry interface,
sample count 1, fixed attachment layouts and no light-count specialization. Twelve fits R0's ≤16 cap
but revises its earlier ten-tuple worksheet placeholder; the actual cook must enumerate keys. Only
depth/shadow/forward/post execute in spatial-only R0. R1 skins and exact-event mode add actual reachable
interfaces/roles; R2 ≤48 remains a cook gate rather than an assumed multiplication. Pipeline preparation
failure blocks readiness with the tuple/source/adapter diagnostic. Nothing compiles because a draw
first encounters an object.

## V1–V4 normal operations and pass ownership

V1 owns one device generation, resource allocator, command recording/submission and completion tracking.
Feature code declares typed pass resources; it receives restricted recording context, never an independent
queue or device lifetime. CPU jobs prepare data independently of GPU resource hazards. A declaration
contains owner, world/view scope, resource format/usage/subresource, read/write/load/clear, producer version,
external side effect, pipeline key and retirement requirements. Output-dead culling preserves exported
cache/query side effects. Reuse requires compatible descriptors and nonoverlapping actual lifetimes.

| Order | Owner / operation | Inputs → outputs and failure |
| --- | --- | --- |
| 0 | F1/W1/E1/P3 commit, once per tick | Immutable endpoint pair/provider leases; views cannot step it. Missing required source prevents that profile's presentation admission |
| 1 | V2 extraction and patching | Lease committed data; apply spawn/despawn/change lists to persistent scene records; acquire product leases; produce owned packet and per-view candidate sets. Stale identity/epoch rejects packet |
| 2 | V1 readiness/preparation barrier | Verify pipeline/product/device/profile generations, upload chunks and reserve resources. Ready packet only; optional unsubmitted work may cancel under pressure |
| 3 | V2 visibility/LOD | Per-view conservative CPU hierarchy/frustum traversal; explicit shadow/reflection caster sets. No GPU readback dependency for gameplay or frame completeness |
| 4 | V3 Sun shadow | Same deformation/alpha cutoff as depth/forward; one conventional map for R0, explicit cascades for R1. Produces source-visibility product, never preattenuated solar intensity |
| 5 | V3 opaque/masked depth | Reverse-Z D32; same vertex/coverage law; no optical interface overwrites opaque receiver depth |
| 6 | V3 light lists/opaque surface shading | CPU-produced finite lists initially; R1 clustered lists. Shade scene-linear surface radiance into RGBA16F using storage scale; unhandled capacity/NaN is error |
| 7 | V3 optional ordered events and joint media | Exact-event or declared sorted approximation, air/fog/water intervals, common source-ray visibility. Produces logical composed scene plus confidence/reactivity; details below |
| 8 | V4 reconstruct/display | Spatial baseline or admitted temporal estimator; fixed exposure/SDR curve; scene-only AA then UI/output. No second medium integration or E1 exposure |
| 9 | V1 submit/retire; V4/effect history commit | Submission receipt identifies packet/view/products. CPU snapshot leases may already be released; GPU products retire only after relevant completion and readers release |

R0 shadow recommendation: one 2048² D32 Sun map, fixed authored light-space volume covering the bounded
fixture; 4-tap equal-weight comparison PCF with nearest comparison samples. Author constant/slope bias
in texel/receiver units, convert using the actual map projection, and expose the values in the recipe.
No local maps in the first fixture. R1 recommendation: three explicit 2048² maps over admitted receiver
distance bands, light-space stabilized to texel increments; include all casters capable of affecting
those receivers and use a 10% overlap blend. Fit each cascade from the **union of both admitted view
receiver supports**, not whichever camera encoded last. This costs 48 MiB D32 rather than 16 MiB before
driver overhead. A distant secondary camera that makes the union too coarse requests its own declared
maps or a different profile; it cannot silently destroy main-view quality. Bias/blend quality is unrun.

R1 light-list recommendation: 32×32 pixel screen tiles ×16 depth bins; logarithmic perspective, linear
orthographic over each view's finite clip range. Conservatively intersect point spheres/spot bounds.
The initial implementation may construct these lists on CPU in persistent scratch; it must report
work and can later replace the internal builder with GPU assignment. Use full spatial bins for
transparent/medium samples, not opaque-depth-trimmed bins. Surface capacity is 32 desktop/browser and
8 mobile; volume participation remains the separate Sun+4/2 subset. A capacity overflow rejects the
requested recipe/view with offending cluster/lights; it is not arbitrary per-pixel truncation.

V4 recommendation for the first **SDR fixed-exposure** profile: authored positive exposure scale, no
automatic meter; `C_e=exposure*L`, `C_t=C_e/(1+Y(C_e))`, then scale by
`1/max(1,max_component(C_t))` to fit linear display RGB. This is a fully stated simple tone/gamut
policy, not a claim of photographic/ACES fidelity. Encode sRGB into a filterable RGBA16F intermediate;
apply a fixed FXAA 3.11 quality configuration; decode to display-linear, composite UI, then encode exactly
once into the selected output format. Choose fixed FXAA thresholds/configuration at cook; their source
and licence must be pinned before coding. [Bevy's versioned FXAA registration](https://raw.githubusercontent.com/bevyengine/bevy/v0.20.0/crates/bevy_anti_alias/src/fxaa/mod.rs)
was inspected for post-tone ordering/input/layout; its shader body was unavailable, so it is not the
algorithm implementation to copy blindly. FXAA's exact distributable source/configuration remains a
small **implementation-input closure**, not permission to invent a new filter. A different admitted
spatial AA provider may replace it after explicit selection, not by accidental shader substitution.

Storage scale `p` is separate from display exposure. The first fixed fixture chooses `p=1/1024` and
declares its HDR radiance envelope; shader/reference checks detect a nonfinite value or a value exceeding
the safe half-float range before conversion. Profile admission/validation must establish the claimed
envelope for its source/material/camera set. General automatic pre-exposure and broad HDR output remain
separate V4 work; the first fixed fixture cannot qualify unrestricted bright emission/grazing materials.
An envelope violation produces a diagnosed unsupported frame, not early colour clamping.

## Same-profile views and deformation

Build an R1 correctness fixture with two distinct camera IDs, both the same profile, one perspective
and one orthographic. Main renders every callback; secondary at half width/height every other callback.
Keep one committed world/source evaluation and one persistent instance table. Each active view computes
its own visibility, projection, light clusters, raster targets and history. Shared shadow/environment
products carry their union support and source generation. Hidden/skipped views retain bounded prior
sample inputs only; their histories do not ping-pong on callbacks that did not produce their image.

For the first wind/deformation producer, select a **cosmetic bounded bend** for one plant fixture:
root-pinned weight `w=clamp((q.y-root_y)/height,0,1)^2`, horizontal direction from E1 wind and
displacement `w*A*sin(phi+omega*t)` with `A=min(bend_cap,gain*|u_horizontal|²)`. A zero wind vector
gives zero displacement, not normalization of zero. Author phase/frequency/gain/cap and keep their
revision; do not call this an inertia/damage/biomechanics model. Evaluate current and prior samples
with the corresponding source/recipe revisions, including current/prior direction. Expand bounds by
`bend_cap`, use the same law in colour/depth/shadow, and reset if prior source inputs are unavailable.
Water and sound consume the same E1 wind cause through their own laws; the plant shader does not
evolve weather. Larger vegetation response remains E1's owning design.

Skins use P4's posed geometry/retention contract. The motion is the same material point through current
and prior skin/morph/bend, object and view transforms; transform subtraction alone is invalid. At an
LOD switch, reset initially. A future provider may retain history only with a proved correspondence
map, including geometry/source revisions and error. Preserve prior evaluator inputs until the bounded
view horizon; beyond it reset rather than pinning an old whole-world snapshot.

For a concrete R1 temporal diagnostic/provider, recommend the following deliberately modest estimator:
store current-to-history motion and prior linear view-depth/validity in RGBA16F, keep two RGBA16F colour
histories plus prior R32F depth and R32Uint stable-instance identity, and generate current identity from
the same depth/coverage pass. Reproject using C07 vectors, rescale valid history by the storage-scale
ratio, reject outside image/invalid motion/cut/changed identity, and reject prior-depth disagreement
greater than `max(0.01 m,0.01*expected_prior_depth)`. Clamp accepted history componentwise to current
3×3 scene-colour min/max; blend 0.9 history/0.1 current. Reactive or unsupported reflective/refractive
pixels use weight 1 current. These depth thresholds/weights are proposed quality parameters, not proved
error bounds. Product/identity generation changes reset first; no stale reused instance slot can pass.
Use a high-sample spatial reference and the motion fixtures to reject ghosting rather than tuning only
stills. This provider preserves C07 meaning but is not a vendor upscaler or a final quality promise.

The identity/current-prior-depth additions cost beyond the earlier generic image worksheet: RGBA16F
motion rather than RG16F adds 4P; current identity adds 4P; prior identity adds 4P. At 720p that is
**10.55 MiB extra logical storage**, split between current/transient and retained history. R1 desktop
must charge it. R2 initially selects the spatial path; do not claim this more complete temporal input
set plus exact events fits the earlier browser transient allocation.

## Transparent storage, deterministic ties and overflow

Canonical RENDER-SPEC currently selects grouped sorted premultiplied alpha and labels intersecting/
refractive cases approximate. **Do not silently replace that selection.** For R1's existing sorted
profile, the authored group order is stable, writes no opaque depth and receives an explicit approximate
label. To make the two-event exact-order promise concrete, recommend a separately selected
`TwoEventPeel` profile below. The new roles/outputs require F2/F4 admission and a canonical decision.

The proposal supports at most two nonrefractive alpha events ahead of opaque depth and detects a third.
It does not promise exact volumetric multiple scattering, unrestricted glass or recursive reflections.
Each accepted event stores D32 depth (4 B), a u32 event ID (4 B) and packed RGBA16F premultiplied
surface colour (8 B), **16 B/pixel/event**. IDs are packet-local checked dense primitive identities
formed from a per-instance/part primitive prefix plus triangle index; MAX is invalid. Prefix/count
overflow rejects the packet. No history identity is inferred from a reused packet-local ID.

The optional WebGPU [`primitive-index` feature](https://gpuweb.github.io/gpuweb/#primitive-index)
cannot be assumed. The portable peel/shade interface uses nonindexed triangle-list **vertex pulling**:
`vertex_index/3` gives a triangle ID; indices/vertices are checked cooked storage ranges; instance index
selects object/part metadata. Normal opaque rendering stays indexed. This adds repeated vertex work
and another structural pipeline interface only where needed. It avoids exploding all vertex attributes
into duplicated corner records or depending on optional fragment primitive IDs.

For each layer, run these finite operations:

1. Clear layer depth to 0 and owner IDs to MAX. Raster candidates ahead of opaque depth and
   lexicographically behind the previous selected `(reverse_depth,event_id)`, if one exists. Depth
   compare greater finds the nearest eligible depth. Mark position invariant and use identical
   vertex/deformation/sample inputs in every role.
2. Reraster eligible fragments at that stored depth; fragment storage `atomicMin` chooses the smallest
   event ID among equal-depth hits. This gives a deterministic tie without 64-bit atomics. Missing
   owner for a covered depth is a numeric/conformance error, not empty geometry. Validate exact depth
   correspondence on each adapter; reject unsupported behavior before advertising this path.
3. Reraster/shade only the selected owner/depth. Store its pre-exposed premultiplied colour as two
   packed u32 half-float pairs in a storage buffer. A unique selected primitive owns each pixel, so
   the pass does not use unordered floating-point accumulation.

After layer two, reraster candidates lexicographically farther but ahead of opaque depth; any survivor
sets an atomic per-view overflow flag. A subsequent resolve reads that flag and produces the explicit
unsupported-optics diagnostic surface instead of presenting a silently truncated image. A bounded
readback later supplies exact CPU diagnostics; correctness of the current output does not wait for
that readback. If a recipe declares a fallback, its pipeline/resources must already be prepared and
its approximation accepted; otherwise the optical view remains failed/loading.

This requires seven transparent geometry walks (three per layer plus overflow), plus clears/resolve.
It deliberately trades repeated raster work for finite storage and portable synchronization. It is a
recommended correctness path for bounded coverage, not a claim of faster OIT. Cook/source content
rejects unsupported topology; dynamic intersections still exercise runtime overflow. Alpha-zero events
are omitted, masked cutouts stay in the opaque path, and opaque depth wins a coplanar opaque/transparent
tie. The lexicographic primitive rule orders coincident transparent surfaces deterministically; it
does not make their physical medium interpretation meaningful, so incompatible coincident interfaces
reject optical admission.

Resolve far-to-near under C08. Each layer's packed colour buffer is a read-write **buffer**, so one
compute invocation per pixel can read and overwrite its own far-layer slot with the composed result,
then combine into the near-layer slot. No invocation reads another invocation's layer slot. This avoids
assuming optional read-write storage-texture formats or an additional full-screen HDR scratch image.
Depth-aware medium upsampling reads separate immutable depth/media inputs. V4 loads the completed slot
as scene colour; its FXAA uses a later filterable intermediate, after event buffers can retire/reuse.
Debug correctness compares this in-place buffer resolve with a separate-target reference.

## R3 optical mapping: one buildable bounded proposal

Use **one named R3 optical-A recipe**: above-water camera, graph/finite shallow surfaces, rigid bed/hulls
and binary cutouts, common water index `n=1.333`, absorption-only water (`sigma_s=0`, authored nonnegative
RGB `sigma_a`), no foam/bubble/focused-caustic promise, Sun-only underwater illumination, environment/
validated rigid reflection. Camera/source support and ray lengths are finite; water camera traversal,
folds/falls/transparent scene solids and heterogeneous optical materials are outside this recipe.
This intentionally narrower first proof leaves the wider WATER-OPTICS capability open. It does not
alter prescribed-sea/shallow physics or the blocked physical mixed-model hull crossing.

Concrete first fixture: one bounded coastal region with the prescribed sea component list, one selected
128×64 shallow-domain source at the canonical 2 m spacing (256×128 m), four rigid hulls, a shared stepped-bed/shore product, one homogeneous fog
box and an authored cloud optical-depth ramp. Add no arbitrary underwater local light until its complete
refracted source path is selected. Admit a downward-looking camera and Sun above 15° for the first mean
interface model; report that envelope prominently. The larger R3 physical workload can be exercised
without pretending these camera restrictions qualify every coastal view.

E1 supplies the physical water endpoint/evaluator and wet support; V2 prepares **one common current
surface mesh** for raster and optical intersection, with a conservative position/normal error against
the physical evaluator. For the sinusoidal graph, grid interpolation has the bound
`epsilon_height <= (h_x² sup|eta_xx| + h_z² sup|eta_zz|)/8`; bound derivatives using the admitted wave
components, then choose grid spacing to meet the declared shore/contact cue. Mesh boundaries share
vertices/edge samples. For the coarse shallow model, preserve its piecewise source/storage semantics;
a smoothed visual seam cannot redefine occupancy or P3 queries. A separate approved smoothing mapping
would carry its own error and geometry product.

F4/V2's rigid bed/solid proxy and the generated water interfaces use WATER-OPTICS's bounded stackless
BVH semantics. Cook static topology/escape indices; evaluate shared current water vertices once and
refit bounds leaf-to-root in a finite depth-ordered dispatch sequence before consumers. TLAS follows
accepted rigid poses. Same current sample is shared across active views; a view needing another current
sample either prepares another bounded product or is deferred. Previous water geometry for history
is separately evaluable/retained, not overwritten by a refit. Report vertex×wave work, refit dispatches,
traversal visit caps and current/prior bytes. A visit cap yields Incomplete, never dark/miss.

The optical event at a water interface changes direction by Snell and applies WATER-OPTICS's Fresnel/
radiance convention. Trace the transmitted branch against the covered regional bed/solids/interface
set; camera-raster peeling cannot supply a first hit after refraction. Count further interface events
against the two-event optical budget along that branch, with runtime overflow. Domain adjacency in
the same water material is not an extra refractive boundary; optical occupancy is the union of the
admitted water volumes, independent of W2 stream cells. Unknown exit from this union's supported
geometry is `UncoveredExit`, not the sky. R3's first above-water fixture expects one entry followed
by a covered opaque bed/hull hit, and explicitly checks a second/failing interface case.

For initial submerged direct light use the **mean horizontal interface** as a named approximation.
Given receiver x, refract the unattenuated source direction through the mean plane, trace from x along
the water direction to that plane, then along the air direction toward the source. Water/air/fog/cloud
and rigid shadow segments each contribute once. For a collimated beam,
`E_water_perp=(1-F)*E_air_perp*cos(theta_air)/cos(theta_water)*exp(-sigma_a*l_water)`.
This is flux geometry at a planar interface; do not apply a second index-squared factor to that
irradiance. It omits wave focusing. The true displaced surface remains the camera/reflection interface;
their difference is a declared lighting approximation to compare against the wider light-space model.
No ordinary dry Sun is added to this through-water direct term. A failed covered mean-light query is
unsupported, not zero illumination. Reflection uses valid rigid hit radiance or the admitted environment
terminal source; unknown submerged transmission cannot use an environment probe as a receiver.

In the water segment `T=exp(-sigma_a*l)` and `S=0`; integrate air/fog segments with the **existing**
RENDER-SPEC common source and coefficient sum, including source-ray attenuation. One fog entry/exit,
one water interface and opaque depth split the ray in order. No “fogged opaque scene copy” is fogged
again behind the water surface. The common evaluator/geometry error must cover the optical intersection;
otherwise this recipe has not met its cue contract even if its finite-volume residual passes.

Cloud attenuation comes from the environment study's thin-plane tau recipe, not visible cloud colour.
Pin slab/reference height, supported receiver altitude/footprint and Sun-angle range; sample vertical
tau at the source-ray/plane intersection and divide by positive source vertical cosine. Interpolate
optical depth before `exp`, not painted darkness. Uniform wind translates the field by the integral of
its committed wind history. All ground, plant, hull, water-incident and fog-source queries use that
same source revision/time; two views cannot move the cloud cause independently. Visible-cloud off
requires no view-march/noise/lighting volume or cloud history in this recipe.

## Lifecycle/failure walkthroughs

| Case | Required sequence and observable outcome | Fault that must be rejected |
| --- | --- | --- |
| Ordinary R0 frame | Tick/commit once → extract owned packet → readiness → shadow/depth/forward → V4/display → submit/retire. Saved action changes one object's committed pose/material cue | A camera callback advances W1/E1, masked role differs, or a pipeline is compiled by first draw |
| Same-profile second view | Shared current world/products; distinct camera/history IDs and independently committed sample serials. Skip secondary; on resume use its actual last image sample or reset | Global frame parity, profile-indexed texture pool, borrowed main camera motion/exposure |
| Feature off | F2 validates retained consumers/substitutes; stop new optional preparation; commit generation; invalidate affected histories; release CPU/GPU owners after actual readers/completion | Hiding water/cloud draw leaves unused captures/volumes/pipelines preparing; turning off water graphics removes buoyancy/source state |
| Source edit/restart | Apply working-file change → admit new source/cook → explicit stop/load; old authority/products retire before new bounded run. Failed edit/cook leaves previous valid source/run unchanged | Live collider/bed/source mutation from watcher, mixed old/new products or stale callback publication |
| Device loss | Invalidate device handles/pending GPU work, retain source/CPU authority according to platform policy; acquire new generation → validate limits → prepare/upload → reset all affected histories → ready view | Old-generation callback installs pipeline/texture, E1 clocks reset because GPU vanished, cached driver pipeline treated as durable product |
| Surface-only change | Reconfigure surface lease/formats; prepare exact changed output descriptors before ready; preserve unrelated device products when valid; reset size-dependent histories | Treat every surface resize as lost world or reuse incompatible target/pipeline formats |
| Layer/visit overflow | GPU resolve emits diagnostic/declared prepared fallback; mark failed optical capability with query/event source IDs; authority continues under its own semantics | Third interface disappears, over-cap ray becomes a miss, previous image represented as current without age/status |
| Missing medium/source support | Reject field/time/ray request or use explicitly admitted alternate; no silent clamps | Calm/clear/zero/sky substituted for unavailable input; air integrated underwater |

Feature-off resource expectations: headless omits the device/extraction/pipeline closure entirely; sky-off
retains only selected illumination products; cloud-draw-off here retains CPU/tau source sampling only;
all-cloud-off returns explicit clear and retires tau products; water-graphics-off retains physical CPU
domain/query/ledger but removes mesh/BVH GPU copies, optical rays and image history when no other reader
needs them; temporal-off removes V4 histories/motion targets when no effect still needs them. A live
switch's temporary old/new overlap is a separately admitted peak, not an off-state steady cost.

## Fixtures, fault injection and proof accounting

| Fixture, dependency and independent comparison | Fault injection / rejection | Measurements and scope |
| --- | --- | --- |
| F0 CPU/cook ABI and pipeline inventory | Misalign a field, remove masked shadow tuple, change shader include after cook, duplicate source ID, unsupported target format, overflow checked byte range | Exact reflected offsets/keys/rejection; no GPU-quality claim |
| F1 R0 first rendered action | Change source epoch after extraction; delay one pipeline; fail upload; reverse projection handedness; alpha fence cutoff differs in shadow | First intended input-to-visible response, cold closure and actual pipeline-ready trace; colour/lux card/BRDF reciprocity/hemisphere oracle plus native moving fence |
| F2 two cameras/one wind/one skin | Same profile IDs, skip 1/2/20 callbacks, change wind recipe, rebase, teleport, missing prior pose; deliberately give rigid vectors to bent leaves | Per-view timestamps/IDs, motion difference against projected material points, shimmer/ghost/disocclusion review, retained pose/history bytes |
| F3 two-event order | Two intersecting coloured panes, coincident primitives and moving third pane; erase one owner; exceed ID prefix; mismatch depth correspondence | Compare independent sorted per-pixel CPU/reference raster at held samples; exact order where promised, typed unsupported overflow, packed in-place versus separate-target resolve |
| F4 common media/light | Absorber before/after narrow emitter; two fog overlaps; tau ramp on ground/plant/water; wind off and source stale | Analytic Beer/segment oracle plus high-sample joint source reference; reject squared tau, independently fogged images or view-driven source phase |
| F5 shared shore/R3 | Dry hull hides Sun but bed remains visible through water; offscreen bed/blocker; optical cap exhaustion; render/collider/bed revision mismatch; blocked mixed physical crossing | Shared interface position/normal error, mean-interface light versus richer reference, ray coverage statuses; water physics residual and visual/contact checks remain separate |
| F6 lifecycle/off/assembly | Device loss during prepare, old callback, resize, source restart, retained physical consumer with graphics off, repeated on/off | Full graph/resource/lease inventory, peak CPU/GPU/staging/residency, pipeline stalls, work debt and same-profile histories; no hidden queued work |

All fixtures need exact source/build/content/toolchain/adapter/driver/browser identity. Reference shaders
and measured loaded artifacts must be recorded. No benchmark is authorized by this document. Runtime
qualification uses BASELINE-SPEC's sustained/cold/lifecycle procedures and separates correctness, visual
acceptance and actual device cost. A clean `./dev check-docs` proves none of those gates.

At 720p, R0 logical HDR8+depth4+encoded-display8+external output4 is 24P = **21.09 MiB** before
shadow/products/staging; the Sun D32 map is another **16 MiB**. FXAA intermediates must follow their
actual lifetimes, not be assumed in-place. At 1080p these image bytes scale by 2.25. R1's three maps
are **48 MiB** persistent; half-dimension secondary targets add 25% while resident and 12.5% average
pixel shading when drawn every other callback. Histories remain resident on skipped callbacks.

Two exact-event records cost 32P = **28.13 MiB** at 720p. Combined with the research account's
19.34 MiB nonhistory image inputs/media/masks, the peak is **47.46 MiB** before extra current products,
allocator padding or overlapping display intermediates. The proposed packed-buffer resolve avoids an
additional 8P HDR scratch; a naive extra target would exceed the browser's **51.2 MiB transient planning
share**. Schedule display-input allocation after event-layer release/reuse only where dependency and
compatible-format lifetime analysis permits it; mixed buffer/texture allocations are not magically
aliasable in WebGPU. If both remain live, charge both and reject/rebalance that recipe. This is a
concrete resource-schedule gate, not a claim that R2+R3 fits.

Work variables are dirty instances D, candidate nodes Q_v, visible vertices/triangles G_v, light-cluster
intersections I_v, overdraw O_v, media rays R_v and adaptive samples S_v. CPU extraction/list work is
`O(D+sum Q_v+sum I_v)` plus sort/pose/upload and validation. GPU main lighting is approximately
`sum O_v*P_v*K_v`, shadows add their own geometry/fill, exact peeling adds seven transparent geometry
walks, and media adds `sum R_v*S_v*participating_lights` plus shadow/source-ray intersections. Water
vertex preparation scales with vertex×wave count and BVH refit nodes; each optical ray pays its visit
cap. Feature culling does not cancel CPU source state required by a physical consumer. Compare CPU work,
GPU critical span, uploads, persistent/transient/retiring/staging bytes and queue age separately.

## Choice audit, remaining gaps and proposed contract deltas

| Point the implementer previously had to invent | One recommendation now supplied | What remains |
| --- | --- | --- |
| IDs/sample scope and two same-profile views | Camera/view generation plus accepted sample serial, independent histories and bounded prior evaluator inputs | Concrete code/tests and measured retention costs |
| Ray/depth reconstruction | Auxiliary 128 B view binding; finite reverse-Z conventions and shared packet origin | ABI integration/reflection/projection conformance |
| Sun/local shadow footprint | One R0 map; three union-support R1 maps, 4-tap PCF; local lights unshadowed in first fixture | Bias/coverage quality, wider local-shadow recipe selection |
| Light assignment | CPU conservative 32² tiles×16 bins first; all-space transparent/media bins; typed overflow | Device/CPU cost, GPU builder challenger |
| Ambient/exposure/display | Explicit irradiance fill, fixed exposure/storage metadata and stated SDR curve | Exact distributable FXAA source/config hash; wider auto-exposure/HDR policy unselected |
| Layer storage/ties/overflow | Separate proposed TwoEventPeel, lexicographic owner, optional-feature-free IDs, packed-buffer resolve and GPU overflow diagnosis | Canonical acceptance of exact-event extension; adapter depth-owner conformance; capacity/performance |
| R3 physical-to-optical mapping | Shared evaluated surface/error, complete rigid regional query, absorption-only optical-A and mean-planar incident light | Optical-A acceptance; wider scattering/focused caustics, traversal/transparent/deforming proxy mappings remain separate unfinished capability design |
| Clouds without visible clouds | Shared typed tau plane/source-time; integrate wind phase, no cloud colour march | Source/GPU parity and cue/angle/footprint qualification |
| Feature removal and stale work | Typed owner/subresource/generation/lifetime descriptors, no private queues, retirement after completion/readers | Actual runtime registration/schema adapters and combined peak |

The remaining **core gaps** are not the tuning rows above: (a) whether the canonical first renderer
adopts the exact-event extension or retains only its currently approximate sorted transparency promise;
(b) which wider optical capability beyond optical-A the game actually admits, with real complete
deforming/transparent geometry, underwater traversal and scattering/incident-field producers;
(c) a complete wider local-light-shadow/provider recipe when required. The source/licence/hash
closure for the selected spatial filter is a separate implementation-input task before implementation,
not an unresolved physical law. Optical-A's validity and the fixed
fixture's HDR envelope are explicit restricted proposals, not evidence that the wider claims were
already complete. No experiment can invent those absent producers.

Everything else listed as unrun is a mathematical/implementation conformance, perceptual, numerical or
hardware uncertainty with an operator and failure rule. If a seven-walk peel or CPU list builder cannot
meet the named budget, redesign that internal representation or select a qualified declared alternative;
do not hide required layers or lights. Replacing the whole renderer is not necessary to perform that work.

Precise deltas returned to the coordinating canonical owners:

- **C06/F4/V1:** accept auxiliary ABI/pass-layout versioning; declare initialized/read/write subresources,
  external side effects and world/view/device scope; preparation callbacks carry all generation keys;
  exact reachable role tuple inventory includes display and event roles. Resource peaks distinguish
  persistent, frame-transient, upload, pending retirement and presentation overlap.
- **C07/V2/P4/E1/V4:** define history commit receipt/sample serial independent of global frame parity,
  camera profile and OS-present result; retain evaluator/source revisions for actual previous material
  points; require conservative deformation/LOD/optical error products and explicit invalidity. Existing
  camera/view identity clauses remain the authority. Per-provider proved mapping may survive an LOD switch.
- **C08/V3/E1/V4:** choose the sorted-versus-exact event policy explicitly; add two-event storage/sort/
  overflow distinctions, opaque/interface/refracted-receiver depths, optical coverage statuses and one
  source-path partition. Register fixed SDR/storage policy as a named initial profile, separate from
  future auto-exposure/HDR. Do not weaken the joint coefficient/source operator for a convenient screen blend.
- **C09/W2/E1/P3/V2:** publish physical water endpoints and common visual/optical geometry with source/bed/
  topology/error/correspondence; keep tau/wind sources headless and once per committed world time;
  declare optical occupancy unions independent of stream/physical-query domains. Terrain/collision/bed
  authority and the blocked physical mouth crossing cannot be inferred from a smooth rendered seam.

Re-reading the proposed frame and all lifecycle rows yields concrete owners/inputs/outputs without
needing a new clock, light law or private submission path. The stated core choices remain outstanding
canonical decisions. Implementation, output quality and sustained target viability are still unproved.

# M1 renderer foundation

3 November 2026. Selected bounded implementation under the earlier
[V1 order](../../plans/BRIEF.md#parked-retained-in-git-history),
[C06–C08](../CONTRACTS.md#c06-gpu-execution-and-lifetime--v1) and the
[render specification](../RENDER-SPEC.md). This is a renderer foundation with
one real lit diagnostic, not M2 clustered-forward/CSM qualification.
The later [M2 scene-colour slice](M2-HDR-DISPLAY.md) supersedes this decision's
single-pass linear-UNORM output while retaining the M1 directional/depth scope.

## Prepared interaction and authority

`thrum-render` receives the V1 executor's device, queue and generation; it creates
no adapter/device and submits no private work. Preparation validates/packs tables,
uploads defaults and creates the required opaque/static pipeline under a completed
validation scope. Encoding never builds pipelines. The host submits and polls.

The synthetic revision-2 fixture renders 10,000 rotated procedural cubes in one
instanced draw, using the specified Lambert + GGX/height-correlated Smith/Schlick
direct closure and emission. One directional source specifies receiver-to-source
direction and perpendicular-receiver RGB illumination, normalized/packed during
preparation. It is diagnostic source data, not a second W1/E1 light authority.
The fixed orthographic camera looks along +Z; world coordinates also define its
clip grid and receiver-to-camera rays are -Z. A Depth24Plus attachment clears to
one, tests Less, writes opaque depth and discards at pass end.

Group 0 uses the existing 160-byte ABI2 ViewRecord (identity current/previous
projection and unit pre-exposure). Group 1 uses a 48-byte diagnostic directional
record: direction, RGB illumination and prepared half-vector/Schlick angular
power. The last terms are invariant for this orthographic camera; preparing them
once removes a normalization/power from each fragment. f64 preparation and scaled
Smith cosine evaluation avoid half-vector/grazing f32 underflow. Nonfinite or
zero light directions, illumination outside [0,10000], projective/nonfinite or
out-of-envelope transforms and degenerate normal axes refuse before readiness.

Output is explicitly clamped linear RGBA8Unorm with opaque alpha. There is no
indirect light or AO contribution; AO cannot darken direct light or emission.
Normal maps, alpha modes, arbitrary cameras, HDR/display conversion, clustered
lights, CSM, motion, cooked content, surface recovery and a running game remain
unbuilt. Full BRDF energy/reciprocity qualification is also open. M2 owns the
clustered/shadow interaction; A6 connects F4 products. R3 remains parked.

## Graph, ABI and lifetime

Declarations name graph-owned identities, immutable descriptors, ownership/scope,
exact accesses, clear/load/store behavior and prepared pipeline generation.
Buffer ranges are bytes; texture ranges flatten complete mip/layers, conservatively
unifying depth/stencil. Undefined reads, loads after discard, conflicting same-pass
writes, foreign IDs and absent/stale required tuples refuse before execution.

Declaration order is execution order; overlapping pairs involving a write produce
hazard dependencies. wgpu owns backend transitions. Exact-descriptor transient
slots may reuse only after strictly earlier last use; this is a logical plan,
not measured physical aliasing/residency. Persistent owners/epochs never pool.
Retirement also waits for CPU readers and GPU completion. Compile graphs at setup
or structure/descriptor/generation changes; encode creates no graph identities.

[ABI2](../RENDER-SPEC.md#2-material-closure-and-binding-abi) retains 80-byte material
and 192-byte instance records. Checked scalar packers write explicit little-endian
offsets into zeroed storage without unsafe casts. Group 2 binds the material table,
four typed texture arrays and one sampler; group 3 binds indexed instances.
Base/emissive arrays decode sRGB, normal/ORM stay linear and factors are scene-linear.
Default layer zero preserves multiplicative semantics. Only default layers, static
instances and zero semantic flags are admitted; no binding-array feature is needed.

Submitted and accepted-history metadata remain separate. Skips do not advance;
view/provider/grid/device/content/profile/origin/correspondence changes reset.
Queue completion cannot certify image acceptance. GPU histories remain unbuilt.

Timestamp batches reserve two queries per pass; unsupported enabled-device features
allocate no query/readback resources or fake zero durations. After the known
[Metal stale-counter fault](../../evidence/render/R0-FXAA-1080P-TIMESTAMP-2026-11-03.md),
the selected protocol resolves/copies only in a later host-owned submission after
render completion. Collection never waits/submits; pending slots cannot overwrite,
and old device generations or zero/error/inverted counters refuse publication.

## Cost and evidence

Graph analysis is quadratic in declared accesses and independent of instance count;
it runs at preparation. 10,000 instances upload 1,920,000 bytes, two materials 160
bytes, view/source 208 bytes and defaults four texels. One draw emits 120,000
triangles/360,000 vertices; there are no per-object CPU bindings or frame uploads.
At 1080p the colour payload is 8,294,400 bytes plus 2,073,600 opaque depth samples;
Depth24Plus chooses physical storage per backend. These are logical counts, not
measured bandwidth/residency. Culling, indirect draws and changed-range uploads
remain future consumers. Timing adds query/readback slots plus bounded labels,
channels and result allocation; it is not a zero-allocation profiler.

CPU checks cover graph hazards/reuse/closure, ABI/reference/domain/capacity,
history resets and source packing. The ignored real-adapter test checks padded
65×33 readback, material/instance strides, direct/emissive/roughness/metalness
response against an independent f64 oracle, grazing light, both depth orders,
stale generations, repeated/disabled timing and 10,000 cubes at 1080p.
The [lit-scene evidence](../../evidence/render/M1-LIT-SCENE-2026-11-03.md) records
Apple M3 Max/Metal pixel checks, native crop inspection and isolated GPU/CPU
intervals. Browser/iPhone execution, sustained pacing/thermal cost and device
memory remain unqualified; repository checks cannot establish those results.

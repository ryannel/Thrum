# M1 renderer foundation

3 November 2026. Selected bounded implementation under the active
[V1 order](../../BUILD-PLAN.md#active-order),
[C06–C08](../CONTRACTS.md#c06-gpu-execution-and-lifetime--v1) and the
[render specification](../RENDER-SPEC.md). This records the implemented foundation,
not a replacement renderer target or M2 qualification.

## Prepared interaction and authority

`thrum-render` receives the V1 executor's existing device, queue and generation;
it creates no adapter/device and submits no private work. Async preparation
validates/packs tables, uploads default textures and creates the finite required
opaque/static pipeline under a completed validation scope. Encoding never builds
pipelines. The host submits and polls its device.

The in-memory diagnostic draws indexed procedural quads with distinct material
rows into linear RGBA8Unorm. It is unlit and proves no PBR, clustered lighting,
CSM, exposure, display conversion, motion, cooked content, surface recovery or
running game. M2 owns the lit interaction; A6 connects F4 products. R3 remains parked.

## Graph and lifetime

Declarations name graph-owned identities, immutable buffer/texture descriptors,
transient/imported/persistent authority, shared/view scope, exact accesses,
clear/load/store behavior, prepared pipeline generation and external effects.
Buffer ranges are bytes; texture ranges flatten complete mip/layers, conservatively
unifying depth/stencil aspects. Storage writers promise to define every selected
byte. Undefined reads, loads after discard, conflicting same-pass write usages,
foreign graph IDs and absent/stale required tuples refuse before execution.

Declaration order is execution order. Overlapping access pairs involving a write
produce hazard reasons/dependencies; disjoint and read/read accesses produce none.
wgpu owns backend transitions. First/last uses permit transient slot reuse only
for exactly equal descriptors after strictly earlier last use. This is a logical
allocation plan, not physical aliasing or measured residency. Persistent owners
and epochs never enter the pool. Actual retirement also waits for CPU readers and
GPU completion; removing a pass cannot release a history.

Compile once at setup or structure/descriptor/generation changes and retain the
plan across frames. The synthetic GPU test compiles before its repeated-frame
loop; encode creates no graph or resource identities.

## ABI, histories and timing

[Revision 2](../RENDER-SPEC.md#2-material-closure-and-binding-abi) uses 80-byte
material and 192-byte instance records. Checked scalar packers write explicit
little-endian offsets into zeroed storage, without unsafe casts. Material/layer
references and queried buffer capacities refuse instead of truncating. Group 2
binds the material storage table, four typed `texture_2d_array` classes and one
sampler; group 3 binds indexed instances. Base/emissive texture arrays decode sRGB, normal/ORM arrays stay linear; material record factors are scene-linear.
Each class shares dimensions/format/mips and reserves default layer zero. No
binding-array feature or shader per material count is required. The diagnostic
admits only default layers, static instances and zero semantic flags.

Submitted and accepted-history metadata remain separate. Skips do not advance;
view/provider/grid/device/content/profile/origin/correspondence changes reset.
Unknown successor acceptance invalidates usable history. Queue completion cannot
certify image acceptance. GPU histories and temporal reconstruction remain unbuilt.

Timing reserves two queries per pass. Unsupported enabled-device features allocate
no query/readback resources and fabricate no zero durations. The initial real
Metal test reproduced the known [same-submission stale-counter fault](../../evidence/render/R0-FXAA-1080P-TIMESTAMP-2026-11-03.md).
The selected nonblocking protocol records render completion, then lets a later
host-owned submission resolve/copy counters only after the callback arrives.
`try_recv` collection never waits or submits. Pending slots cannot be overwritten;
old device generations cannot publish results. Zero/error/inverted counters refuse.
These are GPU pass intervals, separate from CPU encoding and display latency.

## Cost and verification limits

Graph analysis is quadratic in declared accesses, independent of instance count,
and runs during preparation. An optimal alias solver/topological scheduler lacks
a current consumer; revisit if measured many-pass preparation becomes material.

Uploads cost 80 bytes/material and 192 bytes/instance: the 10,000-instance fixture
has 1,920,000 instance bytes and 160 material bytes, plus four default texels. One
instanced draw removes per-object CPU bindings; GPU record reads/sampling remain.
These are logical payloads, not measured bandwidth/residency. Culling, indirect
draws and changed-range uploads remain future ABI consumers. Timing adds 16 bytes
per pass in each resolve/readback buffer and bounded names/results; it is not a
zero-allocation profiler. Each enabled batch currently allocates one label per
pass, two completion channels and a result vector. The host bounds outstanding
batches. Measure CPU encode with timing enabled and disabled before M2 claims
profiler overhead is negligible.

CPU tests cover hazards, subresources, initialization/discard, exact transient
reuse, persistent exclusion, closure/generation, ABI offsets/domains/references,
capacity and history acceptance/reset. An ignored real-adapter test checks padded
65×33 pixel readback, material/instance strides, repeated batches, stale generation,
pending-slot refusal, 10,000 instances and disabled timing:

```sh
PATH="$(dirname "$(rustup which --toolchain 1.99.1 rustc)"):$PATH" RUSTC_WRAPPER= \
  cargo test -p thrum-render --offline --locked --test gpu -- --ignored --nocapture
```

The explicit test fails when no adapter is available. Apple M3 Max/Metal correctness has run;
browser/iPhone, sustained pacing/thermal cost, cook closure, lit quality and device
memory remain unqualified. Repository checks cannot promote those measurements.

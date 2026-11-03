# R0 resource lifetime schedule — selected synthetic-proof candidate, 3 November 2026

This is the first concrete V1 allocation/reuse schedule for the bounded R0
fixture in [RENDER-SPEC §8](../RENDER-SPEC.md#8-r0-first-rendered-game-contract-and-proof).
It resolves the [render/platform limit review's](LIMIT-REVIEW-RENDER-PLATFORM-2026-11-03.md)
cross-frame reuse question for a synthetic proof, not a production renderer or
device-memory promise. F4 admitted product binding and the real V1/V4 graph
remain prerequisites for a ready R0 game. The schedule selects one queue and
one spatial view at a time; wider multi-queue, temporal, water and R3 graphs
need their own lifetime account.

## Fixed first proof and output extent

The first synthetic proof uses an exact 1280×720 internal and output extent,
one perspective view, one 2048² D32 Sun map, one D32 view depth, one
RGBA16Float scene-linear target and one RGBA16Float sRGB-encoded scene target
for FXAA. FXAA samples the encoded target, decodes to display-linear and
writes the selected sRGB output once. The isolated test excludes UI, so no
blend operation reads that encoded output as if it were linear. A later UI
fixture must select a linear composition/attachment conversion path and add
its target cost before claiming the optional UI tuple. The isolated harness
provides one 1280×720 RGBA8Unorm output texture; the output shader explicitly
encodes sRGB. Its 3,686,400 logical bytes are charged separately from
V1-owned textures. A real acquired surface, its buffering and any
capture/readback allocations belong
in a separate platform/output account. Production chooses a supported surface
format and exactly one sRGB encoding path. An actual window whose physical
output extent differs from the admitted fixture selects
a separately checked resize/profile transaction; it is not silently charged
as 720p. A hidden/zero-sized surface skips the view and cannot advance its
submitted sample. Adapter support, requested device limits/features and the
resulting device's actual limits/features are checked before allocation.
Exact output and capture extents and row-padded readback bytes belong to their
respective transactions; the first schedule proof does not enable capture.
For its 1080p resize case the harness allocates a new exact 1920×1080 output
texture (8,294,400 logical bytes) and retains the old 3,686,400-byte output
until every old submission has completed. Its 11,980,800-byte overlap is a
separate harness charge. The proof-local loading diagnostic is a complete
mock output state at the new extent, owned by the harness and independent of
V1 scratch; it asserts lifecycle completeness, not pixels or real platform
availability. Device loss instead reports a host/platform failure until a
new device and output are prepared.
This fixes the first proof's rounding to integer physical pixels without
making 720p an engine-wide maximum or claiming target support.

| V1-owned texture | First write → last GPU read | Logical bytes at 720p | Retention |
| --- | --- | ---: | --- |
| Sun shadow D32, 2048² | Shadow → forward | 16,777,216 (16 MiB) | Queue-ordered scratch; same-descriptor light/source changes overwrite it only after all prior uses are submitted. |
| View depth D32, 1280×720 | Depth → forward | 3,686,400 (3.516 MiB) | Queue-ordered scratch for the view extent. |
| Scene linear RGBA16Float | Forward → tone | 7,372,800 (7.031 MiB) | Queue-ordered scratch for the view extent. |
| Encoded scene RGBA16Float | Tone → FXAA | 7,372,800 (7.031 MiB) | Queue-ordered scratch for the view extent. |

The four textures total **35,209,216 bytes (33.578 MiB)** before alignment,
driver overhead, surface buffers, uploads, products, pipelines or readback.
The last two targets overlap in the tone pass, so equal format does not let
them alias. Shadow and view depth have different extents. The first graph
therefore assumes no within-frame texture alias; a later allocator may prove
one only from exact nonoverlapping subresource lifetimes and descriptors.
At 1920×1080 the same texture set totals 58,249,216 bytes (55.550 MiB).
The implementation derives either subtotal with checked arithmetic rather
than reusing hard-coded values.

## Queue-ordered reuse versus retirement

V1 submits all work using one synchronized queue in pass order. The first
fixture has two credits covering the entire encoded view packet,
its product leases, upload/staging, output and any diagnostic work. V1 reserves
one credit *before* extraction and staging; a canceled or failed unsubmitted
packet releases that reservation, while a successful submit converts it into
a generation/serial-scoped completion hold. No third packet or new staging
allocation is admitted while both credits are held. A full queue defers the
view before extraction/submission, preserves its last
submitted-sample record and does not stall the authority clock. This is a
bounded operational control, not an SDK-wide capacity. Each successful
submission gets a checked `(device generation, queue serial)` identity and
retains its reserved credit until that serial's completion. A later completion
watermark discharges all earlier same-queue serials even if their callbacks
are delayed **only when the queue API certifies all work through that serial
is complete**; a raw native command buffer's individual completion status
does not serve as that watermark. Old-generation callbacks cannot release
new-generation credit.

A later submission may write the same scratch texture after the prior
submission's last read, even while its completion token is pending, if all
old uses were submitted first and no older packet can submit afterward. This
requires hazard-ordered conflicting uses of the **same tracked allocation**;
it does not require independent command buffers to run physically serially.
The pinned wgpu-hal 30.0.1 Metal backend explicitly allows independent
command buffers on one queue to overlap when they share no tracked resource.
Thrum must not infer a global serial GPU schedule or reuse an untracked heap
alias from this rule. [Apple's Metal hazard tracking](https://developer.apple.com/documentation/metal/mtlhazardtrackingmode),
[pinned wgpu-hal Metal queue implementation](https://github.com/gfx-rs/wgpu/blob/v30.0.1/wgpu-hal/src/metal/mod.rs).
This is *queue-ordered reuse*,
not reclamation of the resource. The object and its logical byte charge stay
live. [WebGPU's queue timeline](https://gpuweb.github.io/gpuweb/#queue-timeline)
and [wgpu-hal's Queue contract](https://docs.rs/wgpu-hal/latest/wgpu_hal/trait.Queue.html)
state the ordering; the eventual pinned Thrum wgpu/backend version still needs
an executable proof. CPU writes to upload buffers, readback, a second queue,
external surface ownership or a resource retained by another consumer do not
inherit this reuse rule. `Queue::write_buffer` copies caller bytes into staging
and schedules transfer at submit; that staging has its own lifetime/cost,
separate from the four textures. [wgpu Queue](https://docs.rs/wgpu/latest/wgpu/struct.Queue.html).

For a resize or changed descriptor, the old object may still be used by an
unfinished submission. It remains charged with its old generation until the
relevant GPU completion and any CPU/readback lease end. New differently sized
scratch must be allocated and charged separately, or the new view defers until
old completion and release. Completion alone cannot retire a still-prepared
old graph. A device-generation change invalidates all old GPU handles and
readiness; V1 rejects stale callbacks and rebuilds from retained F4 inputs
on a new device. Terminal device loss discharges GPU submission holds for the
lost generation under the platform's loss event, while separate CPU/readback
leases remain until their owners release them. Product buffers/textures,
shadow source data and pipeline objects retain their own F4/V1 leases; this texture schedule does not make
them free. Submission completion is a lifetime fact, not proof that a pixel
was displayed or a temporal history sample accepted.

At 720p the three view textures sum to 18,432,000 bytes (17.578 MiB).
At 1920×1080 they sum to 41,472,000 bytes (39.551 MiB). If the shadow map is
compatible and queue-ordered across a 720p→1080p transition, an unfinished
old frame plus allocated new view textures costs 76,681,216 logical bytes
(73.129 MiB) before surface and other owners. A target that cannot hold that
overlap defers the resize without evicting the unfinished old frame or
allocating half a new graph. If the old graph remains prepared, retry still
needs the full overlap charge. To progress when each generation fits
separately, V1 enters a visible loading transition: stop old submissions,
drain *all* old GPU and CPU leases, release old prepared view targets, then
prepare the new graph. This path gives up old-graph rollback. The synthetic
proof uses the independently charged harness diagnostic above. A real V1/V4
transition still needs selected platform ownership, output format and failure
behavior before it is an implemented promise. A stalled GPU
completion has no guaranteed finite wait. A descriptor-incompatible shadow
map adds a new 16 MiB while the old one is retained; a mere source/light
generation change can overwrite the same-descriptor scratch in queue order.

## Preparation and failure transaction

F4 enumerates the exact R0 roles and products; V1 checks their descriptors,
format/usage/limits, pass read-before-write and last uses before allocation.
The finite graph is cooked/prepared once per recipe/extent generation. Each
frame references prepared logical slots, so normal submission does not solve
an interval graph, scan a dynamic resource registry or allocate these textures.
V1 keeps one fixed small slot table per prepared extent and a completion
ledger for retired generations. A slot's allocation key includes device
generation, dimension/extent/depth or layers, format and view formats, union
usage, mip count and sample count; view/source identity is a separate content
key. The first fixture uses one view and one shared Sun slot. Two simultaneous
views require either distinct view slots or an explicitly ordered compatible
pool and a separate peak proof; they do not inherit the one-view subtotal.
Checked `u64` multiplication accounts extent, format bytes, sample count and
all simultaneously live generations before creating any new texture. Logical
owner budget is a finite policy estimate, not a physical device-memory bound.

`prepare(new)` first validates every required role and computes a complete
new-plus-retained logical charge. If it exceeds the selected *owner* budget,
defer before creation. On ordinary allocation/pipeline failure with a healthy
device, discard all pending handles and keep the old prepared generation.
After the stop/release loading transition there is no old graph to keep; if
the device is lost, *no* old GPU handle remains valid. Both cases need the
separately owned loading/diagnostic path or an explicit platform failure.
Only after all required objects are prepared does V1 publish the new
generation at a safe barrier. An unsubmitted or failed packet never commits a
V4 sample. A submitted packet pins its F4 products and old generation until
the GPU completion token and other leases release. No source revision, cook
key or physical gameplay state changes because a resize defers.

## Synthetic proof and remaining gates

The standalone [Rust oracle](../../evidence/render/R0-RESOURCE-LIFETIME-ORACLE.rs)
and [evidence record](../../evidence/render/R0-RESOURCE-LIFETIME-ORACLE-2026-11-03.md)
exercise four typed texture roles, checked logical bytes, one scratch set
across two held submissions, a refused third credit, an old/new resize
overlap and 60 MiB V1-owner loading path. They distinguish healthy-device
rollback from failure after old release; preserve CPU lease reasons across
device loss; reject stale device reservations/completions; and check the
separate output overlap. The oracle is a *logical state model*, not V1 code.
Its output diagnostic is a proof-local mock, not a rendered pixel. The numeric
60 MiB is chosen solely to place the test between new-only and overlap costs.

The oracle does not yet model F4 product/staging bytes, pass first/last-use
validation, partial GPU allocation failure, an incompatible shadow descriptor,
two simultaneous views, capture/readback or a real platform surface. Those
remain build/qualification cases, not passed evidence. The real-device run
must capture allocation identity, submission order/completion, old/new
overlap and actual high-water alongside frame time, and compare target budget
and full output/surface cost separately. Full four-slot backend behaviour,
display correctness and ordinary-game visuals remain unrun.

A [narrow public-wgpu Metal diagnostic](../../evidence/render/R0-GPU-LIFETIME-PUBLIC-DIAGNOSTIC-2026-11-03.md)
subsequently verified exact red/blue readbacks after two submissions reused
one tracked 720p texture, plus a green 1080p new texture. It checks every
pixel after completion. It does **not** hold genuine GPU-incomplete old work,
exercise these four scratch roles, or measure driver memory. Those parts of
the backend premise remain unrun.

A later [strict-event Metal diagnostic](../../evidence/render/R0-GPU-INCOMPLETE-LIFETIME-2026-11-03.md)
held three submitted command buffers GPU-incomplete across allocation of a
separate 1080p texture. A/B used one tracked 720p allocation in queue order;
after gate release, independent native tail events and exact full-pixel
readbacks passed in three fresh processes. This qualifies an unfinished-old
submission and old/new logical ownership overlap for that one texture on
Apple M3 Max. It deliberately makes three submissions outside this schedule's
two-credit policy and does not exercise the four-role graph, F4 leases,
retirement, surface ownership or physical memory high-water. The native
gate adds synchronization work and gives no ordinary-frame timing evidence.

A separate [one-frame four-role synthetic Metal chain](../../evidence/render/R0-FOUR-ROLE-SYNTHETIC-2026-11-03.md)
then exercised the selected D32 shadow/view, RGBA16Float linear/encoded and
independent RGBA8 output descriptors through actual read dependencies. A
half-aware CPU oracle matched every one of 921,600 output pixels. This
validates one format/usage/pass combination, but the fixture has no held
frames, resize, two-credit admission, F4 leases, real surface or physical
memory measurement. Its uniform FXAA case is not an edge-quality proof.

The later [held four-role diagnostic](../../evidence/render/R0-FOUR-ROLE-HELD-LIFETIME-2026-11-03.md)
combined those roles with two genuinely incomplete 720p submissions, early
third-credit refusal, 1080p old/new graph overlap, A-only credit reclaim,
distinct native tails and exact A/B/C pixel checks. Old application view and
output owners retired after GPU completion and explicit CPU leases. Its
104,329,312 B selected charge includes separately retained packet readbacks
and conservative cumulative initialization staging; it is a logical
fixed-scenario account, not a driver-residency measurement. F4 leases,
authored scenes, real surface, failure rollback, device-loss handling and
ordinary-frame cost remain open.

# Apple tile recipe and backend dataflow (G3t)

9 November 2026. Planned, unbuilt except the carried creation-counter and traffic-probe work.
[Brief](APPLE-TILE-BRIEF.md) · [Plan](APPLE-TILE-PLAN.md) · [Done when](APPLE-TILE-DONE.json) ·
[Boundary](../decisions/G3T-METAL-BOUNDARY.md) · [G4 consumer](APPLE-PRESENTATION.md).
C04/C06/C07/C08/C12 in [CONTRACTS](../CONTRACTS.md) govern.
The [design review](../../evidence/APPLE-TILE/design-review.md) requested nine changes at `74be9f89`;
this revision addresses them. Independent review of the revised design/JSON is owed. Earlier designs and code were
inspected; this is reconciliation,
not a blind design. Historical reviews remain in the plan log; missing scratch is not current proof.

The planned [T2 capture service](T2-CAPTURE.md) owns submitted-output capture and sequence readback. G3t M0
needs `T2-CAPTURE:M0` and adds its scene-linear comparison stages through the same renderer-owned tap; it does
not implement a rival recorder. Real-display/physical-counter tools retain any attended setup needs explicitly.

## Purpose, range and cues

Use Apple's tile/unified-memory hardware through the existing renderer, with portable as the reference and prepared
fallback. The player sees the same cooked valley and diagnostic field, with intact Sun, stars, local lights,
shadows, mirrored props and interpolation. No new game types or second world, visibility, queue or history authority.

| Cue | Required range and observation | Convincing wrong substitute rejected |
| --- | --- | --- |
| Same scene | Fixed cooked root; moving valley near/far, Sun-facing, local-light/synthetic-mirror (overlay slot 5060) and cut checkpoints; noon and night; native portable vs Apple, Chrome portable | Dropping a pass, darkening overflowing lights or hiding mirrors to improve time |
| Stable motion | Normal-speed frame sequences, cut/reset, changing rows, two slots, dropped views and generation replacement | Matching a still while reusing stale transforms, normals or history |
| Prepared execution | Day/night/capture variants and fallback already prepared; no pipeline creation in play | Moving compilation into the first use of a rare variant |
| Honest hardware use | Actual storage modes, upload calls, eligible bytes, full passes and fallback counts | Calling a recipe Apple tile while always executing portable or equating logical bytes with DRAM bytes |

The quality bar is the same intact, temporally stable scene expected of a shipped high-end game; portable supplies
matched image semantics, not an assertion that the valley's artistic polish is finished. G3s sunset diagnosis stays
parked. Mac cadence is still PLATFORM-RUNTIME-MAC M2; offscreen motion establishes neither display cadence nor latency.
Physical iPhone timing/loss is owed; ios-sim is function only.

Commissioned: registry recipes, graph-derived transient/memoryless attachments, merged scene/present execution,
direct output, slot-safe uploads, shared eligible transients, compact internal transport, GPU light assignment,
existing visibility integration, prepared native segments and paired evidence. G4 owns TAAU/MetalFX/EDR/governor,
real render-scale transitions and pipeline archives. The complete Metal recorder, Metal 4 queue, tile shaders,
Android recipe and further quantization remain later work.

## Research and selection

Inspected on 9 November 2026; these are API/architecture evidence, not measurements of Thrum:

- [wgpu 30.0.1 TextureUsages](https://docs.rs/wgpu/30.0.1/wgpu/struct.TextureUsages.html#associatedconstant.TRANSIENT_ATTACHMENT),
  `TRANSIENT_ATTACHMENT`: attachment-only usage, clear/dont-care load and discard store. This permits the selected
  public-API path only when the compiled variant has no later reader. More recent API changes do not change our lockfile.
- [Apple WWDC20 10603](https://developer.apple.com/videos/play/wwdc2020/10603/), transcript's memory-bandwidth discussion:
  store only future pass inputs; compression and caches separate logical traffic from device-memory traffic. The
  [Thrum probe](../../evidence/render/G3T-TRAFFIC-PROBE-2026-11-08.md) supplies the actual per-encoder counter method.
- [wgpu 30.0.1 Device](https://docs.rs/wgpu/30.0.1/wgpu/struct.Device.html#method.create_buffer_from_hal),
  `create_buffer_from_hal` safety: same device, matching descriptor, initialized, nonzero storage. Shared Metal buffers
  are a selected candidate with explicit import and lifetime proofs, not assumed safe from the signature alone.
- [wgpu 30.0.1 CommandEncoder](https://docs.rs/wgpu/30.0.1/wgpu/struct.CommandEncoder.html#method.as_hal_mut),
  raw encoder access and safety: backend may be absent, raw work cannot destroy/end the command buffer or overlap
  wgpu recording. The locked core's Raw/Wgpu recording distinction requires a dedicated whole raw encoder.

- [wgpu 30.0.1 Queue::write_buffer](https://docs.rs/wgpu/30.0.1/wgpu/struct.Queue.html#method.write_buffer),
  performance considerations, inspected for this revision: writes stage immediately and execute before the next
  submission's command buffers. This supports retaining portable queue-ordered single buffers; pointer writes have
  different lifetime requirements.

Alternatives: keep public wgpu wherever expressive; use an isolated backend for shared-buffer import and native
features only. Reject a mixed raw/wgpu encoder, raw drawable writes, a second queue and an early full native recorder.
A public `map_async` upload fallback is feasible only with unmap before submit; ordinary `write_buffer` remains the
prepared safe fallback. If the ring cannot prove safety or a benefit, retain that fallback and record the candidate
as unpromoted. Do not turn an unsuccessful optimization into a V1 exit change.

## Owners and dependency boundary

| State | Single owner | G3t action / consumers |
| --- | --- | --- |
| Device, surface, lifecycle lease | Platform (`thrum-app`, iOS shell, browser host) | Read generation facts; retain Mac/iOS/background/loss rules |
| Hardware recipe | `thrum-render::capabilities` | Capture once per adapter generation; passes never probe |
| Passes, hazards, storage, phase partition | `thrum-render::graph` | Extend one compiled graph; atmosphere and contributions keep declaring accesses |
| Slots, submit, completion, retirement | `FrameHost` | Add resource handoffs without another queue or completion authority |
| World and publication | Core/W1 | Read committed pages; never change simulation for graphics quality |
| Meshes, membership, facing and visibility | V2 in `thrum-render` | M9 defines one shared class/range mechanism for supported draws and experimental buckets; no second classifier/culler |
| Light and material meaning | V3 existing ABI/cluster oracle | Preserve admission and overflow ranking |
| Accepted sample/history | Existing C07 history owners; G4 reconstruction | Commit only submitted images, reset on invalidity |
| Apple buffers/providers | Isolated renderer `apple_tile` backend | Generation-owned resources; executor supplies recording/completion |
| Route/content | V1 game/G7 | Reuse fixed route and cooked inputs; add G3t proof fixtures without changing G7 baseline |

The existing runtime/sky/content/visibility capabilities are present on main; no new cross-module implementation is
needed for M0. M1 additionally needs `T2-CAPTURE:M1` for new Chrome image proofs and `T2-CAPTURE:M2` for simulator
output proofs; M0's existing Chrome smoke stays distinct. M2 consumes the already-done `PLATFORM-RUNTIME-MAC:M1` loss boundary. Mac M2 is deliberately not a
`needs` link. M9 supplies the shared V2 class/range mechanism within this commission and migrates its bucket consumer,
without promoting the experimental mixed-mesh route. No nonexistent external range milestone is assumed.
Mac M2 qualification is rerun after G3t M2/M3 land: any earlier motion/cadence result stays historical.
G3t M2/M3 each require a functional real Mac window run; display cadence remains a separate Mac M2 proof. G4 first
needs M0 proof tools, M1 eligibility, M3 present input and M11 native
providers. G3t does not wait back on G4; M11 uses bounded provider extent fixtures, and M12 records current readers
and two fixed extents. G4 later reruns these with real reconstruction readers and live scale transitions.

## Mechanisms

**Registry and preparation.** Extend `DeviceCapabilities` with resolved `Portable`/`AppleTile`, stable digest and
feature facts. Apple eligibility requires Metal, unified memory and the supported transient path; unavailable native
facts select portable. Capture native feature facts once at platform creation, including feature availability for G4;
per-display refresh/headroom remains platform state. `THRUM_RECIPE=portable|apple-tile` is a diagnostic override:
reject unknown or unsupported forced selections, never silently mislabel them. Chrome always resolves portable.
Apple tile is opt-in during construction; the normal default remains portable until integrated paired evidence
justifies promotion. Eligibility alone is not a measured win. All selected and fallback pipelines/bind groups/variant
allocations are prepared before readiness. The resident fallback set is explicit:

- Common scene assets, stored HDR/motion/history, uploads and compatible bind groups are shared, counted once.
- Every selected day/star/capture variant's unique attachment descriptor is resident before readiness; a stored
  star/capture depth and a memoryless day depth may therefore both be resident. Count both, never claim their
  per-pass traffic saving as an equal resident-memory saving.
- A stored portable twin of a memoryless-only attachment is not kept solely for a force override. Unsupported
  storage is resolved during preparation; changing the whole recipe uses generation replacement, with the new
  complete allocation set ready before publication and old/new overlap reserved through completion.
- Same-generation optional upload rejection uses queue writes to the same `COPY_DST` buffers. Optional native
  provider rejection uses its already resident portable twin and required outputs/history/bind groups; count every
  unique twin resource. No lazy allocation or compilation is allowed on rejection.

D1.2/D1.3 report unique resident bytes by selected variant, fallback-only resources and retained generations for
both recipes. A complete resident total can show no memory win even when logical attachment traffic falls.
Diagnostics stay
opt-in, following the existing `creation-counters` feature and generation-owned `DeviceSite`.

**Graph storage.** Derive eligibility independently for every prepared day/star/capture variant from compiled live
uses. Only transient render-only attachments with clear/dont-care and discard, no explicit store and no later reader
are eligible; history, persistent, imported, sampled, copied or native-written resources are not. Use the locked
wgpu descriptor restrictions. Allocate eligible Apple textures with `TRANSIENT_ATTACHMENT`; stored textures keep their
required usages. Inspect actual Metal storage modes. Day depth can be eligible; stars, occlusion or G4 readers make
it stored. Motion's explicit store remains ineligible until its producer contract changes. Capture must not widen
normal-path usages. Report reasons per resource/variant and prepare all variants before play.

**One graph, late surface.** Compile scene and present together per generation/variant, with an execute-time surface
binding. The pre-acquire partition contains all scene work; the post-acquire partition contains the surface consumer
and its dependent timing/readback epilogue. Reject scene work placed behind acquisition without a surface dependency.
At first retain the current tone intermediate. In M3 move tone/display encoding into present: one scene-linear HDR
input plus the matching exposure and display headroom writes the chosen surface format. Remove the steady Rgba8
intermediate; capture reads a prepared branch from the same linear input, never a raw drawable. Pass-name timing
replaces positional slots; update app, iOS, browser and capture consumers together.

Healthy acquisition refusal still drops scene views, flushes pending uploads through the existing token path, and
does not commit image history. Observed device loss abandons without any old-queue call. A background iOS lease
refuses submission and resets affected histories. Resize/replacement uses platform generation admission, retains old
resources until completion and rejects stale callbacks. M2 must retain these distinctions under the merged graph.

**Uploads.** Portable retains one queue-ordered buffer per instance, directional/local-light, cluster and bounds
class and its existing prepared bind groups. Queue writes execute before the next submission and after prior
submitted uses; they do not require per-slot copies. Retain its existing dirty tracking and refusal/flush rules.
Only the Apple recipe gives both frame slots independent buffers/bind groups for pointer-written classes. Track each
slot's last applied source/mapping/visual generation and accumulate every compatible
dirty change since it was written. Skips, refusals or a busy slot do not advance that cursor. Gaps, replay timeline or
mapping changes force complete reconciliation. A two-frame dirty union alone is insufficient after dropped frames.

Portable uses `write_buffer` without per-slot duplication. The Apple candidate imports initialized nonzero shared
`MTLBuffer`s from the same
Metal device, with matching size/usages and `COPY_DST` for fallback. CPU contents writes require completion of every
prior GPU read of that slot. A busy slot uses queue-ordered `write_buffer` with a counter; it never writes the pointer.
Do not reuse native storage or release it before generation/slot completion. Register real dependencies and narrowly
review/document unsafe access when implemented; this design grants no blanket lint exception.

**Transient sharing.** Share only scratch whose entire previous use is ordered before its next write on the one
queue. Slot N's passes must finish their reads before slot N+1 overwrites. History, retained capture/readback inputs,
Hi-Z/prepass state and outputs exposed to other consumers retain their own ownership unless that lifetime is proven.
A held-submission witness must include the final present/capture consumer. Report the eligible subset's saved
footprint; do not promise half of total renderer memory or call footprint a bandwidth saving.

**Compact instance transport.** Keep public `InstanceRecord` and its admitted 192-byte pack semantics. Introduce a
private 128-byte synthetic GPU row: three affine rows (48 B), previous simulation translation (12 B), supplied normal
columns (48 B), four IDs (16 B), padding (4 B). Offsets are affine 0, prior translation 48, padding 60, normals 64, IDs 112, stride 128.
WGSL members must realize those offsets/stride explicitly, with CPU
layout tests and shader reads. Current transforms are already required affine by the synthetic admission path.
Preserve all supplied normal values used by shading, current identity/generation and translation interpolation;
accepted-image motion still reads the separate submitted transform history. Update forward, shadow, prepass,
visibility, history and upload together, including private bucket consumers. The cooked placement path remains
separate. Reconstructing arbitrary supplied normals from the model matrix is invalid; an 80-byte restricted profile
would need a separately admitted contract and is deferred. Do not silently generalize translation interpolation to
rotation/scale interpolation.

**Lights.** Extend the existing one-invocation-per-cluster GPU builder beyond its current four-light path. Keep the
32×18×16 grid, declared source capacity and existing conservative projected bounds. At overflow keep the strongest
32 by the CPU oracle's score (maximum RGB candela divided by max(cluster-centre squared distance, 0.0001)); ties
prefer lower source index, output IDs sorted ascending, unused entries deterministic. Preserve orthographic exact
intersection and perspective bounds, zero lights and zero intensity. CPU supplies per-light bounds/inputs, not a
dense cluster list. GPU top-32 selection is bounded (insertion into 32 candidates per cell); use the same f32
operations/order and qualify score boundaries/ties against the CPU oracle. A device/compiler disagreement selects
the prepared CPU oracle path with a visible fallback reason; it cannot claim GPU qualification. Test clustered
membership exactly and resulting lighting against the existing image path. All named 0/4/16/32/64-light cases,
including all-overlap overflow and tie cases, must qualify on the Mac GPU
for M8 to close. Fallback is safe runtime behavior, not a substitute for an overflow qualification; an unqualified
case leaves D8.1 open and requires re-planning if the mechanism cannot satisfy it. Record CPU bounds,
validation/admission and fallback cost as well as GPU work.

**Mirrors.** M9 owns delivery of a shared mechanism *inside V2*, using the existing CPU f64 determinant and
mesh/membership state. Refactor the experimental bucket classifier/layout into that one authority and migrate both
supported direct/frustum/occlusion consumers and the private bucket consumer. G3t does not create a separate
classifier or culler, and this migration does not qualify mixed-mesh rendering. Record source review across upload,
buckets, mesh route, forward, camera/prepass and shadow consumers before claiming D9.1.

Facing sign and admitted raster policy are separate. The supported legacy route currently uses scene-wide cull-none
when any active row is mirrored or degenerate; preserve that policy for the affected order group, including ordinary
rows. Open, unknown-closure, explicitly two-sided and near-singular meshes retain two-sided rendering. Closure alone
does not authorize replacing established two-sided semantics (including views inside closed geometry). Opposite
front faces with back-face culling apply only to already admitted one-sided semantics; no newly narrowed admission
is introduced to make this optimization pass. Mirrored legacy ranges may therefore remain two-sided, with no culling
win claimed. A cull-none substitution is a failure only in an admitted one-sided witness.

The shared range description preserves each path's established direct order, including equal-depth ties: supported
legacy draws use stable runs of existing physical-slot order rather than globally regrouping by class. Coalesce
adjacent runs with identical admitted pipeline state even when determinant signs differ: a legacy scene-wide
two-sided group can remain one draw. Only genuinely different admitted states require additional runs. The private
bucket path retains its documented bucket/logical order. Existing visibility compacts within those ranges; shadow
membership still includes off-camera casters. Compare each to its corresponding direct oracle and the supported
pre-change baseline. Runtime mirror proof uses synthetic overlay slot 5060 and dedicated open/back-side/interior
fixtures. Cooked placements refuse negative scale and the cook bakes mirrored winding; do not claim they exercise
runtime mirror classification. Count all occupied runs/buckets and CPU range maintenance, including the potential
extra draws from stable runs. If no semantic-preserving win is measured, keep acceleration unpromoted.

**Native feature handoff.** C06 and the boundary decision define the planned whole-encoder segment; independent
review is required before its implementation. Close the producing wgpu encoder, record only raw commands in a
separate wgpu-owned encoder, then resume a fresh wgpu encoder. Submit the ordered list once on the coordinator's
queue. Providers never submit, end/commit command buffers or touch the surface. Graph declares all reads/writes.
Tracked same-queue resources provide producer→native→consumer ordering; qualify that assumption on locked wgpu/Metal
with a stateful storage-writing stand-in and wgpu twin before integrating MetalFX.

Initialize native-written textures once through wgpu; never alias/discard them. The generation retains every native
resource until slot completion. Provider construction declares formats, extents, usages, scale range and portable
twin before readiness. Provider-internal history resets after any encoded-but-discarded frame, failed submission,
cut or incompatible generation. Keep the ordinary presentation-drop contract for native frames too: discard and
reset instead of adding submit-without-present. Loss still forbids even a flush; iOS background still forbids submit.
Raw command-buffer errors join the normal device-loss path via generation-tagged completion. Report native GPU
start/end times separately from wgpu query times. A rejected optional provider uses its prepared twin. M10's blend
stand-in is composition proof only; M11 covers provider replacement extents without waiting for G4's live transitions.

**Pipeline preparation boundary (formerly G3t seam 5).** G4 M9 records launch-to-ready stages; G4 M10 owns offline
metallibs/passthrough reflection, parallel pre-warm and a pinned Metal pipeline-cache/archive patch. Preserve
per-pipeline cook/compatibility keys, matching runtime MSL demand and strict archive-miss qualification on each shipped
device. A rejected cache rebuilds before readiness. Upstream contribution is later and does not gate V1; the owner's
per-post authorization remains required. G3t's creation authority/registry and generation retirement are shared
inputs; this plan does not pre-approve a dependency patch, public post or library change.

## Worked execution and creator cases

1. Cook the unchanged valley, capture its root and prepare device/recipe/variants. Attach the cooked contribution
   through the existing scene seam; no authoring dependency enters the runtime. A live asset replacement updates the
   contribution generation and invalidates only matching derived state under C05/C07.
2. Admit a committed sample. Reconcile that slot's dirty data, then write its completed-slot buffers. Encode atmosphere,
   contribution, Sun shadows, air shadows, light lists, existing visibility/prepass, forward/sky/stars and accepted
   transform capture in graph order. Later G4 inserts its reconstruction consumer before present.
3. Acquire late, bind the actual surface, present the matching linear input/exposure, submit once. Only successful
   submitted work advances image history; completion retires slot/generation resources.
4. If acquire fails after native encoding, drop the view, perform only the healthy upload flush and mark the provider
   reset before reuse. If loss was observed, abandon without queue calls instead. If iOS suspended, refuse submission
   under its lease. Retained world/input/cooked source continues under its existing owner.
5. During generation replacement, prepare registry/resources/providers and fallback off the rendering critical path,
   publish at the existing safe boundary, reset incompatible histories and retire old resources after completion.

## Cost account and proof

At 1920×1080, nominal uncompressed attachment sizes are depth D32 8.29 MB, HDR RGBA16F 16.59 MB, motion RG16F
8.29 MB and Rgba8 intermediate 8.29 MB. Removing the intermediate removes its logical write/read (16.59 MB/frame).
Memoryless depth avoids a logical store only in variants without readers; actual physical savings depend on
compression/caches. Only Apple pointer-written frame buffers add one copy of each class; portable retains one
queue-ordered copy. At 10k rows, private 128-byte
transport is 1.28 MB instead of 1.92 MB; Apple two-slot storage costs 2.56 MB plus other classes; portable costs
1.28 MB. Worst-case dirty upload is every
active row; ordinary dirty work is proportional to accumulated changed ranges, bounded by full reconciliation.
Alternating-slot reconciliation can write a persistent change into both Apple
slots (about twice portable upload bytes under steady dirty motion); report actual recipe and pre-change portable
byte/range deltas as tripwires, including full-rebuild bursts. No portable upload-byte increase is assumed beneficial.

9,216 clusters × L lights gives 589,824 candidate tests at L=64. Bounded top-32 insertion adds at most 32 comparisons
per accepted candidate; forward evaluates at most 32 references per covered cluster. Fixed 32-ID storage is about
1.18 MB plus 73,728 B heads, per allocation, before scratch; portable keeps one and Apple duplicates only
pointer-written classes. Record overflow, source counts, CPU bounds, binning and forward
times; no unsupported sub-0.1-ms prediction. Mirrored work follows occupied stable-run/bucket count,
range-maintenance CPU cost and caster coverage.
Sharing only eligible scratch saves one duplicate of that subset. Native segments add encoder/command-buffer owners
and retain provider histories; include them in complete frame time, creation counts and peak generation overlap.
Include all resident prepared variants and portable-twin resources from the registry
rules above, deduplicated by allocation, in both recipe totals; a selected memoryless attachment is not proof that
fallback storage saved memory.

M0 supplies matched baseline identity, same-target repeats, prepared linear-image checkpoints and full-resolution
motion sequences. Existing G7's coarse-grid comparator stays unchanged and is only a route/content smoke test.
G3t retains a full-resolution pre-change native portable baseline at M0 and a Chrome portable baseline after
`T2-CAPTURE:M1`, before M1 changes its rendering, with build/shader/content,
route/sample, format, exposure and H=1 identity. Later milestones compare against that baseline as well as paired
same-build recipes; shared regressions cannot disappear by comparing two changed paths. Chrome is compared to its
own recorded WebGPU baseline, not expected to be bit-identical to a different backend. Baseline artifacts and the
comparator are retained under the storage rules; a new baseline cannot silently replace an earlier one.

The comparison contract is mechanism-specific; no global perceptual allowance is used:

| Milestones | Expected difference and required proof |
| --- | --- |
| M1 storage, M2 scheduling, M4/M5 uploads, M6 sharing, M7 transport, M8 exact light lists | No arithmetic/meaning change: exact scene-linear checkpoint and submitted SDR bytes on the same target, including recipe pairs and the pre-change portable baseline. Membership/IDs remain exact. |
| M3 direct present | Scene-linear bytes are exact. Preserve existing SDR quantization and nearest-source/letterbox mapping in the fused shader, including the sRGB decode/attachment-encode branch, so submitted SDR bytes match the pre-change present oracle exactly. Removing an attachment does not authorize changing tone semantics. |
| M9 ranges | Exact supported-path colour/depth and order-sensitive witnesses against pre-change/direct oracles; preserve admitted raster policy. Recipe pairs are exact. |
| M10/M11 native blend | Only native-vs-WGSL arithmetic of the labelled blend permits at most 1 ULP f16/channel, measured per pixel on its output/history. Pre-blend scene bytes remain exact. Forced portable twin is exact to its own reference; this allowance does not apply to unrelated passes or geometry. |
| M12 integration | Apply the rows above by stage; ordinary paths remain exact. Native stand-in uses only its separately qualified bound. |

M0 must reject negative controls for a phase mismatch, missing contribution, one dropped weak local light in a
saturated cluster, one omitted mirrored cube (slot 5060), and the star layer removed at night. Choose fixed
checkpoints where each contributes nonzero scene-linear pixels, retain per-pixel/crop differences and show that each
fails the applicable comparison, including the blend bound if active. Invisible fixtures are inadequate negative
controls. An exactness failure or omission hidden by the bound is a design issue to resolve before building further,
not a reason to widen a metric. Independent visual review still checks geometry, normals, shadows, lighting and
reset/motion at full resolution; scalar metrics alone cannot close it.

Record source/build/shader/route/cooked hashes, registry digest, device/backend, viewport, mode, warmup, run order and
load. Use paired A/A controls and alternating recipe runs; report p50/p95 CPU upload/encode, full GPU and passes,
allocations, resident fallback/variant bytes and outstanding work. Record validation/admission CPU work separately
from bounds, upload and encode, and record CPU-oracle fallback work/counts for lights and uploads. C12 promotion
requires a sustained paired run: after warmup, repeat each matched route for at least ten minutes (a reproducible
measurement window, not a performance budget), recording actual duration, thermal state at start/throughout/end,
power/load and observed throttling. Repeat if the thermal/load comparison is confounded; unavailable thermal data
leaves performance qualification/promotion open. D5.3 needs this evidence for upload promotion; D12.2 needs it for
integrated promotion. Functional completion may record a safe candidate as unpromoted. M12 also exercises the
representative 10,000-entity field with 1,000–2,000 movers,
full/partial visibility and supported direct/culling paths, so a small valley route cannot stand in for scaling.
Separate display callbacks, submit, GPU completion and physical swaps. Read physical
per-encoder bytes only by the retained Xcode replay method; without a screen/tool slot mark them unmeasured, and do not
block the offscreen functional milestones. No numeric performance/allocation gate; visible hitches, about 25%
comparable regression and unexpected scaling trigger investigation. Accelerated-path promotion requires a measured
win with preserved quality; inconclusive candidates remain unpromoted with prepared fallback.

## Risks and bounded unknowns

- Late readers defeat memoryless: per-variant reasons and later-reader mutation tests expose it.
- Shared resources race: held GPU/capture consumers and changing slot data must show ordered correct output.
- Compact normals/history disappear: supplied-normal, shear/reflection, skipped-frame and submitted-history tests
  compare the wide admitted source to shader output.
- GPU light ranking drifts near ties: exact oracle tests, dense overflow images and explicit CPU fallback reject it.
- Native initialization/lifetimes differ from wgpu assumptions: same-submit stateful twin, discard/reset, held slot,
  extent replacement and generation loss witnesses precede MetalFX.
- Quiet timings, physical traffic and real display cadence may be unavailable. Keep their evidence limits explicit;
  none is supplied by docs checks. Functional real-window runs at M2/M3 cannot be replaced by offscreen output;
  without an unlocked active-display slot those items stay open. M12 records cadence/physical-device rows still owed,
  and Mac M2 must qualify the post-M3 surface path rather than inherit old results.

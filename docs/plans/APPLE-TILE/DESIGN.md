# Apple tile recipe and backend dataflow (G3t)

9 November 2026. Planned, unbuilt except the carried creation-counter and traffic-probe work.
[Brief](APPLE-TILE-BRIEF.md) · [Plan](APPLE-TILE-PLAN.md) · [Done when](APPLE-TILE-DONE.json) ·
[Boundary](../decisions/G3T-METAL-BOUNDARY.md) · [G4 consumer](APPLE-PRESENTATION.md).
C04/C06/C07/C08/C12 in [CONTRACTS](../CONTRACTS.md) govern.
The lead commissioned this scoped re-plan on 9 November after the [second design review](../../evidence/APPLE-TILE/design-review-2.md)
returned seven findings at `224b58a1`. The findings are answered in the plan's disposition table. Independent design
approval is owed to the loop; M0 remains parked and no implementation, qualification or unpark is claimed.
This reconciles current sources rather than claiming a blind design.

[T2 capture](T2-CAPTURE.md) M0 (Mac) and M1 (Chrome) are done. Consume that renderer-owned submitted-output tap;
add scene-linear comparison stages there, never a second recorder. T2 M2's simulator engine capture remains blocked
on [COOKED-DRAW M1](COOKED-DRAW-PLAN.md). It is not on the Mac M3 or G4 native-boundary critical path.

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
parked. Mac M2 owns current-path cadence and Mac M3 its post-G3t-M3 requalification; offscreen motion proves neither.
Physical iPhone timing/loss is owed; ios-sim is function only.

Commissioned now: registry recipes, graph-derived transient/memoryless attachments, one split scene/output graph,
a replaceable pre-tone linear input for G4, pass-name timing, prepared native segments and paired proof. G4 owns
TAAU/MetalFX/EDR/governor, live scale transitions, any new display arithmetic and pipeline archives.

Scope removed from this commission: fused tone/present and removal of the SDR intermediate; M4/M5 pointer uploads
and slot duplication; M6 scratch sharing; M7 compact rows; M8 expanded GPU light assignment; M9 range/classifier
consolidation. Existing queue-ordered uploads, 192-byte admitted rows, CPU overflow selection and V2 raster/order
semantics stay in use. These are named later performance candidates, with no completion or gain claimed here.
The complete Metal recorder, Metal 4 queue, tile shaders, Android recipe and quantization also remain later work.
Retain milestone IDs M0–M3/M10–M12 so consumer references stay stable; M4–M9 definitions are removed, not accepted.

## Research and selection

Inspected on 9 November 2026; these are API/architecture evidence, not measurements of Thrum:

- [wgpu 30.0.1 TextureUsages](https://docs.rs/wgpu/30.0.1/wgpu/struct.TextureUsages.html#associatedconstant.TRANSIENT_ATTACHMENT),
  `TRANSIENT_ATTACHMENT`: attachment-only usage, clear/dont-care load and discard store. This permits the selected
  public-API path only when the compiled variant has no later reader. More recent API changes do not change our lockfile.
- [Apple WWDC20 10603](https://developer.apple.com/videos/play/wwdc2020/10603/), transcript's memory-bandwidth discussion:
  store only future pass inputs; compression and caches separate logical traffic from device-memory traffic. The
  [Thrum probe](../../evidence/render/G3T-TRAFFIC-PROBE-2026-11-08.md) supplies the actual per-encoder counter method.
- [wgpu 30.0.1 CommandEncoder](https://docs.rs/wgpu/30.0.1/wgpu/struct.CommandEncoder.html#method.as_hal_mut),
  raw access safety, rechecked for this re-plan: do not destroy/end the command buffer or overlap recording.
  The locked core's Raw/Wgpu distinction requires a dedicated whole raw encoder.
- Locked `wgpu-hal-30.0.1/src/metal/device.rs:272–278` leaves Metal's fast-math default enabled; compare
  [Apple's compile option](https://developer.apple.com/documentation/metal/mtlcompileoptions/fastmathenabled).
  WGSL operation order cannot establish CPU IEEE f32 bit parity or parity after shader fusion. The re-plan keeps
  the existing shader modules, entry points, constants, arithmetic, source-coordinate dither and Rgba8Unorm
  quantization boundary. It introduces neither a CPU math oracle nor a fused replacement shader.

Alternatives: retain public wgpu wherever expressive, isolate only the native feature segment required by G4.
Reject mixed raw/wgpu encoders, raw drawable writes and a rival queue. Keeping the tone intermediate costs traffic,
but delivers G4's pre-tone insertion without assuming cross-shader equivalence. Shared-pointer uploads and compact
rows are independent optimizations, not prerequisites of MetalFX, EDR or Mac cadence. Recommission them only with
separate feasibility evidence and reviewed semantics. No global math-mode change or relaxed image tolerance is
selected by this plan.

## Owners and dependency boundary

| State | Single owner | G3t action / consumers |
| --- | --- | --- |
| Device, surface, lifecycle lease | Platform (`thrum-app`, iOS shell, browser host) | Read generation facts; retain Mac/iOS/background/loss rules |
| Hardware recipe | `thrum-render::capabilities` | Capture once per adapter generation; passes never probe |
| Passes, hazards, storage, phase partition | `thrum-render::graph` | Extend one compiled graph; atmosphere and contributions keep declaring accesses |
| Slots, submit, completion, retirement | `FrameHost` | Add resource handoffs without another queue or completion authority |
| World and publication | Core/W1 | Read committed pages; never change simulation for graphics quality |
| Meshes, membership, facing and visibility | V2 in `thrum-render` | Preserve existing membership, determinant/raster policy and order; no classifier refactor |
| Light and material meaning | V3 existing ABI/cluster oracle | Preserve admission and overflow ranking |
| Accepted sample/history | Existing C07 history owners; G4 reconstruction | Commit only submitted images, reset on invalidity |
| Native providers | Isolated renderer `apple_tile` backend | Generation-owned resources; executor supplies recording/completion |
| Route/content | V1 game/G7 | Reuse fixed route and cooked inputs; add G3t proof fixtures without changing G7 baseline |

Current main supplies runtime, sky, content and visibility. M0 needs accepted `T2-CAPTURE:M0`; M1 needs accepted
`T2-CAPTURE:M1` for exact Chrome submitted images and elapsed-time motion. Both are delivered. M2 also names the
already-done `PLATFORM-RUNTIME-MAC:M1` loss boundary. Simulator rows before M12 use the diagnostic scene and
`simctl` endpoint/function observations only, never submitted temporal output or elapsed-time engine motion.
M12 alone needs `COOKED-DRAW:M1` for cooked simulator function. No row requires `T2-CAPTURE:M2`.

Mac M2 qualifies the current surface path after T2 M0, independently of this module. Mac M3 waits for Mac M2
(module order) and `APPLE-TILE:M3`: T2 M0/M1 → G3t M0–M3, including the attended real-window rows at M2/M3 and
simulator diagnostic lifecycle checks. Chrome capture is delivered; simulator cooked draw/capture is absent from
this chain. Mac M3 reruns its complete runtime-motion/capture-OFF panel-cadence and windowed-tap/tooling protocols
on the changed build; earlier results qualify only their recorded build. There is no reverse Mac dependency.

G4 receives M0 proof tools, M1 eligibility, M3's replaceable pre-tone colour/exposure seam and M11 native
providers. M11 uses bounded provider extent fixtures and M12 two fixed extents/current readers; neither waits on G4.
G4 later requalifies real reconstruction readers, display math and live scale transitions. G5 owns cooked draws:
COOKED-DRAW may add downlevel facts to the same capability registry and change `thrum-asset-gpu/src/draw.rs`.
G3t must extend that registry in place and preserve the selected cooked direct/indirect path. No draw.rs refactor is
commissioned here. Any mechanical graph/diagnostic creation-site adjustment must preserve G5's changes, path-independent
history and cost; integrate sequentially and rerun its affected parity witnesses. G4 seam 3 owns later target changes
in that shared file. Never restore an older copy to resolve a conflict.

## Mechanisms

**Registry and preparation.** Extend `DeviceCapabilities` with resolved `Portable`/`AppleTile`, stable digest and
feature facts. Apple eligibility requires Metal, unified memory and the supported transient path; unavailable native
facts select portable. Capture native feature facts once at platform creation, including feature availability for G4;
per-display refresh/headroom remains platform state. `THRUM_RECIPE=portable|apple-tile` is a diagnostic override:
reject unknown or unsupported forced selections, never silently mislabel them. Chrome always resolves portable.
Apple tile is opt-in during construction; the normal default remains portable until integrated paired evidence
justifies promotion. Eligibility alone is not a measured win. All selected and fallback pipelines/bind groups/variant
allocations are prepared before readiness. Capture-only variants belong to an explicitly capture-enabled generation;
capture OFF retains no observer-only target/readback/worker allocation or copy work. Enabling capture when its
resources are absent requires preparation and generation publication, not a lazy live toggle. The resident fallback
set within the admitted configuration is explicit:

- Common scene assets, stored HDR/motion/history, uploads and compatible bind groups are shared, counted once.
- Every selected day/star/capture variant's unique attachment descriptor is resident before readiness; a stored
  star/capture depth and a memoryless day depth may therefore both be resident. Count both, never claim their
  per-pass traffic saving as an equal resident-memory saving.
- A stored portable twin of a memoryless-only attachment is not kept solely for a force override. Unsupported
  storage is resolved during preparation; changing the whole recipe uses generation replacement, with the new
  complete allocation set ready before publication and old/new overlap reserved through completion.
- Optional native provider rejection uses its resident portable twin and required outputs/history/bind groups;
  count every unique twin resource. No lazy allocation or compilation is allowed on rejection.

D1.2/D1.3 report unique resident bytes by selected variant, fallback-only resources and retained generations for
both recipes. A complete resident total can show no memory win even when logical attachment traffic falls.
Diagnostics stay opt-in, following the existing `creation-counters` feature and generation-owned `DeviceSite`.

**Graph storage.** Derive eligibility independently for every prepared day/star/capture variant from compiled live
uses. Only transient render-only attachments with clear/dont-care and discard, no explicit store and no later reader
are eligible; history, persistent, imported, sampled, copied or native-written resources are not. Use the locked
wgpu descriptor restrictions. Allocate eligible Apple textures with `TRANSIENT_ATTACHMENT`; stored textures keep their
required usages. Inspect actual Metal storage modes. Day depth can be eligible; stars, occlusion or G4 readers make
it stored. Motion's explicit store remains ineligible until its producer contract changes. Capture must not widen
normal-path usages. Report reasons per resource/variant and prepare all variants before play.

**One graph, late surface.** Compile scene and output together per generation/variant, with an execute-time surface
binding. The pre-acquire partition contains scene work and the current SDR tone pass; the post-acquire partition
contains the surface consumer and dependent timing/readback epilogue. Reject scene work behind acquisition without
a surface dependency. M2 keeps the current graph resources and shaders while changing scheduling only.

M3 makes the existing tone stage consume one explicitly bound scene-linear colour plus its matching exposure and
extent/sample identity. Default input is the current forward composition. G4 can substitute a prepared reconstructed
linear texture, with declared graph edges, before tone; resources/bind groups for admitted choices exist before
readiness. M3 identity substitution uses the same texture, shader and sample coordinates and must change no bytes.
Keep `synthetic_display.wgsl` → Rgba8Unorm intermediate → `present.wgsl`, including implicit quantization, both surface
format branches and nearest-source letterbox mapping. M3 does not move tone arithmetic into a new shader or claim
16.59 MB/frame saved. G4 implements a separate qualified EDR branch when required; retaining this SDR branch does
not authorize applying tone twice. Capture reads the same prepared submitted output and linear checkpoint branches,
never a raw drawable. Pass-name timing replaces positional slots; update native/iOS/browser/capture consumers together.

Healthy acquisition refusal still drops scene views, flushes pending uploads through the existing token path, and
does not commit image history. Observed device loss abandons without any old-queue call. A background iOS lease
refuses submission and resets affected histories. Resize/replacement uses platform generation admission, retains old
resources until completion and rejects stale callbacks. M2 must retain these distinctions under the merged graph.

**Retained transport, lights and visibility.** Both recipes retain the existing single queue-ordered upload buffers,
dirty reconciliation and refusal flush rules. Queue writes precede the next submission's uses; no pointer import,
per-slot duplicate or new scratch alias is built. Keep the 192-byte `InstanceRecord`/pack and every shader consumer,
supplied normals, translation interpolation and separate submitted history. Light-list selection keeps the existing
GPU capacity and CPU overflow oracle, including strongest-32 ranking, ties and sorted IDs; record selected GPU/CPU
path counts. Neither Mac nor Chrome is claimed to gain GPU overflow qualification. Existing V2 membership, legacy
scene-wide two-sided policy for mirrors/degenerates, physical-slot draw order and off-camera shadow casters remain
unchanged. Private bucket consolidation belongs to V2/G3c on a later commission. Synthetic slot 5060 still supplies
a runtime mirror witness; cooked negative winding is baked and is not a runtime-classifier test.

**Native feature handoff.** C06 and the boundary decision define the planned whole-encoder segment; independent
review is required before its implementation. Close the producing wgpu encoder, record only raw commands in a
separate wgpu-owned encoder, then resume a fresh wgpu encoder. Submit the ordered list once on the coordinator's
queue. Providers never submit, end/commit command buffers or touch the surface. Graph declares all reads/writes.
Tracked same-queue resources provide producer→native→consumer ordering; qualify that assumption on locked wgpu/Metal
with the exact stateful copy/select witness below before integrating MetalFX.

Initialize native-written textures once through wgpu; never alias/discard them. The generation retains every native
resource until slot completion. Provider construction declares formats, extents, usages, scale range and portable
twin before readiness. Provider-internal history resets after any encoded-but-discarded frame, failed submission,
cut or incompatible generation. Keep the ordinary presentation-drop contract for native frames too: discard and
reset instead of adding submit-without-present. Loss still forbids even a flush; iOS background still forbids submit.
Raw command-buffer errors join the normal device-loss path via generation-tagged completion. Report native GPU
start/end times separately from wgpu query times. A rejected optional provider uses its prepared twin. M10's
stand-in is composition proof only; M11 covers provider replacement extents without waiting for G4's live transitions.

The diagnostic native witness uses finite, nonnegative RGBA16F texels and exact load/select/store, with integer phase
and a reserved fixture tile holding binary-exact 0/1 state. It copies current scene colour to output/history; on
alternate submissions a declared crop selects the previously submitted history instead. The tile toggles from the
previous state, making stale history/reset observable. Previous and next history textures are distinct, preinitialized
and never aliased; successful submit advances the selection, and discard/cut/replacement forces reset before reuse.
Copy scene/history regions with bit-preserving native blits and portable wgpu texture copies; only the reserved
0/1 tile needs a native storage-write kernel and a WGSL twin. The tile kernel uses integer selection, without
blend, division, pow or CPU floating-point ranking. No-op native work, stale input, hidden WGSL production and failure
to reset must each fail exact output/history readbacks. A small same-submit fixture is the first M10 implementation
step, before route integration; if exact copying or ordering fails, stop for a boundary re-plan, not a wider bound.
Ordinary scene runs disable this labelled diagnostic. Its crop/state changes never redefine the ordinary baseline.

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
2. Admit a committed sample. Reconcile pending dirty data through existing queue-ordered writes. Encode atmosphere,
   contribution, Sun shadows, air shadows, light lists, existing visibility/prepass, forward/sky/stars and accepted
   transform capture in graph order. Later G4 inserts reconstruction before tone.
3. Tone the selected linear input with its matching exposure into the retained SDR intermediate, acquire late,
   bind the actual surface and present, then submit once. Only successful
   submitted work advances image history; completion retires slot/generation resources.
4. If acquire fails after native encoding, drop the view, perform only the healthy upload flush and mark the provider
   reset before reuse. If loss was observed, abandon without queue calls instead. If iOS suspended, refuse submission
   under its lease. Retained world/input/cooked source continues under its existing owner.
5. During generation replacement, prepare registry/resources/providers and fallback off the rendering critical path,
   publish at the existing safe boundary, reset incompatible histories and retire old resources after completion.

## Cost account and proof

At 1920×1080, nominal uncompressed attachment sizes are depth D32 8.29 MB, HDR RGBA16F 16.59 MB, motion RG16F
8.29 MB and Rgba8 intermediate 8.29 MB.
Memoryless depth avoids a logical store only without later readers; physical savings depend on compression/caches.
The SDR intermediate remains 8.29 MB, with its 16.59 MB/frame nominal logical write/read retained. Upload and
instance footprints remain the current baseline (10k × 192 B = 1.92 MB per instance allocation); there is no new
per-slot copy, scratch-sharing gain or compact-row saving. Dirty work remains proportional to existing changed
ranges, bounded by full reconciliation. Record unchanged-path upload bytes/calls and working set as regressions.

Keep the 9,216-cluster grid, current GPU capacity and CPU overflow cost. At L=64, the current overflow path's
589,824 cell/source candidate tests and ranking/admission costs must remain visible, not be called GPU acceleration.
Record existing list/storage allocations rather than multiplying them by frame slots. Native segments add encoder
owners, output/history and prepared-twin resources; complete timing includes their work. At 1080p the diagnostic
requires 33.18 MB for two RGBA16F histories plus 16.59 MB if its output is unique; report actual twin deduplication
and old/new-generation overlap rather than hiding these allocations in the scene baseline. Report every unique
resident variant/fallback allocation and peak old/new-generation overlap. A memoryless attachment's logical traffic
reduction is not equal to a resident-memory or physical bandwidth saving.

M0 supplies matched baseline identity, same-target repeats, prepared linear-image checkpoints and full-resolution
motion sequences. Existing G7's coarse-grid comparator stays unchanged and is only a route/content smoke test.
G3t retains a full-resolution pre-change native portable baseline at M0 and a Chrome portable baseline after
`T2-CAPTURE:M1`, before M1 changes its rendering, with build/shader/content,
route/sample, format, exposure and H=1 identity. Later milestones compare against that baseline as well as paired
same-build recipes; shared regressions cannot disappear by comparing two changed paths. Chrome is compared to its
own recorded WebGPU baseline, not expected to be bit-identical to a different backend. Baseline artifacts and the
comparator are retained under the storage rules; a new baseline cannot silently replace an earlier one.

The comparison contract requires exact same-target scene-linear and submitted SDR bytes for M1/M2/M3 and ordinary
M10/M11/M12 runs, against recipe pairs and the retained pre-change portable baseline. Membership/IDs and draw order
are exact. This compares unchanged shader arithmetic and quantization on the same backend/compiler/profile; it makes
no CPU-vs-GPU f32 or cross-backend promise. T2 `still`/`fixed` checkpoints use matched sample, exposure, format, extent,
headroom H=1 and history/reset identity. T2 `realtime` records prove elapsed-time continuity, not byte equality across
different scheduling. Compare recorded exact sample identities where available; use matched deterministic fixed
routes for checkpoint equality, never equate movie frame indices with world samples. The M10/M11 diagnostic is
exact against its own copy/select twin and declared state history; compare ordinary pre-provider inputs separately.
There is no 1-ULP or global perceptual allowance in this scope.

M0 rejects phase mismatch, missing contribution, a dropped weak light with nonzero pixels in a saturated cluster,
an omitted mirrored cube at slot 5060 and the removed night star layer. Keep native-size difference maps/crops and
prove all controls fail exact comparison. M0 also retains a full-frame current two-pass SDR/sRGB/letterbox control,
including non-1:1 source/destination extents: later identity-input rebinding must reproduce it exactly. These are
existing-path controls, not an unrun fused-shader feasibility claim. If retained same-target exactness fails, stop
for causal diagnosis and independent design re-review; never widen the tolerance, alter math mode or silently
refresh a baseline. Future fusion/GPU ranking requires a separate reviewed contract and feasibility evidence first.
Independent visual review checks full frames/crops and normal-speed motion in addition to exact bytes.

Capture modes are explicit in every visual item: Mac T2 `still`/`fixed` for exact submitted checkpoints, T2 `realtime`
for host-time engine motion. Chrome M1's implemented adapter uses `fixed` checkpoints and `realtime` sequences
(no `still` switch); Mac M0 implements all three modes. Scene-linear readbacks extend that same tap. Existing endpoint smoke
remains endpoint/no-history, not temporal proof. Simulator `simctl` images/video prove function/UI/lifecycle within
recorded coverage only; simulator submitted/runtime-motion proof stays with T2 M2. GPU runs hold the `gpu` lock.

D2.5/D3.5 and final window proof use OS capture of the actual game window (PID/window ID), requiring **attended
setup**: unlocked active Mac session plus Screen Recording permission for the capturing process. Do not request or
change grants by automation. Record capture tool/version, clock and inclusion limits. Compare settled OS window
frames against T2 `still`/`fixed` reference images at the same paused-world controls, camera, content, recipe and
scene extent, inspecting content and post-resize letterbox geometry in content-pixel coordinates. Record stable
settled-state identity; these are matched-state references, not images of the same submission. Dynamic window
observation uses host-time OS video with ordinary controls; it proves functional response only, not exact same-frame
correspondence or panel cadence. Mac M2.3 retains ownership of real-window tap integration and submission
correspondence; this functional observer does not require or qualify that unbuilt adapter. No engine image/log alone
proves a composited window. Unavailable grants keep these items open while unattended work proceeds. Mac M3 owns
capture-OFF panel swaps and Instruments setup separately; this module never treats engine PTS as photons.

Record source/build/shader/route/cooked hashes, registry digest, device/backend, viewport, mode, warmup, run order and
load. Use paired A/A controls and alternating recipe runs; report p50/p95 CPU upload/encode, full GPU and passes,
allocations, resident fallback/variant bytes and outstanding work. Record validation/admission CPU work separately
from bounds, upload and encode, and record CPU overflow work/counts and existing upload costs. C12 promotion
requires a sustained paired run: after warmup, repeat each matched route for at least ten minutes (a reproducible
measurement window, not a performance budget), recording actual duration, thermal state at start/throughout/end,
power/load and observed throttling. Repeat if the thermal/load comparison is confounded; unavailable thermal data
leaves performance qualification/promotion open. D12.2 needs it for integrated promotion; functional completion may
record a safe candidate as unpromoted. M12 also exercises the representative 10,000-entity field with 1,000–2,000 movers,
full/partial visibility and supported direct/culling paths, so a small valley route cannot stand in for scaling.
Separate display callbacks, submit, GPU completion and physical swaps. Read physical
per-encoder bytes only by the retained Xcode replay method; without a screen/tool slot mark them unmeasured, and do not
block the offscreen functional milestones. No numeric performance/allocation gate; visible hitches, about 25%
comparable regression and unexpected scaling trigger investigation. Accelerated-path promotion requires a measured
win with preserved quality; inconclusive candidates remain unpromoted with prepared fallback.

## Risks and bounded unknowns

- Late readers defeat memoryless: per-variant reasons and later-reader mutation tests expose it.
- Scheduling or input rebinding changes output/history: exact fixed checkpoints, cut/skip/replacement and retained
  supplied-normal/light/mirror witnesses reject missing work; deferred optimizations cannot enter through these steps.
- Native initialization/lifetimes differ from wgpu assumptions: same-submit stateful twin, discard/reset, held slot,
  extent replacement and generation loss witnesses precede MetalFX.
- Quiet timings, physical traffic and real display cadence may be unavailable. Keep their evidence limits explicit;
  none is supplied by docs checks. Functional real-window runs at M2/M3 cannot be replaced by offscreen output;
  without an unlocked active-display slot those items stay open. M12 records cadence/physical-device rows still owed,
  and Mac M3 must qualify the post-G3t-M3 surface path rather than inherit old results.

# G3t Metal boundary: one graph, one device owner

6 November 2026; planned handoff reconciled 9 November. **Boundary selected; whole-encoder feature handoff awaits independent design review; complete native recording remains a later candidate.** [Hardware-native](HARDWARE-NATIVE-ENGINE.md) and C04/C06/C07 govern. Functional delivery remains open; the owner kept native recording in G3t.

## Why this boundary exists

[Frame evidence](../../evidence/render/V1-BOUNDED-FRAMES-2026-11-04.md) records the quiet CPU miss and recording/submission allocations. Pinned wgpu 30.0.1 creates fresh encoder/buffer Arc owners and collects nonempty submissions into a Vec each frame. These costs survive retained bundles and pooled uploads: strict zero Rust allocations cannot pass through this ordinary public path. Bundles reduce validation/recording but replay native encoding at `finish`. Primary-source references and alternatives: `tmp/g3b-wgpu-public-path-input.txt`.

The owner-approved eight-second Metal System Trace shows little drawable-wait overlap with this app's GPU work. Native encoding occurs after acquisition at `finish`. Split portable pre-submit stays parked at `30ae560`: more allocations, no whole-frame CPU win. No raw drawable bridge is selected. Trace spans and offscreen rows do not qualify displayed cadence or latency; native recording alone does not remove camera/shadow CPU costs.

## Selected ownership and data flow

One immutable capability registry per adapter generation selects portable wgpu or Apple tile before pipeline closure; passes never probe privately. Both consume the same compiled graph, view contract, material/instance ABI, source generations and pass identities. The graph owns dependencies, culling, attachment lifetimes, history transitions and timestamps. Its executor derives load/store and memoryless eligibility: an attachment read after a render pass must be stored; a last-use attachment may be transient or memoryless where supported. Acquisition is last before the drawable consumer; no feature owns another surface or queue.

One isolated `apple_tile` backend owns native-feature handoff under coordinator generation/submission order; the graph derives attachment storage. The scoped G3t re-plan retains queue-ordered single upload buffers and defers mapped per-slot rings. G4 owns native pipeline archives, MetalFX and EDR providers. Portable remains the running fallback and oracle. Unsupported features, invalid caches or failed optional pipelines select an already prepared path; no compilation during play. Cache keys include adapter, driver, OS/SDK, recipe, shader and ABI revisions. Device loss rebuilds native objects from retained cooked inputs. Raw calls cannot publish history or retire resources before successful submission.

The local macOS 27.2 SDK exposes memoryless textures, `MTLBinaryArchive` and Metal 4 command allocators/archives; availability belongs to the registry. Pinned wgpu-hal uses `MTLCommandQueue`. Core `EncodingApi::set` rejects mixing Raw/Wgpu recording in either order, including a no-op callback. `as_hal_mut` cannot flush pending native encoding. Raw surface writes also do not mark wgpu's surface initialized. A complete native-recorder candidate must own graph-equivalent recording, initialization, lifetimes, surface and completion handoff. Today's closures take a wgpu encoder; full Metal recording needs commands both recipes can consume. A second queue for part of a frame is excluded.

## Planned native-feature segment (9 November)

The [G3t design](../modules/APPLE-TILE.md) selects a narrower native-feature handoff for G4, subject to independent
design review before implementation. End the producing wgpu encoder, use a separate wgpu-owned encoder entirely
through its raw API, then resume a fresh wgpu encoder. Submit the ordered encoder list once on the coordinator's
queue. This does not permit mixed recording, raw drawable writes or a second queue. Public wgpu remains the path for
all work it can express; the complete native recorder is not brought forward.

The graph declares every native read/write. Native-written textures are initialized by wgpu and never aliased or
discarded; generation ownership retains native resources through completion. Keep the existing acquisition-refusal
drop contract and reset stateful providers after encoded-but-discarded work. Loss abandons without queue calls;
iOS background submission stays forbidden. Completion errors join generation-tagged device recovery. Each provider
has a prepared portable twin. G3t M10's exact stateful storage-writing copy/select witness and prepared WGSL twin qualify ordering, initialization,
retention and reset before G4 uses MetalFX. M11 qualifies replacement, rejection, errors and timing. Neither a
stand-in pass nor a repository check qualifies MetalFX, a hardware win or displayed cadence.

G3t M3 now exposes a replaceable colour/exposure input before the existing SDR tone stage. It retains the
Rgba8Unorm intermediate and both shaders/quantization semantics. G4 inserts reconstruction there and owns the later
EDR display branch. Fused tone/present, compact rows, expanded GPU light ranking, scratch sharing and mirrored-range
consolidation are deferred rather than prerequisites of this boundary. No CPU/GPU floating-point exactness premise
or removed-intermediate traffic saving is claimed. Fresh independent review of the scoped re-plan is owed; milestone
states and unpark remain with the loop.

## Owner decision and later performance candidate

At 11:15 on 6 November the owner kept native work in G3t and retired numeric performance/allocation admission gates.
The proposed early complete recorder is **not approved**. Retain it as a candidate for the dedicated performance phase;
public wgpu allocations no longer justify bringing it forward. Functional G3b and visible sky work continue.

If commissioned later, use the same V1 scene, all passes, lights, history and output. First resolve complete graph-command,
resource and surface handoff; compare supported native recording choices through the registry. No partial raw splice,
standalone fixture or rival queue. Measure whole frames, allocations, GPU timestamps/counters, image tolerance and
recovery. Foundation choices use best-practice review now; measured performance claims require later evidence.

Retained-forward commands are parked at `a4b9da8`: three fewer sampled allocations and no repeatable whole-frame win.
Its code, independent review and raw adverse rows remain available in the branch and owned task scratch.

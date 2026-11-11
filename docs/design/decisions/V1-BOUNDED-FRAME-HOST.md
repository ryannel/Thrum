# V1 bounded frame host

6 November 2026. **G3b partial; qualification open.** [C02](../CONTRACTS.md#c02-time-and-committed-state--f1-coordinator-w1-world), [C06](../CONTRACTS.md#c06-gpu-execution-and-lifetime--v1) and [C07](../CONTRACTS.md#c07-render-scene-views-and-temporal-data--v2-with-p4v3v4) govern. Mac uses a display link, render actor and autonomous world lane; browser drives inline authority.

Core owns `ClockedWorld<G>` and native `WorldLane`. The game supplies validated controls, rules, endpoints and projection policy. One engine clock admits time, caps catch-up, retains failed ticks, recovers optional publication and preserves overload across handoff. Publication leases do not recover pages; a separate recovery hook captures derived output. Portable progress serves inline and resident consumers without per-tick storage allocation.

thrum-render owns generic `FrameHost<W,V>`: two GPU slots, extraction handoff, scene/present tokens, submission, completion and history. `FrameWorld` observes the existing clock/lane; `SceneSource` maps committed schema and samples camera/lighting without GPU access. The game facade retains fixture, controls and content products. Renderer timing names stay with the prepared renderer. Monomorphized hooks reuse setup storage; no second runtime or backend framework. Mapping edits invalidate extraction and history; failed preparation must preserve required uploads on retry.

Only observed monotonic time advances authority; display targets select bounded interpolation. Exact page epoch/tick/timeline accompanies extraction on every target. A busy GPU ring sheds an optional view. Platform owns display timing and surface generation, checks acquisition invalidation and drains old GPU uses before resize. Window close stops new work and invalidates the surface lease. Bounded-run cancellation uses the worker’s bounded drop path. A normal finish can wait for its bounded final-frame and GPU-drain replies before detaching. Replay and visual replacement reset history; cooked candidates remain host-bound. The Mac actor encodes before late acquisition, invalidates stale leases and serializes device reconstruction. Held-drawable retry and detached replacement pass on Metal; live-window recovery and physical cadence remain open. Portable split submission stays parked; mixed raw/wgpu recording remains unsupported.

The [Mac plan](../../plans/PLATFORM-RUNTIME-MAC/PLAN.md) owns live
qualification. Record complete 10k/1080p CPU, GPU and allocation rows
as baselines and investigate visible stutter or comparable regressions.
Display-link targets, submit times and GPU samples are independent of
measured physical presentation. Report input-to-photon only if an
identified input and its displayed response are measured together.
Simulator proves function only. [Evidence](../../evidence/render/V1-BOUNDED-FRAMES-2026-11-04.md)
separates ownership, correctness, visuals and measured timing.

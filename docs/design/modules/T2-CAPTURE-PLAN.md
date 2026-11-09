# T2 unattended frame evidence plan

Design: [T2-CAPTURE.md](T2-CAPTURE.md) · [brief](T2-CAPTURE-BRIEF.md) ·
Done when: [T2-CAPTURE-DONE.json](T2-CAPTURE-DONE.json).
Reviews: independent design/JSON review owed; no agent launched, per owner instruction.
Milestone: M0 Mac engine images and runtime motion without a desktop capture grant (open)
Now: evidence ownership, mechanisms and Mac M2 proof split planned on 9 November; no capture implementation started.
Next: fresh /review-milestone design review from the brief, then /build-milestone M0; Mac qualification M2 consumes M0.
Blocked: design review owed before build. Physical panel qualification has an attended setup requirement; it does not block M0.

## Ownership and sequence

T2 owns this child work of TOOLS-DELIVERY; no new crate. Build sequentially. Existing committed pages/world lane,
FrameHost, cooked content, platform loss/lease and scene/controls are available now. M0 must preserve
`PLATFORM-RUNTIME-MAC:M1` (done). M1/M2 here follow M0; no dependency on G3t/G4 or Mac qualification M2.
Mac qualification M2 and APPLE-TILE M0 depend on `T2-CAPTURE:M0`. Later visual milestones use M0 on Mac,
M1 on Chrome and M2 on the simulator at the first proof requiring this tooling. Existing completed evidence is not
invalidated. Captures cannot close an attended panel item.

Implementation ownership: `thrum-render` owns graph tap, targets, staging, fences and generation cancellation;
`thrum-app` owns headless/windowed scheduling and normal control dispatch; core keeps its existing world clock;
existing scripts own manifests, lossless/movie packaging and validation. V1 keeps only authored scenarios, presets,
content and bindings. G3t owns linear recipe comparison and G4 owns output-provider/EDR qualification; both use this
capture service rather than copying it. Follow CONTRIBUTING and the architecture registry if dependencies change.

## M0 Mac engine images and runtime motion without a desktop capture grant: open

- Add opt-in renderer-owned final-output readback to ordinary submitted frames, with bounded asynchronous GPU/CPU
  queues, explicit missing samples and loss/resize/abandon handling. Preserve endpoint still capture semantics.
- Provide fixed-time and monotonic real-time offscreen drivers using shared world/render/control paths. Migrate V1
  sequence copying; retain existing route/endpoint baselines and parity oracles. No AppKit/screen capture prerequisite.
- Deliver a documented, implemented command for stills, fixed-time sequences and real-time sequences. Pin artifacts
  and validate completeness. Command names are left to the builder; no nonexistent CLI is a JSON check.
- Prove the cooked noon 0°–36° two-second pan with initial/final holds, 1920×1080 output and 120 Hz requested
  opportunities, world paused, through the normal semantic control handler. Repeat with running-world motion and
  interpolation between 60 Hz endpoints. Locked-console and unavailable-permission conditions must be recorded.
- Capture and inspect a deliberately injected render/input stall. It must remain visible for its real elapsed duration.
  Saturate/slow the writer, lose a device with a mapping pending and refuse output/finalization; prove honest failure
  and bounded resources without stalling or corrupting the continuing app. Pair capture on/off and report overhead.
- Keep source frames/time records plus 1× preview, contact sheet, crops and compact evidence. The independent reviewer
  runs the retained command, validates provenance and reviews the real-time sequence against the inspection list.

## M1 Chrome WebGPU evidence without desktop capture: open

- Adapt the actual submitted portable output to the same capture semantics; keep DOM/native surfaces excluded in
  the manifest. Exercise the cooked moving route in fixed-time and actual elapsed page/runtime time modes.
- Prove locked-console execution on real hardware WebGPU. Prefer a verified headless browser; record browser flags,
  backend and throttling policy. Reject software fallback or unsupported GPU instead of treating it as this Mac.
  A deterministic route that rejects hidden/throttled pages stays a deterministic route; do not disable lifecycle
  checks or fake visibility to pass the real-time proof. Explicitly label any controlled browser scheduling profile.
- Preserve existing actual-tab/visibility/device-loss qualification. It is distinct from unattended canvas image proof.
  Prove capture cancellation/recovery and browser-nonblocking readback/packaging, plus real-time stall/gap controls.

## M2 iOS simulator evidence without desktop capture: open

- Reuse the renderer tap in the real simulator route under its existing foreground lease and normal input dispatch.
  Prove locked-console run, normal-speed camera/actor response, pending-capture background/foreground and surface/
  device-generation replacement. No capture may submit in the background or publish old-generation pixels.
- Qualify simulator evidence as function only; name the simulated model, runtime, host, engine and surface identities.
  `simctl` snapshots/video can corroborate the simulator UI if available without a desktop grant. If simulator boot
  or UI automation demands an OS grant, record that specific row attended and continue engine evidence work; do not
  claim M2 done unless its required unattended engine-output route is actually proved.

## Proof, retention and review handoff

Run `./dev check` for implementation, focused real-device/lifecycle and negative-control checks under `gpu`, and the
new documented scenario commands. Checks must fail on unavailable required hardware. This planning revision uses
`./dev check-docs`. Exact source images, sample logs and decisive failure controls live in `tmp/t2-capture-<milestone>/`
with a storage record, byte allowance and closeout condition. Promote compact reproducible proof and selected excerpts
to `docs/evidence/render/` under ASSET-LIFECYCLE; ignored scratch alone is not durable selected evidence.

The design reviewer starts with the brief and independently derives coverage before reading the design/JSON. Check
single ownership, final submitted-image correspondence, real-time versus synthetic clocks, output-stage/UI limits,
nonblocking bounded readback and the attended Mac cadence boundary. No proof item is passed by this plan.

## Log

- 2026-11-09 Owner requested unattended visual/live-motion evidence and prohibited subagents. Settled T2 ownership
  with V1/V4/F1 producers and Mac/G3t consumers under C12; selected submitted-image tap plus distinct fixed-time and
  real-time drivers. ScreenCaptureKit permission and Instruments export are independent blockers. Replanned Mac M2
  without waiving built-in-panel cadence or claiming old images qualify the later build. Fresh design review owed.

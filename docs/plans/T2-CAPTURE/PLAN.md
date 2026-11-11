# T2 unattended frame evidence plan

Design: [T2-CAPTURE.md](T2-CAPTURE.md) · [brief](T2-CAPTURE-BRIEF.md) ·
Done when: [T2-CAPTURE-DONE.json](T2-CAPTURE-DONE.json).
Reviews: [M0 third review](../../evidence/T2-CAPTURE/M0-review.md) passed in dff3eb5e; [M1 review](../../evidence/T2-CAPTURE/M1-review.md) passed in e2909ad4.
Milestone: M2 iOS simulator engine motion without desktop capture (claimed)
Now: Native foreground simulator capture, complete motion, lifecycle retirement/recovery and refusal controls pass. See [builder evidence](../../evidence/render/t2-capture-m2/qualified/README.md).
Next: independent lead review of M2.
Blocked: none. Physical iPhone and physical display qualification remain with their named owners.

## Ownership and sequence

T2 owns this child work of TOOLS-DELIVERY; no new crate. Build sequentially. Existing committed pages/world lane,
FrameHost, cooked content, platform loss/lease and scene/controls are available now. M0 must preserve
`PLATFORM-RUNTIME-MAC:M1` (done). M1/M2 here follow M0; no dependency on G3t/G4 or Mac qualification M2.
Mac qualification M2 needs only `T2-CAPTURE:M0`; Mac M3 separately needs `APPLE-TILE:M3` for post-change requalification.
APPLE-TILE is parked after two failed design reviews; its M0 dependency on `T2-CAPTURE:M0` does not authorize a restart. Later visual milestones use M0 on Mac,
M1 on Chrome and M2 on the simulator at the first proof requiring this tooling. Existing completed evidence is not
invalidated. Captures cannot close an attended panel item.
Mac M2.3 commissions and proves integration of the shared tap on a real CAMetalLayer window; Mac M2.4 owns bounded
trace-finalization diagnosis. Neither task is hidden inside unattended M0, and neither adds a reverse dependency.
Before Chrome M1/simulator M2, existing canvas endpoint images may prove fixed-time endpoint/oracle claims and
`simctl` captures may prove simulator function/UI/lifecycle within their recorded limits. New submitted-temporal or
elapsed-time engine-motion claims still need the target's T2 milestone.

Implementation ownership: `thrum-render` owns graph tap, targets, staging, fences and generation cancellation;
`thrum-app` owns headless/windowed scheduling and normal control dispatch; core keeps its existing world clock;
existing scripts own manifests, lossless/movie packaging and validation. V1 keeps only authored scenarios, presets,
content and bindings. G3t owns linear recipe comparison and G4 owns output-provider/EDR qualification; both use this
capture service rather than copying it. Follow CONTRIBUTING and the architecture registry if dependencies change.

## M0 Mac engine images and runtime motion without a desktop capture grant: claimed after rework

- Start with **M0.3's writer feasibility probe**, before full scenario integration: raw sequential spool at
  1920×1080, 120 records/s for four seconds, bounded buffers and 16 GiB output allowance per session. Record storage
  write/flush, queue peaks, drain and deferred lossless packaging/hash validation. Revise an overflowing writer
  before proceeding; do not grow a session-sized RAM queue or shrink the required capture workload.
- Add opt-in renderer-owned final-output readback to ordinary submitted frames, with bounded asynchronous GPU/CPU
  queues, explicit missing samples and loss/resize/abandon handling. Preserve endpoint still capture semantics.
- Provide fixed-time and monotonic real-time offscreen drivers using shared world/render/control paths. Migrate V1
  sequence copying; retain existing route/endpoint baselines and parity oracles. No AppKit/screen capture prerequisite.
- Deliver a documented, implemented command for stills, fixed-time sequences and real-time sequences. Pin artifacts
  and validate completeness. Commands are named by the builder; the JSON already reruns `./dev check` under `gpu`
  for the tap/queue regressions. Target-specific scenario commands remain additional required proof.
- Prove the cooked noon 0°–36° two-second pan with initial/final holds, 1920×1080 output and 120 Hz requested
  opportunities, world paused, through the normal semantic control handler and ordinary presenter/`SurfaceOutput`
  gate. Use one-second initial/final holds. Fresh submissions must not advance the paused world or use a capture-only
  admission bypass. M0.5 judges the pan; M0.6 separately judges running-world motion/interpolation between 60 Hz
  endpoints. Record lock/permission conditions opportunistically, never locking an active user or unlocking by automation.
- M0.7 captures explicit diagnostic scenario render/input stalls, disabled in ordinary runs. Each must remain visible
  for its real elapsed duration; prove the hook does not fire without its diagnostic action.
  Saturate/slow the writer, lose a device with a mapping pending and refuse output/finalization; prove honest failure
  and bounded resources without stalling or corrupting the continuing app (M0.2). M0.8 separately pairs capture on/off
  and records overhead/scaling; M0.9 judges runbook, retention and consumer handoff.
- Keep source frames/time records plus 1× preview, contact sheet, crops and compact evidence. The independent reviewer
  runs the retained command, validates provenance and reviews the real-time sequence against the inspection list.

## M1 Chrome WebGPU evidence without desktop capture: claimed

- Adapt the actual submitted portable output to the same capture semantics; keep DOM/native surfaces excluded in
  the manifest. Exercise the cooked moving route in fixed-time and actual elapsed page/runtime time modes.
- Prove locked-console execution on real hardware WebGPU. Prefer a verified headless browser; record browser flags,
  backend and throttling policy. Reject software fallback or unsupported GPU instead of treating it as this Mac.
  A deterministic route that rejects hidden/throttled pages stays a deterministic route; do not disable lifecycle
  checks or fake visibility to pass the real-time proof. Explicitly label any controlled browser scheduling profile.
- Preserve existing actual-tab/visibility/device-loss qualification. It is distinct from unattended canvas image proof.
  Prove capture cancellation/recovery and browser-nonblocking readback/packaging, plus real-time stall/gap controls.

## M2 iOS simulator evidence without desktop capture: building after COOKED-DRAW M1 acceptance

- Reuse the renderer tap in the real simulator route under its existing foreground lease and normal input dispatch.
  Prove normal-speed camera/actor response, pending-capture background/foreground and surface/device-generation
  replacement with denied read-only Screen Recording preflights before and after under the identified launcher/
  harness identity. Record console state before/after as witness metadata, accepting either state and transitions,
  with unavailable observations recorded with a reason. No manual-lock wait or lock state refusal. Missing, failed
  or granted permission preflights leave capture independence unproved. No capture may submit in the background
  or publish old-generation pixels.
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
The default four-second SDR session allowance is 16 GiB including raw/packaged coexistence; run bulk sessions
serially and verify retained selected roots before reusing their space. Longer/HDR sessions declare their own cap.

The design reviewer starts with the brief and independently derives coverage before reading the design/JSON. Check
single ownership, final submitted-image correspondence, real-time versus synthetic clocks, output-stage/UI limits,
nonblocking bounded readback and the attended Mac cadence boundary. No proof item is passed by this plan.

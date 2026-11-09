# T2 unattended frame evidence plan

Design: [T2-CAPTURE.md](T2-CAPTURE.md) · [brief](T2-CAPTURE-BRIEF.md) ·
Done when: [T2-CAPTURE-DONE.json](T2-CAPTURE-DONE.json).
Reviews: [design review 3](../../evidence/T2-CAPTURE/design-review-3.md) clears M0 build; [M0 review](../../evidence/T2-CAPTURE/M0-review.md) failed M0.3/.4/.8. Rework is builder-verified and reclaimed for lead re-review; no reviewer launched.
Milestone: M0 Mac engine images and runtime motion without a desktop capture grant (claimed after rework)
Now: 9 November M0.3/.4/.8 reworked and target-verified on locked-console M3 Max/Metal. Five full writer probes and five consecutive corrected-build pans complete; engine lead/generations, measured GPU copy bounds, host load and CPU profiles are in the [rework evidence](../../evidence/render/t2-capture-m0/rework/README.md). All nine items are reclaimed; profile Check gates integration.
Next: independent M0 re-review is owed. Mac qualification M2 consumes M0 after acceptance; Chrome M1 and simulator M2 remain open.
Blocked: no M0 delivery prerequisite remains. Attended panel/window/tooling qualifications stay with the named Mac M2 owners and do not follow from engine images.

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
The default four-second SDR session allowance is 16 GiB including raw/packaged coexistence; run bulk sessions
serially and verify retained selected roots before reusing their space. Longer/HDR sessions declare their own cap.

The design reviewer starts with the brief and independently derives coverage before reading the design/JSON. Check
single ownership, final submitted-image correspondence, real-time versus synthetic clocks, output-stage/UI limits,
nonblocking bounded readback and the attended Mac cadence boundary. No proof item is passed by this plan.

## Log

- 2026-11-09 Reclaimed M0 after [first milestone review](../../evidence/T2-CAPTURE/M0-review.md).
  Expanded the finite CPU burst reserve to 96 (six staging, 1 GiB pixel cap); five consecutive full writer probes
  and five consecutive corrected-build pans complete without omission, at unchanged 1920×1080/120 Hz workload.
  FrameHost stamps actual device/presenter/lead provenance; the engine opportunity adapter consumes the shared
  native lead policy. Dependency-ordered real GPU markers measure a readback-copy upper bracket on Metal,
  where encoder writes are unsupported and empty markers produced zero counters. Pipeline creation uses the
  generation site in a setup transition; feature-enabled tests verify accounting. Added per-arm host load/disk
  samples and separate actual CPU profiles: row unpacking, stdout scanning and file writes are observed costs;
  whole-child CPU rises 53%/76%, so lower step time is not claimed as an improvement. Repeated still/fixed/running,
  stall and slow-writer controls on the exact corrected binary; exposed existing alpha clamps. Failed collection
  now retires its slot and restores the CPU lease. Clarified legacy parity (reviewer proved pre/post), alpha-count
  thresholds and input coalescing. Historical attempts remain separate; selected roots/archives verified before
  owned duplicate cleanup. All item statuses/evidence and M0 state updated, with planner/reviewer definitions,
  counters and verdicts unchanged. Fresh locked rebase/Check/push gates integration; lead re-review remains owed.


- 2026-11-09 Built M0 through the existing FrameHost graph, app/game semantic handler and ordinary presenter gate.
  The early writer probe selected uncached raw spill; integrated bursts selected bounded six/32 buffering under
  512 MiB, preserving 1920×1080 and 120 requested opportunities/s. All still/fixed/elapsed cases run while locked
  with denied Screen Recording preflight; the 481-image paused pan and 206 intermediate-alpha running samples
  pass builder inspection. Explicit render/input stalls preserve elapsed duration; slow output fails honestly while
  ticks/rendering continue. Legacy native route checkpoint pairs and endpoint semantics are preserved after removing
  game-side copy/submission. Added queue/loss/resize/abandon, disabled allocation, channel/padding and packaging
  refusal regressions to the Check. The full Check passes; [evidence](../../evidence/render/t2-capture-m0/README.md)
  records cost, limits, exact sources and retention. Only M0 state/items' status/evidence changed in the done-when file;
  definitions, counters and review fields are untouched. Claim integration follows the profile's locked rebase/Check/push.
  Independent milestone review remains the lead's; no reviewer launched. APPLE-TILE and attended Mac items stay parked/open.

- 2026-11-09 Addressed [second-review F1/F2](../../evidence/T2-CAPTURE/design-review-2.md): Mac M2 now
  qualifies the current path after T2 M0 only; new Mac M3 owns post-G3t motion/cadence with an explicit Apple M3
  dependency. Recorded APPLE-TILE parked after its second failed design review in plans and consumers; no restart
  or resolution of its seven findings is claimed. Replaced Ground E image-quality comparison with worded pan
  criteria and restricted the PNG to scene/framing identity. T2 M0 `replans` is 2; M1/M2 remain at 1. All are open
  with no new evidence. `./dev check-docs`, JSON/dependency validation and historical-proof preservation checks pass.
  Independent design re-review of this revision is owed; no agents launched.

- 2026-11-09 Addressed [review F1–F10](../../evidence/T2-CAPTURE/design-review.md) without commissioning implementation.
  F1: Mac M2.3 owns real-window tap integration/proof. F2: M2.2 gains a capture-OFF traced cadence arm alongside two
  capture-ON observer/content arms. F3: Mac M2.4 explicitly commissions bounded trace diagnosis, with trigger/stop rule.
  F4: raw spill, deferred packaging, 16 GiB per-run allowance and early M0.3 throughput probe. F5: ordinary
  presenter/SurfaceOutput gate and paused-tick proof. F6: `./dev check`/`gpu` checks plus target-specific runs.
  F7: separate pan, running-world, stalls, overhead and documentation criteria. F8: cited historical locked-console
  feasibility with its missing local probe limitation; future lock condition is opportunistic and recorded.
  F9: explicit diagnostic-only stall actions and ordinary-run control. F10: cleared Mac M2.2's stale claim evidence
  and preserved scoped interim Chrome/simulator capture. T2 M0/M1/M2 `replans` incremented to 1, `fails` reset to 0,
  all remain open. Mac M2 increments to 3, preserving the Apple tile M3 dependency and historical M0/M1 proofs.
  Latest review still requests changes; fresh independent design/JSON/consumer re-review is owed, no agents launched.

- 2026-11-09 Owner requested unattended visual/live-motion evidence and prohibited subagents. Settled T2 ownership
  with V1/V4/F1 producers and Mac/G3t consumers under C12; selected submitted-image tap plus distinct fixed-time and
  real-time drivers. ScreenCaptureKit permission and Instruments export are independent blockers. Replanned Mac M2
  without waiving built-in-panel cadence or claiming old images qualify the later build. Fresh design review owed.

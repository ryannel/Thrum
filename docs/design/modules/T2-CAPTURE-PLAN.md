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

## Log

- 2026-11-09 Owner scope correction removes console lock as an admission/pass condition from open M2.1 and applies
  the same policy to M2.2 lifecycle capture. Record actual console observations as witnesses in either state; denied,
  attributed before/after Screen Recording preflights and the submitted-output tap establish independence from
  desktop capture. The M2 builder applies this policy to its implemented target harness and any reused COOKED-DRAW
  simulator helper, including refusal controls for missing/failed/granted preflights and acceptance controls for
  locked/unlocked/changing observations. The earlier blocked probe remains history, with no new pass claimed.
  M2 remains blocked on COOKED-DRAW M1 acceptance and retains function-only scope. Accepted M0/M1 definitions,
  evidence and verdicts are preserved. Their lock witnesses are valid historical capability proof, and future
  consumers record lock state without arranging it. No failure counters or dependency edges change. Independent
  review of the revised definitions is owed to the lead/loop. No agents launched and no owner decision remains.
  `./dev check-docs` and `git diff --check` pass.

- 2026-11-09 Planner linked M2 to its prerequisite: `needs` `COOKED-DRAW:M1`, G5's capability-selected direct
  cooked draw path and the iOS route's cooked attach. Items, bar and counters are unchanged, and M2 stays blocked. The lead may `unblock` it once
  the link lands, because the loop then holds M2 until COOKED-DRAW:M1 is accepted.

- 2026-11-09 Attempted M2 on the real locked iPhone 18 Pro / iOS 27.2 simulator. Shared booted-device
  install/container requests wedged; a fresh owned simulator boots and runs the ordinary diagnostic foreground route.
  The cooked scene fails on the existing G5 contribution: Apple iOS simulator GPU omits INDIRECT_EXECUTION;
  `thrum-asset-gpu` creates indirect meshlet argument buffers and issues indirect draws. The initial adaptation
  reports the explicit missing-downlevel-flag validation error; a minimal probe with capture disabled records the
  actual adapter/flags, diagnostic presents and cooked validation failure. This is a missing G5 target path, not
  an attended desktop-capture grant. Did not implement another owner's renderer fallback; removed the unverified
  capture prototype and retained the minimal reproducible probe instead. Both M2 items/state stay open; only their
  evidence fields change in the criteria. M1's selected PNG hashes and compressed evidence were verified after its
  passed review, then 16,008,456,538 bytes of owned scratch were retired with a closeout record. Integration records
  this blocker, not a milestone claim. `./dev check-docs` passes and criteria ownership validation confirms that
  only M2 evidence fields changed. The profile Check runs in the locked integration gate. Resume after G5
  provides the simulator-capable cooked path.


- 2026-11-09 Built M1 with the existing renderer-owned tap, normal WebShell submission/lifecycle and V1 route.
  Fixed-time samples wait explicitly. Elapsed-time samples consume actual page time, initial pause release and
  ordinary overload policy. Five consecutive final runs verify 2,407 original images on this less-compressible
  cooked workload, independently of M0's writer margin. A local LaunchServices app records denied preflight and
  launches normal sandboxed Chrome. Final loss/resize/stall/ring/export controls and existing tab/lifecycle/route
  consumers pass. The earlier saturation arm drained during timer pacing and is retained as a superseded control.
  Native frames, the declared cut, normal-speed Chrome playback and exact preview PTS were inspected. Full Check
  passes under gpu. M0 follow-ups correct live-compression text, preserve disk errors, remove the stale runbook
  home path and close out 23.52 GB after selected-root verification. Only M1 state/status/evidence changes in the
  done-when file. M1 claims await locked integration and the lead's independent review. No reviewer launched.


- 2026-11-09 Fixed second-review M0.3/.8 with original-frame-hashed temporal XOR/Zstd, separate finite codec/I/O
  ownership and 256 MiB encoded burst reserve. Five native writer probes and thirty consecutive integrated sessions
  retain every original image; twenty are strictly disk-capped below the review's normal 280 MB/s floor. Actual
  thread CPU/wall and E/P endpoint IDs diagnose the sleeping baseline; an explicit active-pacing control stabilizes
  unfiltered OFF medians to ~2% spread, with paired region cost, scene/copy GPU, load/profiles, memory/bytes and
  full flush reported separately. Raw slow-sink saturation refuses while ticks/rendering continue. Original engine
  provenance/lead and renderer ownership are unchanged. All exact sources verified; compact selected views replace
  80.71 MB historical duplicates with a Git restoration audit. Only permitted done-when fields changed; both failure
  counters and reviewer verdicts remain. Fresh Check/integration gates the claim; lead re-review owed, no reviewer.

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

- 2026-11-09 Claimed M2 after accepted COOKED-DRAW M1. The ordinary simulator renderer uses the shared bounded writer/tap with actual adapter, source extent, submission/lease history and callback disposition. Three pans plus running/stall/fresh lifecycle intervals preserve 3,130 originals without omissions. Pending lease/device controls cancel, background counters hold, and explicit ring/Raw slow controls refuse while the world advances. Actual UIKit touch/rotation regression passes independently of semantic input. Denied attributed preflights pass before/after with naturally changed console witness. Builder native-frame/timing inspection complete. All attempts, finite resources, startup service-reset side effect and source retention are documented. Full Check passes serially, iOS Clippy and ten capture tests pass. Integration repeats Check. Only M2 state/status/evidence change in DONE. Independent review is owed.

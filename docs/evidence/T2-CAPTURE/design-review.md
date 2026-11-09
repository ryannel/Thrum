# T2-CAPTURE design review (with the Mac M2 re-plan)

9 November 2026. Independent `/review-milestone` design review of revision `1fb6b422`: the
[brief](../../design/modules/T2-CAPTURE-BRIEF.md), [design](../../design/modules/T2-CAPTURE.md),
[plan](../../design/modules/T2-CAPTURE-PLAN.md) and [done-when file](../../design/modules/T2-CAPTURE-DONE.json),
plus the same commit's re-plan of [Mac M2](../../design/modules/PLATFORM-RUNTIME-MAC-PLAN.md)
([criteria](../../design/modules/PLATFORM-RUNTIME-MAC-DONE.json)) and its consumer edits (ORCHESTRATE, BUILD-PLAN,
AI-DEVELOPMENT, C12, RENDERING, PLATFORM-RUNTIME, TOOLS-DELIVERY, APPLE-TILE, maturity ledger, Mac M2 evidence page).
Reviewer: Claude Opus 5.5. No milestone is judged, so no done-when file is changed. Nothing was built or run on a GPU;
this is a document and source read plus the retained 9 November probe records. `./dev check-docs` passes on
`1fb6b422` and both JSON files parse.

**Verdict: DESIGN CHANGES, 10 findings.** The design meets the owner's aim for engine images and elapsed-time runtime
motion. It drops the Screen Recording prerequisite everywhere, and it keeps fixed-time and real-time claims apart
properly. Its stall and gap controls would catch the usual fake, a smooth fixed-step movie passed off as live. But
Mac M2 has no route to done. M2.2 needs a windowed engine tap that no T2 item builds or proves. It would measure
panel cadence with capture switched on. And it still depends on a parked Instruments failure that nobody owns. Two
mechanisms also need settling before M0 is built: writer throughput for the required interval, and the paused-world
render gate.

## Capabilities derived from the brief (written before reading the design)

1. Exact-build stills, fixed-time sequences and separately labelled elapsed-time runtime sequences of the real engine
   output on the named GPU and recipe, with no OS Screen Recording grant and with the console locked.
2. Runtime capture keeps input, committed state, interpolation and history, frame identity, clock domains and missing
   samples. A slow run must not turn into a smooth movie.
3. The real render graph and temporal path, with bounded asynchronous readback and bounded disk and encoding work.
   Disabled capture adds no copy, readback, allocation or synchronization.
4. A first proof on Mac: the cooked noon pan with a paused world while locked and without the permission, then a
   running world with interpolation. Full frames, native-size crops, 1x motion, and negative controls for timing and
   capture gaps.
5. Chrome and the iOS simulator qualified separately, never as native panel evidence.
6. Every later visual milestone names its capture mode, surfaces, target and inspection list.
7. Panel cadence, compositor and native UI, OS input and focus, and EDR or photon proofs stay separate. Attended rows
   are labelled with their reason. No security bypass, no automatic unlock, and no offscreen sequence silently
   standing in for a display result.
8. Overhead and scaling are measured, with no numeric gate, and historical Mac evidence is preserved.

## Coverage against the brief

| Capability | Where the design covers it | Verdict |
| --- | --- | --- |
| 1 | Modes table, M0.3, rollout table | Covered, but the locked-console mechanism is supported by evidence the design doesn't cite (F8) |
| 2 | Walk-one-frame steps 1–5, completeness section, M0.4 | Covered. The stall control and PTS-from-host-time rule are strong |
| 3 | Steps 2–4, cost section, M0.1 and M0.2 | Covered on paper. Writer throughput for the required interval isn't shown (F4) |
| 4 | M0.4, Mac M2.1 | Covered. The paused-world render gate isn't addressed (F5) |
| 5 | M1, M2 | Covered |
| 6 | AI-DEVELOPMENT and ORCHESTRATE additions | Covered. Chrome and iOS have a capture gap until M1 and M2 exist (F10) |
| 7 | Attended table, Mac M2.2 | Covered for labelling. The M2.2 route is incomplete (F1–F3) |
| 8 | Cost section, M0.5, the historical motion pass kept | Covered |

Nothing outside the brief was added: no crate, no native encoder, no Metal recorder, no raw drawable bridge.

## What checks out

- **The code facts the design states are accurate.** `FrameHost::capture_pixels` waits, and `begin_capture`
  redraws the current committed endpoint without committing history (`crates/thrum-render/src/runtime.rs:1295-1330`).
  V1 `motion.rs` owns a game-side copy (`:255`) and waits on the poll (`:273`). It also steps one tick per frame with
  `host.set_camera_yaw`. Migrating that path is necessary, as the design says.
- **The core unattended mechanism is plausible on this machine.** The retained probe
  `tmp/platform-runtime-mac-m2/console-probe.txt` (01:59 CET, `IOConsoleLocked = Yes`,
  `CGSSessionScreenIsLocked = Yes`) was followed at 02:00 CET by a successful offscreen Metal render. That was the
  route-only parity run, exit 0 and byte-identical
  ([record](../render/mac-m2-pan/parity-20261109.json)). Offscreen Metal readback has therefore already completed
  while the console was locked and without Screen Recording. I verified this from the files and did not rerun it.
  The console was still locked at review time.
- **Clock honesty.** Real-time mode refuses a synthetic frame counter. PTS come from host sample or submission time,
  not copy completion. Skipped renders hold the prior image as labelled duplicates. Overflow makes the interval
  inconclusive. M0.4 demands that a fixed-time movie cannot satisfy it, and compares stall duration with PTS. Together
  these catch the main convincing fake.
- **Single ownership.** The ownership split is consistent across T2, C12, RENDERING, PLATFORM-RUNTIME, TOOLS-DELIVERY
  and APPLE-TILE: T2 owns scenarios and artifacts, V1/V4 the tap and readback, F1 the drivers. G3t consumes the tap and
  doesn't build a rival recorder.
- **Attended reasoning for panel cadence is genuine.** A locked session occludes the game window. The app then stops
  presenting (`crates/thrum-app/src/app.rs:599`, `:1157`), so no built-in-panel swaps exist to attribute.
- **The re-plan's bookkeeping follows the transition table.** Mac M2 goes to `open` with `replans: 1` and
  `fails: 0`. Its changed items are reset. The historical `0d53d3aa` pass is kept as history only, and APPLE-TILE M0
  gains `replans: 1` and `needs: T2-CAPTURE:M0`.

## Findings (worst first)

### F1. Mac M2.2 depends on a windowed engine tap that no T2 item builds or proves (verified from documents)

Mac M2.2's `inspect` says "Use T2-CAPTURE engine output for both arms" of the live AppKit-window run. The plan's
M2.2 work item says engine capture "of both arms replaces mandatory ScreenCaptureKit movies". So M2.2 needs the
**real-time windowed tap** (design modes table row 3, and step 3: "one final engine texture feeding both the normal
present pass and readback", no drawable lease extension, direct output kept when disabled).

No T2 M0 item delivers it. M0.3 lists "stills, fixed-time sequences and separately labelled real-time **offscreen**
sequences". M0.1's "preserving ... platform lease/loss rules" is a constraint, not a windowed proof, and M0.4 is
offscreen only. Mac M2 `needs` only `T2-CAPTURE:M0`. The windowed tap would therefore first be built inside an
attended unlocked slot, with no done-when criteria for these properties:

- tap pixels equal what was presented;
- no lease extension;
- present path unchanged when capture is disabled;
- surface-loss cancellation.

A windowed proof can't run while locked, because the window is occluded. So it can't simply be added to the
unattended M0.

**Should be:** a separate T2 item or milestone, `M0w windowed tap` (attended setup: unlocked active panel, no Screen
Recording), that Mac M2.2 `needs`. Or move these criteria into Mac M2.2 as its own item. Either way, prove the tap on
a real CAMetalLayer surface with a known pattern and a surface-loss control.

### F2. M2.2 would qualify panel cadence of a capture-perturbed app (verified from documents)

M2.2 requires engine capture on **both** the untraced and traced arms. With capture on, the tap at 1920×1080 and
120 Hz adds about 1 GB/s of readback (design cost section), plus an extra copy or present pass. It also turns off
G3t's direct-output path (step 3). The design itself says "captured application timing is never an uncaptured
baseline". Yet the only fresh swap trace M2.2 requires is captured. The qualified cadence would then be the
captured app's, not the shipping configuration's.

**Should be:** add a capture-off traced arm (same build, input and store) as the cadence row. Keep the capture-on
arm for content and observer overhead, and label any cadence figure from it as "under capture".

### F3. Mac M2 has no route to done: trace finalization is parked with no owner (verified from documents)

M2.2 requires "a fresh readable Game Performance trace and `displayed-surfaces-interval` export". The plan
(`PLATFORM-RUNTIME-MAC-PLAN.md:74`) keeps Instruments trace finalization "parked after three attempts; only a
separately resumed tool investigation may spend further attempts on it". No milestone, module, build-plan line or
maturity entry commissions or owns that investigation. BUILD-PLAN's G3b line names only the T2 dependency. So the owner
unlocking the console still cannot close M2, and the milestone reads as `open` when it is in fact blocked behind
an unowned task.

**Should be:** name the owner and trigger for resuming trace finalization (an item in Mac M2 or a T2 tooling item,
with its stop rule). Or record M2.2 as blocked on it in the status lines and in BUILD-PLAN, so the critical path is
visible.

### F4. Writer throughput for the required continuous interval isn't shown to be feasible (suspected, not measured)

M0.4 and Mac M2.1 require every 1920×1080 frame at 120 Hz requested opportunities, over a two-second pan plus holds.
That is about 0.995 GB/s raw (design's own figure), so roughly 4–5 GB per run. The design forbids blocking the
renderer. It treats any capture overflow as inconclusive, and forbids reducing resolution or sample count. It defers
faster encoding "only if measured throughput requires it".

It never states a writer strategy that sustains 120 lossless 1080p frames per second inside bounded CPU queues. One
such strategy is to spill raw rows to disk with a worker pool and encode afterwards. Single-threaded PNG at 1080p is
far slower than 8 ms. Without that, M0.4 and M2.1 may be unpassable by construction, or push the builder toward
oversized queues.

**Should be:** state the writer strategy in the design (for example, raw spill with N workers, lossless packaging
after the interval). State the per-run storage allowance under ASSET-LIFECYCLE. Make an early M0 step a measured
throughput probe.

### F5. Paused-world offscreen frames hit an existing no-render gate (verified in code; consequence suspected)

`FrameHost::finish_advance` returns without rendering when `completed_ticks == 0`, there is no recovered
publication, and `surface.is_none()` (`crates/thrum-render/src/runtime.rs:946`). With the world paused, as the Mac
pan requires, no ticks complete. A headless driver that passes no output target therefore submits nothing.

The design's "supply an engine-owned output target instead of a drawable" can avoid this, but only if that target
goes through the same presenter and `SurfaceOutput` path as a drawable. That isn't stated. The risks are a
capture-only bypass of the admission gate, or ticking the world to force frames.

**Should be:** state that the headless target goes through the presenter path. Have M0.3 or M0.4 ask that
paused-world camera edits produce fresh submissions through the ordinary gate, with no capture-specific branch.

### F6. No item has a `check`, so the loop reruns nothing (verified from documents)

All T2 items are `ask`/`inspect` only, with no `lock`. Row padding, odd widths, channel order, stale-generation
rejection, ring saturation, disabled-capture zero allocation and incomplete-manifest refusal are naturally automated
real-Metal tests. G0 already makes Metal regressions fail without a device. The plan's reason ("no nonexistent CLI is
a JSON check") doesn't apply to `./dev check`.

**Should be:** give M0.1 and M0.2 (and M1.2/M2.2 for their targets) `"check": "./dev check"` with `"lock": "gpu"`.
Keep the `ask` naming the specific tests the reviewer must see.

### F7. Items bundle several qualities (verified from documents)

- M0.4 judges three things: the paused pan, the running-world interpolation, and the stall control.
- M0.5 judges five: overhead, working-set scaling, the runbook and handoff, storage retention, and refusal of an
  incomplete manifest. The refusal also duplicates M0.2 and M0.3.

A failure then can't point at one thing.

**Should be:** split M0.4 into the paused pan, the running world and the stall control. Split M0.5 into overhead and
documentation, and drop the duplicate refusal.

### F8. The locked-console condition: evidence not cited, and how it is obtained not stated (verified from documents)

The design rests on locked-console Metal execution but cites none of the existing evidence (see "What checks out").
`parity-20261109.json` doesn't record the lock state, even though the probe file shows it was locked. The plan also
doesn't say how M0.3's locked condition is obtained. It could be opportunistic, while the owner is away, or by an
agent locking the session, and an agent locking an in-use session would disrupt the owner.

**Should be:** cite the probe and the parity run as the feasibility basis. Say the locked condition is taken
opportunistically and recorded, never forced on an active user, and never unlocked by automation.

### F9. The stall-injection mechanism is unspecified (suspected)

M0.4 needs deliberately injected render and input stalls. The design doesn't say where the hook lives. A stall flag
compiled into ordinary builds or defaulted on is a known fingerprint of work that only looks finished.

**Should be:** a declared scenario or diagnostic action that is off unless the scenario requests it. The M0.4 `ask`
should check it can't run in an ordinary run.

### F10. Leftovers and capture gaps in consumers (verified from documents)

- Mac M2.2 keeps `"evidence": "docs/evidence/render/mac-m2-pan/README.md#review-verdict-and-open-cadence-row"` while
  `status` is `open`. That is a builder field carried from the old item. Clear it, so a later claim isn't read
  against historical evidence.
- ORCHESTRATE's chrome-webgpu and ios-sim capture columns now name only the unbuilt T2 M1 and M2. AI-DEVELOPMENT says
  that until they exist, consumers record a dependency or an unavailable result. That blocks fixed-time and
  function-only work the existing canvas endpoint capture and `simctl` screenshots could still prove, and nothing
  says whether that is intended. State which existing captures stay valid for which claims until M1 and M2 land.

## Done-when judgeability

| Item | Could a reviewer judge it? | Would it catch a convincing fake? |
| --- | --- | --- |
| T2 M0.1 | Yes, by source trace and tests | Mostly. Without a `check` (F6), tests can rot between reviews |
| T2 M0.2 | Yes | Yes, given the stale-callback and saturation controls |
| T2 M0.3 | Yes, if the lock condition is recorded | Yes on clocks. Needs the paused-world gate question (F5) |
| T2 M0.4 | Yes | Yes. The stall-versus-PTS comparison defeats the fixed-step fake. Bundled (F7); throughput risk (F4) |
| T2 M0.5 | Yes | Weak as bundled (F7) |
| T2 M1.1, M1.2, M2.1, M2.2 | Yes | Adequate. Each names its refusal controls and forbids faked visibility |
| Mac M2.1 | Yes | Yes. It forbids the fixed-step, endpoint-redraw and parity substitutes |
| Mac M2.2 | Not as written: its windowed tap is unproved (F1) | No, for the shipping cadence claim (F2). Unreachable while trace finalization is parked (F3) |

## Owner's aim

The aim is met for engine images and runtime motion. Mac M2.1, T2 M0 and every later Mac visual milestone need
neither an unlock nor Screen Recording, and the existing locked-console offscreen run supports the design's
feasibility. The remaining attended row, M2.2's panel cadence, is genuinely attended: a locked session stops
presentation. But as planned, unlocking alone doesn't close it (F1–F3).

DESIGN CHANGES: 10 findings

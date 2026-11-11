# PLATFORM-RUNTIME-IOS M1–M7: records

Moved from the module plan on 11 November 2026, when the plans took the orchestrate 1.7.0 shape. M1–M6 were planned, built and reviewed before the loop, so these sections are their only acceptance record; nothing here was accepted by the loop. M7 is the loop's milestone, and [its review](M7-review.md) is its record; its work list is kept for the handoff notes under it. Sections are unchanged apart from their place.

## Historical reviews

2026-11-08 M5/M6 close review of `route-full6` and the uncommitted tree on `d063f2e8` (fresh subagent, brief
`tmp/g3b-ios-rotation/close-review-brief.md`). M6 passed. M5 needed changes, with one blocking finding: winit's
creation and layout `Resized` events reconfigured each new window at a transient extent, so `Resized` now reads the
layer extent. The non-blocking findings were also taken: publish inside the background-checked section, recheck the
candidate after a refit, and treat an error after the loss as a held drawable. The same reviewer rechecked
`route-full7` and passed both M5 and M6.
2026-11-08 independent review of the M5/M6 revision (fresh subagent, brief in the session; verdict: changes
required, three blocking: one extent source, stale-extent claim, `destroy` needs a poll; all taken into M5/M6 above).
2026-11-07 independent design/plan review (fresh subagent given the commission, this plan, the contracts and
the work-in-progress tree): changes required, two blocking. Answered below; see *Review answers*. Thrum Core's
rulings on seam 2 (approved with conditions) are answered in *Root conditions*.

## Review answers

1. *Seam 2 overstated the witness* (blocking). The admission witness is now the controlled actor's committed position,
   read through the facade fix in seam 2. Media time is named (FFI `CACurrentMediaTime`).
2. *Release before `suspended` returns was false* (blocking). The guarantee is revocation, with physical release at most
   one bounded section later; a Swift detach bridge closes the window and removes the sublayer, witnessed by weak
   references.
3. *Unbounded `nextDrawable`.* Bounded-section argument, acquire-wall witness and pending-install state stated above.
4. *Lifecycle intent, `Busy`, overload, prepare gap.* Stated in the walks and world survival.
5. *Touch cancellation at resign-active, HID driver.* Stale release uses replacement with a held finger; the driver
   is a self-hosted XCUITest runner, so the route needs no person.
6. *Stall placement and assertions.* The stall stays after acquire; thresholds and rows listed above, including
   background during a stall.
7. *Outstanding job tracking.* `try_send` to `try_recv`, stated above.
8. *Wording and cost.* Lock wording and proxy events corrected.
9. *Milestone shape.* Touch moves earlier (M2) so the driver risk lands before lifecycle; baseline screenshot added;
   resize labelled unproven; `Send` kept as a compile-time assertion.

## Root conditions

Thrum Core approved seam 2(a) and 2(b) with conditions. Each, in turn:
1. *Sample only when the route needs a position.* The owner reads the page only for a `sample-actor` route command;
   display callbacks and render reports never do. The route samples before and after each touch row.
2. *Workspace and Wasm checks together; keep the committed-movement test.* `./dev check` (which includes the game's
   Wasm Clippy) and `./dev check-web` run on the same tip; `tests/host_gpu.rs` asserts the admitted axis moved the
   committed actor.
3. *Real simulator HID driver, XCUITest separate.* The touch rows use real simulator HID input synthesised by a
   self-hosted XCUITest runner that attaches to the running app; it is not an app-side test and adds no code to the
   app. The simulator control tool needs a person's per-device grant and was refused while unattended; when granted,
   the same rows can be repeated with it, and the checker's witnesses are driver-independent.
4. *Fail on boot failure, a missed interaction or a missing committed-state witness.* Boot, launch, link and plist
   failures stop `check-simulator.sh`; each touch row has a bounded window and fails when the app sees no gesture; each
   position sample fails on a timeout or an absent actor.
5. *Manual steps and simulator identity.* The route log records device name, UDID, device type and runtime, states that
   there are no manual steps, and logs each `simctl`, Darwin notification and XCUITest command with its time.
6. *Stale input on the actual old window id, with before/after positions.* The held finger's later events arrive under
   the detached window's id (`old_window_touches`); the committed position sampled after the replacement cleared input
   and again after those events must match within 1 mm, and no moving axes may be sent after the clear. In every
   logged run UIKit delivered the events before the first sample, so the samples do not bracket them: the proof is
   that no moving axes were sent after the clear, and the actor then stays put. The log says which case occurred.
7. *Exact lifecycle guarantee and a race exercised without a GPU/fence wait.* See *Linearization point* above and the
   raced-present row.

## M1 Off-main frames: done

Done when:
- [x] The packaged game launches on a fresh simulator and presents through `CADisplayLink`; the checker still fails on
  boot, launch, link, ProMotion plist or a missing proof.
- [x] Thread witnesses show display targets and touches on UIKit main, frames on the render owner and ticks on the
  world lane. Pending frames never exceed two, and acquisitions never exceed presentable encodes.
- [x] A 5 s stall inside the render owner's lease section does not stall UIKit: display callbacks keep arriving,
  UIKit handlers during it stay under 1 s, and world ticks advance during the stall.
- [x] Visual check: the scene renders as before (screenshot compared with the fresh baseline).
Close review (2026-11-07, fresh subagent, read-only, `route-fresh-7`): PASS. No UIKit path waits on the owner or GPU;
the plan's "`try_lock` only" overstated it (reworded). In-game review: startup matches the pre-change baseline
(letterboxed sky and ground, the two cubes, the entity grid); hidden is black; the scene returns after replacement
and both resumes.
Work:
- [x] Render owner, lease slot and request/reply types; prepare on the owner.
- [x] Absolute display target and owner-side lead.
- [x] State file and checker v2 (startup and stall rows); iOS Clippy in the checker.

## M2 Touch input: done

Done when:
- [x] One real simulator swipe moves the controlled actor's committed position and visibly moves the player
  (before/after captures). The actor is the grid's top-right square, unidentifiable among 10,000; a one-eighth nudge
  steps it just past the grid edge, where the cropped before/after captures show it alone against the sky.
- [x] A cancelled touch clears the axes, and the actor stops: held input clears on background and replacement on the
  simulator; a UIKit `Cancelled` phase is covered by the gesture unit test (the simulator route never produced one).
Close review (2026-11-07, fresh subagent, `route-fresh-8` at `ccce4402`): PASS. Visual: the nudge crop shows the
actor alone past the grid edge, about 130 px right of it, matching +0.412 m; the swipes leave the frame and are
witnessed by committed position.
Work:
- [x] Gesture state machine with unit tests (hold, cancel, epoch, stale release, second finger).
- [x] Touch rows in the checker with a bounded wait window for the driver.

## M3 Lifecycle and surface recovery: done

Done when:
- [x] A real background (Settings) / foreground (`simctl launch`) round trip keeps the PID and world epoch. Ticks freeze while
  backgrounded, presentation resumes on a new epoch, revocation held when `suspended` returned, and no detached
  window stays alive.
- [x] A foreground surface replacement presents again on a new window and epoch with the same world, with ticks
  continuing; a finger held across it has its later release counted stale, and the actor stays at rest.
- [x] A replacement during a lease-section stall defers release, rejects the old epoch's drawable and recovers; a
  background during a stall recovers too.
- [x] Hiding the view denies drawables, counted apart from missing ones, while ticks continue; showing it resumes.
- [x] After background and replacement, a new swipe still moves the player.
Close review (2026-11-07, fresh subagent, `route-fresh-8` at `ccce4402`): PASS, both earlier blockers verified fixed.
Visual: hidden is black; the scene returns after replacement, both resumes and the recovered swipe. The
resume-during-preparation branch is proven by review only; no route row exercises it.
Work:
- [x] Shell state machine, Swift detach bridge, world lifecycle requests/replies; Darwin-notification route commands.

## M4 Closeout: done

Done when:
- [x] `./dev check` green, the named simulator route passes, evidence paragraphs and iOS maturity updated, Thrum Core
  confirms the G3b wording, and an independent in-game review passes. Thrum Core accepted the closeout and recorded
  the G3b wording in BUILD-PLAN itself (`tmp/g3b-ios-boundary/NOTES.md`, "Root acceptance of the simulator closeout").

## M5 Rotation: done

Done when:
- [x] Mid-route, after the touch rows, the simulator rotates portrait → landscape → portrait through real device
  orientation changes (XCUITest `XCUIDevice.orientation`, no person). Each rotation bumps the surface epoch through
  `Resized`, reconfigures once at the full-bounds extent, and presents again on the same lease slot and PID (the same window unless a drawable was held, walk step 6).
- [x] Old-extent presents are counted, not inferred: the owner compares each drawable's size with the extent the shell
  last observed before presenting, and the route reports every old-extent present after a resize (target zero, with
  any residual race named). At most one present on a resize-revoked epoch: the revocation race bound. A section
  that passed its last epoch and extent check before `Resized` landed may present once after it (seen once, at host
  load ~455, `route-full5`). Preventing it would need UIKit to wait on the section. Every old-extent present must be
  that raced present, and `resizeAspect` shows it undistorted.
- [x] A frame strip at 20 fps from `simctl io recordVideo` (an offscreen capture of the running game) covers each
  rotation; I review it myself and record what shows between the bounds change and the first new-extent present.
  No distorted frame may persist after that present.
- [x] Presentation is continuous: the first new-extent present comes within six frame replies of the resize (dev runs
  measured two and three: one reconfigure wait for the at most two pending frames), and no frame error occurs.
- [x] The world survives: ticks advance through both rotations, and the controlled actor's committed position sampled
  before and after each rotation (no input; the idle actor has no drift) agrees within 1 mm.
- [x] Input still maps: a swipe in landscape moves the committed actor (screen-up is forward in either orientation).
- [x] `./dev check` passes on the tip; the evidence record, BUILD-PLAN line proposal and maturity entry are updated;
  an independent close review of the route and the strip passes.
Work:
- [x] Measure first: dev run 1 showed the squeeze above and a two-reply gap; dev run 2 (full-bounds extent,
  `resizeAspect`) showed one epoch bump per rotation, a three-reply gap, zero old-extent presents and no distorted
  frame in either strip.
- [x] One extent source (`outer_size`) for attach, `Resized` and backstop.
- [x] `Resized` handling and resize witnesses (resize events, epoch per resize, gap replies, presented extents,
  old-extent presents).
- [x] XCUITest orientation tests; route rows; video capture and strip.

## M6 Device loss: done (Thrum Core approved the seam with conditions)

Seam: owned iOS code only, reusing the reconstruction API the Mac render actor uses. Thrum Core's verdict
(`tmp/g3b-ios-boundary/NOTES.md`, "Root verdict on M6 simulator device recovery") approved it within the existing iOS
ownership and accepted `Device::destroy` as a function-only simulator witness. Any shared API change returns for
review.
Walk:
1. *Generations.* Preparation captures `DeviceCapabilities` at platform generation 1 and builds the host with
   `FrameHost::new_with_capabilities`, so host and platform generations match; every replacement is strictly newer.
   The loss callback is registered before the host is built and records its own generation.
2. *Detection.* `destroy` only marks the device invalid; wgpu fires the lost callback from maintenance once the queue
   empties. After the route's `lose-device`, the owner calls `destroy`, then non-blocking `poll(Poll)` once per job
   until the flag is set (never `Wait`). From the flag onward it makes no call on the lost device.
3. *Revocation.* On loss the shell (UIKit, no wait) revokes the surface lease by bumping the epoch and records the
   lost generation; the display link and world lane keep running; requests carry no acquisition until recovery.
4. *One recovery in flight.* UIKit sends one `Recover { from }` job and accepts its reply only if `from` is still the
   current generation. A lost candidate, a stale reply, a rotation or a background arriving during recovery leaves
   exactly one recovery in flight: rotation updates the extent the candidate is fitted to
   (`PreparedDevice::resize_surface` before publication); background defers publication to resume; a lost
   candidate is retried from the same base generation, and the shell fails after three failed attempts.
5. *Publication.* The owner snapshots inputs, requests a fresh adapter and device (callback first), prepares at the
   current format and extent, checks candidate loss and the lease generation, publishes, swaps its `Gpu`, and
   configures only under the current lease and epoch. The background check and the publication happen inside one lease
   section. Suspend sets `background` before it revokes, and it cannot take the surface while the section holds it, so
   no publication lands in the background. Candidate loss is checked again after a refit.
6. *A drawable held across the loss replaces the surface.* wgpu-core's `Surface::discard` checks device validity
   before clearing the acquired texture, so a drawable dropped after `destroy` leaves the surface "acquired"
   (`AlreadyAcquired` on the next acquire; the first configure fails with `PreviousOutputExists` after clearing it).
   Replacement does not depend on that quirk. A frame error after the loss is treated as having held a drawable. When a lost frame had acquired a
   drawable, UIKit runs the replacement path (revoke, detach, fresh window and wgpu surface) alongside the recovery;
   the new device configures the new surface. A loss with no drawable held keeps its window. Found in M6 dev run 1.
7. *Route witnesses.* `lose-device` sets a flag the owner consumes at its next job (or inside a held section) and
   calls `destroy`. `hold-recover` parks the recovery after preparation and before the refit check, so a rotation or
   background lands between preparation and publication; it is capped at 300 s (the XCUITest rotation took 109 s at
   host load ~190). `stale-wake` makes the owner send the previous generation's loss through the lost-callback wake
   and the shell send one request still carrying it; both must be refused. A candidate discarded by a background
   consumes its generation number, so generations stay strictly increasing but may skip.
Done when:
- [x] The route observes the real wgpu destroyed-device callback, a fresh device generation and surface epoch, and a
  newly submitted and presented frame on the replacement, on the same lease slot and PID (the same window unless a drawable was held, walk step 6).
- [x] No old-generation present: a frame held across the loss (`stall-render` with `lose-device` landing inside it)
  and a stale old-generation wake are both refused, counted, and cannot present on the replacement.
- [x] The world and controls keep authority through the handoff: ticks advance during recovery, the committed actor
  agrees within 1 mm before and after, and a swipe after recovery moves it.
- [x] Overlap: a loss during a rotation and a loss across a background each end in one coherent recovery (one
  publication, no second in flight).
- [x] UIKit never waits: handlers during recovery stay under 1 s; preparation runs on the render owner.
- [x] The evidence states the simulator limit: `destroy` exercises recovery, not detection. wgpu-hal's Metal backend
  maps no command-buffer error to a loss, so a physical iPhone row must first show that a real GPU fault or watchdog
  loss is detected at all, then the same recovery. That row stays open.
- [x] Install contention is not fatal (daily main review M1, 8 Nov, same lifecycle path): a `try_install` that meets
  the owner inside `section()` keeps `restore` set and retries from the next reply or target, and attaches the new
  window to the scene only after the install succeeds, so no window is left in the scene.
- [x] `./dev check` passes; evidence and maturity updated; independent close review passes.

## M7 Scene and internal render size follow the oriented iOS surface: claimed

The [child design](../../plans/PLATFORM-RUNTIME-IOS/DESIGN.md) owns this correction across the UIKit shell, shared `thrum-render`
`FrameHost`/prepared-device path and V1 threaded facade. G5 remains the cooked draw owner; G4 is not a prerequisite
for native-resolution framing. Preserve explicit fixed-render-extent surface-only consumers on Mac, browser and
capture. Active loop definitions begin at M7 in the new DONE file. M1–M6 below are historical pre-loop closeouts,
not newly claimed/accepted milestones; their evidence and review history are retained without forging acceptance.

Work and proof:
- [x] Add the coupled scene/surface extent transition on the render owner and expose it to the existing iOS host.
  Account for all targets/bindings, camera aspect, history reset, pending frames, zero extents and retryable failure;
  preserve size-independent content and surface-only caller behaviour (M7.1).
- [x] Replace iOS's fixed 640×360 preparation/resize with the current full-bounds pixel extent. Extend actual
  target/projection/presenter witnesses and assertions in the existing route, then run
  `bash examples/v1-game-ios/ios/check-simulator.sh` under `gpu` on its fresh owned simulator (M7.2).
- [x] Inspect native startup/landscape/returned-portrait screenshots, full-resolution crops and both rotation strips
  against M7.3. Full viewport alone is insufficient: actual internal targets must match too.
- [x] Refit scene and presenter together when rotation overlaps prepared-device recovery. Extend the existing
  background/replacement/loss rows, including `hold-recover` plus rotation; retain M5/M6 lifecycle bounds (M7.4).
- [x] Record attachment bytes, pixel count and peak transition overlap. Prove identical extents reuse resources and
  steady frames add no resize allocation/readback work; simulator results remain function only (M7.5).
- [x] Update selected evidence under `docs/evidence/render/ios-extent/`, iOS maturity and the G3b work list; close
  owned scratch under `tmp/ios-extent/` and record the consumer handoff (M7.6).

No dependency on unaccepted COOKED-DRAW or T2 simulator capture is added: M7 uses the existing diagnostic scene.
After M7 acceptance, COOKED-DRAW must rerun M1.9–M1.11 on the integrated host, retaining every M1.10 requirement.
The checkpoint evidence is accessible through `git show b0dae34c:docs/evidence/render/cooked-draw/README.md` and
`git show b0dae34c:docs/evidence/render/cooked-draw/rotation/layout-failure.json`; do not import its code wholesale.
The lead approved the reviewed design at `5dacae7b`; the child design remains the exact reviewed specification.
M7 is claimed for independent milestone review. COOKED-DRAW M1 continues to wait on M7 acceptance.
No owner decision is needed within this scope.

Planner validation: `RUSTC_WRAPPER= ./dev check-docs` and `git diff --check` pass. The loop's schema,
planner-delta and dependency-cycle checks pass, and all COOKED-DRAW item values are unchanged. An in-memory
`cmd_ready` preview using the loop's post-plan authorisation shows iOS `design_owed`, then M7 `ready` after design
approval, while COOKED-DRAW M1 waits on M7. This previews the pending commit; the ordinary CLI reads the integrated
tip and will see these definitions only after the enclosing loop lands them. No acceptance file was written.

## Follow-ups (outside M5/M6)

- M5/M6 close-review carry-overs (not blocking):
  - the old-extent witness mostly repeats the epoch check; only the strip covers the moment before `Resized`;
  - the route checks that the landscape swipe moves the actor, not its direction;
  - "ticks advance during recovery" is checked across the row;
  - install contention (`install_retries`) is proven by review, not exercised.
- UIKit handler timing: `route-full7` at load ~210 measured 0.88 s outside stalls and 38 ms during stalls and
  recovery; judge against a quieter host.
- Physical iPhone: a real GPU fault or watchdog loss must first be shown to be detected (wgpu-hal Metal maps none),
  then the same recovery.
From the 7 Nov close review and the 8 Nov daily main review (`chunk-E.md`), triaged; none affects the function claim:
- Formal happens-before for revocation during configure (a read-modify-write epoch check).
- Touches on a Busy-deferred window before `finish_release` are dropped, not counted stale (L1; no state effect,
  `clear()` already revoked the id).
- Route only: a replacement while hidden creates a visible window (L2); Darwin observers are re-registered on each
  scene connect, so a reconnect would double each command (L3).
- An acquired drawable that `submit` does not present reports `NoView`; give it its own status (L4).
- `debug_assert!(frame.epoch <= current_epoch)` checks nothing (L5); removed with the M6 witness changes.
- `CAMetalLayer` properties set by configure on the render thread land in an implicit transaction; check compositing
  properties apply promptly on a physical device (L6).
- Can a pre-Resume overload report start a second episode in `WorldPolicy`? Trace `report.overload` and add a unit
  test (L7).
- L8 (no-op example binaries off macOS) is outside this shell; passed to the overseer.

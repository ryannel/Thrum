# Platform runtime iOS shell plan (G3b)

Design: [PLATFORM-RUNTIME.md](PLATFORM-RUNTIME.md#i1-phone-shell-lifecycle-slice--selected-target-partial-implementation),
[V1 iOS surface decision](../decisions/V1-IOS-SURFACE.md), contracts [C02](../CONTRACTS.md#c02-time-and-committed-state--f1-coordinator-w1-world),
[C06](../CONTRACTS.md#c06-gpu-execution-and-lifetime--v1), [C07](../CONTRACTS.md#c07-render-scene-views-and-temporal-data--v2-with-p4v3v4)
and [C11](../CONTRACTS.md#c11-input-audio-and-ui-lifecycle--p1a1u1-with-f1).
Commission: Thrum Core's G3b iOS boundary (`tmp/g3b-ios-boundary/NOTES.md`).
Reviews: 2026-11-08 M5/M6 close review of `route-full6` and the uncommitted tree on `d063f2e8` (fresh subagent, brief
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
Milestone: M5 and M6 closed · Updated: 2026-11-08
Now: M1–M6 closed on the simulator. Close review passed on `route-full7`. Evidence:
`docs/evidence/render/G3B-IOS-SIM-ROTATION-LOSS-2026-11-08.md`.
Next: Thrum Core's call on the BUILD-PLAN G3b wording; a quieter timing rerun of the UIKit handler rows; the
physical-iPhone rows (loss detection, timing) remain open.
Blocked: none.

Scope: the iPhone 18 Pro / iOS 27.2 simulator as a function target. M1–M4 covered off-main frames, touch, lifecycle and
surface replacement; M5 adds rotation and M6 device loss. Simulator timing, a physical iPhone and G3b's Mac live rows
are out of scope. Portrait sizing and non-integer presenter scaling belong to G4 (Thrum Core's disposition of the
simulator visual findings): rotation proves the lifecycle, not the framing. Pre-change reference:
`tmp/g3b-ios-lifecycle/late-acquire/` (serial UIKit callback, 601 presented, 600 completed) and
`tmp/g3b-ios-lifecycle/current/*.png`; M1 captures a fresh baseline screenshot from the pre-change build before the
first post-change run.

## Shell design

The shell keeps the existing V1 game, `FrameHost`, world lane and frame tokens. It adds no scheduler, queue, clock or
game rule. It moves the work UIKit currently runs inline onto the two owners the engine already has.

| Owner (thread) | Holds | Does |
| --- | --- | --- |
| UIKit main (winit event loop) | `UIWindow`/`UIView`/`CAMetalLayer` via `Arc<Window>`, Swift `CADisplayLink`, `WorldControls`, surface epoch, lease slot (install/remove only), gesture state, shell state | Forwards display targets and touches, runs lifecycle, creates and detaches surfaces. Never polls the GPU or waits on a section: the lease slot is `try_lock` only; the `FrameWorker` and `WorldLane` slot mutexes it takes are held by the other side only for a move, never across work |
| World lane (`WorldLane`, thrum-core) | `ClockedWorld<GameWorld>`, rules, held axes | The only world clock; admits changed axes as `InputFrame`s through `Simulation::admit_frame`; publishes pages; wakes UIKit |
| Render owner (`thrum_render::frame::FrameWorker`, thread `thrum-v1-ios-render`) | wgpu adapter/device/queue, `ThreadedFrameHost`, surface configuration | Prepares the device and host, advances/encodes/submits, acquires late, presents, polls completion |

The handoff from UIKit to the render owner is the `FrameWorker`'s one request slot and one reply slot. UIKit marks a
job outstanding from `try_send` until its `try_recv`, and keeps only the latest display target meanwhile. `FrameHost`
keeps at most two submitted frames. Media time on the owner comes from `CACurrentMediaTime` through a one-line FFI
declaration in owned code (`objc2-quartz-core` is registered macOS-only).

**Surface lease.** One `Mutex<Option<Leased>>` slot holds the current `wgpu::Surface` and its last configuration. Only
UIKit installs or removes a surface, always with `try_lock`. The render owner locks the slot only for configure, a
format query during prepare, and the acquire → submit → present section; no lock is held across encoding or
advancing. A shared `AtomicU64` surface epoch changes on every invalidation (suspend, replacement, extent change).
Each request carries the epoch it was made under. The owner checks it on entering a section, after acquiring and again
before presenting. A mismatch drops the drawable unpresented (`skip`) and counts a stale rejection.

*Linearization point.* The slot mutex is the single point that orders revocation against a section. Revocation is
"bump the epoch, then `try_lock` the slot"; a section is "lock the slot, check the epoch under the lock, acquire,
check, submit, check, present, unlock". An epoch load alone does not stop a revocation landing between it and a
native call, so the guarantee is stated per case:
- *Slot free when UIKit's `try_lock` runs.* UIKit takes the surface out before `suspended` returns. Any later section
  finds the slot empty (`NoLease`). Revocation and release are both complete when the callback returns.
- *Slot held (`Busy`).* That section was admitted under the old epoch before the revocation. It is the only section
  that can touch the old surface afterwards, and it can do at most this: a revocation before its post-acquire check
  drops the drawable unsubmitted; one before its pre-present check drops the rendered drawable unpresented; one after
  the pre-present check lets it present that one already-acquired drawable onto the old layer. The layer is still
  attached then, because the detach runs only after the section releases the lease (on its reply, before UIKit sends
  another job). The witness `raced_presents` counts this case; it is at most one per revocation.
- *After `suspended` returns* no new section is admitted on the old surface: the next section's check under the lock
  sees the new epoch, and UIKit has taken the surface on the reply before any further job.

The route exercises the third sub-case without any GPU or fence wait on UIKit: `stall-present` holds the section after
its last epoch check and before `present` for 8 s, a replacement lands inside that window, and the checker requires
exactly one raced present, a deferred release, the old window's detach after the release, and every UIKit handler
during the hold under 1 s. `stall-render` holds after acquire and requires the drawable to be dropped stale instead.

*Why the section is bounded.* wgpu's Metal backend sets `allowsNextDrawableTimeout = false`, so `nextDrawable` can
block while every drawable is held. The owner presents or drops each drawable before leaving the section, keeps
`desired_maximum_frame_latency = 2`, and enters only after an encode that can present, so a drawable is free unless
the compositor holds them all. The witness records the maximum acquire wall. UIKit never waits on the section: a
held section makes its removal `Busy`, and the release completes on the next reply. If the owner never replies, the
surface stays revoked (no present can happen) and the checker fails on missing replies; the UI stays responsive.

**Walks.**

- *Foreground start.* `resumed` creates the window and attaches it to the active scene (existing Swift bridge),
  creates the surface on the main thread (raw-window-metal requires it), installs it, and sends one `Prepare` job.
  The owner requests the adapter and device, briefly locks the slot to pick a format, builds `FrameHost`, moves its world
  onto the lane (`into_threaded`) and replies with `WorldControls`. The display link starts on that reply whenever the
  shell is active, including after a resume that arrived during preparation. Between `into_threaded` and that reply the world
  lane already runs and UIKit cannot yet pause it; if the app resigned active meanwhile, the shell has recorded the
  background intent and requests the pause as soon as the controls arrive. UIKit then starts the display link if
  active. Startup compilation runs off UIKit.
- *Display target.* The link callback sends the absolute `targetTimestamp`. UIKit sends a frame request if no job is
  outstanding, else keeps the newest target. The owner computes the lead with the shared `display_target_lead`, then
  `advance` → `encode_scene` → (lease section) epoch check → `get_current_texture` → epoch check → `submit` →
  epoch check → `present`. A drawable is acquired only after an encode that `can_acquire_surface`.
- *Missing or denied drawable.* `Timeout`/`Occluded`/`Outdated` from wgpu is *missing*. The encoded view is skipped and
  committed ticks stand. While the shell hides its view, requests carry `present: false`, and the owner advances
  without acquiring. That is *denied*. Both are counted separately.
- *Resize and rotation* (M5). The surface extent is the view's full bounds in pixels (winit `outer_size`, the size
  `Resized` reports and the Metal layer tracks). winit's iOS `inner_size` is the safe area; using it configured a
  drawable smaller than the layer, which the layer stretched. Attach, `Resized` and the dispatch backstop use the one
  source. The current window's `Resized` is handled on UIKit as it arrives: an extent change bumps the surface epoch
  at once; an in-flight section then drops its old-extent drawable at its next check, exactly like a revocation. The
  next request carries the new epoch and extent. The owner reconfigures only when no frame is pending (`Reconfigure`
  replies meanwhile), resizes the host's presenter bindings with `resize_surface`, then configures the surface. The
  window, surface, lease and world stay the same: rotation is not a replacement.
  *What the epoch cannot stop.* UIKit animates the layer's bounds through a rotation, and an acquire just before
  `Resized` can still return an old-size drawable under a current epoch (Metal reports no `Suboptimal`). With
  raw-window-metal's default `kCAGravityResize`, the first dev run showed the new landscape drawable squeezed into the
  still-portrait animating bounds (`tmp/g3b-ios-rotation/dev-run1`). The Swift bridge now sets the Metal sublayer's
  gravity to `resizeAspect` when the surface is created: during the animation the compositor scales the drawable
  without distortion, and at rest the drawable matches the bounds, so nothing changes there. Before each present the
  owner also compares the drawable's size with the extent the shell last observed (a witness value beside the epoch)
  and drops a mismatch; a present found mismatched afterwards is counted as an old-extent present.
- *Suspend* (`willResignActive`, winit `suspended`). UIKit, synchronously and without waiting: stops the display link,
  bumps the epoch, clears the gesture and sends zero axes, records the background intent and requests
  `WorldLifecycle::Pause`, then `try_lock`s the slot to take the surface. **The synchronous guarantee is revocation**:
  when `suspended` returns, no request can acquire, submit or present on the old surface. The physical release follows
  at most one bounded section later: if the owner is inside a section, removal is `Busy`, and UIKit takes the surface
  on that job's reply, before sending any further job. Detach then runs on the main thread: drop the `wgpu::Surface`,
  then a Swift bridge hides the `UIWindow`, clears its `windowScene` and the scene delegate's reference, and removes
  the `CAMetalLayer` sublayer from the view. winit's iOS `Window` does not close the `UIWindow` on drop, and
  raw-window-metal never removes the sublayer it adds, so without the bridge both would outlive the surface. The
  bridge keeps weak references to detached windows and reports how many are still alive. No GPU wait or fence wait
  runs on UIKit; wgpu's Metal unconfigure only clears a format.
- *Resume* (`didBecomeActive`). A new window and surface (a fresh `UIView` and `CAMetalLayer`) are installed under
  a new epoch. The first request reconfigures, and `WorldLifecycle::Resume` follows the pause reply.
- *Replacement while foreground.* The same path without the world pause: revoke, detach, create both again. It is
  reproducible through a shell command and is the "actual surface loss" row.
- *Stale callbacks.* Display targets that arrive after suspend are ignored by shell state. Old-epoch requests cannot
  acquire or present. World lifecycle replies are consumed by the one designated caller, matched to its request; a
  `Busy` request is retried on the next world wake or frame reply. Touches for a detached window keep its old window id;
  the shell routes their non-start phases to the gesture, which counts them stale.

**World survival.** The world lane runs independently of both UIKit and the render owner. Background pauses it through
the ordered lifecycle slot (local single-player policy, as `thrum-app`'s pause intent). Replacement and denial never
pause it. A fixed-clock overload drains before resume, as on the Mac. Its identity is the `SnapshotId.epoch` in each
lifecycle reply; committed progress is the reply's tick and `check_health`.

**Input.** One active touch, identified by winit touch id plus the shell's input epoch, acts as a virtual stick: the
offset from touch-down divided by 80 points, clamped to the unit square, with screen-up as forward, a 10% dead zone
and steps of one eighth. Changed axes go to `WorldControls::set_controls`, the same mailbox the Mac keys use. The
world step turns a change into an `InputFrame` through `Simulation::admit_frame`. Cancel, suspend and replacement bump
the input epoch and send zero axes. UIKit cancels touches at resign-active, so the stale-release row uses replacement
with a finger held: the old window's later move and release arrive under its old window id and are counted stale.
Admission is witnessed by the controlled actor's committed position from the world lane's published page (seam 2),
so a route that moved the camera instead would fail.

**Touch driver.** `simctl` has no touch command and `idb` is not installed. The route builds a self-hosted XCUITest
runner without an Xcode project (`build-uitest.sh`: Xcode's `XCTRunner.app`, one compiled UI-test bundle and an
`.xctestrun` file) and runs one gesture per row with `xcodebuild test-without-building`. The runner attaches to the
running app by bundle identifier and synthesises real HID touches; the PID is unchanged across gestures. The
simulator control tool was tried first and needs a per-device grant from a person.

**Test triggers** sit at the shell/owner boundary. They are accepted only when the app is launched with
`THRUM_V1_ROUTE=1`. They arrive as Darwin notifications on the main run loop (`notifyutil -p
org.thrum.v1.<command>` inside the simulator): `replace-surface`, `hide-view`, `show-view` and `stall-render` (the owner
sleeps inside its lease section, after acquiring, for 8 s (5 s until 8 Nov), long enough for a revocation posted from the host to land
inside it), `stall-present` (the same hold after the last epoch check, just before `present`) and `sample-actor` (one
read of the controlled actor's committed position from the world lane's published page; display callbacks and render
reports never read the page). Opened URLs were tried first and rejected: iOS asks the
user to confirm each one. Background is real: the checker launches Settings and returns with `simctl launch`, which
foregrounds the running process.

**Witnesses.** The app rewrites one key=value state file at each transition and every 120 presented frames. It records
thread names per role (display forward, touch, lifecycle, prepare, frame, world wake), the maximum UIKit handler wall
(overall and during a stall), the maximum acquire wall, targets coalesced, maximum pending frames, epochs, immediate
and deferred releases, live detached windows, stale rejections, presents under a revoked epoch (zero, except the one admitted final-check race that the `stall-present` row
requires),
missing and denied drawables, presented frames, world epoch/tick at each lifecycle reply, gesture
starts/clears/stale events, sent axes, the controlled actor's committed position, and PID. A compile-time assertion
keeps the render job `Send`.

**Checker thresholds** (function, not timing): every UIKit handler that runs while the owner holds an 8 s stalled
section stays under 1 s, so none waited on it (simulator wall time elsewhere is reported, not gated: host load
deschedules callbacks);
display callbacks and world ticks advance during the stall; zero presents and acquisitions while hidden with ticks
advancing; at least one stale rejection and zero revoked-epoch presents after a stalled replacement; zero live
detached windows after each recovery; a background during a stall recovers like a plain background.

**Cost.** Per frame UIKit does one atomic load, one `try_send`, one small request move and handles about three
proxy events (target, render reply, world wake). The owner does at most two slot locks and one atomic load for the
extent witness. Per touch move, three float comparisons. No per-frame allocation is added: witness file writes and
thread naming run only under `THRUM_V1_ROUTE=1`, thread roles are recorded once, and the owner names its thread on
preparation and the first frame only (daily main review M2, 8 Nov). Window/surface recreation is a lifecycle cost,
not a frame cost.

## Seams for Thrum Core

1. Ownership and lifecycle walk: the design above.
2. Public interfaces: `FrameHost::new`, `into_threaded`, `ThreadedFrameHost`, `WorldControls`,
   `thrum_render::frame::{FrameWorker, display_target_lead}` express the route. Shared changes requested:
   - `.config/architecture.toml`: `v1-game-ios` may depend on `thrum-core` (`WorldControls::request_lifecycle` takes
     `WorldLifecycle`); `Cargo.lock` gains that one dependency line.
   - v1-game host facade: `ThreadedFrameHost::controlled_position()` currently reaches `WorldPort::inline()`, which is
     unreachable on a threaded host. Read the controlled actor from the world lane's current published page instead,
     so the admission witness reads committed state.
3. Input: touch reaches the existing typed admission path through `WorldControls::set_controls`. No new shared input,
   clock or lifecycle API.

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

## Log
- 2026-11-08 Overseer brief: M5 rotation and M6 device loss added; M4 marked done on Thrum Core's acceptance.
- 2026-11-08 Plan review: changes required. winit's iOS `inner_size` is the safe area while `Resized` is the full
  bounds (fixed: one source, `outer_size`); the epoch cannot stop the compositor scaling the last drawable between
  the bounds change and `Resized` (claim restated, old-extent presents counted, gap measured first); `destroy` fires
  the callback only from a poll. Thrum Core approved M6 with conditions (generations, revocation, one recovery,
  stale-generation witness), folded into M6.
- 2026-11-08 Daily main review (`chunk-E.md`): M2 fixed (witness I/O and thread naming only under the route); M1
  folded into M6; lows triaged into *Follow-ups*. Two full-route runs failed on host-load timing in existing rows,
  not on rotation: a 10 s held touch released before a slow `replace-surface` notification arrived (`route-full1`),
  and a revocation posted straight after `stall-render` landed before the stall held (`route-full2`). The held touch
  is now 30 s and the route checks the finger was still down at the replacement; the stall is 8 s and the route waits
  for the owner to report the held section before posting the revocation.
- 2026-11-08 M6 development runs (`tmp/g3b-ios-rotation/m6-dev1..6`). In dev run 1 the held-frame loss faulted
  the surface: wgpu-core's `discard` checks device validity before clearing the acquired texture, so the surface
  stays acquired for any device. A loss that held a drawable now replaces the surface (walk step 6). Three route
  defects followed:
  - the recovery hold sat before preparation, so the rotation never forced a refit;
  - the 90 s hold expired before a 109 s XCUITest rotation at load ~190;
  - the background row counted the designed resume reinstall as a replacement.
  The hold now sits after preparation with a 300 s cap. The route also gained the `stale-wake` row. Dev run 6 passed
  every device row: generations 1→2→3, stale wake and request refused, refit to 2622x1206, one publication after
  resume, and a swipe after recovery moved the actor 9.6 m.
- 2026-11-08 Full route `route-full5` (load 455 at start) failed M5's "zero presents on a resize-revoked epoch":
  one section that passed its checks before `Resized` presented once after it, at the old extent. That is the
  revocation race bound, which UIKit cannot close without waiting. The strip shows a rigid, aspect-preserved
  rotation and no squeezed frame. The done-when now names the bound: at most one, and it must be the only
  old-extent present. That change goes to the M5 close review.
- 2026-11-07 Plan written from Thrum Core's commission.
- 2026-11-07 Independent review: changes required. Revised: revocation guarantee, detach bridge, committed-position
  witness, touch before lifecycle, thresholds.
- 2026-11-07 First dev-simulator runs: Rust leaves the iOS main thread unnamed, so the witness asks
  `pthread_main_np`. `thrum-v1://` URLs raise an "Open in" prompt, so route commands moved to Darwin notifications.
  The simulator control tool needs a person's grant, so touch moved to a self-hosted XCUITest runner. All rows then
  passed on the dev device (`tmp/g3b-ios-lifecycle/dev-run7/`). Screens: startup matches the baseline, hidden view
  black, scene back after replacement and resume. The swipe's on-screen movement is not identifiable among 10,000
  grid entities under a fast day cycle; the committed-position witness (seam 2b) carries that row.
- 2026-11-07 Thrum Core approved seam 2 with conditions (answered in *Root conditions*). Added the `sample-actor`
  witness and the `stall-present` raced-present row. Each `simctl spawn` takes about 1.3 s, so waiting to observe a stall
  before posting the revocation spent most of the 5 s window (dev run 8 missed it); the route now posts the revocation
  straight after the stall request and proves it landed inside by the deferred release. Dev run 9
  (`tmp/g3b-ios-lifecycle/dev-run9/`) passed every row: exactly one raced present, swipes moved the committed actor
  9.7 m each, and the held finger's two old-window events moved it 0.000 m.
- 2026-11-07 M1–M3 close review (fresh subagent): M1 PASS. M2 blocked on visible movement (fixed: nudge row with
  cropped captures; `dev-run10` shows the lone square past the grid edge). M3 blocked: a resume during preparation
  left the display link stopped because only `Starting` started it on the prepared reply (fixed). Non-blocking notes
  taken: lock wording, stale-row bracketing logged. Others recorded for follow-up: in-section epoch check as a
  read-modify-write for a formal happens-before on configure; touches on a Busy-released window before
  `finish_release` go uncounted; stale detection keys on touch id; a real UIKit `Cancelled` phase is unit-tested only.
- 2026-11-07 Thrum Core accepted the race witness (exactly one admitted present, detach after release) and the XCUITest
  driver as the real simulator input witness. The `release_surface` and `Take::Busy` comments now describe the
  final-check race. Overloads on fresh runs (`route-fresh-7`: 1 overload, 893 refused ticks; `route-fresh-8`: 3, 799)
  come from host load on the shared Mac starving the simulated world lane. The fixed clock refuses the backlog and
  drains it before resume. The disposition is logged with each run, and these runs make no physical-device timing claim.
  One display-target callback took 1.6 s in `route-fresh-8` outside any stall, also host descheduling; it is reported
  and not gated.

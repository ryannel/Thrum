# Platform runtime iOS shell plan (G3b)

Design: [PLATFORM-RUNTIME.md](PLATFORM-RUNTIME.md#i1-phone-shell-lifecycle-slice--selected-target-partial-implementation),
[V1 iOS surface decision](../decisions/V1-IOS-SURFACE.md), contracts [C02](../CONTRACTS.md#c02-time-and-committed-state--f1-coordinator-w1-world),
[C06](../CONTRACTS.md#c06-gpu-execution-and-lifetime--v1), [C07](../CONTRACTS.md#c07-render-scene-views-and-temporal-data--v2-with-p4v3v4)
and [C11](../CONTRACTS.md#c11-input-audio-and-ui-lifecycle--p1a1u1-with-f1).
Commission: Thrum Core's G3b iOS boundary (`tmp/g3b-ios-boundary/NOTES.md`).
Reviews: 2026-11-07 independent design/plan review (fresh subagent given the commission, this plan, the contracts and
the work-in-progress tree): changes required, two blocking. Answered below; see *Review answers*.
Milestone: M1 off-main frames · Updated: 2026-11-07
Now: all route rows pass on the session's dev simulator (local checkpoint `0e37cd26`); fresh-device checker run next.
Next: Root's verdict on seam 2, then the committed-position touch witness and M1–M3 closes.
Blocked: pushing waits for Root's verdict on the `thrum-core` permission (`./dev check` refuses it until then).

Scope: the iPhone 18 Pro / iOS 27.2 simulator as a function target. Simulator timing, a physical iPhone, device
loss and G3b's Mac live rows are out of scope. Pre-change reference: `tmp/g3b-ios-lifecycle/late-acquire/` (serial
UIKit callback, 601 presented, 600 completed) and `tmp/g3b-ios-lifecycle/current/*.png`; M1 captures a fresh
baseline screenshot from the pre-change build before the first post-change run.

## Shell design

The shell keeps the existing V1 game, `FrameHost`, world lane and frame tokens. It adds no scheduler, queue, clock or
game rule. It moves the work UIKit currently runs inline onto the two owners the engine already has.

| Owner (thread) | Holds | Does |
| --- | --- | --- |
| UIKit main (winit event loop) | `UIWindow`/`UIView`/`CAMetalLayer` via `Arc<Window>`, Swift `CADisplayLink`, `WorldControls`, surface epoch, lease slot (install/remove only), gesture state, shell state | Forwards display targets and touches, runs lifecycle, creates and detaches surfaces. Never polls the GPU or waits; its locks are `try_lock` only |
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
  onto the lane (`into_threaded`) and replies with `WorldControls`. Between `into_threaded` and that reply the world
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
- *Resize* (designed, not exercised: the simulator route has no rotation step). An observed extent change bumps the
  epoch. The owner reconfigures and resizes the host's surface bindings only when no frame is pending, as the Mac actor
  does.
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
sleeps inside its lease section, after acquiring, for 5 s, long enough for a revocation posted from the host to land inside it). Opened URLs were tried first and rejected: iOS asks the
user to confirm each one. Background is real: the checker launches Settings and returns with `simctl launch`, which
foregrounds the running process.

**Witnesses.** The app rewrites one key=value state file at each transition and every 120 presented frames. It records
thread names per role (display forward, touch, lifecycle, prepare, frame, world wake), the maximum UIKit handler wall
(overall and during a stall), the maximum acquire wall, targets coalesced, maximum pending frames, epochs, immediate
and deferred releases, live detached windows, stale rejections, presents under a revoked epoch (must stay zero),
missing and denied drawables, presented frames, world epoch/tick at each lifecycle reply, gesture
starts/clears/stale events, sent axes, the controlled actor's committed position, and PID. A compile-time assertion
keeps the render job `Send`.

**Checker thresholds** (function, not timing): every UIKit handler that runs while the owner holds a 5 s stalled
section stays under 1 s, so none waited on it (simulator wall time elsewhere is reported, not gated: host load
deschedules callbacks);
display callbacks and world ticks advance during the stall; zero presents and acquisitions while hidden with ticks
advancing; at least one stale rejection and zero revoked-epoch presents after a stalled replacement; zero live
detached windows after each recovery; a background during a stall recovers like a plain background.

**Cost.** Per frame UIKit does one atomic load, one `try_send`, one small request move and handles about three
proxy events (target, render reply, world wake). The owner does at most two slot locks. Per touch move, three float
comparisons. No per-frame allocation is added. Window/surface recreation is a lifecycle cost, not a frame cost.

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

## M1 Off-main frames: in progress
Done when:
- [ ] The packaged game launches on a fresh simulator and presents through `CADisplayLink`; the checker still fails on
  boot, launch, link, ProMotion plist or a missing proof.
- [ ] Thread witnesses show display targets and touches on UIKit main, frames on the render owner and ticks on the
  world lane. Pending frames never exceed two, and acquisitions never exceed presentable encodes.
- [ ] A 5 s stall inside the render owner's lease section does not stall UIKit: display callbacks keep arriving,
  UIKit handlers during it stay under 1 s, and world ticks advance during the stall.
- [ ] Visual check: the scene renders as before (screenshot compared with the fresh baseline).
Work:
- [ ] Render owner, lease slot and request/reply types; prepare on the owner.
- [ ] Absolute display target and owner-side lead.
- [ ] State file and checker v2 (startup and stall rows); iOS Clippy in the checker.

## M2 Touch input: pending
Done when:
- [ ] One real simulator swipe moves the controlled actor's committed position and visibly moves the player
  (before/after captures).
- [ ] A cancelled touch clears the axes, and the actor stops.
Work:
- [ ] Gesture state machine with unit tests (hold, cancel, epoch, stale release, second finger).
- [ ] Touch rows in the checker with a bounded wait window for the driver.

## M3 Lifecycle and surface recovery: pending
Done when:
- [ ] A real background (Settings) / foreground (`simctl launch`) round trip keeps the PID and world epoch. Ticks freeze while
  backgrounded, presentation resumes on a new epoch, revocation held when `suspended` returned, and no detached
  window stays alive.
- [ ] A foreground surface replacement presents again on a new window and epoch with the same world, with ticks
  continuing; a finger held across it has its later release counted stale, and the actor stays at rest.
- [ ] A replacement during a lease-section stall defers release, rejects the old epoch's drawable and recovers; a
  background during a stall recovers too.
- [ ] Hiding the view denies drawables, counted apart from missing ones, while ticks continue; showing it resumes.
- [ ] After background and replacement, a new swipe still moves the player.
Work:
- [ ] Shell state machine, Swift detach bridge, world lifecycle requests/replies; Darwin-notification route commands.

## M4 Closeout: pending
Done when:
- [ ] `./dev check` green, the named simulator route passes, evidence paragraphs and iOS maturity updated, Thrum Core
  confirms the G3b wording, and an independent in-game review passes.

## Log
- 2026-11-07 Plan written from Thrum Core's commission.
- 2026-11-07 Independent review: changes required. Revised: revocation guarantee, detach bridge, committed-position
  witness, touch before lifecycle, thresholds.
- 2026-11-07 First dev-simulator runs: Rust leaves the iOS main thread unnamed, so the witness asks
  `pthread_main_np`. `thrum-v1://` URLs raise an "Open in" prompt, so route commands moved to Darwin notifications.
  The simulator control tool needs a person's grant, so touch moved to a self-hosted XCUITest runner. All rows then
  passed on the dev device (`tmp/g3b-ios-lifecycle/dev-run7/`). Screens: startup matches the baseline, hidden view
  black, scene back after replacement and resume. The swipe's on-screen movement is not identifiable among 10,000
  grid entities under a fast day cycle; the committed-position witness (seam 2b) carries that row.

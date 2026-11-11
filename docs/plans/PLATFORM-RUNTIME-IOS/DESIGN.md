# iOS runtime: scene extent follows the oriented surface (G3b)

9 November 2026. **Selected design; M7 implementation and independent design review owed.**
Child of [PLATFORM-RUNTIME](../../design/modules/PLATFORM-RUNTIME.md#i1-phone-shell-lifecycle-slice--selected-target-partial-implementation).
The existing [shell design and M1–M6 history](DESIGN.md#shell-design) retain lifecycle ownership.
[Plan](PLAN.md) · [done when](DONE.json).
Contracts: [C06](../../design/CONTRACTS.md#c06-gpu-execution-and-lifetime--v1) and
[C07](../../design/CONTRACTS.md#c07-render-scene-views-and-temporal-data--v2-with-p4v3v4).

## Outcome and ownership

On the iPhone 18 Pro / iOS 27.2 simulator, startup and every settled portrait/landscape orientation fill the
current full-bounds pixel surface with a correctly projected scene. Internal rendering uses that same extent,
with a full-surface viewport. Rotation changes the camera aspect and visible field, preserving vertical field of
view; it never stretches or crops a stale 16:9 image to hide borders. OS status overlays and rounded corners are
outside the engine image. The bar is the undistorted, edge-filling orientation behaviour of a shipped Unity iOS
player using its native-resolution camera viewport. This milestone qualifies function only.

**G3b / PLATFORM-RUNTIME-IOS owns this work**, spanning its existing UIKit adapter and shared render-owner
implementation. UIKit produces the one authoritative oriented full-bounds pixel extent and surface epoch. The
render owner consumes that pair, controls target lifetime, and derives the scene projection, render targets and
presenter from it. G5 owns cooked draw selection and content, not view sizing. G4 owns future reconstruction and
dynamic resolution; neither is necessary for native-resolution correctness. This supersedes the old G4 framing
deferral for this baseline only. Shared `thrum-render` changes are part of M7, not a second runtime or scene owner.

The failure was read from builder checkpoint `b0dae34c` with `git show`, including
`docs/evidence/render/cooked-draw/README.md`, `rotation/layout-failure.json` and both rotation screenshots.
The recorded 640×360 source is letterboxed into 1206×2622 portrait and 2622×1206 landscape surfaces. Existing
`FrameHost::resize_surface` and `PreparedDevice::resize_surface` resize the presenter while retaining the scene
extent; the iOS owner supplies a fixed `RENDER_EXTENT`. These are observations, not successful extent proof.
The prior plan/code were inspected before this scoped design; this is not a blind redesign.

## Mechanism and seams

Keep the existing `outer_size`/layer full-bounds observation and epoch-checked lease protocol. Safe-area sizes,
orientation names and hard-coded model dimensions must not become competing extent sources. At startup, prepare
the scene at the current nonzero extent. At resize, coalesce to the newest `(epoch, extent)` on the existing render
owner. Defer while submitted frames remain; poll through the existing nonblocking completion path. Once safe,
prepare the replacement extent-dependent scene resources and presenter bindings and publish them together before
encoding/acquiring for that extent. Failed or stale preparation leaves a coherent prior generation unavailable for
new-extent presentation, with an explicit retry/error. Never publish half a resize or wait for the GPU on UIKit.

Expose the operation through the shared host and V1 threaded facade; its exact API shape is the builder's choice.
Preserve the existing surface-only resize semantics for callers intentionally using a fixed render extent
(Mac, browser and explicit offscreen captures). iOS explicitly chooses the new coupled scene/surface operation.
It updates all extent-dependent attachments and graph declarations, camera aspect, viewport, contribution surface
bindings and presenter source/destination coherently. Retain game/world, camera pose, lighting/exposure and cooked
selection; reset size-dependent history under C07 rather than reusing pixels from another sample grid. Retire old
GPU resources through existing completion ownership. Do not re-upload size-independent content every frame.

Device recovery must retain this sizing policy: a candidate prepared before a rotation is refitted to the latest
scene **and** surface extent before publication, with the existing generation/loss/background checks. Zero-sized
or revoked surfaces do not configure, allocate zero-sized targets or submit; the next valid extent recovers. A
same-extent request reuses prepared resources. Ordinary rotation keeps PID, window, surface lease and device
generation; only surface epoch advances. The existing bounded final-check race during UIKit's animation remains
explicit; `resizeAspect` may preserve the last drawable during animation, but no stale framing survives the first
current-extent present. M5's race bound and M6's loss rules are unchanged.

Alternatives rejected: changing only the presenter still renders a low-resolution stale-aspect scene; stretching
or cropping masks the error; rebuilding the entire host/world loses identity and duplicates authority; assigning
this to cooked assets duplicates view ownership. A governor or reconstruction provider would add a new policy
before this baseline is correct. Revisit native resolution only with a separately planned G4 quality policy.

## Proof, cost and handoff

M7 uses the existing diagnostic scene and XCUITest orientation driver independently of unaccepted COOKED-DRAW.
It needs no `COOKED-DRAW:M1` or T2 simulator capture acceptance. Extend route witnesses to expose actual scene
target dimensions, projection aspect and presenter viewport alongside current surface/drawable dimensions and
epoch. Derive these from the resources/matrices used by an identified presented frame, not echoed requests.
Tests must reject presenter-only resizing, stretching/cropping and stale recovery candidates. Simulator records
and native screenshots/rotation strips jointly prove the geometry and the visible outcome; screenshots alone
cannot prove internal resolution. Missing tools, skipped gestures or unavailable evidence fail the row.

Cost scales with pixel count: 1206×2622 is 3,162,132 pixels versus 230,400 at 640×360 (about 13.7×). Record attachment
formats/counts, bytes and peak old/new overlap; normal frame pixel work grows accordingly. Allocate on distinct
extent transitions only, coalesce transient extents and reuse identical extents. No per-frame target recreation,
extra readback outside route diagnostics, device recreation or CPU/GPU wait is justified. Do not turn simulator
wall times into speed, cadence, power or physical-iPhone claims; no invented performance gate.

After independent M7 acceptance, COOKED-DRAW M1 reruns its unchanged M1.9–M1.11 proofs on the integrated host.
M7 establishes sizing for the shared scene route; it does not accept cooked visuals. T2-CAPTURE M2 continues to
need COOKED-DRAW M1. No owner decision is required for this correction. Independent design review is owed to the
lead/loop; this planner cannot launch agents.

## Scope of the shell (M1–M7)

Moved from the module plan on 11 November 2026, when the plans took the orchestrate 1.7.0 shape.

Scope: the iPhone 18 Pro / iOS 27.2 simulator as a function target. M1–M4 covered off-main frames, touch, lifecycle and
surface replacement; M5 adds rotation and M6 device loss. M7 adds native-resolution scene framing at the current
oriented extent, superseding the earlier G4 framing deferral for this baseline. G4 retains reconstruction/governor
policy. Simulator timing, a physical iPhone and G3b's Mac live rows are out of scope. Pre-change reference:
`tmp/g3b-ios-lifecycle/late-acquire/` (serial UIKit callback, 601 presented, 600 completed) and
`tmp/g3b-ios-lifecycle/current/*.png`; M1 captures a fresh baseline screenshot from the pre-change build before the
first post-change run.

Commission: Thrum Core's G3b iOS boundary (`tmp/g3b-ios-boundary/NOTES.md`).

## Shell design

Moved from the module plan on 11 November 2026, when the plans took the orchestrate 1.7.0 shape. This is the design M1–M6 were built to, before the module had a design file of its own.

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
  replies meanwhile), resizes the host's presenter bindings with `resize_surface`, then configures the surface. That is the historical
  M5 surface-only behaviour; M7 replaces the iOS call with a coherent scene-and-surface extent transition. The
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

Moved from the module plan on 11 November 2026, when the plans took the orchestrate 1.7.0 shape.

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

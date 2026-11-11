# iOS runtime: scene extent follows the oriented surface (G3b)

9 November 2026. **Selected design; M7 implementation and independent design review owed.**
Child of [PLATFORM-RUNTIME](PLATFORM-RUNTIME.md#i1-phone-shell-lifecycle-slice--selected-target-partial-implementation).
The existing [shell design and M1–M6 history](PLATFORM-RUNTIME-IOS-PLAN.md#shell-design) retain lifecycle ownership.
[Plan](PLATFORM-RUNTIME-IOS-PLAN.md) · [done when](PLATFORM-RUNTIME-IOS-DONE.json).
Contracts: [C06](../CONTRACTS.md#c06-gpu-execution-and-lifetime--v1) and
[C07](../CONTRACTS.md#c07-render-scene-views-and-temporal-data--v2-with-p4v3v4).

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

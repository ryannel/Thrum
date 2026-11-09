# PLATFORM-RUNTIME-IOS M7 review

9 November 2026. Reviewed and built clean commit `749d2afa283ba57976e714ca67c2196090454eb5`.
All six items pass. No blocking findings inside or outside the items. One verified advisory documentation finding.
This accepts diagnostic-scene simulator extent correctness and sampled visual function only.

## Finding

**A1 — advisory, verified: stale design status.** `docs/design/modules/PLATFORM-RUNTIME-IOS.md:3`
still says M7 implementation and independent design review are owed; its closing paragraph also says design review
is owed. Compare the approved design review and the current plan's claim. Update those status sentences during the
next authorised documentation edit. The technical design, extent policy, limits and consumer handoff agree;
this stale status does not break an extent item or the quality bar. I left the design unchanged as instructed.

## Independent execution and identity

I read the definitions, technical design and implementation before using the builder's evidence conclusions.
I reviewed the entire milestone diff, including selected images, structured route records, manifests, logs and
status changes. Because this is the module's last active milestone, I also read the complete iOS shell, render
owner, witness, lease, gesture, world policy, Swift bridge, build/driver scripts and UI tests. I followed the changed
shared host, preparation, device publication, presenter, target/history and V1 camera/contribution seams on both sides.
No other agents participated. No implementation was edited.

Commands run from this checkout:

```sh
RUSTC_WRAPPER= ./dev check
RUSTC_WRAPPER= ./dev check-docs
python3 /Users/ryannel/.claude/plugins/cache/ryannel-skills/orchestrate/1.4.0/skills/orchestration/build/scripts/agent_loop.py lock gpu -- env THRUM_V1_ROUTE_OUT=/private/tmp/platform-ios-m7-route bash examples/v1-game-ios/ios/check-simulator.sh
```

All exited 0. The workspace check passed **651 tests, 0 failed, 0 ignored**, including both new native Metal extent
tests, Clippy, formatting and the web compile gate. GPU checking and the simulator route ran sequentially.
The initial sandbox attempts could not access sccache/CoreSimulator; the successful check disabled the wrapper,
and Metal/simulator execution used approved sandbox escalation. These initial access failures were not product failures.

The full named route built the release Rust library, linked the Swift app, built the XCUITest runner and ran every
row without partial/skip settings. Fresh owned device `5C021818-4E0F-44EC-84FE-8C59ABCEDDE8` was an
**iPhone 18 Pro, iOS 27.2** (`com.apple.CoreSimulator.SimDeviceType.iPhone-18-Pro`,
`com.apple.CoreSimulator.SimRuntime.iOS-27-2`), Xcode 27.2 / 27B52. PID **10255** survived the route.
The shell's cleanup exited successfully; a separate `xcrun simctl list devices -j` confirmed the UUID was absent.

Reviewer run identity:

- Commit: `749d2afa283ba57976e714ca67c2196090454eb5`; patch hash at execution was the SHA-256 of empty input.
- Source manifest digest: `a34741ea487e1499c29c4a0d6d8dd2e46b220f4bc51d4d2696b1c11c1804bcf2`.
- Built executable SHA-256: `4025612029defb412cb31480785c8fdfc6455c53bd76ac159a97f02c0bf16479`.
- Route log SHA-256: `a5ad8c7810ed4b4d36688612e0721f847124bf91694732f1e66d4a8f1c3f9769`.

The reviewer source digest equals the builder's retained manifest; I independently checked every source-file hash
and every retained-file hash, with no differences. The reviewer binary is identified separately from the builder's.
Raw reviewer states, native screenshots, recordings, crops and contact sheets remain in
`/private/tmp/platform-ios-m7-route`; build products are ignored under `target/`. They are disposable review scratch,
not additional repository changes. Selected durable builder evidence is in
[the extent evidence](../render/ios-extent/README.md); this report records my independent observations and identities.

## Inspection list and observations

Before looking at images I wrote this list: all four engine edges; horizon/background continuity; cube and square
grid perspective; startup portrait, landscape and returned portrait; rotation samples through settling; and a join
from each settled image to target, projection, viewport and drawable witnesses. OS overlays and rounded corners
must be distinguished from engine borders. Background, replacement and recovery are additional lifecycle edges.

I inspected my three native simctl captures as whole images, then unscaled top/bottom bands (192 pixels high),
left/right bands (192 by 768 pixels) and central object crops (768 square). Landscape's 2622-pixel horizontal bands
were also split into two 1311-pixel pieces to inspect them without viewer downscaling. The crop command was
`ffmpeg -i INPUT -vf crop=W:H:X:Y -frames:v 1 OUTPUT`; dimensions/coordinates are recorded in reviewer
`review-crops.json`. Whole images were fitted to the viewer; the smaller crops were inspected at native pixel size.

| Inspection entry | Observation |
| --- | --- |
| Startup portrait | Sky/background and geometry reach every engine edge. Grid and cube faces have consistent perspective. No fixed 16:9 band remains. |
| Settled landscape | The wider horizontal field reveals both central cubes and more surroundings. The horizon continues across the sides. All edges contain current scene/background; there is no stale-content border. |
| Returned portrait | The narrower horizontal field returns with the same vertical projection. The visible cube/grid retain perspective; no squeeze or crop-to-fill substitution appears. Darkening follows the existing day cycle. |
| OS content | Time, Settings return text, status icons, island/corner masks and home indicator belong to the compositor/UI. They are not evidence of engine letterboxing. |
| Both rotations | I inspected all 120 chronological samples in six 5-by-4 contact sheets at 360 pixels per sample height. UIKit briefly rotates the aspect-preserved image within the animated surface; settled samples have no persistent stretching, stale border or missing scene. This was frame-sequence inspection, not live video playback. |
| Internal sizing | Actual resources, source/destination, drawable and viewport match each settled surface; matrix-derived aspect and vertical FOV match. A filled screenshot alone was not used to accept resolution. |

Both reviewer strips contain **60 samples at 20 fps**, over three seconds: one second before the observed resize and
two after. Video seeks were 5.559253 s (landscape) and 5.351325 s (portrait), with resize observation respectively
6.559253 s and 6.351325 s after the record command. Polling is 0.1 s; image sampling is 50 ms. First current-extent
presentation was bounded by the engine reply witness at two and three replies. Simctl has no exact engine-frame or
scanout identity: the join uses settled same-epoch screenshot brackets and the resource witnesses, with
command/observation alignment for the strips. It proves sampled transition function and settled framing, not every
unsampled frame. M5's final-check race remains at most one admitted present per revocation; the separate forced
stall-present row exercised exactly one. Ordinary rotations had zero resize-raced and zero old-extent presents.

Reviewer screenshot SHA-256 values:

- Startup: `28cf869fe0864693523d0ab5c2d0b30730bcc9414aefd713743a21f0b4c117e8`.
- Landscape: `3bda0c3510e37d746e347066d25659dc7b984675befb7c7636703a3f8af01348`.
- Returned portrait: `8e76c27ef73a67c042c2bedbab97c003eacf46522f9f6077a5d6085d0c24ca38`.

## Item verdicts and boundary answers

### M7.1 — pass

The required check passed. The two new GPU tests inspect real attachment dimensions and the matrix/viewport of an
encoded token through portrait/landscape/return, compare resource handles and revision on identical requests,
exercise zero/oversized and stale-preparation refusal without partial publication, refuse pending frames and retry
after completion, reject an old-grid visual candidate, check history reset, refit a detached device candidate and
retain the fixed-render surface-only letterbox semantics.

`runtime/extent.rs:48` defers zero and busy requests; it prepares the replacement ring and presenter binding before
publication at lines 83–89, after the supplied current-epoch predicate. Failure leaves the live ring untouched.
The same-extent branch returns before allocation. `reset_replacement_history` invalidates extraction and requests
C07 history/upload resets. New target graphs and target-local bindings come from the existing preparation path.
The V1 threaded facade delegates directly to this host; it creates no replacement host, world or device.
The game camera source derives aspect from the new extent while retaining its pose/FOV policy.

The matrix witness is the `scene.view` matrix whose bytes `PreparedSynthetic::update_frame` writes to the selected
view uniform before encoding; the exclusive encoded token prevents a concurrent host mutation. Attachment witnesses
read wgpu textures. The viewport uses the same helper as `record_present`, and encode rejects a different destination
extent. Thus these are resource/encoded-policy observations rather than requested-extent echoes. No GPU readback is
claimed for the matrix. Contribution layouts/pipelines are size independent; contributions receive the new matrix
and history reset through `ContributionFrame`, so rotation does not rebuild cooked pipelines or select another set.

### M7.2 — pass

The full fresh-device route and cleanup passed. My settled records show:

| State | Submission | Epoch | Device generation | Actual attachments, presenter source/destination and drawable | Full viewport | Matrix aspect / vertical FOV |
| --- | --- | --- | --- | --- | --- | --- |
| Startup | 837 | 2 | 1 | 1206×2622 | 0,0,1206,2622 | 0.459954244 / 64.999997° |
| Landscape | 6942 | 17 | 1 | 2622×1206 | 0,0,2622,1206 | 2.174129348 / 64.999998° |
| Returned portrait | 8134 | 18 | 1 | 1206×2622 | 0,0,1206,2622 | 0.459954242 / 65.000004° |

All four attachment sizes agree in each row, and the recorded presented epoch equals the current shell epoch.
Both screenshot brackets retained their settled epoch/extent. Rotations preserved PID and install/replacement
counts, with two and three replies to resume presentation (bound six). Returned portrait dropped one stale frame,
then resumed correctly. The actor moved 0 m through either rotation. Assertions derive dimensions from the actual
resource/drawable values and projection, without model-size constants. Tools and gestures run through failing
subprocess calls/timeouts; absent frame witnesses fail, and skipped/partial route modes explicitly exit 2.

### M7.3 — pass

Every inspection-list entry above passes the milestone's shipped-player orientation/framing bar. Native scene targets
and the actual encoded projection agree with the images, excluding presenter-only scaling. The diagnostic fixture's
cubes/grid supply the square-geometry check; the numerical matrix establishes preserved 65° vertical FOV across
orientation. The sampled animation and settled images establish function within the stated capture/race limits.

### M7.4 — pass

The full route exercised both background rows, plain/stalled/raced replacement, stale-generation wake/request
refusal, ordinary loss, held-drawable loss, real rotation after candidate preparation, and deferred recovery across
background. The rotated recovery's submission 8767 at epoch 27 / generation 4 has all four 2622×1206 attachments,
matching source/destination/drawable and full viewport. Background recovery's submission 9397 at epoch 32 /
generation 6 returns to matching portrait targets and full viewport. Each recovery published exactly once;
background deferred once, publishing after resume. The controlled actor stayed unchanged across recovery and moved
9.725 m on the final real swipe. One held frame was abandoned with zero lost-device empty submissions.
UIKit's maximum during stalls/recovery was 10,537 μs, within the existing one-second structural bound.

`PreparedDevice::resize_scene_surface` prepares slots and bindings before changing the candidate. Device publication
uses the candidate extent and refreshes the current source camera/lighting. In iOS `render.rs:563–620`, zero extents
defer, the post-preparation hold precedes the latest-extent refit, candidate loss is checked after refit, and
publication rechecks background/current extent inside the lease section. Generation/owner checks reject stale
candidates. UIKit retains try-lock lease removal and bounded mailboxes, with GPU polling/creation on the render
owner. World, source camera/lighting and contribution selection remain with the original host. Native tests cover
invalid/zero/stale/busy and consumed-candidate paths; candidate-loss refusal is also traced in source rather than
claimed as a newly injected simulator row.

### M7.5 — pass

Actual per-slot allocation witnesses report six textures and **77,922,704 unpadded texel bytes** at either orientation.
The graph/resource account agrees: Depth32Float depth (4 B/pixel), Rgba16Float scene (8), Rg16Float motion (4),
Rgba8Unorm output (4), a 1024×1024×3 Depth32Float Sun array and 512×512 Rg32Float air-shadow target.
Two slots total 155,845,408 bytes. Ordinary transition overlap is at most two app-held rings (311,690,816 bytes);
recovery refit includes the lost host ring plus old/new candidate rings (467,536,224 bytes). These exclude buffers,
driver padding/residency, drawable pools, pipelines and size-independent assets, as the evidence states.

My 120-present steady interval retained revision 1 (submissions 717→837), with no target-ring transition. The zero
creation counts are an inference from that revision plus the allocation-site/source audit and native handle-reuse
test, not a global driver allocation measurement. Normal configuration short-circuits at the existing epoch/extent;
no steady-frame resize allocation, pipeline rebuild or resize readback is added. The bounded request slot coalesces
to the latest extent; stale preparation is discarded and old resources follow completion ownership. Pixel work is
3,162,132 versus 230,400 baseline pixels, **13.72453125×**. No simulator timing gate or physical-device performance
claim is inferred.

### M7.6 — pass

`RUSTC_WRAPPER= ./dev check-docs` passed. The evidence includes commands, source/binary/model identity, inspection,
resource accounting, actual extent records, strips/recipes, cleanup and retained hashes. Plan, G3b work list and
maturity separate implementation, builder correctness/visual observations and device qualification. The design's
technical extent contract agrees, subject to advisory A1's stale status wording. COOKED-DRAW M1 still depends on M7;
its M1.9–M1.11 definitions, including every M1.10 visual requirement, are unchanged. T2 simulator M2 still needs
COOKED-DRAW M1. Diagnostic M7 acceptance grants neither cooked visual nor submitted-frame capture acceptance.
The selected record explicitly retires raw builder videos/frame dumps/intermediate runs after retention verification;
its retained masters, recipes and hashes are present. Ignored builder scratch is not part of this clean checkout.

## Scope and disposition

I found no hard-coded success path, canned replacement for actual targets, weakened/deleted test, default-on test-only
rendering path or output file standing in for computation. Route-only diagnostics observe the production renderer;
the resize/recovery operations are not gated on route mode. This review makes no physical-iPhone, real-fault detection,
GPU-speed, compositor-cadence, photon or cooked-visual claim. All item verdicts are `pass`; M7 state becomes `done`
and `fails` remains 0. Only this report and the M7 verdict/state fields in its done-when file are changed.

PASS M7

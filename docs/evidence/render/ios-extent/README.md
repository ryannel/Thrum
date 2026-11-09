# iOS oriented scene extent · M7

9 November 2026. Builder proof for the reviewed G3b design. M7 is claimed for lead review.
The diagnostic scene qualifies simulator function. Cooked visual acceptance and physical-device budgets remain open.

The full route ran on a fresh owned iPhone 18 Pro / iOS 27.2 simulator under the GPU lock and deleted it afterwards.
[Identity and binary hash](records/identity.json), [source hashes](records/source-files.json),
[commands and results](records/route.log), [cleanup](records/cleanup.json) and
[retained-file hashes](records/retained-sha256.json) identify the run. Runtime sources match the source manifest.
The pre-change route also passed, with the fixed-aspect bars visible in [baseline portrait](images/baseline-portrait.png).

Ran the named routes with the pinned Rust toolchain and Xcode 27.2. The profile gate passed 651 tests under `gpu`; [verification](records/verification.json) records the commands and results.

```sh
python3 /Users/ryannel/.claude/plugins/cache/ryannel-skills/orchestrate/1.4.0/skills/orchestration/build/scripts/agent_loop.py lock gpu -- env RUSTC_WRAPPER= ./dev check
python3 /Users/ryannel/.claude/plugins/cache/ryannel-skills/orchestrate/1.4.0/skills/orchestration/build/scripts/agent_loop.py lock gpu -- env THRUM_V1_ROUTE_OUT=tmp/ios-extent/final-route bash examples/v1-game-ios/ios/check-simulator.sh
RUSTC_WRAPPER= ./dev check-docs
```

| Item | Builder finding and evidence |
| --- | --- |
| M7.1 | Coupled target-ring/presenter publication, final epoch predicate, retryable zero/busy/invalid/stale paths and C07 reset. Two native Metal GPU regressions inspect actual textures, encoded matrix and viewport through portrait/landscape/return, retain resource handles on identical requests, reject old visual candidates and cover device refit plus fixed-render surface-only behavior. The existing threaded facade owns the operation. |
| M7.2 | [Startup](records/01-startup-extent.json), [landscape](records/12-landscape-extent.json) and [return](records/13-portrait-extent.json) identify submissions 836, 6886 and 8094. Actual four attachments, presenter source/destination and drawable agree at 1206×2622, 2622×1206 and 1206×2622. Viewports cover each whole surface. Matrix-derived aspect is 0.459954, 2.174129 and 0.459954, with 65° vertical FOV throughout. PID/window/lease/device survive rotation, epochs 16→17→18, two replies to present, zero old-extent or resize-raced presents. |
| M7.3 | Inspected all three native whole images, four edge crops and object crops using the [native crop recipe](records/crops.json). Sky/ground reaches every engine edge, horizon stays coherent and grid/cube faces preserve perspective. Portrait's narrower horizontal field follows the matrix. Night darkening follows the existing day cycle. Status/Settings overlays and rounded corners belong to iOS. See [startup](images/01-startup.png), [landscape](images/12-landscape.png) and [return](images/13-portrait.png). |
| M7.4 | Full route proves replacement, both background rows, held-drawable loss, stale-generation refusal, post-preparation rotation and background-deferred publication. [Rotated recovery](records/16-loss-rotation-extent.json) has actual landscape targets and full viewport. Controlled actor moves 0 m across recovery and 9.638 m on the final swipe. UIKit's stall maximum is 15,331 μs against the existing one-second structural bound. |
| M7.5 | [Resource account](records/resources.json) records six textures / 77,922,704 bytes per slot in both orientations, 155,845,408 bytes per two-slot ring, 311,690,816 bytes ordinary transition overlap and 467,536,224 bytes during recovery refit. These are app-held texel bounds excluding driver padding, buffers and other device resources. Native pixel work is 13.7245× the 640×360 baseline. [120 steady presents](records/steady-extent.json) retain revision 1 with zero target-ring replacements. Normal frames reuse bindings and add no resize allocation/readback. |
| M7.6 | Plan, G3b work list and iOS maturity distinguish this builder proof from acceptance. COOKED-DRAW reruns unchanged M1.9–M1.11 after M7 acceptance, retaining all M1.10 requirements. T2 M2 still needs COOKED-DRAW M1. Owned scratch closeout is recorded below. |

Both rotation strips contain 60 samples at 20 fps, covering one second before the observed resize and two after.
Reviewed every cell in [landscape 1](images/12-landscape-motion-1.png), [2](images/12-landscape-motion-2.png),
[3](images/12-landscape-motion-3.png), and [portrait 1](images/13-portrait-motion-1.png),
[2](images/13-portrait-motion-2.png), [3](images/13-portrait-motion-3.png).
UIKit briefly rotates an aspect-preserved image within its animated layer. Settled output fills the oriented surface
without residual bars or stretching. The first new-extent present takes two replies. M5 still permits one already
admitted final-check race per revocation, exercised separately by the stalled-present row. Strip alignment uses
command/observation time, with 0.1-second polling and 50 ms samples. Simctl exposes no engine frame identity or
scanout timestamps. Screenshot capture records bracket same-extent presented submissions. These observations
establish sampled function and settled framing, with no simulator cadence claim.

Selected screenshots, strips, recipes and records are committed here. Owned raw videos, frame dumps and intermediate
runs were retired after retained hashes and source identity verification. Build outputs are cleaned at builder stop. The loop retains
this worktree for lead review. Its owned scratch record is `tmp/ios-extent/NOTES.md`.

# PLATFORM-RUNTIME-IOS design review

9 November 2026. Reviewed integrated revision `fc906ed9d783a06f093f358522fdff4a27bcb8eb`.
The selected child design, M7 plan and all six active done-when items are judgeable and consistent with the
commission. No blocking or advisory findings.

## Requirements and scope

The requirements come from the build plan's V1 exit and G3b row, the architecture brief's mobile requirements,
the platform family's lifecycle brief, the V1 iOS surface decision and C02/C06/C07/C11. This correction must make
the current oriented full-bounds pixel surface determine the scene targets, projection and presenter. Startup,
portrait/landscape rotation and recovery must produce an undistorted image that fills the settled engine surface,
preserving vertical field of view and the existing world, camera and lifecycle ownership.

UIKit owns the extent and surface epoch. The render owner owns resource preparation, publication, acquisition and
submission. Size-dependent histories must reset, GPU resources must retire safely, and UIKit must stay responsive
through pending frames and recovery. Explicit fixed-render-extent callers retain their surface-only operation.
G4 owns later reconstruction/governor policy. G5 owns cooked content and its visual acceptance. Simulator proof
establishes function, with physical-device performance remaining a separate qualification.

I read the child design before the family brief, then assessed its scope against the governing requirements above.
This is a review of a scoped correction to an existing runtime. I consulted the existing host, preparation,
device-recovery, contribution, V1 camera/facade and iOS render-owner paths to assess the proposed seams. I also read
the existing simulator entry script and the COOKED-DRAW/T2 dependency definitions. No other agents participated.
Historical M1–M6 closeouts supplied lifecycle context. Their previous verdicts were not re-evaluated.

## Design assessment

The mechanism addresses the reported failure at its owner. The current `FrameHost::resize_surface` and
`PreparedDevice::resize_surface` change the presenter while retaining the scene extent. The iOS preparation path
supplies `RENDER_EXTENT`, and V1's scene source derives projection aspect from the render extent it receives.
Consequently a presenter correction alone cannot establish native scene resolution or the required projection.
The proposed coupled operation covers scene attachments, graph declarations, projection, viewport, contribution
bindings and presenter source/destination as one transition.

The publication rules are sufficient to guide implementation. Requests coalesce to the newest epoch/extent,
submitted frames defer resizing through nonblocking completion polling, and preparation precedes publication.
Failure or staleness retains a coherent generation while preventing its use for new-extent presentation. Zero
extents defer, identical extents reuse resources, and C07 invalidation prevents reuse of another sample grid.
These rules permit implementation through the existing render owner and threaded facade without creating another
world or device for ordinary rotation. The exact API signature can be settled during implementation within these
invariants.

Recovery includes the consequential extra step. A candidate prepared before rotation must refit scene resources
and presenter together before publication at the latest extent, with loss, generation and background checks.
The existing `hold-recover` point is after preparation, so the planned real rotation can expose a candidate that
only resized its presenter. M7 also retains the bounded final-check race during UIKit animation and distinguishes
that transient from the first current-extent present. The retained M5/M6 protocol supplies the reply bounds,
world-survival checks and held-drawable recovery cases.

The cost model identifies the material increase from 230,400 to 3,162,132 pixels, about 13.7 times the baseline.
It requires attachment formats/counts, target bytes and peak old/new overlap, including reuse and creation-count
evidence. Allocation belongs to distinct extent transitions, with transient requests coalesced. These are
appropriate obligations for this correctness correction. The design makes no measured performance claim and
requires no invented simulator timing gate.

The dependency handoff is coherent. M7 uses the existing diagnostic scene, has no upstream milestone dependency,
and precedes COOKED-DRAW M1. The latter's M1.9–M1.11 still require cooked rotation, visual completeness and sampled
pan continuity on the integrated host. T2 M2 continues to depend on COOKED-DRAW M1. Diagnostic-scene acceptance
therefore cannot close either consumer's distinct proof obligations.

## Done-when audit

| Item | Judgment available to the later reviewer | Plausible substitute or failure caught |
| --- | --- | --- |
| M7.1 | Run `./dev check`, inspect the shared operation and threaded facade, and read GPU tests against actual resource and projection/viewport state. The question explicitly requires pending, same, zero/invalid, failed/stale and history cases plus the surface-only regression. | Updating request fields or only the presenter, publishing partial resources, losing retryability or changing fixed-render-extent consumers. |
| M7.2 | Run the full named simulator route on its fresh owned model/runtime. Join identified presents to actual scene targets, encoded projection, presenter source/destination, viewport, drawable, epoch and process/device identity in all three settled orientations. | Echoed requests, hard-coded phone dimensions, skipped gestures, process restart or presentation that never resumes. |
| M7.3 | Inspect native whole images, full-resolution edge/object crops and both 20 fps rotation strips, joined to presented-frame witnesses. | Stretching, crop-to-fill, stale borders and persistent distortion. Image coverage and the M5 race must be recorded, so sampled simctl output cannot establish every submitted frame or display cadence. |
| M7.4 | Read recovery/publication code and the full M7.2 route's background, replacement, held-drawable loss and post-preparation rotation records, plus host failure tests. | Publishing an old-size recovery candidate, refitting only its presenter, accepting a stale/lost generation or losing retained world/camera/lighting/content state. |
| M7.5 | Read allocation sites and resource records for both orientations, peak overlap and repeated identical extents. | Per-frame recreation, accumulating obsolete extents, omitted overlap or simulator wall time presented as a device performance result. |
| M7.6 | Run `./dev check-docs` and compare the evidence, maturity entry, work list and consumer handoff. | Documentation claiming cooked visual acceptance from diagnostic images, conflating maturity dimensions or leaving owned scratch without closeout. |

M7.4 and M7.5 use source/evidence questions rather than separate commands. Their required runtime observations are
part of M7.2's full route, and their questions identify the records and implementation paths the reviewer must
examine. That makes them independently answerable within the milestone. Missing records cannot support a pass.

The planned visual inspection covers all four engine edges, horizon and diagnostic geometry at startup, landscape
and returned portrait, including the transition through the first current-extent present. It also distinguishes
OS overlays and rounded corners from engine borders. Combined with actual target and matrix witnesses, this can
reject a convincing full-screen image rendered at the wrong resolution or aspect.

## Review limits

This verdict approves the design and acceptance criteria for M7. Runtime implementation, simulator function and
visual acceptance remain open. This review performed source and document inspection only. It captured no images
or motion and ran no GPU or simulator qualification. The implementation read was limited to the seams named above.

Documentation validation passed with `RUSTC_WRAPPER= ./dev check-docs`. `git diff --check` passed, and the final
worktree status contains only this new review file.

DESIGN OK

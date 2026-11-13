# SHOWCASE design review

**Result: DESIGN OK — no Must Fix findings; two Suggestions.**

Reviewed commit `555c80067c5b0778b579d1e79573e7b49f0067d8`, 13 November 2026. This is a design review, not a claim that the scene, its visuals or its performance have been proved. No previous `design-review.md` exists in this checkout. No agents were started, no product runs were made and no captures were taken.

## Findings

### S1 — Suggestion, verified: name the comparison transform for the EDR row

**Where:** [DESIGN.md](../../plans/SHOWCASE/DESIGN.md), lines 167–181; [DONE.json](../../plans/SHOWCASE/DONE.json), M7.5 and M8.1–M8.5; [APPLE-PRESENTATION design](../../plans/APPLE-PRESENTATION/DESIGN.md), “EDR” and “Evidence conventions.”

The exit deliberately compares an Apple row with EDR against portable output. The inherited route metric averages 8-bit sRGB, while APPLE-PRESENTATION's EDR output is linear floating-point colour with highlights above SDR white and display-dependent headroom. M7.5 asks whether the rows use an encoding all three produce, but a common encoding alone does not specify how their different display transforms are compared.

Name the comparison stage and conversion, including how headroom is handled, before calibrating the exit thresholds. Keep that comparison's claim distinct from the EDR appearance claim. In particular, clipping an EDR image into an SDR preview could conceal highlight differences or create the flat highlights M8 asks the reviewer to reject. T2 already requires colour/stage metadata and explicitly says an SDR preview cannot certify physical EDR brightness.

This is a clarification, not a missing EDR implementation: APPLE-PRESENTATION owns the transform and its capture/display qualification; SHOWCASE explicitly waits for its closing proof, and M7.5 already requires a shared comparison encoding. The three negative controls and the independent M8 visual review also remain required.

### S2 — Suggestion, verified: budget the long motion proofs explicitly

**Where:** [DONE.json](../../plans/SHOWCASE/DONE.json), M5.8–M5.9 and M8.2–M8.5; [DESIGN.md](../../plans/SHOWCASE/DESIGN.md), “Proof and builder handoff”; [T2 design](../../plans/T2-CAPTURE/DESIGN.md), “Cost and failure account.”

SHOWCASE requires twenty-second motion windows and a full day cycle on all three rows. The inherited valley day is about five minutes. T2's retained throughput qualification and 16 GiB allowance cover a four-second SDR session, and its design requires a separate allowance for longer or HDR sessions. At T2's stated 1080p/120-sample SDR rate, five minutes is roughly 300 GB of raw pixel traffic before compression; this is not an estimate of compressed disk usage.

Add a short handoff note requiring duration, extent, colour format, expected peak storage and retained excerpts to be declared for these longer sessions, using T2's existing preflight and completeness rules. Do not shorten or slow the required motion proof merely to fit the old allowance. This is operational planning, not a new acceptance gate: T2 already supports configurable session limits and requires later content to establish its throughput.

## What the human asked for

I read [ACCEPTED.json](../../plans/SHOWCASE/ACCEPTED.json)'s complete `briefs` list first. It contains one lead task: plan row 13, the V1 exit, the showcase valley on Mac and in Chrome, reviewed with its closing record, depending on the last milestones of modules 1–10 and 12. It is not presented as a direct human quotation.

The governing requirements come from [BRIEF.md](../../plans/BRIEF.md), especially decision 1 and G7. They require:

- One reusable engine running the same representative scene on Mac Apple tile, Mac portable and Chrome portable, with 10,000 entities and 1,000–2,000 movers.
- The complete exit feature set, including physical sky and time controls, cooked content, camera/lights/shadows, culling, MetalFX, EDR, the governor, interpolation, hot reload, profiler and demo UI.
- Correctness and independent visual review of the fixed route; whole-frame, startup and hitch records; a visual digest, platform status and device work owed.
- Preservation of the previously established multiplayer-fitness and creator-experience requirements in the closing account.
- Performance figures as baselines and tripwires, not numeric admission gates; no addition of sea, clouds, transport, physical-iPhone qualification or website publication to V1.

The design addresses these requirements. The desert scatter and balloons are content choices, explicitly recorded as assumptions in the plan. They are not substituted for an unrecorded request for characters or animals. The brief does not require those. Exactly 1,500 movers is within its stated range. Excluding entity animation, entity textures and LOD does not remove the scene's cooked textures or the final visual quality bar.

## Design and boundary assessment

- **Visible workload:** the new layout moves the representative entities into the actual valley. A generated layout, cooked mesh rows, per-slot fixed basis and one simulation rule provide a credible path to the proposed static props and translating balloons. Ground checks use the shipped terrain, not a second runtime height function. Counts alone cannot close M5: still and real-time inspections must show the content and motion.
- **One scene on three rows:** the publication, layout, route and controls are shared. Device-specific texture profiles, reconstruction and display range are disclosed. Resolved-state reports and fallback controls address the earlier route slice's weakness of merely echoing requested settings.
- **Content and visibility:** CONTENT-LOOP M2 supplies cooked rows and atomic replacement; CULLING supplies the bucket route, mode resolution, Sun-caster handling and the Direct oracle. The 21-mesh limit matches their 64-bucket interface. SHOWCASE does not invent a second loader or visibility owner. Cooked terrain is not an instance occluder under CULLING's design; SHOWCASE's non-empty prepass/hidden-slot witness must therefore come from the entity content, which its boulders and scatter can provide.
- **Simulation:** closed loops are implemented by a registered rule in committed simulation, not by rendering animation. Complete authority-state corrected replay, an hour of actual stepping and visible motion checks cover distinct promises. Preserving four registered peers keeps the existing ownership context in view.
- **Dependencies:** M5/M6 can use CONTENT-LOOP M2 and CULLING M3 without waiting for final Apple presentation. This earlier start is declared in the plan and does not permit an early exit. M7.1 refuses completion until every prerequisite is accepted and APPLE-PRESENTATION M11 exists and is accepted. ARCHITECTURE commissions the later planner to add M11 to the dependency list and deliver the missing resolved presentation values. Thus the currently unplanned presentation milestones are an explicit dependency, not silently omitted exit scope.
- **Evidence interfaces:** CREATOR's completed timing feed supplies the frame reports. PLATFORM-RUNTIME-MAC supplies the qualified presented-time witness. T2 supplies submitted images and elapsed-time motion. The design separates capture-off measurements from capture-on pictures and correctly limits Chrome's callback-clock claim. Physical iPhone and other hardware claims remain owed.
- **Performance policy:** the early mesh sweep makes content cost a design input. Its 8.33 ms selection rule does not itself gate delivery: the rule explicitly permits the lightest set when none meets the interval, with the gap recorded. Final visual review still applies, and visible stutter/regressions must be investigated. This preserves the human's baseline/tripwire policy rather than replacing it with a required numeric score.
- **Existing behaviour:** separate exit content and source assembly preserve the valley source root, grid/field configurations and registered exact baselines. The plan commissions the verifier at every milestone and forbids relaxing another owner's comparisons.

## Done-when review

I read all 38 items and their milestone bars. The table below assesses whether they can be judged at their stated stage and whether their combined proofs reject plausible substitutes. It does not assign implementation verdicts.

| Items | Assessment |
|---|---|
| M5.1 | Judgeable from retained paired sweep runs, build/load identity and the selection rule. Final prop meshes need not exist for the initial stand-in sweep. |
| M5.2 | Judgeable by regeneration and independent shipped-terrain checks, including the mover loops and camera path. A layout validating its own stored heights is explicitly rejected. |
| M5.3 | Judgeable through the game's rule execution and complete-state replay; the hour must be stepped, not sampled only at its endpoint. |
| M5.4 | Cooked rows and bucket counts have the required prerequisite. The independent CPU reference and fallback question prevent reported entity totals alone from proving the draw path. |
| M5.5 | The inherited comparison has an explicit calibration rule and three controls; removal of all entities must be detectable where they are in view. This is parity evidence, supplemented by visual inspection below. |
| M5.6 | Browser loading, tick-driven advancement and pause are observable through the existing browser harness. Its report alone would be weak; M5.5 and M5.9 independently require the corresponding pixels and motion. |
| M5.7 | Submitted stills and native-size near/middle/far crops can judge placement and visible repetition against the content bar. |
| M5.8 | Real-time held and moving views can judge continuous motion and shadows. See S2 for session planning. |
| M5.9 | Chrome's own fixed and real-time output can expose missing props and stalled movers; it is not certified by Mac results alone. |
| M5.10 | Existing exact-baseline verifier; the claim is explicitly preservation, not visual quality. |
| M6.1–M6.2 | Relative oracle runs on the new content are possible after M5. Mac requires every submitted flythrough frame; Chrome deliberately requires checkpoints. This is narrower browser coverage, supplemented by its real-time visual item and CULLING's own prerequisite proofs. |
| M6.3 | Paired cost rows and non-empty occlusion witnesses make the default choice judgeable and reject a purported Occlusion row that merely repeats Frustum. |
| M6.4–M6.5 | Real-time edge and silhouette inspections exercise the assembled content rather than relying on equality over an empty scene. |
| M6.6 | Existing exact-baseline verifier. |
| M7.1 | The prerequisite check deliberately fails while presentation M11 is absent. It does not pretend those features are already available. |
| M7.2–M7.4 | Each row must actually finish the stops/flythrough and report resolved state. M7.2 also questions the reporting source and omitted-selection/fallback controls. These are feasible with the commissioned provider reporting seams. |
| M7.5 | Cross-row comparison is judgeable once its common comparison representation is named; see S1. Controls and per-row baselines remain required. |
| M7.6 | Control declarations and preset application can be compared directly on all rows. |
| M7.7–M7.8 | Real edits and controls run in the exit scene. CONTENT-LOOP's row/texture replacements promise no pipeline creation, so the native counter assertion is compatible with hot reload. Chrome's profiler may honestly report an unavailable reason. |
| M7.9–M7.10 | Existing frame feeds support per-frame records and labelled startup proxies. Cause attribution needs investigation, not inference from a percentile; the reviewer questions explicitly require that investigation. |
| M7.11 | Existing exact-baseline verifier. |
| M8.1–M8.5 | Distinct still and temporal visual judgements cover all three rows at the final configuration, including light/haze consistency, reconstruction artifacts and time transitions. Prior module passes do not replace them. See S1/S2 for colour interpretation and long captures. |
| M8.6 | Attended, capture-off presented-time evidence uses the Mac witness already qualified by its prerequisite. It does not claim movie timestamps are display swaps. |
| M8.7 | Visible-window Chrome evidence is explicitly callback time, not physical presentation time. Its limitation is disclosed rather than passed off as a stronger measurement. |
| M8.8 | Clause-by-clause evidence at the claimed commit makes the closing record auditable, including exit clauses outside SHOWCASE's new code. Historical evidence is not automatically current evidence. |
| M8.9 | Digest provenance is checkable against the captures independently reviewed in this milestone. |
| M8.10 | Known defects and outstanding physical-device qualifications remain visible in the record, ledger and README. |
| M8.11 | Existing exact-baseline verifier. |

## Scope of this review

Read the request record, project brief, module-list entry and relevant interface/dependency/baseline sections, the complete SHOWCASE design/plan/done-when file, the governing G7 route decision, project review guidance and graded examples, relevant C02/C04/C05/C07/C12 contracts, CONTENT-LOOP and CULLING designs, and the relevant capture, presentation, frame-runtime and profiler design sections. No glossary exists at `docs/plans/GLOSSARY.md`.

This review checks the proposed assembly and its proof obligations. It does not certify the external research citations or re-review all prerequisite implementations. The new automated commands are builder deliverables, so they were not run as though SHOWCASE already existed. `./dev check-docs` passed (751 Markdown files, 4,646 local links, 30 module records, crate policy and current design hashes); `git diff --check` passed. Only this review file is changed.

DESIGN OK

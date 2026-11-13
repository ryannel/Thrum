# GAME-UI design review

13 November 2026. Reviewed commit `faa2d8433be9e7d9a6b83b7acc31b201308e0ff1`.

**Design changes requested: two Must Fix findings.** The design supplies the intended health bar, timer,
choices and join screen, with reusable engine ownership and explicit scope assumptions. Two mechanisms do
not compose with the interfaces they depend on: EDR output and the publication of a changed font's layout.
Building the design as written would require resolving these during implementation.

## Must Fix findings

### MF1 — The interface stage does not support the already-planned EDR output path

**Verified by comparing the producer and consumer designs; not a GPU reproduction.**

[GAME-UI's stage and colour mechanisms](../../plans/GAME-UI/DESIGN.md#1-the-interface-stage), lines 158–184,
require an interface stage after the display transform but before a present copy, drawing through an sRGB
view of an image containing encoded sRGB. However,
[APPLE-PRESENTATION's EDR design](../../plans/APPLE-PRESENTATION/DESIGN.md), lines 332–350 and 434–443,
already specifies a linear `Rgba16Float` drawable, with the display transform performed by the present pass
itself. That normal EDR frame has neither the assumed encoded image nor the assumed later copy.

This is a required configuration: the project brief includes EDR, GAME-UI follows the V1 exit, and M1.5
requires its interface in every offered display mode. GAME-UI lines 506–508 and PLAN's M1 instructions
treat this as a possible future surprise and tell the builder to stop if it occurs. The upstream design
already establishes it. Applying the SDR mechanism before the EDR present transform would put interface
pixels through the transform; applying it after the transform requires a different target/encoding path
that this design has not specified. M1.1's unconditional authored 8-bit-code language also needs its
relationship to M1.5's floating-point output made explicit.

**Resolution:** settle the EDR stage with the output owner now: its target, placement relative to the final
transform, linear blend and SDR-white convention, capture point, and preservation of uncovered scene pixels.
Make the existing M1 proofs judge the appropriate encoding in each mode. The review does not prescribe a
new intermediate or an extra pass; that choice belongs to the planner and output owner. Preserve the
existing SDR path and no-interface baselines.

### MF2 — A font publication can draw new glyphs with the previous layout

**Verified in the specified update order and the publication contract; not a running-engine reproduction.**

[GAME-UI's font mechanism](../../plans/GAME-UI/DESIGN.md#4-fonts), lines 242–245, explicitly switches the
sheets and table first, then relays out text when the interface sees the table; it permits old spacing until
the next frame. The layout and paint list belong to the shell, while rendering uses a frame request
(lines 421–428). There is no readiness/commit agreement that includes the newly derived runs and boxes.

Consider a run whose two glyphs have 8-pixel advances and fit a 16-pixel box. Replace the font with one at
the same cooked size whose glyphs and advances are 12 pixels wide. The allowed switch frame draws the new
glyphs at offsets 0 and 8: they overlap, and the second extends to pixel 20 before the layout can wrap,
elide or resize the box. Both glyphs can remain whole and the following frame can equal a fresh start, so
[M2.8](../../plans/GAME-UI/DONE.json), lines 244–247, can pass this implementation. Its question checks the
settled frame, while its statement permits either generation for each glyph during the edit window.

This conflicts with text staying within its box (DESIGN line 260), the no-overlap/readability promise, and
[CONTENT-LOOP's one-publication rule](../../plans/CONTENT-LOOP/DESIGN.md#one-publication-one-switch-m1-m2),
lines 222–247: prepare the affected presentation closure and select one generation before extraction.
[U1's lifecycle rule](../../design/modules/AUDIO-UI.md#focus-input-and-accessibility) also explicitly says
not to publish half-relayout. Stable sheet-frame handles keep atlas rectangles correct, but do not update
the positions and boxes derived from font metrics.

**Resolution:** include dependent runs, layout and paint state in the font replacement's readiness and
publication agreement, with a defined outcome when a recorded switch frame is discarded. Specify how the
shell's prepared state joins the render owner's switch. M2 can prove generation-consistent runs without
waiting for M3's tree; M3 can prove the corresponding boxes. Acceptance must reject a mixed-metric frame,
not merely require eventual equality with a fresh start.

## Suggestions

### S1 — Define which nodes must not overlap

**Verified wording ambiguity.** M3.5 says that no two nodes overlap. The specified tree intentionally puts
text over panels, a fill over a bar's track, and children inside parents. Limit this condition to the
independently placed status elements whose overlap would obscure information, while retaining the
containment and safe-area checks. This is a clarification of the evident intent, not another Must Fix.

### S2 — Shorten proof procedures without losing their discriminating cases

Several inspection fields prescribe capture dimensions, durations, crops and repetitions in detail. Keep
the required range, observable result and meaningful failure controls, but leave equivalent execution
routes to the builder and reviewer. For example, M4.9 needs observable event-to-frame response and clean
modal transitions; repeating the script exactly three times is not itself that proof. This does not hold
up building.

## Request coverage and milestone judgement

I read the complete task in RECORD's `briefs` first: “Plan module GAME-UI. It needs: RENDER-2D”. It adds no
human requirements beyond the approved project brief and the recorded swarm-survival request. Those ask
for an original swarm survivor, reusable 2D support and multiplayer. The module list assigns GAME-UI the
in-frame status display, level-up choice and join screen on Mac, Chrome and the iOS simulator.

The design addresses that assignment. Game rules, screens and session meanings remain with the game;
rendering, assets and platform events retain their existing owners. The interface scene is a suitable
incremental proof before SWARM exists. Cooked Latin text, integer scales, link/paste tickets, no gamepad,
and deferred text editing/accessibility/localisation are explicitly listed assumptions, not hidden
reinterpretations of the human's request. I found no unsupported expansion that should stop this module.
The four seam rulings assign implementation ownership, and waiting for RENDER-2D M5 avoids overlapping
changes to its unfinished path.

I read all 60 items, their checks/questions/inspections, and each milestone's quality bar:

| Milestone | Can its evidence be judged at that point? |
|---|---|
| M1.1–M1.12 | The raw paint-list proof needs no text or tree. CPU comparisons, absent-feature controls, real target output and lifecycle checks are meaningful. The EDR composition must first be resolved by MF1. |
| M2.1–M2.14 | Cooking, raw runs, scale changes, world rows and font inspection can precede the tree. Source metrics and cooked-texel comparisons distinguish resampling and fabricated text. M2.8 permits the mixed-layout substitute described in MF2. |
| M3.1–M3.13 | The tree, dirty-state controls, independent layout cases, status sweep and crowd captures are available after M2. S1 clarifies intentional layering; MF2 must carry through to font-dependent boxes. |
| M4.1–M4.13 | Routing tests and the game-owned moving player can expose input leakage, interrupted contacts and false activations. Scripted connection states are legitimate here: SESSION integration belongs to SWARM. Mac system-delivered input and withheld clipboard access are explicitly distinguished from what is proved. |
| M5.1–M5.8 | The simulator can prove touch, rotation, safe areas and lifecycle after M4. Dependency checks and runnable guide examples can judge reuse. The ledger correctly separates simulator function from physical-device, EDR-panel and human interaction claims. |

Costs are recorded separately from correctness, and no timing threshold is turned into an acceptance gate.
The proof plan includes submitted engine captures and real-time sequences; none were generated or judged
in this design review. There was no previous GAME-UI design review to reconcile. I inspected the complete
GAME-UI design, plan and done-when file, the planning probe, request records, and relevant contracts and
producer/consumer design sections. I did not audit runtime implementation or verify the proposed font
libraries. This verdict concerns design feasibility and proof adequacy, not implemented or visual quality.

Only this review file is changed; DONE.json and the design remain untouched.
Validation: `./dev check-docs` passed (785 Markdown files, 5,153 local links, 30 module records,
crate policy and current design hashes). `git diff --check` passed.

DESIGN CHANGES: 2 must_fix findings

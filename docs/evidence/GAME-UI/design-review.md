# GAME-UI design review

13 November 2026. Reviewed commit `97b83136ffb65764a32329a0c5e50cf38b72d3bb`, against the previous
review of `faa2d8433be9e7d9a6b83b7acc31b201308e0ff1`.

**Design approved, with one Suggestion and no Must Fix findings.** The design meets GAME-UI's part of the
human's request: reusable in-frame status, choices and joining UI for the swarm-survival game on Mac,
Chrome and the iOS simulator. Both previous blocking findings are resolved. This is approval of the design
and its proof plan, not evidence of implemented behaviour, visual quality or device performance.

## Previous Must Fix findings

### MF1 — The interface stage does not support the already-planned EDR output path: resolved

**Verified by reading the revised producer/consumer agreement and acceptance items; no GPU run.**

[Mechanisms 1 and 2](../../plans/GAME-UI/DESIGN.md#1-the-interface-stage), lines 178–243, now distinguish
the encoded 8-bit display image from APPLE-PRESENTATION's linear `Rgba16Float` presentation target. The
float stage draws after the present transform, at output extent, with white at 1.0 and linear blending;
capture reads that completed target. The transform's uncovered half-floats, including highlights above
1.0, must remain bit-identical. A shared render pass is preferred, with an explicitly costed load/store
fallback. Neither path introduces another submit owner.

This agrees with [APPLE-PRESENTATION's EDR mechanism and normal frame](../../plans/APPLE-PRESENTATION/DESIGN.md#techniques-and-evidence),
[T2's final-output tap](../../plans/T2-CAPTURE/DESIGN.md#mechanism-and-ownership), and updated
[seam ruling 1](../../plans/ARCHITECTURE.md#seam-rulings-for-game-ui). The existing dependency through
RENDER-2D and the V1 exit puts the EDR work before GAME-UI M1.

M1.1 now judges each encoding correctly. M1.2 checks untouched scene pixels in both modes; M1.4 uses a
moving, reconstructed, exposure-changing valley to expose an interface inserted before the transform;
M1.5 checks SDR white and transparent blending over highlights. M1.6 covers display-mode transitions and
M1.10 records their cost. The former instruction to discover the already-known EDR incompatibility during
building has been replaced by a design for it.

I independently repeated probe 5's half-float arithmetic using Python `struct`: all 256 opaque sRGB codes
round back to themselves; the maximum code error is 0.0372281, and the maximum adjacent half-float step
through 16 is 0.260283 encoded codes. This supports storage feasibility only. GPU blend precision remains
for M1 to prove.

The newly explicit limitation on text over bright EDR highlights is not hidden: PLAN lists the assumption,
the design identifies when the technique must be revisited, and M5.7 requires the ledger to record the
unqualified visual case. SDR crowd readability still has visual acceptance items. Approval does not certify
the interface's appearance on an EDR panel.

### MF2 — A font publication can draw new glyphs with the previous layout: resolved

**Verified by walking the specified publication states against CONTENT-LOOP; no running-engine reproduction.**

[Mechanism 11](../../plans/GAME-UI/DESIGN.md#11-a-list-and-its-sheets-change-together), lines 449–488,
prepares runs, wraps, boxes and a paint list beside the live layout. The render owner must hold the prepared
list before the interface reader is ready. Selection uses the sheet generations lent to that frame;
residency retains publication authority. Refusal keeps the old presentation, and a discarded switch frame
commits neither layout. Current game values update both layouts during preparation. This is consistent with
[CONTENT-LOOP's one-publication rules](../../plans/CONTENT-LOOP/DESIGN.md#one-publication-one-switch-m1-m2)
and updated seam ruling 3.

The former 8-to-12-pixel advance example can no longer pass by settling one frame later: M2.8 compares
every submitted frame, with changing text values, against complete old and new references and exercises
discard and refusal. M2.6 applies that requirement to scale changes, M2.9 carries it into Chrome, and new
M3.14 adds tree boxes and art-frame size changes. M2 can prove runs without waiting for the M3 tree.
M4.2 also defines which acknowledged layout the shell hit-tests while publication is pending; mechanism 11
states the acknowledgment delay rather than claiming physical-display synchronization.

## Suggestion

### S3 — Make logical font revision identity explicit when metrics change without new glyph pixels

**Verified wording ambiguity; a faulty implementation is only a possibility, not an observed defect.**

Mechanism 11, lines 451–458, calls the stamp a set of sheet generations, while
[mechanism 4](../../plans/GAME-UI/DESIGN.md#4-fonts) gives advances and kerning to the font table.
CONTENT-LOOP's lease identity rules allow unchanged products to retain their texture and view. Clarify that
the logical generation used for layout selection covers the table as well as the glyph sheet, and cannot
be reduced to a GPU-view change serial. A kerning-only edit with unchanged glyph pixels would be a useful
additional case for M2.8.

This is a clarification and sharper case, not a new hold: the design already requires the font table and
its derived layout to participate in one publication, and a logical generation can carry that identity
without changing the selected mechanism.

## Previous Suggestions and changed-item audit

S1 is resolved: the design defines an **element**, and the revised overlap items distinguish separately
placed status elements from intentional text/panel, fill/track and parent/child layering. S2 was taken in
part: arbitrary durations and repeat counts became meaningful event or motion coverage; dimensions,
scales and states remain as the claimed range. Those retained ranges are appropriate and do not hold up
building.

I read all 61 current items and all five quality bars, and compared every changed item with the previous
version. The changed set matches the task; M3's quality bar changed, its dependencies did not.

| Changed item or field | Judgement against the previous findings |
|---|---|
| M1.1 | Encoding-specific opaque and blended comparisons resolve MF1's 8-bit/float ambiguity. |
| M1.2 | Uncovered output remains exact in both modes, including EDR highlights. |
| M1.4 | The 3D reconstruction/exposure case rejects drawing the interface too early. |
| M1.5 | Explicit float white, headroom and highlight blending replace the unspecified EDR path. |
| M1.6 | Mode changes join the existing lifecycle proof without first-use pipeline creation. |
| M1.9 | Forced 8-bit visual inspection is explicit; camera coverage replaces a fixed duration. |
| M1.10 | Both modes and the shared-pass/fallback choice are measured without a numeric performance gate. |
| M2.6 | Whole-frame old/new scale references reject mixed-scale offsets. |
| M2.8 | Every submitted edit frame, discard and refusal reject MF2's eventual-correctness substitute. |
| M2.9 | Chrome must also remain whole through the font edit. |
| M2.13 | Gaining a digit and crossing pixels preserve the discriminating motion coverage. |
| M3 quality bar; M3.5; M3.8 | Element-level overlap resolves S1 while retaining readability, containment and safe-area requirements. |
| M3.9 | Element-level wording and a complete bar cycle preserve value stability and resize inspection. |
| M3.10 | Chrome still inspects real-time value changes through a full bar cycle. |
| M3.14 | The new proof extends MF2's solution to boxes, art dimensions and scales after the tree exists. |
| M4.2 | Pending-publication hit testing has a specified shell-side layout. |
| M4.9 | All modal response events remain inspected; removing three mandatory repeats does not weaken the outcome. |
| M4.10 | Element-level overlap permits intended layering while preserving ticket/reason readability. |
| M5.7 | The additional EDR readability limitation is recorded separately from correctness. |

The remaining milestone order is still judgeable: M1 uses raw rows; M2 adds cooked runs; M3 adds the tree;
M4 adds routing and game-owned example screens; M5 proves simulator integration and reuse. Scripted join
states do not impersonate a working network session, which belongs to SESSION/SWARM. Exact CPU references,
unchanged-host controls, changing-value publication comparisons, routing properties and direct visual
inspection remain complementary proofs. No new item requires a later GAME-UI milestone to exist.

## Request coverage and review limits

I read both complete tasks in RECORD's `briefs`, oldest first, and the project brief before judging the
revision. The original task is “Plan module GAME-UI. It needs: RENDER-2D”; the second asks to resolve the two
review findings. Neither overrides the human's swarm-survival, reusable 2D and multiplayer requirements.
The module list assigns GAME-UI health bars, timers, menus and a join screen. The design delivers that
engine capability while leaving gameplay, screen meanings and networking with their owners.

The previously recorded scope assumptions remain explicit: cooked Latin text and whole scales, tickets
from links/launch/paste, keys/pointer/touch, and deferred text editing, localisation, accessibility bridges
and gamepad support. No new unsupported expansion or concealed narrowing warrants a Must Fix.

Read: the complete GAME-UI design, plan, done-when file and design diff; both briefs; the previous review;
the planning probe and planner response; the relevant architecture rulings, C05/C06/C07/C11/C12, U1,
RENDER-2D sheets/stage interface, CONTENT-LOOP leases/publication, APPLE-PRESENTATION EDR, and T2 capture
sections. The project and skill calibration examples were consulted. No runtime code audit, GPU run,
visual capture, font-library audit or physical-panel judgement was performed.

Only this review file is changed. DONE.json and all planner files are untouched.

Validation: `./dev check-docs` passed (791 Markdown files, 5,223 local links, 30 module records,
crate policy and current design hashes). `git diff --check` passed.

DESIGN OK

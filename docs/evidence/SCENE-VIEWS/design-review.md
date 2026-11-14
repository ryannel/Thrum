# SCENE-VIEWS design review

Reviewed 14 November 2026 at `96a17fc44abe67030dcf31bd16badcdf1419ca44`.
Result: **Design changes requested — 2 Must Fix findings.**

The design addresses the requested off-screen scenery reflections with a credible second raster view,
but its altitude-only air folding does not supply the transport its water consumer requires. One acceptance
item also requires the displayed image to remain unchanged when its reflection changes. These need resolving
before building. The remaining findings are suggestions.

## Must Fix findings

### 1. `folded-air-transport` — folding altitude alone does not produce the reflected path's light

**Verified by reading the producer and consumer and by a numerical counterexample.** Affects M1.5 and the
transport assumed by M2.3 and WATERS M14.

[DESIGN.md](../../plans/SCENE-VIEWS/DESIGN.md), lines 163–170, specifies replacing the altitude of samples
below the mirror plane with their reflected altitude, then claims this supplies the air of the unfolded
reflection path. The seam ruling repeats that operation. The planner's part B proves a density correction
only; it integrates neither scattering nor sunlight visibility.

The existing producer computes one phase function from the straight view direction and the Sun direction
in [aerial.wgsl](../../../crates/thrum-render/src/atmosphere/aerial.wgsl), lines 25–29. On the physical path,
the direction changes at the water. Folding position without transforming the direction on the below-plane
segment therefore gives the wrong in-scattered light even when density and transmission are exactly right.
Phase functions depend on the angle between light and viewing directions; see the primary explanation in
[PBRT, Phase Functions](https://pbr-book.org/4ed/Volume_Scattering/Phase_Functions). The reproduction uses
Thrum's direction convention and its own `g = 0.8` equation in
[common.wgsl](../../../crates/thrum-render/src/atmosphere/common.wgsl), lines 41–46.

For an ordinary flat-water case, put the camera at `(0,40)`, its image at `(0,-40)`, the water intersection
at `(40,0)` and a receiver at `(41,1)`, in metres in a vertical section. Let the Sun be 45 degrees up in the
receiver's direction. The straight mirrored ray has Sun cosine 1 throughout; the physical camera-to-water
segment has cosine 0. The corresponding Mie phase factors are **3.580986 and 0.013640**, a factor of 262.53.
Both models in the reproduction use the same folded exponential density and have transmission 0.999714792.
Their in-scatter differs nonetheless.

This is an interface defect, not just an unproved fidelity improvement. [WATERS DESIGN.md](../../plans/WATERS/DESIGN.md),
lines 1176–1180, subtracts the actual camera-to-water in-scatter from this image and divides out that
segment's transmission. With the altitude-only image, it subtracts a different quantity from the one the
producer added. In the illustrative single-band reproduction, the recovered water-to-receiver in-scatter
is 40.21 times its true value. That number demonstrates the algebraic failure; it is **not** a prediction
of an engine pixel's brightness or a measured visual error.

The same missing path mapping affects terrain shadowing. The shared forward shading calls
[synthetic_aerial_shadowed.wgsl](../../../crates/thrum-render/src/synthetic_aerial_shadowed.wgsl), lines 48–50,
with an interval midpoint on the straight mirrored ray. Its height-based visibility test then samples a
point below water instead of the corresponding air point above it. Changing only `atmosphere/aerial.wgsl`
does not correct that caller.

**Resolution:** define transport on the physical reflected path, including the sample position, direction,
Sun visibility and atmosphere boundary handling on each segment, and reconcile the resulting colour with
WATERS' subtraction. M1.5's independent reference must establish both transmission and in-scatter for that
path, including a non-horizontal Sun, rather than copying the altitude-only operation. Retain the unchanged
no-fold path. The planner should choose the mechanism; this review has not supplied an engine implementation.

Evidence: [reproduction](design-review/fold_transport.py) and [output](design-review/fold-transport.txt).
Run `python3 docs/evidence/SCENE-VIEWS/design-review/fold_transport.py` from the repository root.

### 2. `knob-output-invariant` — M3.3 requires both a changed reflection and an unchanged presented image

**Verified contradiction in the written item and consumer.**

[DONE.json](../../plans/SCENE-VIEWS/DONE.json), line 242, requires changing each view knob to both ends of
its range to change the view as declared while leaving “the main view's submitted bytes as they were.”
The planned mirror scene reads that view into its pool in `after-opaque`
([DESIGN.md](../../plans/SCENE-VIEWS/DESIGN.md), lines 251–260).

For example, shorten the secondary view's far distance until the reflected far hill disappears. The main
camera's geometry and lighting should remain unchanged, but the pool pixels in the submitted image must
change. Requiring those bytes to remain identical rejects the intended working consumer. Disconnecting
the pool from the changed view would satisfy that part of the item while breaking the example.

**Resolution:** identify the protected output precisely, such as the main opaque result before its
consumer runs, or a separate scene without a reader. Separately require the mirror consumer to show the
declared knob effect. This is a correction to an impossible acceptance condition, not a request for more
test procedure.

## Suggestions

### 3. `runtime-extent-lifecycle` — explain how the live extent control relates to preparation

**Verified ambiguity; Suggestion.** The slot extent is fixed at preparation (DESIGN lines 119–123), changing
it requires a new preparation (line 265), but it is also a runtime knob (lines 216–219 and M3.3). C04 permits
transactional live replacement, so these can coexist. State whether the knob changes an active viewport
inside prepared capacity or publishes replacement targets, and how prepared pipelines are retained to meet
M3.3. That will make the consumer and governor contract concrete without requiring a particular technique.

### 4. `equality-envelope` — distinguish expected profile differences from numerical error

**Verified ambiguity; Suggestion.** M1.1, M1.6, M2.1 and M2.3 measure a bound first and then assert it.
The ordinary-main-view reference and the separate shadow, air and visual checks are valuable safeguards;
these items are not dismissed merely because a tolerance is not yet numeric. However, explicitly compare
matching lighting/shadow profiles for the equality cases, and justify a nonzero bound from its cause rather
than making the observed maximum the acceptance rule. The default secondary view has one cascade and no
fog, so unconditional equality to every main-view configuration is not its promised behaviour.

### 5. `two-slot-content` — add a content check for simultaneous distinct requests

**Verified proof coverage gap; Suggestion.** The planned cost record asks for two views, but the content
proofs and mirror example principally exercise one. WATERS intends two water levels in one frame. A compact
comparison of two different cameras or clip planes against their separate references would expose shared
uniforms or visibility buffers overwritten by the last request. This sharpens the existing slot contract;
it does not justify withholding approval by itself.

## Fit to the human's request

I read the sole task in [RECORD.json](../../plans/SCENE-VIEWS/RECORD.json) under `briefs` and the approved
[project brief](../../plans/BRIEF.md) before judging the design. The task asks for high-end scenery
reflections, underwater support with caustics, shafts and adapting exposure, and the new scene-view module
needed by WATERS M14. The approved brief keeps sea and clouds outside the V1 exit and makes performance
figures baselines and tripwires, not acceptance gates.

For this module, that means a reusable producer of scenery the main camera cannot see, with the scene's
lighting, correct camera/clip/depth/colour semantics, same-frame delivery, cooked and entity geometry, and
Mac/Chrome consumers. The design covers those responsibilities. Its own cascades, clusters and visibility
avoid the obvious failure of borrowing main-camera products. Slots, one frame graph, shared shader text,
capability admission and absent-feature checks fit the engine contracts. Raster views are a defensible
choice: [Epic's planar-reflection documentation](https://dev.epicgames.com/documentation/en-us/unreal-engine/planar-reflections-in-unreal-engine)
describes rendering the scene again to include off-screen objects, with resolution and scene-complexity costs.
No ray-tracing implementation is required merely to approve this scope.

Underwater optics, caustics, shafts and the meter are assigned to WATERS in its “Under water” and “Light
under water” sections. They are not missing responsibilities of SCENE-VIEWS. This review checks that
allocation, not the correctness of the whole WATERS design. The excluded fog, after-opaque effects,
independent histories, future portal/probe use and iOS target are explicitly recorded assumptions in
[PLAN.md](../../plans/SCENE-VIEWS/PLAN.md), rather than hidden reductions of the request. The small mirror
example is appropriate reusable-engine proof and does not add water to the V1 exit.

## Milestone proof audit and review limits

I read all 24 items, including their automated checks, questions and inspection lists. There is no previous
SCENE-VIEWS design review at this checkout. No milestone verdict or planner field was changed.

| Items | Design-review judgement |
|---|---|
| M1.1–M1.4 | Main-view references, off-frustum/occluded subjects, independent lighting and CPU shadow expectations can reject the principal false substitutes. Account for matching profiles as suggested above. |
| M1.5 | The intended physical reference is useful, but the designed mechanism cannot supply it: Must Fix 1. |
| M1.6–M1.10 | Clipping/reconstruction, absent/idle work, lifecycle, seam admission and retained baselines are judgeable with M1 and its named dependencies. Existing exact-baseline verification also covers Chrome without requiring the new Chrome mirror yet. |
| M2.1–M2.2 | Cooked direct/indirect references and contribution opt-in are available after M1. The declared direct fallback is explicit rather than a successful skip. |
| M2.3–M2.6 | The marker oracle and actual pool inspections cover geometry, illumination, haze and elapsed-time motion in the M2 example. Correct transport remains dependent on resolving finding 1. Temporal reconstruction quality is deliberately handed to WATERS M14.6 rather than claimed from these temporal-off sequences. |
| M2.7 | Retained-baseline verification is judgeable at M2. |
| M3.1–M3.2 | Chrome compares against its own main view and repeats real visual inspections after the example exists. |
| M3.3 | Its submitted-image invariant is inconsistent with the live reader: Must Fix 2. |
| M3.4–M3.7 | Paired costs and investigations, public documentation, separated maturity claims and exact baselines are judgeable at M3. No performance number becomes a gate. |

I independently read the SCENE-VIEWS design, plan, done-when file and proof source; the architecture entry
and seam rulings; C04, C06, C07, C08 and C12; the hardware-native decision; and the relevant WATERS producer/
consumer descriptions. Code inspection was scoped to atmospheric transport, its phase function and lent
aerial products, shadowed aerial composition and the contribution graph seam. This was not a full code
review of those modules. No reading or judgement was delegated.

The planner's `views_proof.py` ran and reproduced its recorded A–C results. The independent counterexample
above also ran. No GPU implementation, visual capture, runtime milestone test or device performance was
judged: the requested review is of an unbuilt design. In particular the 16-phase one-dimensional edge
average is useful preliminary evidence, not qualification of moving reflections under MetalFX.

`./dev check-docs` was run and failed on two pre-existing missing local links in
`docs/evidence/APPLE-TILE/M10-handoff.md`: `M10/README.md` and `M10/rebase-source-identity.json`.
The handoff is tracked at the reviewed commit and those targets are absent there. The checker reported no
broken link in this review. Only this review and its evidence folder were added; all existing files,
including `DONE.json`, remain unchanged and the additions are uncommitted for the review script.

DESIGN CHANGES: 2 must_fix findings

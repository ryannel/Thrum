# CLOUDS design review

**Changes requested: two Must Fix findings.** The revised design fits the human's commission, including the
request to choose Thrum's mechanisms independently of Broadsides. Its placed-shape capability and Sun-disc
phase still contradict their acceptance requirements. These need design decisions before building.

Reviewed commit: `f2418fb348cd19faf73df292cee640ee83d97dfe`, 14 November 2026. Previous review:
`docs/evidence/CLOUDS/design-review.md` at `a0c0bda30e1da38cff06ff4ef7f90c25e030ef28`.
This judges the design and its reviewability, not running clouds. No planner file or milestone verdict changed.

## Must Fix findings

### 1. Must Fix — shadow-only-density-mismatch — verified, different cause after revision

**M8.9 cannot deliver identical ground-field bytes for the declared capability sets when Shapes is present.**
M6.2 also requires that field to match the volume's coarse density. The old detail-noise mismatch is fixed,
but the revision introduces a resource omission with the same consequence for placed clouds.

[DESIGN.md](../../plans/CLOUDS/DESIGN.md), lines 317–322, defines coarse density as full density with
only the detail-noise term replaced by its mean. Lines 510–518 make a nearby placed cell read its volume
slab and feed that same function to the Sun's path. Yet the capability table, line 542, supplies leased
volumes only with Sky; Shade gets footprints alone. Lines 546–548 and M8.9 require identical shade with
and without Sky. A footprint stores each vertical column's sum and occupied bottom/top, not its
distribution of extinction with height.

That information cannot recover a volume's oblique Sun path. The independent
[counterexample](design-review-2/revision-counterexamples.py), section B, constructs two small shapes with
identical footprints, including their occupied bottom and top, but different interior density distributions.
At a 45° Sun their path optical depths are **3.959798 and 0.565685**, giving factors **0.019067 and
0.567971**. A footprint-only evaluator receives the same inputs for both and cannot produce both answers.
The [output](design-review-2/revision-counterexamples.txt) records exact voxel integration; this is not a
measurement of an accepted library asset. Shared constant detail erosion does not recover the missing data.

The column form can supply an approximate shadow without the volume, but it is a different density along
oblique rays. Using it only when Sky is absent breaks byte equality; using it in both profiles changes the
specified Sun-path density and M6.2's oracle. The model's vertical bound remains valid in either case,
which is why the previous placed-column proof does not settle this conflict.

**Resolution:** specify the shadow representation and its required resources for placed shapes, consistently
with and without Sky. Reconcile the capability table, full/coarse definition and M6.2/M8.9 with that
choice. Prove the result with a nonuniform placed volume under an oblique Sun, retaining K3's same-shade
promise and an explicit relationship to the visible volume. The planner owns that choice; this review does
not select a replacement mechanism.

### 2. Must Fix — sun-disc-phase-normalization — verified, new

**M1.8's rendered phase cannot integrate to one under the specified Sun-disc formula.** M5.1 adopts the
same formula. [DESIGN.md](../../plans/CLOUDS/DESIGN.md), lines 400–410, adds half the Sun's squared angular
radius to the HG lobe's squared angle. In
[first_principles_proof.py](design-proof/first_principles_proof.py), lines 20–22 and 38–40,
that increases the denominator everywhere without compensating the numerator. It removes energy rather
than redistributing it over the Sun's disc.

The design proof's normalization check, line 78, explicitly passes `spread=0.0`. Its reported unit integral
therefore checks the point-Sun phase, while its appearance comparisons use the spread phase. I integrated
the latter independently by a closed-form integral of the widened HG lobe and by two quadrature
resolutions; all agree. At the proof's Sun radius, the complete HG–Draine mixture integrates to:

| Droplet diameter | Integral with Sun-disc spreading | Missing first-order energy |
|---|---:|---:|
| 10 µm | 0.981129 | 1.89% |
| 20 µm | 0.926592 | 7.34% |
| 40 µm | 0.810888 | 18.91% |
| 50 µm | 0.767690 | 23.23% |

These are inside the supported 5–50 µm range, not quadrature error. Section A of the
[counterexample script](design-review-2/revision-counterexamples.py) and its
[output](design-review-2/revision-counterexamples.txt) reproduce the calculation. The original
[Jendersie–d'Eon supplemental](https://research.nvidia.com/labs/rtr/approximate-mie/publications/approximate-mie-supplemental.pdf),
sections 2 and 3.1, supports the normalized mixture and diameter fit; it does not establish normalization
of this added spreading rule.

**Resolution:** settle a Sun-disc treatment consistent with the intended energy and angular behavior, and
make the normalization proof exercise the phase actually rendered. Keep the point-Sun fit check separate
from the finite-Sun check, and state which mean cosine M1.8 requires after spreading. The proof must meet
the supported droplet range, not pass only by disabling the new operation. This objection is to the
spreading mechanism, not to choosing the droplet phase.

## Disposition of the previous review

I checked the four previous Must Fix findings against the revised design, plan and items before looking
for revision defects. I read and reran the planner's standard-library revision proof; its
[rerun output](design-review-2/revision-proof-rerun.txt) is retained.

| Previous name | Disposition |
|---|---|
| `numeric-performance-gates` | **Resolved.** The cost account and M3 plan no longer stop on a timing number. All ten cost items require paired measurements and investigation, explicitly saying no figure decides the item. M12.6 permits a recorded performance-phase follow-up. Visual smoothness and attended cadence remain obligations. |
| `placed-volume-query-contract` | **Resolved for the original column mismatch.** The query now uses the shape's own footprint, not a dome with an equal mean. M1.6/M1.7 establish the model's pointwise bound; M6.2/M7.6 distinguish GPU filtering error and nearby clear samples. The asymmetric-shape reproduction confirms the column relationship. It does not prove oblique shadow equality, which is finding 1. |
| `sun-fallback-discontinuity` | **Resolved.** A continuous floored path direction replaces the elevation switch; a border blends transmission to the mean far transmission. ARCHITECTURE ruling 1 agrees. M2.4/M5.5 test heterogeneous sky across the cutoff and boundary. The rerun shows changes shrinking with step size where the old rule had a fixed jump. |
| `shadow-only-density-mismatch` | **Original detail-noise cause resolved; same-shade obligation still fails for the new shape capabilities.** Both profiles now integrate the same constant-detail coarse density, and M3.8 requires nontrivial erosion. Finding 1 preserves the name but identifies the different missing resource and affected item. |

All three earlier Suggestions were taken: slab sampling has a coarsest level and slab-local coordinate
clamp, with contrasting-neighbour tests in M6.1; map age and conformance time are explicit and presentation
slots are separated from weather columns; M7.5 allows recordings long enough for whole lives. The slab
proof confirms isolation through level 7 and mixing at the excluded level 8.

## Fit to the human's request

I read all three tasks in `RECORD.json` under `briefs`, oldest first, and the approved project brief before
judging the design. The approved brief remains the authority for purpose, the V1 exit and performance
policy. The later owner words authorize reuse of Broadsides and ask for an independent choice of cloud
capabilities; they do not make Broadsides' previous assembly a requirement.

The design preserves K1–K7: recognizable weather, G3s lighting, matching shade, structural life and drift,
accepted shapes, temporal stability and visitor controls on Mac and Chrome. A common medium with optional
capabilities is a reasoned response to the later task. Its comparison of retained and changed choices
discloses exposure to the old design and gives reasons beyond reuse. Clouds remain outside the V1 exit,
behind selections, with the sea's existing optical interface preserved. No change to SEA M1's work is
required by the cloud revisions I inspected.

Game-directed shapes are additional scope, but the plan identifies them as an assumption, bounds their
capacity and gives them proofs. The geometry/camera restriction, time-lapse range and deferred formation
demo are likewise disclosed. Moving the accepted library to M6 uses the recorded reuse permission;
provenance and third-party restrictions remain. M10 still checks all twelve shapes and the growth track
in Thrum's light. This review neither reaccepts that content nor authorizes publishing it.

## Changed-item coverage and milestone reviewability

I read all **113 current items**, including every changed/new item listed in the task, and compared the
removals with the previous version. These are design assessments, not milestone verdicts.

| Milestone | Changed/new items read | Assessment with earlier milestones available |
|---|---|---|
| M1 | M1.1, M1.6–M1.13 | CPU field, footprint, directed-shape, provenance and interpolation proofs need no renderer. The new optics requirement needs finding 2 resolved. |
| M2 | M2.1–M2.12 | Shade-first remains achievable on EFFECT-GRAPH M2 and ATMOSPHERE M5. New continuity and repeated-tick mapping checks reject the former fallback and tick-stepped drift. Conformance names sample age; cost records include accelerated weather. |
| M3 | Quality bar; M3.4, M3.8, M3.9 | Numeric timing admission is removed. Motion is checked against independent display-time displacement, including two frames of one tick. The procedural shadow-only proof is coherent; placed shapes arrive later and need finding 1. TAAU qualification remains M9. |
| M4 | M4.2, M4.4 | Actual Chrome state, recovery and submitted motion are required. Aged samples are compared at their source ticks; callback intervals are separated from presented time. |
| M5 | M5.1, M5.3–M5.9 | Droplet light and aureole inspections, sunset continuity and Chrome motion can be judged after M4. Finding 2 affects the optical mechanism. Cost acceptance follows the brief. See S1 for the local-reach diagnostic. |
| M6 | Quality bar/dependency; M6.1–M6.4, M6.6, M6.8–M6.11 | CONTENT-LOOP M1 supplies the lease. Slab isolation, column bounds and directed placement have independent tests. Accepted content enters here with source hashes. Finding 1 prevents the later capability proof from agreeing with the volume-shadow contract. |
| M7 | M7.4–M7.6, M7.8 | Wind, whole-life recordings and filtering-aware column comparisons are reviewable; the too-short recording prescription is gone. Performance is recorded without a numeric gate. |
| M8 | M8.8–M8.10 | Settled atmosphere extensions support world-light proofs. M8.9 tests all capability sets, making finding 1 unavoidable rather than a hypothetical unused combination. |
| M9 | M9.7, M9.8 | TAAU and controls have named dependencies. Knob invariants distinguish authority from placed-slot presentation changes. Cost records retain the full scene. |
| M10 | M10.1–M10.6; removed M10.7/M10.8 | The licence wait is removed on the later owner instruction; source-content proof moved to M6.6. Anatomy, growth, mixed sky and Chrome inspections remain, with a new all-shapes sky inspection. Old M10.7's cost item is now M10.5 without its numeric gate; old M10.8's baseline verifier is now M10.6. Neither proof vanished. |
| M11 | M11.6 | Paired sea-scene cost records replace numeric admission. SEA M8 supplies the receiver and reflection consumer; unchanged integration items preserve depth ordering. |
| M12 | M12.3, M12.5, M12.6, M12.8 | Governor and MetalFX existence is checked before work. Cadence remains attended, capture-off evidence; investigations may leave tuning follow-ups. Public documentation covers capabilities and directed shapes. |

The visual items continue to reject the principal convincing substitutes: a panorama instead of volume,
unrelated scrolling shade, moving noise instead of growth, separately composited hero clouds, and static
history masking motion errors. No additional build blocker was established from the other changed items.

## Suggestions

### S1. Suggestion — scope the local-reach invariance diagnostic — verified wording ambiguity

M5.1 asks whether doubling local reach leaves summed optical depth unchanged within quadrature error.
The revised local segment uses full density and the distant segment coarse density. Changing the split
therefore changes the integral by the full-minus-coarse density over the moved interval, even with exact
quadrature. The diagnostic remains useful on a fixture where the densities agree. Clarify that scope, or
distinguish representation error from quadrature error. This requires no new testing procedure and does
not invalidate the selected local-plus-distant approximation.

## Evidence, limits and validation

I performed the reading and judgement myself, without delegates. I read the design, plan, record, previous
review and current items, then the planner's revision log. I checked relevant contracts, retained cloud
ownership/transport rules, the port decision, CLOUDS seam rulings, EFFECT-GRAPH's tick/interpolation rules
and the existing atmosphere provider in `cloud.rs`/`cloud.wgsl`. I read both new proof scripts, reran the
revision proof and wrote the independent counterexamples above. I checked the phase fit against the primary
supplemental source. I did not reread Broadsides code/assets, rerun the older NumPy field sketch, or run GPU
or visual tests: there is no implemented cloud renderer to qualify. No device or visual-quality pass is claimed.

Validation: `./dev check-docs` passed: 815 Markdown files, 5,658 local links, 30 module records, crate policy
and current design hashes. The [check log](design-review-2/revision-check-docs.txt) records the result.
`git diff --check` passed. The final working-tree inventory contains only this review and four new evidence
files in `design-review-2/`; earlier evidence is preserved. The temporary Cargo output was removed, and all
changes are left uncommitted for the review command.

DESIGN CHANGES: 2 must_fix findings

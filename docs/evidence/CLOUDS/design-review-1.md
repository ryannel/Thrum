# CLOUDS design review

**Changes requested: four Must Fix findings.** The design covers the commissioned sky and its consumers, but
building it as written would conflict with the owner's performance policy and with its own optical and
placed-volume acceptance criteria.

Reviewed commit: `a0c0bda30e1da38cff06ff4ef7f90c25e030ef28`. This is a design review, not a judgement of running
clouds. There was no previous CLOUDS design review. No milestone verdict or planner file was changed.

## Must Fix findings

### 1. Must Fix — numeric-performance-gates — verified

The plan makes a numeric performance result a condition of building and passing, contrary to the approved
project brief. [DESIGN.md](../../plans/CLOUDS/DESIGN.md), lines 442–452, says an automated check fails above
the refresh interval or at about 25% regression, and requires re-planning before M5 if the refresh cannot be
held. [PLAN.md](../../plans/CLOUDS/PLAN.md)'s M3 instructions consequently tell the builder to stop.

This reaches [DONE.json](../../plans/CLOUDS/DONE.json): M2.8, M3.9, M5.7, M6.8, M7.8, M8.8, M9.8, M10.7,
M11.6 and M12.5 require the whole frame to fit 8.33 ms wherever the clouds-off frame does. For example, a
clouds-off frame of 8.2 ms and a clouds-on frame of 8.4 ms fails, regardless of the investigation or measured
cause. Calling these checks “tripwires, never gates” does not change their consequence.

[The project brief](../../plans/BRIEF.md), decision 3, and the
[owner's performance decision](../../design/decisions/PERFORMANCE-PHILOSOPHY-2026-11-06.md) explicitly replace
numeric gates with measurements, investigation and later tuning. C12 says the same. The module brief also
asks for paired records and tripwires rather than a budget gate. The planner's full-refresh assumption cannot
override that instruction.

**Resolution:** keep the cost account, paired measurements, regression investigation, visual smoothness and
attended cadence evidence. Make the performance items prove that work and its recorded disposition, rather
than fail on the timing number alone. Remove the numeric-only stop/re-plan rule throughout the design and plan.

### 2. Must Fix — placed-volume-query-contract — verified

The placed-volume mechanism does not establish the pointwise relationship to the weather query that M7.6
requires. [DESIGN.md](../../plans/CLOUDS/DESIGN.md), lines 176–178, calls the queried column optical depth a
bound on what is drawn. Lines 317–324 instead scale an independently shaped volume to the cell's **mean**
column optical depth. M1.6 and M6.2 require preservation of that mean; M7.6 requires that over **any** ground
point the drawn optical depth never exceeds the query and is zero wherever the query says clear.

Equal means cannot supply that guarantee. Take an isolated cell with two equal-area columns of optical depths
1 and 3, and a placed shape with columns 3 and 1. Both have mean 2, yet the first placed column exceeds the
query. More generally, if the placed columns must be everywhere no larger than the procedural columns and
have the same integral, they must equal them almost everywhere. An independently authored silhouette is not
guaranteed to have that distribution. A positive blend weight does not remove the violation. Clipping the
shape to the procedural dome is an additional mechanism, changes the accepted anatomy, and does not by itself
preserve the required mean.

This is central to consolidating the accepted library with the procedural authority, not an extra accuracy
case. The design proof explicitly leaves placed volumes unrun. The algebra is reproduced in
[counterexamples.py](design-review-1/counterexamples.py), section 2, with
[output](design-review-1/counterexamples.txt).

**Resolution:** settle what the headless column query promises for a placed cell, how the common density
preserves that promise, and which quantities are bounds versus averages. Reconcile M1.6, M6.2 and M7.6 with
that definition while retaining the accepted shapes and one weather authority. Demonstrate the relationship
with a differently shaped volume; normalization of its mean alone is insufficient.

### 3. Must Fix — sun-fallback-discontinuity — verified

The low-Sun mechanism cannot deliver M5.4–M5.5's continuous sunset. [DESIGN.md](../../plans/CLOUDS/DESIGN.md),
lines 228–233, switches every receiver from its spatially varying optical depth to one `tau_far` below the
lowest valid elevation, initially 2°. It makes the same switch outside the field's region. This is also the
specified behaviour of [CLOUDS seam ruling 1](../../plans/ARCHITECTURE.md#seam-rulings-for-clouds).

Under a broken sky, a clear Sun ray can have `tau = 0` immediately above the cutoff while the far mean is 2.
Its factor then jumps from 1 to 0.135335. A ray with `tau = 4` jumps from 0.018316 to 0.135335. Neither jump
shrinks when the Sun-angle step is halved. At 2° the Sun is still above the horizon, so this need not be hidden
by a zero direct-light term. Moving a receiver across the validity boundary has the corresponding spatial
problem. See section 1 of the [counterexample output](design-review-1/counterexamples.txt).

I also read the existing consumer in `crates/thrum-render/src/atmosphere/cloud.rs` and `cloud.wgsl`, and
[ATMOSPHERE's provider contract](../../plans/ATMOSPHERE/DESIGN.md#environment-identity-and-products). Its
current unit-transmission fallback explicitly requires the provider to approach that value continuously.
Changing the fallback value does not remove that obligation. No transition to the new mean is specified.

**Resolution:** define a continuous transition in elevation and at the spatial support boundary, including
how the far value remains defined after the detailed product becomes invalid. Reconcile the producer,
consumer ruling and M2.3/M5.4/M5.5. A heterogeneous sky crossing the cutoff must show the claimed continuity;
clear and uniformly overcast cases alone would hide this defect.

### 4. Must Fix — shadow-only-density-mismatch — verified specification conflict

The design cannot simultaneously use the specified full density for its Sun-path product, remove detail
erosion in shadow-only mode, and retain identical shadow bytes. [DESIGN.md](../../plans/CLOUDS/DESIGN.md),
lines 182–191, makes shape noise and detail erosion part of the one density evaluated by every consumer.
Lines 211–214 fill the Sun-path product by marching that density. Lines 333–335 then omit detail noise from
the shadow-only profile while promising the same ground field. M3.8 requires both resource omission and byte
equality; M2.8 also requires the detail resource to be absent.

For an edge segment whose extinction is halved by detail erosion, a 100 m path at the stated interior scale
of 0.05 m⁻¹ has optical depth 2.5 with erosion and 5 without it. The corresponding factors are 0.082085 and
0.006738. This is an illustration of the contradiction, not a measurement of a cloud asset; see section 3 of
the [counterexamples](design-review-1/counterexamples.txt).

A shared coarse shadow representation could make equality possible, or an equivalent evaluator could retain
the same density without the stored detail texture. Neither is the currently specified mechanism. Simply
omitting erosion only when visible clouds are off changes shade, contrary to K3.

**Resolution:** specify the density and approximation that the Sun-path product actually integrates in both
profiles, and the resources needed for it. Keep its relationship to the view density and CPU oracle explicit,
update the cost account if necessary, and make M3.8 test a case where detail erosion changes extinction.

## Suggestions

### S1. Suggestion — define sampling of the stacked volume's mip tail — verified

Stacking shapes does not by itself make every mip stay inside its slab, as DESIGN.md lines 313–315 claim.
The existing cooker (`crates/thrum-cook/src/texture.rs`, lines 102–113 and 384–416) downsamples the entire
volume until all dimensions reach one. Twelve stacked 128³ shapes become a 1×1×12 level, then a 1×1×6 level
that averages pairs of shapes. Section 4 of the [counterexample output](design-review-1/counterexamples.txt)
reproduces that recurrence with constant-valued slabs.

State the usable mip range and slab-local filtering/addressing policy, including growth keys. This can be
resolved by a bounded sampling policy within the chosen representation; it need not introduce a new loader.
If special cooking is chosen, settle that ask with CONTENT-LOOP, whose design explicitly leaves atlases to
the consumer's later request. M6.1 already requires isolation, so its test should include the coarsest sampled
levels and contrasting neighbouring slabs.

### S2. Suggestion — distinguish current weather from aged presentation samples — verified ambiguity

DESIGN.md lines 199–214 updates the weather map at about 2 Hz and the Sun product in tiles. Its cost account
spreads Sun updates over 8, 32 and 128 frames. Neither the timebase of “2 Hz” nor the interpolation/age policy
is stated. At 60×, half a second of wall time spans 30 seconds of weather evolution. The continuous CPU
function alone does not prove that replacing an aged texel is visually continuous. M2.6 and M9.5 provide
useful motion inspections that should expose this during building.

Clarify the sample time used for conformance in M2.2/M4.2 and how age enters the allowed error. Also narrow
M9.7/M12.3's “no weather-map texel changed” to authoritative weather meaning: the map explicitly contains a
placed slot and weight, while placed range is a quality knob that changes those presentation fields. The CPU
query remaining unchanged is the appropriate invariant; identical presentation bytes are a different claim.

### S3. Suggestion — let lifetime inspections run for the lifetime — verified

M7.5 prescribes 60-second recordings at 8× and 60× and asks reviewers to follow at least three clouds from
birth to death. At 8× that covers eight weather minutes; the first profile's cells live 12–30 minutes.
Keep the whole-life requirement and leave the recording duration to the builder. This is a reason to shorten
the procedural proof instructions, not add another capture checklist.

## Fit to the request and milestone reviewability

I derived the commission first from the sole verbatim task in RECORD.json, the approved project brief and the
CLOUDS brief. The required result is one deterministic weather authority with procedural and accepted placed
clouds, G3s lighting, matching shade, formation and thinning, stable motion, visitor controls, Mac and Chrome
support, and consolidation in Thrum without making clouds a V1 exit gate. The design covers all seven named
cues and includes the sea consumer. The sky overlay and haze scale support the requested shared lighting;
they are relevant additions. The licence hold, stand-in-first sequence and off-by-default scene selections
respect the stated boundaries. The geometry restriction, time-lapse range and timing interpretation are
explicit assumptions for the human, rather than silently omitted scope. The performance interpretation still
needs finding 1's correction because the approved brief wins.

I read all 107 items in DONE.json and considered each with preceding milestones available. The table groups
that assessment; it does not assign milestone verdicts.

| Milestone | Items | Assessment at its planned endpoint |
|---|---:|---|
| M1 | 11 | CPU-only proofs are available independently; density/query semantics need finding 2 resolved. |
| M2 | 10 | Shade can be proved before a view march; GPU provider extension has an identified owner. Findings 1, 3 and 4 affect its rules. |
| M3 | 12 | First visible frame, transport and motion outputs can be judged before TAAU; full reconstruction is deliberately later. Findings 1 and 4 apply. |
| M4 | 5 | Actual Chrome conformance and submitted captures are required, rather than a Mac proxy. Clarify aged-state comparison as in S2. |
| M5 | 8 | The lighting inspections reject a simple gradient and cover sunset. The fallback must change to make their continuity requirement attainable. |
| M6 | 9 | Cooked stand-ins prove the lease and hybrid route without copying the unlicensed library. Resolve finding 2 and the mip policy. |
| M7 | 9 | Preset transitions and whole-life motion reject a moving texture substitute. The pointwise bound conflicts with placed volumes; S3 corrects the recording prescription. |
| M8 | 9 | Reflection/diffuse overlay and haze have explicit owning-module changes and preservation checks. No new prerequisite is hidden here. |
| M9 | 9 | TAAU and controls are explicitly deferred until APPLE-PRESENTATION M3 and CREATOR M1. Clarify quality-knob invariants as in S2. |
| M10 | 8 | Accepted content is explicitly required after the licence, so stand-ins cannot close K5. Source hashes and anatomy inspections are useful independent proofs. |
| M11 | 7 | SEA M8 provides the receiver and reflection consumer; the sea-depth ordering is explicit. |
| M12 | 10 | Governor and MetalFX existence checks precede work; attended panel evidence is separate from engine capture. Numeric timing gates still require correction. |

The visual lists generally address convincing substitutes: shared bases and parallax rather than a sky dome,
cloud-aligned shade rather than an unrelated scrolling field, structural life rather than drifting noise,
mixed occlusion rather than a separately composited hero, and fast motion rather than static history hiding
artifacts. The numerical findings above are requirements that these later inspections cannot reconcile on
the planner's behalf.

## Evidence and limits

I read the design, plan, all done-when items, record and log; the retained cloud ownership/transport rules;
relevant C04/C07/C08/C12 contracts; CLOUDS interface rulings; the atmosphere provider on both sides; and the
volume cooker and lease contract. I also read the design proof's Python source and saved output. I did not
inspect Broadsides code or assets, qualify the accepted library anew, or run GPU/cloud visual tests: the cloud
renderer is not built. No running-quality or device-performance conclusion is claimed.

The new [counterexample script](design-review-1/counterexamples.py) was run with Python's standard library;
its [output](design-review-1/counterexamples.txt) records the four small checks above. These are algebraic and
cooker-recurrence examples, not engine test results. The planner's NumPy proof was not rerun: the available
`python3` has no NumPy. Attempts to open the cited Frostbite and Nubis³ PDFs through the web reader exceeded
its document-size limit; the review does not independently certify their quoted timings or paper summaries.

Validation: `./dev check-docs` passed (810 Markdown files, 5,528 local links, 30 module records, crate policy
and current design hashes); [the log](design-review-1/check-docs.txt) records the result. Build output was
directed to `/tmp/thrum-clouds-design-review-target`. The final working-tree check showed only this review
and its evidence folder added, with all previously tracked files unchanged.

DESIGN CHANGES: 4 must_fix findings

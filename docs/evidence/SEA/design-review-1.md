# SEA design review

**Design changes requested: 5 Must Fix findings.** Reviewed commit
`53767d79505df0d3ed7be9edc1553c4a7fb0c0c7`, 13 November 2026. This is a design review, not a
qualification of running water, visual quality or device cost. There was no previous design review.

The design addresses the requested sea, bodies, wakes, foam, optics, reconstruction and controls, with sensible
ownership and an early first-frame milestone. Building it literally would nevertheless conflict with the owner's
performance policy, leave two optics mechanisms unresolved, fail the promised interpolation endpoints after a
hitch, and require an impossible foam test. These need resolution before approval.

**MF1 — Must Fix, verified: numeric performance gates override the approved brief.**

[DESIGN.md](../../plans/SEA/DESIGN.md), lines 419–429, says an automated check fails above the refresh interval
or at about 25% pass growth. [PLAN.md](../../plans/SEA/PLAN.md), lines 45–47, makes an over-budget first frame
stop further building. The affected items are M3.6, M6.8, M7.9, M8.7, M9.10 and M10.5 in
[DONE.json](../../plans/SEA/DONE.json).

This contradicts the approved [project brief](../../plans/BRIEF.md), decision 3, and the
[owner's performance decision](../../design/decisions/PERFORMANCE-PHILOSOPHY-2026-11-06.md): figures are
baselines and investigation tripwires, not admission gates, and whole-frame tuning comes later. Calling the
checks “Tripwires, never gates” does not change their effect. For example, a functional, visually acceptable
8.5 ms first frame would block M5 solely on a number under the written plan.

Resolve this throughout the design, plan and items: retain paired measurements, investigation of regressions
and visible stutter, the full-refresh ambition and independent visual review, but remove automatic numeric
admission and timing-only build stops. This is an explicit human requirement, not a suggestion to relax quality.

**MF2 — Must Fix, verified design gap: body rectangles do not define the region receiving their reflections.**

[DESIGN.md](../../plans/SEA/DESIGN.md), lines 314–321, copies and traces only inside padded screen rectangles
of registered bodies; everywhere else the surface reads neither colour nor depth. M8.5 enforces that restriction,
while M8.3 requires the craft and floats to reflect in adjacent calm water.

A direct projection is not a reflection footprint. With a horizontal camera 2 m above calm water and a 2 m tall
body 20 m away, the body's vertical screen interval, divided by focal length, is `[-0.1, 0]`. Its reflected
image occupies `[-0.2, -0.1]`. The water point reflecting its top is only 10 m from the camera. The receiving
pixels lie outside the opaque silhouette even with both the body and reflection plainly on screen.
[The calculation](design-review-1/counterexamples.py) and [its output](design-review-1/counterexamples.txt) are retained.

“Padded” supplies no bound or method covering these receiving pixels over the specified view heights and wave
normals. The source texels may be confined to bodies, but that does not confine the pixels that need to trace
and read them. A small silhouette margin would truncate an ordinary reflection; an unspecified arbitrarily
large margin does not establish the claimed cost saving either.

Define the source and receiving regions, their mapping and conservative coverage or an explicit bounded
approximation. Demonstrate that the gather can supply M8.3's reflection outside the direct body rectangle, and
make M8.5 judge that resolved mechanism. Keep the no-whole-scene-copy objective; this finding does not require
abandoning it.

**MF3 — Must Fix, verified interface gap: the gathered colour is not the physical radiance used in the optics equation.**

[DESIGN.md](../../plans/SEA/DESIGN.md), lines 300–320, combines physical reflected/background radiance with
water transport, applies camera-to-surface air, then applies exposure. Its gather copies opaque scene colour
and depth, without specifying how that colour becomes either of those physical inputs.

The actual producer in [synthetic.wgsl](../../../crates/thrum-render/src/synthetic.wgsl), lines 560–561, has
already applied camera-to-object aerial transport and `scene_storage_scale()`. The
[ATMOSPHERE interface](../../plans/ATMOSPHERE/DESIGN.md#environment-identity-and-products), lines 235–252,
separately lends unexposed lighting and aerial transport only from the active camera. It does not supply a
surface-to-reflected-object segment. [C08](../../design/CONTRACTS.md#c08-light-materials-and-transport--v3-with-e1-and-v4)
requires constituent transport once and a declared approximation where composition is not exact.

Substituting the copied scene value into `L_behind` or `L_reflected` therefore reapplies exposure and carries
the original camera path into a second air composition. In the clear-air scalar example in the retained
calculation, radiance 10, exposure 0.25 and water transmission 0.5 should contribute 1.25 to stored colour;
using the raw copy contributes 0.3125. Undoing exposure alone still leaves the wrong air path, especially for
the submerged float whose copied opaque shading treated its whole camera path as air.

Specify the gather output's units and transport ownership, how existing camera-path transport is removed,
replaced or deliberately approximated for reflection and refraction, and the approximation's valid range.
Resolve any additional producer need through the existing seam owners. Acceptance must distinguish correct
single exposure/transport from a pleasing frame at one exposure. M3.3's light inspection and M8.4's cloud
factor test do not settle this later scene-colour composition. This finding verifies the missing interface
conversion, not a bug in an SEA shader that has not been built.

**MF4 — Must Fix, verified: alternating banks cannot supply the promised consecutive endpoints after skipped ticks.**

[DESIGN.md](../../plans/SEA/DESIGN.md), lines 141–157, advances operators once by a bounded tick count and
keeps two persistent banks written on alternating ticks. M2.4 requires those banks to contain the committed
tick and the one immediately before it through a hitch, discarded frame and resets.

Start with banks for ticks 9 and 10, then submit tick 12 after missing tick 11. Writing the new tick's bank
leaves ticks 9 and 12 if indexed by tick parity, or 10 and 12 if alternating by update count. Neither supplies
11 and 12. The retained calculation shows the parity case. Marking reconstruction history invalid cannot
manufacture the missing endpoint used to position the surface. This also needs a rule for stateful wake and
foam, which cannot simply be reevaluated from absolute time as a constant ambient spectrum can.

The [effect seam](../../plans/EFFECT-GRAPH/DESIGN.md#state-time-and-the-tentative-frame-m2) executes each
producer once per frame and distinguishes advancing, resetting and keeping state; it does not create the
missing endpoint. Its capture frame keeps state whatever the requested tick
([state.rs](../../../crates/thrum-render/src/contribution/state.rs), `EffectState::begin`). M2.4's expectation
of “the model's sea for that frame's tick” must also respect that read-only capture rule.

Define the endpoint contents, timestamps and interpolation policy on multi-tick advances, resets and capture
frames, including wake and foam and the extra work/storage if needed. Align M2.4 and later stateful lifecycle
proofs with that policy. The normal two-frames-per-tick case is already explained; the missing case is an
ordinary hitch explicitly promised by the milestone.

**MF5 — Must Fix, verified: M7.1 forbids the stock transfers it requires.**

[DONE.json](../../plans/SEA/DONE.json), line 566, requires “With no cause every stock only falls”, then
requires fresh film to become aged film and submerged air to become fresh film. With no external births,
start with fresh film and zero aged film: rupture must increase aged film. Start with submerged air and zero
film: rising air must increase fresh film. These are the intended delayed responses, not new booked causes.

The retained calculation uses ordinary equal-lifetime transfer from fresh to aged to the sink: after one
tenth of a lifetime fresh is 0.904837 and aged has risen from zero to 0.090484, while their sum decreases.
A correct transfer model cannot satisfy the item; an implementation that merely decays every stock can satisfy
its first clause while losing the requested material response.

Replace per-stock monotonicity with the intended accounting of transfers and sinks, allowing recipient stocks
to rise without external input. Keep source-off persistence and eventual disappearance. This is an impossible
acceptance requirement as written, not a request for a more elaborate test procedure.

**S1 — Suggestion, verified limitation: distinguish free propagation from the forced, bounded wake update.**

The design proof is useful evidence for dispersion and the two bands. Its `Level.step` uses a fixed `DT=1/60`,
half pressure kicks and a per-step border mask. It does not test the design's variable-size hitch update,
the stationary loaded craft, or the feedback from body forces. The exactness claim in
[Tessendorf's eWave paper](https://jtessen.people.clemson.edu/reports/papers_files/ewavealgorithm.pdf), equations
19–20, concerns free propagation; it does not make these added operators exact for an arbitrary step.
State the chosen catch-up bound and the forcing/damping approximation, and extend the eventual lifecycle
proof to them. Existing M6.3 correctly requires the coupled craft to settle, so this is a proof clarification,
not a finding that eWave cannot make the requested wake.

**S2 — Suggestion, verified ambiguity: make the query's time convention explicit.**

DESIGN lines 209–220 evaluate long waves at the requested use tick but read cascades/wake at their state tick,
then describe a result with one evaluation tick and game-side extrapolation of only the latter parts. State
whether results expose those parts and their times separately or normalize them before delivery. This will
prevent extrapolating an already predicted long wave twice and make M5.1–M5.2 easier to judge.

**S3 — Suggestion: shorten inspection procedures without shrinking the required range.**

Several items prescribe capture order, crops and sequence duration in addition to the cue and operating
range. Keep the named T2 capture mode, target, cue and range; leave the reviewer the route to demonstrate them.
The existing inspection coverage is valuable and should not be replaced by a generic “looks good”.

**Scope and coverage.** I first read the full planner task under `briefs` in
[RECORD.json](../../plans/SEA/RECORD.json), the approved project brief and SEA's module brief. The commission
is a reusable Mac/Chrome sea with S1–S7, shared lighting and asynchronous physical queries, independent oracles,
bounded capacity and runtime knobs, outside the V1 exit. The later planner task permits planning now as a
recorded assumption; it does not revoke the performance decision or authorize copying unlicensed code.

I then read the whole SEA design and plan and every item in DONE.json: 86 items across ten milestones. No
previous review existed, and no item or plan was changed. I also inspected the architecture's SEA ownership,
interfaces and seam rulings, the water and port decisions, relevant C02/C04/C06–C09/C12 contracts, the
EFFECT-GRAPH state design and implementation, ATMOSPHERE's environment handoff and forward shader, and the
local and skill calibration examples. I read both planner proof scripts and their saved outputs; I did not
rerun their NumPy simulations or inspect Broadsides. The review is not a blind technique study.

| Requirement / milestone scope | Assessment beyond the findings above |
|---|---|
| S1; M1–M4 and M9 | The long-wave/cascade split, moments, horizon and native/browser first-frame proofs cover the requested sea. CPU oracles precede GPU and scene work. |
| S2; M5 | GPU query, stale generations, capacity refusal, fallback and game-owned integration are covered. Scheduled bobbing is explicitly challenged. |
| S3; M6 | A dispersive, persistent field and query-driven floater reject a wake decal. The reduced wake reach is disclosed in the plan as an assumption, so it is not a hidden narrowing. |
| S4; M7 | Births, transport, source-off trails, whitecaps and the requested unfinished foam looks have proofs; MF5 must first make the reference criterion coherent. |
| S5; M8 | Shared Sun/cloud attenuation, depth and visual optics are covered in intent; MF2 and MF3 leave the gather/composition mechanism incomplete. |
| S6–S7; M9–M10 | Controls, overlays, TAAU, MetalFX, cuts and governor integration have appropriate target-specific proofs. MF4 affects their temporal inputs. |
| Cross-module readiness | M2, M3, M7 and M9 name the needed producer milestones. M10 explicitly waits for the as-yet-unplanned governor/MetalFX milestones, with an architecture migration instruction to replace its temporary readiness check. This is disclosed unfinished dependency work, not a false completion claim. |
| Scope and ownership | Inland water, the three parked spray/sheet/gas cues and multiplayer are explicitly deferred. Sea code remains reusable; content/body integration stays in the example. Licensing and port timing are stated assumptions. No new V1 exit requirement is introduced. |

Only the small algebraic counterexamples were executed for this review. There is no built SEA scene to run,
and no visual or presented-cadence evidence is claimed. The counterexamples explain design defects; they are
not substitutes for the later Metal, Chrome, oracle and independent visual reviews. `./dev check-docs` passed;
its output is recorded in [check-docs.log](design-review-1/check-docs.log). Only this review and its evidence
folder were added, and they are left uncommitted for the review script.

DESIGN CHANGES: 5 must_fix findings

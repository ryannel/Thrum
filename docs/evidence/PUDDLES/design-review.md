# PUDDLES design review

**Design changes requested: five Must Fix findings.** The design substantially addresses the commission,
but its reflection and rain-state mechanisms have gaps, and its second milestone needs a prerequisite it
does not declare.

Reviewed 14 November 2026 at `783c3b1acbf1154dc97e3d533169cd4c719221b4`.
This is a design review, not implementation, visual or device qualification. There was no previous design review.

## Must Fix findings

### 1. Mean-ground mirror clipping loses the foot of a reflected object

**Verified by tracing the two designs.** P5 and M7.2 require off-screen scenery reflected down to its foot
at the waterline. [PUDDLES DESIGN](../../plans/PUDDLES/DESIGN.md), lines 385–390, assigns a hollows body and
its film one mirror plane at the mean height of its ground. [WATERS DESIGN](../../plans/WATERS/DESIGN.md),
lines 1158–1175 and 1188–1191, renders only geometry above that plane; depth correction operates on that image.

Take a yard whose mean ground is 0.20 m, a puddle at 0.05 m and a post standing in it. The post's bottom
0.15 m above water is clipped out of the mirror source, despite the puddle being inside the 1 m plane
tolerance. No lookup correction can recover geometry absent from the image. With a mirror view assigned,
WATERS also casts no screen-space reflection ray (line 1186). A yard on a slope can put other puddles outside
the tolerance altogether, leaving them without off-screen scenery even when they dominate the view.

Resolve the plane selection, clipping and lookup together for puddles at different elevations and for film.
The proof must cover a low puddle reflecting an object through the mean-ground plane, not just a nearly level
yard. This does not require an unlimited view count; it requires a supported range that actually delivers P5.

### 2. Switching rain off erases existing rings

**Verified specification conflict.** M7.1 says changing the rain rate resets no ring. The band design
([DESIGN](../../plans/PUDDLES/DESIGN.md), lines 338–359) sums drops from the preceding second, but supplies
only the current tick's rain record, holds no band state, and thresholds the schedule by the rate.
M4.2 explicitly requires the passes not to run with no rain.

Stop rain while a 0.4-second-old ring is spreading. Its response still has another 0.6 seconds of the
designed lifetime, but the next zero-rate frame must stop evaluating it. Retaining the old texture would
freeze it instead. A soft threshold in rate does not solve an abrupt source-off, nor does current rate tell
an evaluator whether a candidate drop actually fell before a rate change. The same gap affects arbitrary-time
capture and reconstruction history.

Specify the bounded committed input history or equivalent state that preserves births and their responses,
and distinguish no new drops from no live rings. Reconcile M2.6, M4.1–M4.2 and M7.1 around that behavior.
Show an on/off/restart transition with old rings continuing and no births before the restart.

### 3. The local rain partition does not preserve first-hit receivers

**Verified from the stated rules; the shelter counterexample is reproduced.**
[DESIGN](../../plans/PUDDLES/DESIGN.md), lines 177–187, keeps the lowest canopy underside and its transmission,
then declares a single rain partition. Put a half-through crown below an opaque roof. The selected crown
passes half the rain to the ground; a point between crown and roof gets full rain. Both are sheltered by the
opaque roof. This is ordinary static overlapping shelter, not the explicitly excluded moving shelter or
slanted rain. M1.5's isolated roof and crown cases would miss it.

There is a second instance of the same ownership gap at the water/ground boundary. Lines 239–264 route rain
through the ground stores even where a puddle covers the ground, explicitly counting that ground as open.
Lines 298–300 then report full pores under a puddle independently of those stores. Thus rain first hitting
standing water can fill the open-ground store instead, and that class's state also describes uncovered ground.
Closing the body's total account cannot establish the local receiver assignment.

[C09](../../design/CONTRACTS.md#c09-environmental-fields-and-physical-exchange--e1-with-p3), lines 394–397
and 426–435, requires first-hit ownership and explicitly says a global mass sum cannot certify the local
assignment. The design's declaration of this approximation does not reconcile that interface.

Settle exposure through overlapping static shelter and the transfers between submerged ground and puddle
water. If some arrangements are unsupported, name and refuse them at preparation. Keep the local receiver
proof separate from the total-account proof. This finding does not ask for the parked receipt machinery or
a live runoff solver.

### 4. The no-film acceptance claim fails after the pores saturate

**Verified by running the planner's own store function on a longer shower.** M2.1 says ground whose pores
drink faster than rain takes no surface water. The design likewise says earth that drinks faster than rain
never shows sheen (lines 268–276), and M5.5 permits earth's sheen only once its puddles form.

The proof's earth has 3 mm of pore capacity, a 15 mm/h soak rate and 0.5 mm of relief. At 10 mm/h, with drying
off, its ten-minute demonstration ends before saturation. Continue that same shower: pores fill at 1,080 s;
film is 0.333333 mm at 1,200 s and 0.5 mm at 1,800 s. This follows exactly from stopping absorption when the
finite pores fill. It happens before runoff from the full relief needs to create a puddle.

Reconcile finite absorption, continuing infiltration, film and puddle losses, and make the acceptance claim
match the chosen behavior through saturation. A ten-minute fixture cannot prove the current unconditional
claim. The fix can be a corrected physical claim rather than a more complicated model, provided the requested
wet-ground behavior and water account remain coherent.

### 5. M2 extends an interaction reference that its prerequisites do not provide

**Verified against both done-when files.** PUDDLES M1 depends on WATERS M1; PUDDLES M2 adds no external
dependency. Yet M2.7–M2.8 extend the existing interaction reference, preserve its near level, and run
`cargo test --locked --offline -p thrum-water-model --test interaction` unchanged.

[WATERS DONE](../../plans/WATERS/DONE.json), M6.1–M6.2, builds that reference and test. WATERS M1 supplies
the map, surface evaluator and conformance harness, not the interaction solver. The claim in
[PUDDLES PLAN](../../plans/PUDDLES/PLAN.md) that both CPU milestones can proceed after WATERS M1 therefore
cannot hold as written. With only the declared predecessors built, the reviewer cannot judge M2.7–M2.8.

Place the fine-interaction work after its actual producer, or explicitly reconcile moving that producer
earlier with WATERS. Keep the independent stores and rain-response work early if useful; do not require the
PUDDLES builder to invent an undeclared substitute for WATERS M6.

## Suggestions

- **Verified underspecification — damp-band persistence.** Lines 290–296 define the band's height from how
  fast the level has "lately" fallen, divided by drying rate. Specify what remains when a tide stops falling,
  a puddle becomes empty, or drying is zero. M2.3 and M5.6–M5.7 need a fading band, but no update or lifetime
  for that retained number is given. A compact state rule would make these ordinary transitions judgeable.
- **Verified weak numerical bounds.** M4.1, M4.6 and M5.2 accept whatever GPU difference is first measured.
  Record the measurements, but distinguish expected numerical error from an incorrect evaluator; otherwise
  a biased implementation can define its own passing tolerance. The visual items help reject conspicuous
  defects, so this is a suggestion to sharpen the numerical proofs rather than another build blocker.

## Coverage and evidence

I read the sole verbatim planner task in [RECORD](../../plans/PUDDLES/RECORD.json) and the approved
[project brief](../../plans/BRIEF.md) before the design. The derived requirement is cohesive water: rain
wets exposed ground, fills and joins hollows, produces rings from drops and feet, reflects scenery and sky,
dries in order, and wets the shore, on the island and optional V1 valley selection, including Chrome.
The brief's engine reuse, unchanged V1 exit and non-gating performance baselines remain authoritative.

I read the complete PUDDLES design and plan and all 72 acceptance items, the architecture's module brief,
interfaces and seven rulings, relevant WATERS producers and consumers, the cited C4 contracts, review guidance
and graded examples. There is no plans glossary. No reading was delegated.

The design's hollows body, shared optics/query, optional capabilities, CPU state ownership and explicit cost
account fit the commission. No second renderer is proposed. Missing falling rain, visible runoff, lake/river
feeding, tracks, slanted rain and shade-dependent drying are explicitly disclosed assumptions, not silently
omitted requirements; I have not turned them into new requirements. The visual quality claims remain unproved
until their milestones run.

Acceptance coverage: M1 has independent topology, fill and account oracles, but needs the local receiver case
above. M2 needs saturation and input-history reconciliation and its missing producer. M3's topology/query and
waterline inspections are judgeable after WATERS M8. M4's steady-rain inspections do not settle rate transitions.
M5's screen-space sheen can be judged before WATERS M14; off-screen reflection correctly waits until M7,
but the chosen mean-ground plane still fails there. M6's valley integration is gated on SHOWCASE M6.
M7's shared controls, reconstruction and off-screen mirror checks are otherwise appropriately placed after
WATERS M10, M14 and M16. Exact baselines protect the unselected scenes; they do not prove the new effects.

Executed:

- `python3 docs/evidence/PUDDLES/design-proof/puddle_proof.py`: completed; [output](design-review/planner-proof-rerun.txt).
  Its reported merges, account residual and ring arithmetic reproduced. This is corroboration of those
  small demonstrations, not an independent proof of their implementation.
- `python3 docs/evidence/PUDDLES/design-review/ordinary-inputs.py`:
  [source](design-review/ordinary-inputs.py), [output](design-review/ordinary-inputs.txt).
  The store case executes the planner's function; canopy and clip cases apply the written rules directly.
- `./dev check-docs`: passed (866 Markdown files, 6,428 local links, 30 module records, crate policy and
  current design hashes). Only this review and its evidence folder were added; the plans and code are unchanged.

No engine captures, motion review or device timings were taken. The findings about clipping, source history
and prerequisites are verified design/interface contradictions, not claimed observations of a running game.

DESIGN CHANGES: 5 must_fix findings

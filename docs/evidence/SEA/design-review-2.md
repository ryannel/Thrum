# SEA design review

**Design approved, with one Suggestion and no Must Fix findings.** Reviewed commit
`7c7744bb5a632ad26c9376b4d2a8746f2a5f399b`, 14 November 2026, against the previous review at
`53767d79505df0d3ed7be9edc1553c4a7fb0c0c7`.

The revised design does what the human asked for: a reusable Mac and Chrome sea with the brief's S1–S7,
shared lighting, asynchronous body queries, physical wakes, persistent foam, controls and reconstruction
proofs. It keeps sea outside the V1 exit and respects the owner's performance policy. All five previous
Must Fix findings are resolved. This approves the design and its acceptance criteria; it does not qualify
running water, visual quality or device performance.

**S4 — Suggestion, verified evidence limitation: distinguish the foam transfer proof from the complete hitch proof.**

[DESIGN.md](../../plans/SEA/DESIGN.md), lines 314–344, and M7.11 in
[DONE.json](../../plans/SEA/DONE.json) describe transfers, bounded births and transport during a hitch.
Part D of [revision_proof.py](design-proof/revision_proof.py) exercises only the source-free, unsaturated
linear transfer chain. It contains neither the full-cover loss rule, births, drift nor spreading. Its
long-step equality therefore establishes the transfer result for those inputs, not equality of the complete
foam update. Near full cover, surfacing air introduces the additional bounded transfer that the linear
matrix alone does not represent.

Keep that distinction explicit when implementing and recording M7.11: judge the composed update against the
reference with active births and transport, including cover near one, and identify its numerical tolerance.
This is sharper proof coverage, not a build blocker: M7.11 already requires births and drift to agree with
the reference, M7.1 requires bounded cover, and the design states where excess surfacing air goes. No new
mechanism or more prescriptive test procedure is requested.

**Previous Must Fix findings, checked against the revised assembly**

- **MF1 — Resolved: numeric performance gates override the approved brief.** DESIGN lines 557–570 now
  explicitly separate measured figures from admission. The check fails for missing measurements or an
  uninvestigated tripwire, not for crossing a timing threshold. PLAN's M3 no longer stops later building
  on frame cost. M3.6, M6.8, M7.9, M8.7, M9.10 and M10.5 all require paired records and investigation and
  explicitly exclude a numeric verdict. The full-refresh ambition, visual smoothness and M10.6's attended
  presentation witness remain. This matches project-brief decision 3 and the owner's performance decision.

- **MF2 — Resolved: body rectangles do not define the region receiving their reflections.** DESIGN lines
  379–414 distinguish source rectangles from receiving rectangles, construct reflection/refraction volumes,
  project them into a receiving mask and trace from receiving sea pixels into the gathered source. Reach,
  slope allowance, fading and off-screen fallback are explicit. The old counterexample is covered: the
  2 m body at 20 m, viewed from 2 m, has a receiving vertical interval approximately `[-0.205, 0]`, which
  contains the reflected image outside its direct silhouette. Part C's 36 ray cases report zero missed
  receivers, including reflected hits outside the source rectangle. M8.3 now requires a whole reflection;
  M8.5 tests receiving coverage independently against CPU rays. M8.7 records the potentially large gale
  mask with zero, one and nine bodies. The plan discloses the registered-body, 64 m reach and 20° normal
  limits as an assumption rather than quietly promising general scene reflection.

- **MF3 — Resolved: the gathered colour is not the physical radiance used in the optics equation.** DESIGN
  lines 416–447 now invert the forward pass's exposure and camera-path transport before the sample enters
  water composition. The gather's storage scale and the surface's inverse scale are stated. I checked the
  producer in [synthetic.wgsl](../../../crates/thrum-render/src/synthetic.wgsl), its clear and shadowed aerial
  variants, and [ATMOSPHERE's lent operations](../../plans/ATMOSPHERE/DESIGN.md#environment-identity-and-products).
  The clear-path inverse matches the existing producer. The reflected short air segment, camera-direction
  radiance, vertical submerged illumination, shadowed in-scatter and clamped storage are declared
  approximations; the design no longer claims that ATMOSPHERE supplies an arbitrary reflected segment.
  Part E recovers stored colour 1.142 instead of 0.354 from the raw copy or 1.001 from exposure-only
  correction. New M8.9 requires an independent CPU expectation, two exposure scales and a distant case
  that rejects both wrong substitutes. Existing visual optics items still judge the resulting appearance.

- **MF4 — Resolved: alternating banks cannot supply the promised consecutive endpoints after skipped ticks.**
  DESIGN lines 142–188 now promise the last two committed updates, each timestamped, and blend across
  their actual span. After ticks 9 and 10 followed by tick 12, the banks are 10 and 12; at alpha 0.5 the
  weight is 0.75, not an invented tick-11 endpoint. Full frames seed both banks, captures preserve them,
  and discarded frames commit nothing. This matches EFFECT-GRAPH's state table and `EffectState`'s
  capture/submission rules. The design declares the interpolation error and an eight-tick catch-up bound
  with counted dropped ticks. M2.4 and M4.2 follow the new table; new M6.10 and M7.11 cover stateful wake
  and foam lifecycle. The ordinary two-presentations-per-tick interpolation remains, and M3/M9 still
  require submitted motion and visual stability. This is an explicit bounded approximation rather than
  an impossible claim that the missing endpoint exists.

- **MF5 — Resolved: M7.1 forbids the stock transfers it requires.** DESIGN lines 314–327 and M7.1 now allow
  recipients to rise, constrain film plus yield-weighted air inventory, name the sink and require eventual
  disappearance. The independent closed-form expectation must reject simple decay of each stock. Part D
  shows fresh film becoming aged film: at 6 seconds fresh is about 0.368, aged about 0.533 and inventory
  about 0.901. Recipient growth and falling total inventory are compatible. The impossible per-stock
  monotonicity requirement is gone; S4 above concerns the scope of the additional hitch evidence.

The earlier Suggestions were also taken. **S1** is addressed by distinguishing free propagation from forcing
and border damping, selecting the eight-tick bound and adding M6.10. Its measured 0.63% periodic-hitch error
belongs to the proof's single 256 × 0.5 m level and moving Gaussian source; it is not a GPU or coupled-hull
qualification. **S2** is addressed by normalizing query answers to a requested use tick, retaining the source
state tick and extrapolating a late use only from that use tick (DESIGN lines 245–255; M5.1–M5.2).
**S3** is addressed by shortening inspection procedures while keeping capture modes, cues and operating ranges;
DESIGN lines 599–603 retain the shared extent, native-size viewing and consecutive motion at 1x.

**Changed-item audit**

I read every current item (89 across ten milestones), compared the previous and current criteria, and checked
all 38 changed or added items named in the request. The table groups items only where the assessment is the same.
Each remains judgeable at its milestone, subject to the dependencies and the already-disclosed M10 readiness
check for the future governor and MetalFX milestones.

| Changed or added items | Assessment against the earlier review and the request |
|---|---|
| M2.4; M4.2 | MF4: state assertions now use timestamped committed endpoints and the declared lifecycle table on Metal and Chrome. |
| M3.2, M3.3, M3.5 | S3: near water, vista, shared lighting and motion inspections retain their cues and modes. The first-frame lighting inspection is no longer relied on to prove the later gather conversion. |
| M3.6 | MF1: first-frame paired measurement and investigation replace the numeric gate. |
| M4.3, M4.4 | S3: Chrome visual parity and temporal observation remain; browser callback intervals remain labelled separately from presentation. |
| M5.1, M5.2 | S2: use-time normalization and a single late extrapolation are explicit. Same-tick cascade accuracy stays separate from prediction error. |
| M5.5 | S3: rest, 5 and 15 m/s, trade wind and gale, chase and side views remain. |
| M6.5, M6.6 | S3: two speeds, turn, stop/restart, spreading, floater response and moving-window seams remain. |
| M6.8 | MF1: paired wake cost and investigation replace the numeric gate. |
| M6.10 (new) | MF4/S1: bounded multi-tick forcing, dropped ticks, resets, discard and capture are testable; the held-at-last-position substitute must fail. |
| M7.1 | MF5: transfer accounting permits recipients to rise and rejects independent decay. |
| M7.5, M7.6 | S3: wind range, two heights, cap persistence, pattern repeat, hull/stern appearance and the minute-long source-off trail remain. |
| M7.9 | MF1: paired foam cost and investigation replace the numeric gate. |
| M7.11 (new) | MF4: foam catch-up and lifecycle now have acceptance coverage. S4 above identifies the narrower scope of the design's preliminary proof. |
| M8.1, M8.2 | S3: hours, winds, view heights, two submerged depths and thin-crest lighting remain. |
| M8.3, M8.5 | MF2: whole reflected bodies, receiving-region coverage, source-only gathering and fallback have complementary visual and numerical proofs. |
| M8.4, M8.6 | S3: one cloud factor and labelled stand-in, plus Chrome's specified optics comparison, remain. |
| M8.7 | MF1/MF2: cost is a record; it now includes body-ray work under both calm and gale coverage. |
| M8.9 (new) | MF3: independently computed transport and exposure expectations reject the original composition errors. |
| M9.3, M9.4, M9.5 | S3: preset/hour/height range, 1–20 km vista and the complete calm-to-gale transition remain. |
| M9.6, M9.7, M9.8 | S3: TAAU, pan, fast craft, cut, rebuild and field overlays remain on their named targets. |
| M9.10 | MF1: complete gale cost and investigation replace the numeric gate. |
| M10.2, M10.7 | S3: MetalFX comparison and the combined gale scene remain, with chase and elevated views. |
| M10.5 | MF1: both Mac recipes, tick/repeat frames and showcase extent remain as measurements without numeric admission. |

**Scope and evidence**

I read the two planner tasks under `briefs` in [RECORD.json](../../plans/SEA/RECORD.json), whole and oldest
first, the approved project brief and SEA's module brief before judging the revision. The re-plan task asks
for the five findings to be resolved; it changes no human requirement. I then read the previous review,
the complete current DESIGN and PLAN, every current done-when item, and their substantive changes from the
reviewed base. I also read the project's review guidance and calibration examples, the SEA ownership and
seam rulings in ARCHITECTURE, the port and performance decisions, and relevant C02/C04/C07–C09/C12 contracts.
The producer checks described above cover the effect-state implementation and the atmosphere/forward handoff.
No reading was delegated. This was not a blind technique-selection study, and I did not inspect Broadsides.

The revised design preserves game-owned presets and body integration, GPU-owned queries and wakes, capability
limits, counted refusal, public controls and governor seams. It adds no unrequested inland, multiplayer or
parked spray work. Licensing and port timing remain disclosed assumptions. The acceptance sequence reaches a
first native frame and then Chrome before adding wakes, foam and full optics. Later reconstruction and closure
still wait for their producers; this review does not claim those future dependencies are already built.

I read the complete revision proof and its imported wake operator, then reran parts A–E unchanged with:

```sh
UV_CACHE_DIR=/private/tmp/sea-design-review-uv PYTHONDONTWRITEBYTECODE=1 uv run --no-project --with numpy python docs/evidence/SEA/design-proof/revision_proof.py
```

The [rerun output](design-review-2/revision-proof-rerun.txt) records the numerical results. It is a rerun of
planner evidence, not an independently implemented oracle. The review also checks the reflection geometry,
transport inverse and endpoint example directly against their producers and definitions. No SEA engine
scene exists yet, so no GPU, visual, Chrome or display-cadence qualification was attempted. The original
long-wave and wake simulations were not rerun as separate programs.

`./dev check-docs` passed; [the output](design-review-2/check-docs-revision.log) is retained. Only this review
and its evidence folder were changed, left uncommitted for the review script. DONE.json and all planner fields
remain untouched.

DESIGN OK

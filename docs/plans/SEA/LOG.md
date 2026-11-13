# SEA log

- 2026-11-13 Owner, through the lead, when asked whether SEA and CLOUDS planning could start now: "On 2 use the
  fable planner like normal but you can bring that across when it makes sense." The lead's reading, recorded as
  an assumption in the plan: planning starts now, and this plan decides when the Broadsides Rust work is brought
  across, inside the port decision.
- 2026-11-13 Planned from the lead's task and [the brief](BRIEF.md): a design, a plan and a done-when file with
  ten milestones. M1 needs nothing; M2 names `EFFECT-GRAPH:M2`; M3 waits on `EFFECT-GRAPH:M3`, with
  `ATMOSPHERE:M5` and `CREATOR:M1`; M7 names `CONTENT-LOOP:M1`; M9 waits on `APPLE-PRESENTATION:M3`; M10 carries
  a check that fails until `APPLE-PRESENTATION`'s M7 and M8 are Done, since they are not planned yet.
- 2026-11-13 Planner's decisions on the five questions the brief left open. Cost account: three 256² cascades
  below 20 m, at most 192 analytic components above, a field cache for the long waves, everything stepped on the
  tick and drawn as a blend of two ticks. Wake operator: eWave on two body-following levels with complementary
  bands, which is how the water decision's two levels avoid carrying a wave twice. Rigid bodies: integrated in
  the example scene; the engine owns the query and the wake source. Refraction source: opaque colour and depth
  gathered only inside the screen rectangles of bodies, on every recipe, with no whole-scene copy. Foam look:
  feathered caps, pale water over submerged air and a widening trail are inside S4; wind streaks are not.
- 2026-11-13 Planner's decisions the brief did not name. The sea draws after opaque geometry and writes scene
  depth, so reconstruction and a later cloud march see it. A body's wake source is the vertical force the water
  put on it, and its own static dent is added back in its query. The display blends two tick banks, written on
  alternating ticks, because a history pair does not offer its older side on a skipped frame. Chrome is proved in
  its own milestone (M4) and then in each later one. Packages: `thrum-sea-model`, `thrum-sea` and `sea-scene` in
  `examples/sea`.
- 2026-11-13 Planner's reproductions, in [the evidence folder](../../evidence/SEA/design-proof/). Wake: transverse
  wavelength within 2% of theory at 3 and 6 m/s, a crest that travels on at its phase speed after source-off, two
  banded levels within 2.2% of one fine level, and 69% error when both levels carry the whole source. Long waves:
  f32 with f64 phase wrap is within 0.05 mm of f64 at 4 km from an anchor 50 km out after 24 hours, against 18 mm
  unwrapped; the declared tolerance is 2 mm.
- 2026-11-13 Asks of other modules, settled as [seam rulings](../ARCHITECTURE.md#seam-rulings-for-sea): an
  after-opaque pass may write scene depth, an effect's own pass may read scene colour and depth as textures, and
  the shell, page and capture tap serve a second ticking game. All three are built by this module's builder. No other module's design, plan or done-when
  item changed, and no contract's text changed.
- 2026-11-13 Not read for the design: the Atlas GDC 2019 slides (over the fetch limit), Crest's source and
  Horvath 2015. Rabaud and Moisy 2013 was read as an abstract only.
- 2026-11-13 Design review: [changes requested](../../evidence/SEA/design-review.md), five Must Fix findings and
  three Suggestions. The design, the plan and the done-when file were revised the same day; the entries below
  say how. New reproductions are parts A to E of `revision_proof.py` in
  [the evidence folder](../../evidence/SEA/design-proof/).
- 2026-11-13 MF1, numeric gates. No timing figure decides an item or a milestone. `sea_cost.py check` fails only
  on a missing or incomplete record or an uninvestigated tripwire. M3.6, M6.8, M7.9, M8.7, M9.10 and M10.5 were
  written again as paired records with investigations; M3 no longer ends `BLOCKED` on its frame time; M10 was
  renamed, since its name claimed the refresh.
- 2026-11-13 MF2, reflections. The gather's source rectangles (what is copied) and each body's receiving
  rectangle (which sea pixels may show it) are now separate. The receiving rectangle bounds every mirror image
  and refracted view of the body's box within a reach of 64 m and a slope allowance of 20°. Part C: no ray
  outside it in 36 cases. M8.5 holds that bound; M8.3 asks for whole reflections. Only registered bodies
  reflect, which is a new assumption in the plan.
- 2026-11-13 MF3, units. The gather turns copied scene colour back into radiance with the exposure scale and
  `ATMOSPHERE`'s lent camera aerial lookup, and the design declares four approximations with their range. Nothing
  new is asked of `ATMOSPHERE`. New item M8.9 holds a panel's reflection and submerged view to the equation at
  two exposures and two distances.
- 2026-11-13 MF4, hitches. The two banks are the committed state and the state committed before it, each with its
  tick, blended by time across the real span; a full frame writes both at one tick; a capture frame reads and
  changes nothing. Wake and foam take one step of up to 8 ticks (part A: 0.16% of RMS for one 8-tick step, 0.6%
  with one a second). M2.4 and M4.2 were written again to the endpoint table; M6.10 and M7.11 are new lifecycle
  items.
- 2026-11-13 MF5, foam. M7.1 was written again as an account of transfers and sinks: recipients rise with no
  birth, the inventory never rises, every stock ends at nothing (part D).
- 2026-11-13 Suggestions, all three taken. S1: the wake's source and border are stated as approximations, and
  the catch-up bound of 8 ticks is set from part A. S2: a query answer is valid at its use tick, and carries that
  tick and its state tick (design, M5.1, M5.2, the module list's body-query row). S3: inspections name the mode,
  the views and the range, and leave the route to the reviewer; the design's capture rule says so once.

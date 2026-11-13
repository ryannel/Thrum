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

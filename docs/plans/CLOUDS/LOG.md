# CLOUDS log

- 2026-11-13 Owner, through the lead, when asked whether SEA and CLOUDS planning could start now: "On 2 use the
  fable planner like normal but you can bring that across when it makes sense." The lead's reading, recorded as
  an assumption in the plan: planning starts now, and this plan decides when the Broadsides Rust work is brought
  across, inside the port decision.
- 2026-11-13 Planned from the lead's task and [the brief](BRIEF.md): a design, a plan and a done-when file with
  twelve milestones. M1 needs nothing; M2 names `EFFECT-GRAPH:M2` and `ATMOSPHERE:M5`; M3 waits on
  `EFFECT-GRAPH:M3`; M6 names `CONTENT-LOOP:M1`; M9 waits on `APPLE-PRESENTATION:M3` and names `CREATOR:M1`; M10
  carries a check that fails until the owner's licence for the library is in this log; M11 waits on `SEA:M8`;
  M12 carries a check that fails until `APPLE-PRESENTATION`'s M7 and M8 are Done, since they are not planned yet.
- 2026-11-13 Planner's decisions on the four questions the brief left open. Cost account: 2D weather fields with
  a vertical profile, light from one summed Sun-path product plus a few local samples, a layer traced at one ray
  for sixteen render pixels and reconstructed over four frames; 17 to 42 texture reads a sky ray against
  Broadsides' 142 density evaluations. One march for both paths: a placed volume stands for one cell of the
  weather field, the weather map names its slot, and the two forms' weights sum to one. Sun direction: the
  environment identity's true Sun, which `ATMOSPHERE` M5 already lends. Opt-in looks: none is carried over by
  name; filtering density by the ray's footprint is kept as a mechanism, and the four-band base look is left.
- 2026-11-13 Planner's decisions the brief did not name. Shade comes first and alone (M2), so shadow-only is the
  first thing proved. Clouds light the world through `ATMOSPHERE`: an overlay in its reflection and diffuse
  products and one scale on its haze. The cloud layer ends at render extent, read as what the port decision's
  "no private upscaler" forbids. Below the lowest valid Sun elevation the ground takes a mean far optical depth,
  so an overcast sunset stays dark. Placed volumes are proved with stand-in shapes (M6) and the accepted library
  arrives late (M10), so the licence holds back as little as it can. Packages: `thrum-cloud-model` and
  `thrum-clouds`; the example is a selection in `examples/v1-game` and `examples/sea`.
- 2026-11-13 Planner's reproduction, in [the evidence folder](../../evidence/CLOUDS/design-proof/). Weather
  field: coverage takes cloudy columns from 0 to 100% with no step; column optical depth changes by at most 0.4
  a second of 41; births are Poisson in time, and a half-period birth jitter showed a peak of 4 at the
  generation period that a full-period jitter removed. Sun-path product at 80 m columns: ground shade within
  0.02 to 0.05 RMS in transmittance at 60° and 20° and 0.09 at 6°; inside cloud 0.06 to 0.08 alone and 0.02
  with three local samples over 120 m.
- 2026-11-13 Asks of other modules, settled as
  [seam rulings](../ARCHITECTURE.md#seam-rulings-for-clouds): `ATMOSPHERE` takes a GPU-written Sun-path field
  with a far value, lends aerial transport beyond 32 km, composes a sky overlay into its reflection and diffuse
  products and scales its solar in-scatter by one number; `examples/v1-game` and `examples/sea` gain a cloud
  selection that is off by default. All are built by this module's builder. No other module's design, plan or
  done-when item changed, and no contract's text changed.
- 2026-11-13 Not read for the design: the Red Dead Redemption 2 course notes (Bauer 2019), the 2015 and 2017
  Nubis talks, and the Broadsides cloud shaders. The Nubis decks were read as slide text; they carry no speaker
  notes.

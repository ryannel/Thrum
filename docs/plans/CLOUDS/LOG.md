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
- 2026-11-14 [Design review](../../evidence/CLOUDS/design-review.md): changes requested, four Must Fix findings
  and three suggestions. The lead's task: "Resolve its 4 Must Fix findings in the design and the plan. Its
  Suggestions are yours to take or leave; say which you took and why." The design, the plan, the done-when file
  and seam ruling 1 were revised. Reproduction:
  [revision_proof.py](../../evidence/CLOUDS/design-proof/revision_proof.py) with its output beside it.
- 2026-11-14 MF1, numeric gates. No timing figure decides an item, a milestone or the next one's start (brief
  decision 3). The ten cost items (M2.9, M3.9, M5.7, M6.8, M7.8, M8.8, M9.8, M10.7, M11.6, M12.5) were written
  again as paired records whose tripped wires each carry an investigation; `cloud_cost.py check` fails only on a
  scene that cannot be measured, an incomplete record or a tripped wire with no investigation. The stop in M3's
  note and the re-plan rule in the design are gone, and M12 was renamed.
- 2026-11-14 MF2, placed volumes. A placed cell's column in the field is its shape's own footprint, derived
  from the voxels by the model and given to the field like the type table; the dome of equal mean is gone. The
  volume form and a column form built from the same footprint are each bounded by the query at every point
  (proof part B). The design says which quantities are bounds and which averages. M1.6 was written again, M1.7
  is new (M1's later items moved down one), and M6.2, M6.3 and M7.6 were written again; GPU columns are held to
  the query at texel centres within a bound measured first.
- 2026-11-14 MF3, low Sun and the field's edge. The Sun-path product's columns follow the true Sun with their
  elevation floored at the profile's lowest (2 degrees), so the product is filled at every Sun elevation and
  nothing switches. Outside the field a receiver fades across a border to a far value, the mean transmittance
  of the outermost level's ground slice (proof part A). Ruling 1 carries the path direction, the border and
  the far value. M2.3, M2.5 and M5.5 were written again, and M2.4 is new (M2's later items moved down one), on a
  broken sky.
- 2026-11-14 MF4, shadow-only. The Sun's path and the overlay integrate a coarse density, the one function with
  the detail noise at its mean, in both profiles; the view march and its local light samples use the full
  density (proof part C: 0.001 RMS in transmittance over an 80 m column). M3.8 was written again on a sky the
  detail noise erodes, and records the difference between the two densities' Sun paths.
- 2026-11-14 Suggestions, all three taken. S1: the march reads a stacked volume no coarser than one texel a
  slab and holds its coordinate inside the slab (proof part D); M6.1 tests contrasting slabs at every level
  read; nothing is asked of the cooker. S2: cadences bound age in weather time, each part of the map is held to
  the model at the tick it was written (M2.2, M4.2), and a knob changes no query answer and no column the map
  holds (M9.7, M12.3). S3: M7.5's recordings run as long as whole lives need.
- 2026-11-14 Owner, through the lead: "You can fully use everything in broadsides. But we should make sure that it
  aligns with the approach we want. But yes, you can copy that code out and reuse as is if it's useful." Then: "We
  can fully reuse anything from broadsides, but it's there as a resource where it's useful, it shouldn't dictate
  design."
- 2026-11-14 Owner, through the lead, on clouds: "So in broadsides you will find both a procedural cloud setup as
  well as pre-baked clouds. The idea being that we could support both those two sky types as well as a hybrid as
  needed by the game being built." Then: "That's the approach we were going with in broadsides that might not be
  the right approach here. The agent should look at that work from first principles and figure out how to design
  the best possible atmospheric system for the thrum game engine. Maybe that's a flexible set of cloud
  capabilities built from what we did in broadsides, maybe it isn't, but it should start from first principles."
- 2026-11-14 Lead's reading, recorded as assumptions in the plan: the owner's last words govern; the design of
  13 November, the brief and the port decision are inputs to judge; Broadsides code, shaders and assets may be
  copied as is where the design finds them useful; Broadsides stays unmodified.
- 2026-11-14 Design judged again from first principles; the verdict on each inherited choice is in
  [the design](DESIGN.md#judged-from-first-principles). Kept, for reasons that do not rest on Broadsides: the
  weather field with its CPU query, the air frame, 2D fields with a profile, the Sun-path product, a placed
  volume as a cell of the field, light and shade through `ATMOSPHERE`, the layer ending at render extent, and all
  seven cues. The four Must Fix resolutions of the review stand.
- 2026-11-14 Changed: the two Broadsides paths and their hybrid are one medium whose sources are data, and
  shade, the visible sky, the world's light and shapes are capabilities a scene leaves out at no cost (new item
  M8.9; "shadow-only" is Weather and Shade). K5 is restated as what the engine does.
- 2026-11-14 Changed: the first scattering order takes the droplets' phase, Henyey-Greenstein plus Draine, as
  type data (M1.8, M5.1 and M5.3 written again; M5.8 is new). Part E of
  [first_principles_proof.py](../../evidence/CLOUDS/design-proof/first_principles_proof.py): the reference lobe
  is 3.9 times too bright at 5 degrees from the Sun and 2 to 3.4 times too dim looking away from it.
- 2026-11-14 Changed: drift is shown at the display's time and evolution stays on the tick (M1.1, M2.7, M3.4 and
  M7.4 written again; M2.11 is new). Part F: at 60x in a 40 m/s wind a near cloud would step 0 then 37 pixels.
- 2026-11-14 Changed: a game can place shapes itself, as directed shapes in the weather state, in a profile that
  places by the game and not by the hash (new items M1.12, M6.9 and M6.10), as an assumption.
- 2026-11-14 Changed: Broadsides may be copied where it does what the design states. The licence assumption was
  deleted, answered by the owner's words above; third-party material keeping its own licence is an assumption.
  The accepted library is the example's content from M6 (M6.6 written again from the old M10.2; the stand-ins
  are gone), M10.1's licence check is deleted, and M10 now judges the twelve shapes in the finished sky. M1.11
  was written again. The design names where Broadsides has each mechanism.
- 2026-11-14 Found while planning: the growth track's eleven key volumes are not in Broadsides' Git. They are in
  that folder's `tmp/evidence-store/cloud-route-2026-11-08/m2-authored-growth-v2/source-keys-replay.tar.gz`
  (SHA-256 `e24dcd2306836ac0843eed25341807403356d2bb3f1fb385c0f003ec37a63795`, verified), and
  `coherent_growth.py` there makes them again. M6's note says so.
- 2026-11-14 [The port decision](../../design/decisions/SEA-AND-CLOUD-PORT.md)'s cloud half now summarises this
  design in place of the 6 November profile. The module list's row, its licence line and its consolidation
  paragraph were brought into step. No seam ruling, contract, exact baseline or other module's item changed.
- 2026-11-14 Water was re-planned as one system: the sea's surface, optics and example now belong to `WATERS`, and
  `SEA` keeps only the wave field. M11 waits on `WATERS:M8` in place of `SEA:M8`, and its example is
  `examples/water` (package `water-scene`, `scripts/check_water_browser.mjs`). Nothing else of this module changed.
- 2026-11-14 Second design review: changes requested, two Must Fix
  ([review](../../evidence/CLOUDS/design-review.md)). A footprint cannot give a placed shape's shade under a
  slanting Sun, yet Shade without Sky was given footprints alone; and the Sun-disc spread on the first order's
  lobe removed 2% to 23% of its energy from 10 to 50 µm. The four earlier findings were found resolved.
- 2026-11-14 Resolved, a shape's shade: the Sun's path integrates the forms that are drawn, so Shade holds the
  leased volumes wherever Shapes is declared, with Sky or without. Headless still needs footprints only, and a
  cell in its column form is shaded through it in every profile. M6.2 and M8.9 written again; M1.15 and M6.12
  are new. M6.2's Sun-path bound is M6's own measured one, since a hard edge sits within a column's width.
  New assumption: a shadow-only scene with shapes keeps its library loaded.
- 2026-11-14 Found in [part H](../../evidence/CLOUDS/design-proof/second_revision_proof.py): the Sun's path
  must read a slab at its finest level. An average to the 80 m column spreads an opaque shape's shade, 0.11 to
  0.18 RMS against 0.01 to 0.05. Not chosen: a path from the column form in both profiles (0.04 to 0.14 RMS
  from the volume's shade), and a small always-resident coarse copy of each shape, which is that average.
- 2026-11-14 Resolved, the Sun's disc: the first order's phase is averaged over the disc in the model, by
  quadrature, and read from a table. Part G: it integrates to 1.000000 from 5 to 50 µm, and its mean cosine is
  the point Sun's times (1 + cos ρ)/2. Not chosen: the widened lobe scaled back to unit energy, a lobe of lower
  g that is up to 70% too bright 1° to 2° from the Sun. M1.8 keeps the point-Sun checks, M1.14 is new for the
  rendered phase, M5.1 is written again, and Broadsides' `spread` rule is not copied.
- 2026-11-14 Suggestion S1 taken: M5.1's local-reach check is scoped to a fixture with no detail erosion, where
  the full and the coarse density agree. It cost one clause and removed a check that could not pass as worded.
- 2026-11-14 Found while resolving: within 2° of the Sun, part E of `first_principles_proof.py` used the widened
  lobe. Part G gives those figures again: the reference lobe is 60 times too dim at half a degree, not 46. The
  supplemental of Jendersie and d'Eon was read, sections 2 and 3.1. Plan notes were shortened to hold the plan
  under its word limit; no milestone's scope changed beyond the items named above.

- 2026-11-14 M1 builder implemented the renderer-free CPU weather authority, directed footprints and volume
  paths, density/conformance references, and point/disc optics. [Evidence](../../evidence/CLOUDS/M1.md) and
  [phase measurements](../../evidence/CLOUDS/M1-phase.md) distinguish the numerical proofs from later visuals.
  All 14 item checks and the full project gate passed before integration; the detailed counts and revisions
  are in the evidence. Provenance was inspected with no Broadsides file copied. Integration rebased onto
  `145cadb2` and correctly requested a replacement baseline proof after two covered upstream scripts changed.

- 2026-11-14 M1 integration refresh on `3cb38017` passed all 14 automated item checks. The exact verifier
  reported `PASS exact baselines` in 919.17 seconds: 38 and 4 native comparisons and 481 records in each
  Chrome set passed. The replacement covered-content key is
  `35ee716d5af06cc0e5cdcbaac1a9fe42ad061651dcdf20136fea84744a3a2fcc`; complete check outputs are retained
  under shared Git check key `fe466581b59a5ce1514b20dc`.

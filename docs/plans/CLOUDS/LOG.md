# CLOUDS log

- 2026-11-23 M6 cooks the accepted twelve library volumes and eleven growth keys into
  two leased stacks, with model-derived footprints and a continuous column fallback.
  The example uses the same world state and density for the visible cloud and its
  Sun path. Metal and Chrome WebGPU checks cover the published products, drawn cloud,
  headless queries and shade-only profile. The paired 1440p record includes the
  refresh tripwires and the resource bytes for both Metal device profiles.
  [Builder evidence](../../../.orchestrate/evidence/CLOUDS/M6/README.md)

- 2026-11-15 Resumed M13 at `2b14b2b00` with all ten builder items passed.
  Stopped inherited job `20261115-205432-68753`, still waiting for the GPU, because
  its old integration helper holds a device lock across merges. Current integration
  uses the canonical merge directly; M13.10 cites the shared main-branch exact-images
  check, retaining the earlier target receipt as history. The two numerical fixes
  and their CPU-only scope remain documented in [the evidence](../../../.orchestrate/evidence/CLOUDS/M13.md).

- 2026-11-15 M13's rebase at `f0e05146` changes no cloud-model implementation or
  test. All ten item commands passed again, including 38/4 native comparisons and
  four Chrome sets of 481 records in 934.62 seconds. The fresh covered-content key is
  `5d0a42e74391bc3cf13989e264c11fd4d572afd3ec66347369ec95d79971ac7b`.
  Canonical integration then found unpublished APPLE-PRESENTATION M3 evidence links.
  Upstream now carries that repair. Its wording is kept during integration.

- 2026-11-15 M13 adds surface/datum and fog references, radiative cooling, visibility,
  Sun paths, medium coefficients and haze to the renderer-free model. All nine CPU
  and dependency commands passed on `db3d8514` on mac-metal. The saturated-overcast
  regression and exact shore-margin probes pass. [Evidence](../../../.orchestrate/evidence/CLOUDS/M13.md)
  and its receipt separate CPU correctness from visuals and performance, neither qualified.
  M13.10 passed at `4e8e9f27`: 38/4 native comparisons and 481 records in each of four
  Chrome sets, in 936.36 seconds. Its receipt retains the target artifact identities.
  All ten builder items have target evidence; canonical integration remains queued.

- 2026-11-15 The full milestone check passed all 15 automated commands on `e866a1d7`,
  including `./dev check`. M1.13 passed on `7e41bac7`: 38/4 native comparisons and 481
  records in each of four Chrome sets, in 960.25 seconds. Every builder item has target
  evidence. [Verification identities and diagnostics](../../../.orchestrate/evidence/CLOUDS/M1-final-verification.json)
  retain the binary, Wasm, cooked-root and check-log identities. No cloud visual or speed
  qualification follows from this CPU milestone.
- 2026-11-15 Canonical integration found duplicate handoff-link repairs; rebasing onto
  `0f8cdd18` kept the upstream CREATOR and CULLING versions. Cloud-model sources, optical
  references, M1 planner fields and the covered-content key are unchanged, and the exact
  baseline gate passed after rebase. The project gate must pass on the combined tree and
  canonical merge must report `MERGED` before completion is claimed.
- 2026-11-14 Earlier unchanged capture attempts failed separately: Chrome night linear
  differed only in tick/alpha identity (all 481 decoded frame hashes matched), then native
  route capture reported 90 readback-ring omissions. Neither attempt counts as a pass.
  The retained verification record distinguishes the measured facts from the browser
  work-slice inference and names the owned failed scratch and its release condition.

- 2026-11-14 M1 resumed against the planner's revised clear-column convention. Source
  `f188edfc` retains the weather's type/base and sets clear top=base; `0b097fab` explicitly
  checks every coverage-zero column. Temporal and spatial tests measure top, thickness,
  coverage and optical depth, include different type bases and clear/cloud transitions,
  and reject a switched cell in each quantity. The unchanged reviewer probe's top maxima
  are now 25.799/14.199/7.615 m at 4/2/1 seconds; [current evidence](../../../.orchestrate/evidence/CLOUDS/M1.md)
  retains its output. The older contradiction and blocked handoff below are historical.
- 2026-11-14 All 13 CPU/dependency item commands passed on `cb8f2aeb`, with full outputs
  under shared Git check key `212e9f1f636916305ff19efa`. Builder inspection found the ask
  proofs met as described in the current evidence. `./dev check-docs` passed after fixing
  broken links in two upstream handoffs to explicitly name their unmerged evidence paths.
  The initial GPU-queued check was cancelled before any capture so evidence could be
  finalised; replacement job `20261114-220907-79067` continues the baseline verification.
  No new baseline or full project-gate pass is claimed by this entry.

- 2026-11-14 M1 review follow-up: implemented bounded shape thresholding and edge-only detail
  erosion; revision evidence (`../../evidence/CLOUDS/M1-revision.md` on branch `build/clouds`) records CPU tests, Clippy
  and 252 fully removed edge samples among 891 positive uneroded probes. No visual claim.
  M1.4 remains open: the actual-top jumps reproduce and cannot converge to zero with both
  the designed fixed condensation base and M1.2's all-zero clear sentinel.
- 2026-11-14 Proposal for the planner: define top only where a column is cloudy, retaining
  clear zeros, and require thickness continuity through clear transitions; separately test
  top where defined, including a changing type-table base. This is a proposal, not an adopted
  interpretation or a change to the items. [Handoff](../../../.orchestrate/evidence/CLOUDS/M1-handoff.md).

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
- 2026-11-13 Planner's reproduction, in [the evidence folder](../../../.orchestrate/evidence/CLOUDS/design-proof/). Weather
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
- 2026-11-14 [Design review](../../../.orchestrate/evidence/CLOUDS/design-review.md): changes requested, four Must Fix findings
  and three suggestions. The lead's task: "Resolve its 4 Must Fix findings in the design and the plan. Its
  Suggestions are yours to take or leave; say which you took and why." The design, the plan, the done-when file
  and seam ruling 1 were revised. Reproduction:
  [revision_proof.py](../../../.orchestrate/evidence/CLOUDS/design-proof/revision_proof.py) with its output beside it.
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
  [first_principles_proof.py](../../../.orchestrate/evidence/CLOUDS/design-proof/first_principles_proof.py): the reference lobe
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
  ([review](../../../.orchestrate/evidence/CLOUDS/design-review.md)). A footprint cannot give a placed shape's shade under a
  slanting Sun, yet Shade without Sky was given footprints alone; and the Sun-disc spread on the first order's
  lobe removed 2% to 23% of its energy from 10 to 50 µm. The four earlier findings were found resolved.
- 2026-11-14 Resolved, a shape's shade: the Sun's path integrates the forms that are drawn, so Shade holds the
  leased volumes wherever Shapes is declared, with Sky or without. Headless still needs footprints only, and a
  cell in its column form is shaded through it in every profile. M6.2 and M8.9 written again; M1.15 and M6.12
  are new. M6.2's Sun-path bound is M6's own measured one, since a hard edge sits within a column's width.
  New assumption: a shadow-only scene with shapes keeps its library loaded.
- 2026-11-14 Found in [part H](../../../.orchestrate/evidence/CLOUDS/design-proof/second_revision_proof.py): the Sun's path
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
- 2026-11-14 Owner, through the lead: "Oh and for clouds we also need to be taking into account mist / fog. We
  want a plan that's able to deliver on top quality atmosphere." The lead's task: revise the plans so the
  atmosphere reaches shipped quality as a whole (sky, clouds, haze, mist and fog: ground fog, valley mist, fog
  banks over water, low cloud the camera can enter), lit consistently and changing with the weather and time of
  day, with one authority for each kind of state.
- 2026-11-14 Ownership settled from first principles, in
  [the module list](../ARCHITECTURE.md#the-atmosphere-as-a-whole). Fog is cloud at the ground, so the weather
  field owns every condensate and the amount of haze; `ATMOSPHERE` owns the air's optics and a view volume that
  integrates air, haze and this module's medium coefficients jointly near the camera; this module's trace
  carries condensate beyond a handover 300 to 500 m out. Not chosen: a separate fog provider below the cloud
  base, a second fog grid, an analytic height fog, and fog marched per pixel from the camera.
- 2026-11-14 Replaced, listed in [the design](DESIGN.md#judged-again-for-the-whole-atmosphere): fog as a later
  provider split from cloud by height; the camera and all geometry below the cloud base; a trace that ignores
  depth; haze under cloud as one number with no shafts; the Sun's path ending at the cloud base; and the reading
  that `ATMOSPHERE` is not reopened. The plan's assumption on the camera and geometry was rewritten.
- 2026-11-14 Added: cues K8 to K11; the surface field, the fog table, pooling, held and drifting rows, the
  ground cooling that makes mist follow the day, haze as a visual range, the point extinction and visual-range
  query, the Sun path's ground band with a surface visibility, medium coefficients, the handover, the interval a
  ray marches and its two stops, and the Ground capability. Milestones M13 (after M1), M14 to M16 (after M10)
  and items M11.8 to M11.10; M11 renamed. M8.2 now names a scene without the view volume. Rulings 6 to 8 are
  `ATMOSPHERE`'s own M7 and M8.
- 2026-11-14 M1's items are unchanged, and it builds what it was building. Its note gained one sentence: ground
  condensate is M13's. M3's note asks that the trace be written over the general interval.
- 2026-11-14 Reproduction: [whole_atmosphere_proof.py](../../../.orchestrate/evidence/CLOUDS/design-proof/whole_atmosphere_proof.py).
  The handover composes to the exact transmittance and to the in-scatter within 0.002; pooled mist covers 19%
  to 89% of a 200 m relief with no step; one integrated value forms mist by 22 h and clears it by 8 h, with none
  under overcast or wind; a ground band of 80 m columns and 9 heights is within 0.011 RMS in transmittance where
  a level-layer rule is off by up to 0.31; today's fixed haze is a 244 km visual range.
- 2026-11-14 Read for this: Hillaire 2015 (slide text), Unreal's volumetric fog documentation (whole), and the
  abstracts of Bauer 2019 and Kovalovs 2020. Not read: Bauer's deck (232 MB), Wronski 2014, the Nubis³ pages on
  low fog-like cloud beyond one quoted line. The plan's notes were shortened to hold it under its word limit.
- 2026-11-14 Found while resolving: within 2° of the Sun, part E of `first_principles_proof.py` used the widened
  lobe. Part G gives those figures again: the reference lobe is 60 times too dim at half a degree, not 46. The
  supplemental of Jendersie and d'Eon was read, sections 2 and 3.1. Plan notes were shortened to hold the plan
  under its word limit; no milestone's scope changed beyond the items named above.

- 2026-11-14 M1 builder implemented the renderer-free CPU weather authority, directed footprints and volume
  paths, density/conformance references, and point/disc optics. [Evidence](../../../.orchestrate/evidence/CLOUDS/M1.md) and
  [phase measurements](../../../.orchestrate/evidence/CLOUDS/M1-phase.md) distinguish the numerical proofs from later visuals.
  All 14 item checks and the full project gate passed before integration; the detailed counts and revisions
  are in the evidence. Provenance was inspected with no Broadsides file copied. Integration rebased onto
  `145cadb2` and correctly requested a replacement baseline proof after two covered upstream scripts changed.

- 2026-11-14 M1 integration refresh on `3cb38017` passed all 14 automated item checks. The exact verifier
  reported `PASS exact baselines` in 919.17 seconds: 38 and 4 native comparisons and 481 records in each
  Chrome set passed. The replacement covered-content key is
  `35ee716d5af06cc0e5cdcbaac1a9fe42ad061651dcdf20136fea84744a3a2fcc`; complete check outputs are retained
  under shared Git check key `fe466581b59a5ce1514b20dc`.

- 2026-11-14 M1 resumed after the clean-checkout verifier failed its 16 GiB disk preflight before
  `apple_tile_m2`, rather than an image comparison. The failed record on `bb86a1be` had passed all
  38 comparisons in `apple_tile_m1`. This session found only 15 GiB available and no running owned
  background job or open handle in this worktree's build output. Cleaning this worktree's reconstructible
  development build with `cargo clean --profile dev` removed 5.6 GiB (Cargo's reported logical size),
  restoring 20 GiB available. No references, bounds or capture preflights were changed. Verification
  below records the rerun; the earlier failure alone establishes no new baseline pass.

- 2026-11-14 The resumed item check passed all 14 automated commands. Exact baselines on `41362d03`
  passed in 914.51 seconds: 38 and 4 native comparisons, and 481 records in each of four Chrome sets.
  The covered-content key
  `35ee716d5af06cc0e5cdcbaac1a9fe42ad061651dcdf20136fea84744a3a2fcc` also matched `6ce3247b`.
  All 13 CPU/dependency commands passed there, under shared Git check key `3d502e1cc2edc60446e8eac2`.
  `./dev check-docs` passed after fixing two handoff links; the equivalent upstream fix is now kept.
  No retained baseline, sampling rule, numerical bound or M1 criterion changed.

- 2026-11-14 Rebased onto SEA M2 and dropped the duplicate private handoff-link fix. Covered inputs at
  `f83c2549` hash to `df6dcdc5e3fca468d0c8a0ae2ea9ad4cfff52c2c3fde8084b2aa2bd86e31293e`, matching
  [SEA's retained verifier record](../../../.orchestrate/evidence/SEA/M2-baselines.json): Mac and Chrome passed in
  965.46 seconds, with 38 and 4 native comparisons and 481 records in each of four Chrome sets.
  All 13 CPU/dependency commands were refreshed there, under shared Git check key `d82f679aa755f3194b973a13`.
  Canonical integration keeps GPU exclusivity and removes owned development builds before unlocking.

- 2026-11-14 `ATMOSPHERE`'s design review found this design's producer and its ruling 7 at odds: the ground
  band is filled with the whole path to the Sun, the cloud band's value included, while ruling 7 and M14.3
  had the consumer add the cloud band again, which squares an upper cloud's factor. Settled with `ATMOSPHERE`
  and the module list: the band holds the whole path and a receiver makes one read. Ruling 7's summary, the
  Sun's-path section's first paragraph and M14.3's second sentence now say so; the fill is unchanged. The
  handover's in-scatter figure is qualified by that review's bank at the ramp. No other item changed.

- 2026-11-14 [The design review of the ground revision](../../../.orchestrate/evidence/CLOUDS/design-review.md) asked for
  changes, four Must Fix. Re-planned; M1 is untouched. (1) The trace keeps a stop at every pixel's depth in
  place of two stops, and a carried stop is used only on its own surface; M15.1 and M15.7 rewritten. (2) A
  ground row's world factors (height over the surface, datum, shore) stay with the world and its air factors
  (extent, noise) ride the wind; the weather map holds only the latter; M13.3, M14.2 and M11.8 rewritten.
  (3) The headless point query is the coarse density, exact except through a placed shape known only by its
  footprint, where it is the column form and says so; M13.5 rewritten. Continued below.
- 2026-11-14 The same re-plan, continued. (4) One ceiling, `ATMOSPHERE`'s form, decides M13.6, M13.7, M14.2
  to M14.4 and M15.1 to M15.3, each with wrong substitutes that must fail; no item sets its own bound. Measuring
  it showed the far segment's single front distance 2.6 times outside the ceiling in 8 km haze, so the trace
  now takes the lent air in parts (a quarter of the ceiling); M3.6's form holds for a ray through one cloud.
  Suggestions taken: Chrome motion in M14.9, M15.9, M16.6 and M11.10, mist under MetalFX in M12.2, M11.1's
  owner, M11.2's water exclusion, and depth against density at amount zero. The figures are parts VII to XI of
  [the design proof](../../../.orchestrate/evidence/CLOUDS/design-proof/README.md).
- 2026-11-14 M1 was Blocked on two of its items: M1.2 made every answer of a clear column zero and M1.4 had
  top never step, so top jumped by the base's height at each clear/cloud transition
  ([review](../../../.orchestrate/evidence/CLOUDS/M1-review.md), [handoff](../../../.orchestrate/evidence/CLOUDS/M1-handoff.md)). Settled:
  type and base are the weather's in cloud and in clear air; a clear column has no thickness, its top exactly
  at its base, and its coverage, age and optical depth are exactly zero. The builder's proposal, top defined
  only in cloud with zero kept for clear air, was not taken: the weather map filters base and top, so a zero
  beside a cloud hangs cloud below the shared base; zero is a legal height for a deck on the ground; and M1.4
  would have had to stop naming top. Continued below.
- 2026-11-14 The same re-plan, continued. M1.2 and M1.4 are rewritten to agree. M1.4 names top and thickness,
  a type ramp between rows of different base, a point that turns from clear to cloudy, and a switched-on cell
  that must fail; it asks that halving the step shrink the largest change by a stated factor, not halve it.
  M1.12 says clear where it said zero. The design now says a placed shape's extents are its geometry and step
  at its silhouette, which M1.4 leaves out by name. The M1.6 fix on `build/clouds` (`08184d54`) stands. No
  other item, seam or bound changed; `ATMOSPHERE`, `WATERS` and the rest read none of these answers.
- 2026-11-14 M12 now waits for `APPLE-PRESENTATION:M7` and `M8`, the governor and MetalFX, which have done-when
  entries from today. M12.1 is unchanged. The design is unchanged.
- 2026-11-15 M13 review fixes on `2ea3a62be` give saturated radiative fog a finite cooling threshold
  and preserve footprint-only visibility's column sum without changing the older `Field` density calls.
  The full CPU crate suite passes. Three-day tests cover four moisture levels, and footprint integrals
  cover erosion 0/0.4/1, three scales, three rotations and partial envelopes. The unchanged reviewer probe
  now returns 1.5 at erosion 0.4 and exactly zero saturated amounts from 11:00 through afternoon.
  [Fresh proof and scope](../../../.orchestrate/evidence/CLOUDS/M13.md) retain the probe output; baseline and merge checks follow.
- 2026-11-16 M13's second-review fix on `0df0b5f26` confines footprint-sum preservation to visibility
  queries. Sun paths, physical density and both medium entry points retain column-form erosion.
  M13.1–9 CPU/dependency checks pass on mac-metal; new regressions cover eroded footprint-only and
  supplied-volume Sun rays at four elevations, partial envelopes, ground overlap and medium agreement.
  [The receipt](../../../.orchestrate/evidence/CLOUDS/M13-sun-path-fix.json) records source and output; existing visibility
  integral regressions still pass. No GPU speed, rendered fog or display qualification is claimed.
- 2026-11-16 Resumed M13 at `df12d883c`; all ten builder proofs remain recorded.
  The milestone helper confirms M13.1–9 on mac-metal. M13.10 returns the expected
  `NOT_ON_MAIN`, pending the shared exact-images run after merge. Its 600-second
  workspace gate times out during GPU tests without a reported test failure;
  canonical integration runs the gate with its merge timeout. [CPU evidence](../../../.orchestrate/evidence/CLOUDS/M13.md)
  includes the latest Sun-path and medium regressions; no visual claim is added.

- 2026-11-16 M2 resumes Weather/Shade with publication-owned weather, integer map axes and
  GPU-written atmosphere shade; CPU diagnostic and absent providers retain their paths.
  Mac Metal item probes, default-limit conformance, submitted display-time mapping and
  prescribed GPU-shade integration pass. [Bounds](../../../.orchestrate/evidence/CLOUDS/M2-conformance.md)
  retain the large coarse-grid/low-Sun errors and age's zero-contribution discontinuity.
  The workspace command passes; canonical integration proves its exact rebased source.
  [Cost](../../../.orchestrate/evidence/CLOUDS/M2-cost.json) records refresh and memory excess with investigations;
  [valley inspection](../../../.orchestrate/evidence/CLOUDS/M2.md) follows on the merged build.
- 2026-11-16 Planner: M2's block answered, on the lead's task ("settle and assign the shared display-time
  interface change before M2.7's uneven 60× shade motion can be completed"). The interpolation alpha stops at the
  committed tick, so it is not the display's time under a lead. `EFFECT-GRAPH` gains M4, the display phase, and
  M2 depends on it. Drift is taken at the phase, predicted past the tick from committed state. M2.7's inspection
  reads the time record and M2.11 covers both sides of a tick; M2's proof is reset.
  [The reading](../../../.orchestrate/evidence/EFFECT-GRAPH/M4-plan/README.md). M1's and M13's notes left the plan.
- 2026-11-16 Planner: M2's drift revised after the design review's one Must Fix (MF1), with `EFFECT-GRAPH`.
  One tick's travel carried across a prediction of several ticks stepped 6 m during a wind ramp with nothing
  changed. Travel past the committed tick is now carried a tick at a time by the step's own arithmetic, exact
  while the targets stand; a changed target corrects by the change. The phase describes the consumed target.
  M2.11 rewritten, M2.13 added for the model, M2.7 reads lead status.
  [Check](../../../.orchestrate/evidence/CLOUDS/M2-plan/display-travel.txt). S1 to S3 taken; S4 left, since M1 is Done.
- 2026-11-17 Planner, with `EFFECT-GRAPH` M4's re-plan: the design said display time is the consumed target less
  one tick, which is true only until the world's clock first stops. Three places now say it is the world's time
  ([the timing model](../EFFECT-GRAPH/DESIGN.md#the-timing-model)). No item and no mechanism changed: this module
  reads the tick and the phase as before.
  [Valley inspection](../../../.orchestrate/evidence/CLOUDS/M2.md) records the subsequent merged build.
- 2026-11-16 M2 source `5ec67bed0` merged with the workspace gate passing; fixed-still
  completion bookkeeping is corrected. M2.6's four views and receiver passage pass builder
  and fresh-agent inspection. M2.7's complete 60× sequence visibly steps unevenly.
  Submitted alpha alternates near .92 and 1, requiring 36.8/3.2 m wind steps at ~8 ms.
  [Proposal](../../../.orchestrate/evidence/CLOUDS/M2-temporal-blocker.md): the planner should assign a shared
  display-time policy to HOST/EFFECT-GRAPH that preserves unclipped target information;
  CLOUDS cannot recover it from the existing alpha. Other possible pop causes remain unisolated.

- 2026-11-17 M2 consumes EFFECT-GRAPH's display phase and predicts travel by the model's own
  tick arithmetic; ramps, target changes, cuts and repeated-tick GPU dispatch behavior pass.
  Source `05ca24a7d` merged with the workspace gate passing. The merged fixed views and
  selected 1×/60× motion pair pass builder inspection and a fresh visual opinion; a zero-wind
  control adds local formation/dissolution evidence. The loaded first pair and its steps remain
  retained. [Evidence](../../../.orchestrate/evidence/CLOUDS/M2-display-phase-captures/README.md) keeps complete
  sources, per-frame target/travel tables and sampled-image limits; no physical cadence claim.
- 2026-11-17 Ruling 8 now passes the haze's kind with its amount: the weather state holds an Ångström exponent
  beside the clear air's visual range, default 0, and the cloud selection hands both to `ATMOSPHERE`'s haze
  input ([ATMOSPHERE's check](../../../.orchestrate/evidence/ATMOSPHERE/haze-kind-check.md)). M16.4 gains the exponent's
  control and M16 gains M16.11. The visual range, the query and every built item are unchanged.

- 2026-11-17 M3 adds a development Sky trace/compose and far-aerial candidate at
  checkpoint `22181a666`, then retains a taller, more eroded third look. Three Metal
  diagnostic fixed stills and fresh opinions still fail M3.3's broad sunlit-side cue,
  reaching the plan's three-look stop. These are early, incomplete implementations:
  the phase, ambient integration and temporal pipeline remain provisional or absent.
  [Attempts](../../../.orchestrate/evidence/CLOUDS/M3-attempts/README.md) and
  [handoff](../../../.orchestrate/evidence/CLOUDS/M3-handoff.md) distinguish confirmed runs from open work.

- 2026-11-17 M3 resumes at `b911eb4a5` with uncommitted lighting and temporal work,
  retained in checkpoint `8f7f15c55`. Scoped Clippy, formatting, the Metal slab
  test and all three submitted-frame tests pass; M3.4 has builder proof.
  [Evidence](../../../.orchestrate/evidence/CLOUDS/M3-resume.md) distinguishes those tests from
  unproved lighting, far-air accuracy, motion quality and cost. The plan's recorded
  three-look stop still needs the lead's direction; no fourth look was taken.
  [The rewritten handoff](../../../.orchestrate/evidence/CLOUDS/M3-handoff.md) names the remaining work.

- 2026-11-17 The owner said “Carry on with M3”; source `b845ab8bb` adds retained
  far-air/slab/shadow-only probes and bounded air parts. `e706eef48` records
  M3.5, M3.6, M3.8, M3.10 and M3.11 builder proofs alongside M3.4.
  [Three resumed looks](../../../.orchestrate/evidence/CLOUDS/M3-resumed-looks/README.md) still fail
  bright sunlit sides; fine Sun grids and lower water do not recover them.
  An unshadowed diagnostic suggests cache refinement alone is insufficient;
  this is not a proven failure of the model. [Costs](../../../.orchestrate/evidence/CLOUDS/M3-cost.md)
  retain the over-refresh investigations; the lead is asked for a lighting diagnosis.

- 2026-11-17 M3's focused diagnosis at `843203a07` retains six Metal rays and
  an independent f64 source/transport reference, with [point and pixel tables](../../../.orchestrate/evidence/CLOUDS/M3-lighting-diagnosis/README.md).
  Density, phase, ambient and local-source arithmetic agree closely; the Sun
  cache retains up to 0.2882 absolute T error against its own split model.
  Noon RGB pixels differ by at most 1.1%; no production lighting fix or fresh
  visual look is claimed. A planner ruling on lighting/exposure and that cache
  approximation is requested; [handoff](../../../.orchestrate/evidence/CLOUDS/M3-handoff.md) records checks and limits.

- 2026-11-17 M3 resumes with a cumulative Sun-depth interpolant at `851a7a126`.
  [The independent reconstruction](../../../.orchestrate/evidence/CLOUDS/M3-sun-reconstruction/README.md)
  reproduces the old cache error; the selected height cubic reduces maximum T
  error from 0.2882 to 0.1802 without a light gain or added resources.
  Metal cumulative-depth and native route-admission tests pass; a bounded
  native capture route admits M3's ten-second pan. Visual and quality/cost
  acceptance remain separate from this numerical correction.

- 2026-11-17 Plan, answering M3's block (`needs_context`: a ruling on the multiple-scattering model). [Cloud light](DESIGN.md#cloud-light)'s
  Sun term is now a beam (the droplet's phase thinned at its broad lobe's rate, and one middle lobe) and a diffuse two-stream
  solution along the Sun column, with a lit plane from two depths and a view boundary, in place of four fixed orders whose weights sum to one.
  [Parts XII to XVI](../../../.orchestrate/evidence/CLOUDS/design-proof/multiple-scattering-proof.txt) hold it to photon transport: a level layer's returned flux within
  0.017 from a 45° Sun up; box faces typically a factor of 1.35 out, where the four orders are 40 and octaves 14. M3.6 and M5.1 are
  written again; M3.13, M3.14 and M14.13 are new; M2's note shrinks to its review. M1.8's and M13.7's clauses on the orders stand as
  built until M14.13 removes `Orders`: rewording them would send two Done milestones back through build and review for no change
  in what is built. The design needs a review before M3 resumes.

- 2026-11-17 Human: "Ok let's lower the bar." Thrum is a game engine: the most plausible result as fast as possible, not physical
  accuracy; references become diagnostics. Plan: [how the module is proved](DESIGN.md#how-the-module-is-proved) replaces both numerical
  acceptance sections. Dropped M3.13, M3.14, M5.1, M5.5, M15.2 and M15.3. Rewritten without reference bounds: M3.5, M3.6, M3.8, M4.1,
  M6.2, M6.9, M7.6, M12.4, M14.2 to M14.4, M14.13, M15.1 and M15.4. Eleven cost records pass on their check alone. The Sun term stays
  as a starting point a builder may tune by eye. Design review MF1 is answered by a boundary weight: the view boundary is off inside
  the handover for both integrators and fades in beyond it, so near and far fog take one source. No item of a Done milestone changed.

- 2026-11-18 Plan, on the lead's task to resolve [the design review](../../../.orchestrate/evidence/CLOUDS/design-review.md)'s MF1, which read the design
  before the boundary weight of the entry above. Settled as one boundary rule for every consumer of the Sun term, with both integrators
  taking the term and its depths through the same two shader functions; [part XVII](../../../.orchestrate/evidence/CLOUDS/design-proof/handover-light-proof.txt)
  and [the proof's README](../../../.orchestrate/evidence/CLOUDS/design-proof/README.md) hold the cases and figures. M14.13 is written again and M15.13,
  an inspection, is new. S5 taken in the design; M2.11's wording and S4's M1.2 are left, since M2 and M1 are Done. No item of a Done
  milestone changed. The design needs a review.

- 2026-11-18 Plan, on the lead's task to resolve [the design review](../../../.orchestrate/evidence/CLOUDS/design-review.md)'s two findings with the
  smallest check each. MF1: presence probes passed a GPU drawing twice the query's extinction, so M14.2 gains one sight line whose
  optical depth is held to the public query's, M6.9 the map's column at a point inside a directed shape, and M6.2 and M7.6 the drawn
  column's upper bound at a point inside, each at the allowance its test states and each failing at twice the amount. MF2: M3.9's
  check also runs `cloud_cost.py refuse`, which passes only when a record with no clouds-visible run is turned away; `check` holds the
  record and its fresh measurement to one list. No budget and no reviewer question is added. Taken: the M3 test handoff, as a note.
  Left: S5's M2.11 and S4's M1.2, since M2 and M1 are Done. No item of a Done milestone changed. The design needs a review.

- 2026-11-18 Plan, answering M3's block (`needs_context`: how to break up the repeated domes and shelves three shape trials left;
  work on `build/clouds` at `6c696409396a`, unmerged). Cause, from its code and [part XVIII](../../../.orchestrate/evidence/CLOUDS/design-proof/README.md):
  a cell's column was one dome, whose scaled outline is the same for every cell, under one octave of 200 m noise, so pitch, depth and
  taper only resized one outline; the shelf is a shallow cell's dome under a deep one's. Decided: the column is
  [a cluster of turrets](DESIGN.md#one-weather-field-in-the-air-frame) under the cell's dome, and [the shape noise](DESIGN.md#one-density-in-m¹)
  is a billow field in one tiling volume, as shipped clouds form theirs. A billow field on the dome alone left the dome. M3.1 is written
  again to name the cue. M2.2's test compares each map level with the model's column at that level's footprint; its statement stands.
  The level dark band is not explained: risk 12 says what to do first. No interface or Done item changed. The design needs a review.

- 2026-11-18 Plan, on the lead's task to resolve [the design review](../../../.orchestrate/evidence/CLOUDS/design-review.md)'s two findings on the turret
  revision. MF1: no share stands in for unresolved turrets. The turret field takes no footprint, and every level of
  [the weather map](DESIGN.md#the-weather-map) holds the query's column at its texel's centre, as M2 built it; the far level samples
  turrets, which M3.2 judges. This replaces the entry above's footprint comparison: M2.2's test compares at centres, as written. MF2:
  one clearing factor scales a cell's coverage with its thickness before cells combine. [Part XVIII](../../../.orchestrate/evidence/CLOUDS/design-proof/cloud-shape-proof.txt)
  is corrected and holds both cases; its turret figures moved to 0.090 and 0.243. M3.15 is new. The review listed no Suggestions; of
  MF1's two routes the point column was taken, since an area value needs a second rule and a new M2 test. No interface or Done item
  changed. The design needs a review.

- 2026-11-18 Plan, answering M3's second block (`needs_context`: three turret looks kept a bright toward-Sun underside under a dark
  stripe; work on `build/clouds` at `f38c7d77ad24`, unmerged). Of the lead's two routes, the cause was named and the cue stays in M3:
  controls on Metal put it in the lit plane inferred at each sample. Decided: [the type sets the plane](DESIGN.md#cloud-light), level
  for a layered type and halfway to the Sun for a heaped one, with no depth read for it.
  [The record](../../../.orchestrate/evidence/CLOUDS/M3-plane/README.md#the-decision) holds the figures, the routes not taken and what it costs.
  Risk 12 is written again, and M3.3's question gains the plane. No interface or Done item changed. The design needs a review.

- 2026-11-18 Plan, answering M3's third block (`needs_context`; work on `build/clouds` at `5f84d354a3bf`, unmerged). Lead decision:
  "Move M3.1's form cue (domes, shelves, the silhouette break) out of M3 into a later CLOUDS milestone that comes after the other
  cloud work", after about 12 h of look tuning and one Metal trial of erosion 0.20 that did not clear it. M3.1 is written again to
  ask for the sky at a glance, which the retained captures show. M17 is new, between M11 and M12, with the form cue as M17.1, a
  Chrome look, a regression look and the query tests. [Risk 2](DESIGN.md#risks) carries the erosion diagnosis, what was tried and
  what is unexplained; until M17 the named flaws fail no other item. No mechanism, interface or Done item changed, and no new look
  is planned in M3. The design needs a review.

- 2026-11-18 Plan, resolving the two findings of the design review of the parking. M17.1 and M17.2 judge the fair preset with the
  placed share at 0, a sky of weather alone, from 2 m and from 40 m on both targets, and the form flaws are excused only until M17
  starts. M17.3 looks at the presets as they ship. The earlier attribution suggestion on the lit plane is taken. M3 and M4 are
  unchanged, and no mechanism or interface changed. [The record](../../../.orchestrate/evidence/CLOUDS/M17-plan.md) holds each finding and its
  answer. The design needs a review.
- 2026-11-17 The corrected-source fresh look still fails M3.3's bright side.
  [Independent photon transport](../../../.orchestrate/evidence/CLOUDS/M3-model-diagnosis/README.md)
  reflects 84.456% at thick-slab V1 noon versus the planned formula's 5.489%.
  Proposed for the planner: retain the droplet first order and Beer transport,
  replacing higher orders with a conservative, physically validated source.
  [Motion diagnostics](../../../.orchestrate/evidence/CLOUDS/M3-motion-diagnostics/README.md)
  find stride-one edges cleaner; final integrated visual/cost proofs stay open.

- 2026-11-18 M3 implements the beam and diffuse light and anchors far-air transmission at 32 km.
  [Three independent Metal looks](../../../.orchestrate/evidence/CLOUDS/M3-beam/README.md) show bright lit faces
  and clean ridge occlusion, but repeat shallow domes and shelves. M3.1 hits the three-look stop.
  M3.2's far range and M3.7's stable motion and cut sharpness remain open.
  Required-GPU transport, frame contract, shadow-only and far-air checks pass.
  The [paired cost and refusal check](../../../.orchestrate/evidence/CLOUDS/M3-cost.md) pass under load.

- 2026-11-18 M3 resumes on the owner's instruction. [Metal horizon and motion proofs](../../../.orchestrate/evidence/CLOUDS/M3-refresh/README.md)
  retain the corrected far-plane composition and complete half-extent refresh.
  M3.2 and M3.7 have builder evidence. The frame contract and loaded cost/refusal checks pass.
  Three more shape trials retain the deeper mature cells and still expose repeated domes and shelves.
  M3.1 remains open under the plan's three-look stop.

- 2026-11-18 M3 builds cleared turrets and shared GPU billow noise at `acbf93577`.
  [Metal evidence](../../../.orchestrate/evidence/CLOUDS/M3-turrets/README.md) covers clearing and exact weather centres,
  the distant haze, and first-frame cost. Three turret looks still show shelves and a bright underside
  in the toward-Sun views. Isolating beam, diffuse and ambient places the strongest stripe in solar diffuse.
  M3.1 and M3.3 remain open. The [handoff](../../../.orchestrate/evidence/CLOUDS/M3-handoff.md) records the stop and next diagnosis.

- 2026-11-18 M3 sets the diffuse plane from the resolved type at `8fd3e8108`, removing three column reads per lit sample.
  [Metal proof](../../../.orchestrate/evidence/CLOUDS/M3-type-plane/README.md) passes the lighting and ridge checks, and refreshes the horizon and motion evidence.
  The [first visible submission](../../../.orchestrate/evidence/CLOUDS/M3-cost.md) cost 42.770 ms under load. The cost check and its refusal passed.
  M3.1 still shows shelves and an overhead fold at turns 0.25, 0.50 and 0.75. The fold survives in the trace's opacity.
  The plan limits tuning to the turn. The [handoff](../../../.orchestrate/evidence/CLOUDS/M3-handoff.md) records the remaining form problem.

- 2026-11-18 The lead's one [Metal look at erosion 0.20](../../../.orchestrate/evidence/CLOUDS/M3-erosion/README.md#metal-trial-at-020) completed all seven fixed views.
  The builder and independent inspector still saw shelves, an overhead silhouette break and repeated caps.
  M3.1 remains open. The branch restores 0.61 for the prior passing proofs.
  The lead's next step is to move the form cue to a later milestone.

- 2026-11-19 M3.1 passes the revised whole-sky inspection on the retained 0.61 Metal matrix.
  [The six views and lossless translation pair](../../../.orchestrate/evidence/CLOUDS/M3-type-plane/README.md#m31-form)
  show separate clouds with open gaps and relative motion between near crests and distant clusters.
  Integrating with RENDER-2D updates four V1 graph snapshot entries for M3's imported noise textures.
  Shade adds the shared shape texture, and Sky also adds detail. Pass order, transient lifetimes and the other 18 entries match.

- 2026-11-19 M3.9 checks all 31 named byte counts in both committed and fresh Sky runs.
  [The Metal cost check](../../../.orchestrate/evidence/CLOUDS/M3-cost.md#m39-review-fix) passed and the refusal controls rejected
  the all-absent record and all 31 single-resource omissions.

- 2026-11-19 M4 carries Shade and Sky into the Chrome game, including cooked-valley attachment.
  The [target evidence](../../../.orchestrate/evidence/CLOUDS/M4/README.md) records both Chrome checks, the matched portable Metal
  stills, the valley shade passing the receivers, and the ten-second pan at the page's callback cadence.
  The shared Metal and Chrome test expresses thickness and optical-depth errors in the M2 fixture's physical scale.
  The shared `exact-images` check supplies the merged baseline verdict.

- 2026-11-19 M4 releases the lifecycle observer's device handles and unloads the recovered page before closing
  Chrome. The [shutdown evidence](../../../.orchestrate/evidence/CLOUDS/M4/README.md#chrome-shutdown-after-device-recovery)
  records the verifier's shutdown timeout and the follow-up Chrome runs.

- 2026-11-20 M4.2 now checks each written Sun-path ground slice against an independent replay through the verified
  weather map and carries that comparison through retained rows. The [Chrome lifecycle evidence](../../../.orchestrate/evidence/CLOUDS/M4/README.md#m42-ground-field-review-correction)
  records 19 passing submissions, including five on the replacement device. The review's constant-depth control
  fails at the first ground-slice texel. The production Sun-path shader assembles to the same bytes.

- 2026-11-20 M5 tunes the heaped diffuse plane, densifies Sun-path height nodes and extends the local march at grazing angles.
  [Metal and Chrome evidence](../../../.orchestrate/evidence/CLOUDS/M5/README.md) shows the same noon relief and cloud shade,
  the last pink light on the tall cloud, star occlusion, and the held 30° aureole through two 20-second pans.
  Fine shading terraces remain in some native crops. The dusk evidence records gaps between submitted frames.
  The [paired 1440p fixture](../../../tests/fixtures/clouds/low-sun-cost.json) records the loaded timing tripwires and the 32 MiB increase.
  The shared exact-image check supplies the verdict on the merged work.
- 2026-11-20 Planner, from the cleanup review (L8-05, L8-07): M14.13's check searched two crate folders with `! grep`, which passes if either is renamed; it now searches all of `crates` and `examples`. M6.6, M12.4 and M12.9 named files under `docs/evidence`, which the merge refuses; they now name this milestone's evidence and the provenance record in the module's evidence folder. Where provenance records are kept in Git is an open assumption in [DEAD-TOOLS' plan](../DEAD-TOOLS/PLAN.md). All four milestones were Not Started.
- 2026-11-20 Planner, on the owner's decision of that day as the lead gave it: provenance records are tracked in Git. M6.6 and M12.9 now name the module's provenance record tracked under `docs/provenance`, and M6 waits for `DEAD-TOOLS` M5, which puts it there, so the first copied library is recorded where a clone gets it. The design still names the working-folder file; `ORCHESTRATE.md` says that means the tracked record, and the design is corrected at this module's next re-plan.

- 2026-11-20 M5 filters the cached Sun depths at grazing elevations and resolves the closest backlit rims with a bounded direct march.
  Metal and Chrome inspection found warm flanks, cast shade, bright rims around darker cores, and steady held 30° pans.
  The 4° valley view retains foreground aerial warmth. The closer views separate the dark cores from the rims.
  The native route prepares at the authored angle. The 45° pitch and 270° yaw routes reached their first Metal submission.
  [Target captures and diagnostics](../../../.orchestrate/evidence/CLOUDS/M5/continued/README.md) record the corrections.
  The [loaded 1440p pair](../../../tests/fixtures/clouds/low-sun-cost.json) records first fill, trace and compose tripwires.

- 2026-11-23 M3 planning archive. The handoff at `5f84d354a3bf` can be read with
  `git show 5f84d354a3bf:docs/evidence/CLOUDS/M3-handoff.md`. The retained type-plane captures used
  erosion 0.61 for the at-a-glance inspection. M17 carries the close-up form work.

## M18 workspace timing baseline

- 2026-11-23 Both cases stopped at ten seconds in the workspace run at `747909d1c` and passed its isolated retry. Both also exhausted the retry in the failed attempt at that same commit. Times below are from the passing retry.

| Seconds | Test |
|---:|---|
| 5.437 | `thrum-clouds::it portable_limits_gpu::weather_and_sun_path_work_with_default_limits` |
| 5.215 | `thrum-clouds::it sun_path_gpu::coarse_sun_paths_match_the_cpu_reference` |

- 2026-11-23 Design review Must Fix 1: M18.1 now selects both GPU cases in the workspace Nextest build with
  its normal parallel scheduling. The first focused run must show both passing within 9 seconds each under
  builder load, leaving 1 second before the 10-second limit. Isolated times are recorded for comparison.
- 2026-11-23 Design review suggestions: corrected the provenance and historical Git paths so builders can find
  both records, and scoped the Chrome runner instruction to the V1 game so M11.5 uses the water scene's runner.

- 2026-11-23 M18.1 uses a 96-square weather map and 16-square Sun-path grids. The original assertions and
  tolerances pass on both Metal profiles. With the default workspace scheduling the first focused run took
  2.139 s native and 2.145 s portable. The isolated runs took 2.309 s and 2.083 s respectively.
  [Mac Metal evidence](../../../.orchestrate/evidence/CLOUDS/M18/README.md).

## M6 review fixes

- 2026-11-23 The headless query applies authored slab floors and marks footprint estimates from hash-selected cells.
  The game scripts only its own demo shapes. Placement storage retains metadata and rows, with Metal evidence of
  80 bytes for a drifting instance, 144 after changing the weather anchor and zero on an unchanged frame.
- The mixed demo spreads formation across weather time, including the distant bank at the placed range. The
  visible slab uses bounded filtering at billow scale. Builder and fresh inspection found matching edge softness,
  no placement seam and both overlap directions in the Metal matrix. Consecutive 1× and 60× submissions show a
  continuous range crossing. Chrome submitted the resident R8 stacks with no validation errors.
  [Target evidence and reproduction](../../../.orchestrate/evidence/CLOUDS/M6/recheck/README.md).

- 2026-11-24 M6.3 retains the placed-weather map until its windows or instance data change. On Metal, the
  paused capture and in-place pan queued zero placement bytes, skipped the map pass and kept identical
  texels. Drift queued one 80-byte row. Moving the fixture's anchor queued 176 bytes, including both changed
  relative windows. Restoring residency while paused updated the volume weight and preserved column optical
  depth. [Metal test and reproduction](../../../.orchestrate/evidence/CLOUDS/M6/map-refresh/README.md).

- 2026-11-24 M6.2 refreshes the placed cloud's Sun path when a paused camera changes its form or
  moves a Sun window. Metal readbacks match a fresh host at the same weather tick and 20° Sun for
  both moves. Repeated destination frames upload zero placement bytes and skip the placed map.
  [Paused camera proof](../../../.orchestrate/evidence/CLOUDS/M6/paused-camera/README.md).

## WP7 builder checkpoint

- 2026-11-24 The V1 valley now has four weather presets, a four-minute weather sequence, and held-Sun captures for wind and cloud life on both targets. Metal tests establish exact zero-coverage identity and CPU/GPU agreement on the weather map and traced columns. The paired capture-off 1440p record measures fair, broken and overcast under the same view. [Builder evidence](../../../.orchestrate/evidence/CLOUDS/WP7/README.md).
- The fixed captures still lack distinct broken-bank gap edges and enough overcast underside relief at a 15° Sun. In consecutive Metal submissions, the return to clear drops part of the overhead bank visibly. The wind receipts establish equal travel across consecutive scheduled targets. The realtime takes leave the visible cadence and the three complete lives in broken weather open. [Handoff](../../../.orchestrate/evidence/CLOUDS/WP7-handoff.md).
- 2026-11-24 A deeper condensation ramp gives overcast a shaped underside at 15° while hiding the disc. At intermediate coverage the deck openings also evolve on the pitch-scale field. The existing Sun-path comparisons still pass. Independent inspection follows three whole fair and broken cloud lives on Metal at each speed. The fair 60× retake resolves the earlier hitch across ten consecutive ticks. [Lifetime evidence](../../../.orchestrate/evidence/CLOUDS/WP7/life-qualification.md).
- The matched Metal matrix covers both Sun angles at both heights. Sunward noon views on Metal and Chrome show bright broken-bank gap edges and distinguish the diffuse overcast glow from the clear solar disc. Chrome retained 2,721 exact 60× images and independent inspection followed three whole lives. [Preset and Chrome evidence](../../../.orchestrate/evidence/CLOUDS/WP7/weather-looks-qualification.md).
- Faster weather-row refresh during global ramps removes the clearing spur's abrupt retreat. The longer opening stage leaves time for the holes to grow. The layered type stays in effect until coverage reaches zero, so no late cumulus grows. Independent inspection passed the full four-minute Metal sequence. [Realtime evidence](../../../.orchestrate/evidence/CLOUDS/WP7/last-clear-opinion.md).
- The explicit 1× Metal wind turn preserves the crown and its linked shade. The 60× retake retains 2,425
  exact frames and contains visible longer steps where render work overruns scheduled opportunities. The
  Sun-path shader avoids optical reads for empty columns and both measured pairs show shorter passes.
  Those larger steps were visible in neighboring originals. [Retake evidence and diagnosis](../../../.orchestrate/evidence/CLOUDS/WP7/wind-final.md).
- The offscreen capture clock now keeps a future display target when its request runs late. Metal traces
  distinguish missed requests from occupied frame slots and preserve both in the record. The rebased cloud
  column fixture explicitly selects the existing path without the view volume.
  [Target and thread timing](../../../.orchestrate/evidence/CLOUDS/WP7/wind-host-stage-join.json).
- The same 120 Hz Metal release route retained 2,640 exact images without skipped targets or frame-slot
  refusals. The four longest submission intervals still sampled adjacent future targets, and neighboring
  originals show even cloud and shade travel. The loaded comparisons preserve the remaining misses.
  [Wind qualification](../../../.orchestrate/evidence/CLOUDS/WP7/wind-qualified.md).

## WP7 review repair checkpoint

- 2026-11-25 The first review repair passed the Metal and Chrome stills by lifting the bounds of nearby weather columns. The full cloud-model suite caught the mismatch with the shared condensation-level contract. The next candidate preserves that level and shapes the visible bank with erosion inside its column. The 45 cloud-model tests pass after restoring the contract. [Contract failure](../../../.orchestrate/evidence/CLOUDS/WP7/fix/lift-cloud-model-tests.log) · [Retest](../../../.orchestrate/evidence/CLOUDS/WP7/fix/within-cloud-model-tests.log)
- The paired cost check generates its investigation from the passes and work measured in its own run. The earlier fixture retains its own attributed notes. [Stale report rejection](../../../.orchestrate/evidence/CLOUDS/WP7/fix/cost-provenance-regression.txt)
- The deeper intermediate type retains the common condensation level. It gives broken banks visible lobes and illuminated irregular gaps through the skyline. The current compiled build passed the three executable WP7 items and independent fixed inspection at both heights and Sun angles on Metal. Chrome's noon stills also passed. [Metal matrix](../../../.orchestrate/evidence/CLOUDS/WP7/fix/WP7.1-metal-appearance.md) · [Cross-target opinion](../../../.orchestrate/evidence/CLOUDS/WP7/fix/deep-bank-opinion.md) · [Executable items](../../../.orchestrate/evidence/CLOUDS/WP7/fix/deep-item-checks.json)
- The bank texture keeps its own world phase across 200 m render-anchor changes. The fresh Metal cycle preserves overhead landmarks through all four weather transitions, including the three points where the previous build jumped. Metal and Chrome motion inspections followed distinct lives at the required rates. The 720p Apple Tile fast-wind take and the two straight 1× controls each retain 2,514 consecutive 120 Hz intervals, and the 480p turn and still-air controls also retain consecutive samples. [Metal cycle](../../../.orchestrate/evidence/CLOUDS/WP7/fix/phase-cycle-opinion.md) · [Metal lifetimes](../../../.orchestrate/evidence/CLOUDS/WP7/fix/WP7.5-metal-lifetimes.md) · [Chrome](../../../.orchestrate/evidence/CLOUDS/WP7/fix/WP7.7-chrome.md) · [Wind](../../../.orchestrate/evidence/CLOUDS/WP7/fix/WP7.4-metal-wind.md)

## WP7 second review repair

- 2026-11-25 Broken banks use the same broad shape in the visible layer, the retained Sun path, the browser ground-reference shader, and the CPU full and coarse density. At the matched Metal gap the visible and ground optical depths are 0.0039 and 0.0028. At the adjacent occupied bank they are 1.528 and 1.647. Both paths agree with their CPU references within the test tolerance. The weather-map check covers the shipped broken endpoint and its ramps. [Metal and CPU evidence](../../../.orchestrate/evidence/CLOUDS/WP7/r3/WP7.6-metal.md)
- The repaired broken sky retains relief and bright edges in the fixed Metal and Chrome views. The four-minute Metal sequence preserves the overhead landmarks as the holes close and reopen. Orchestrate passed its executable checks and the branch test command. [Fixed views](../../../.orchestrate/evidence/CLOUDS/WP7/r3/fixed-weather.md) · [Weather cycle](../../../.orchestrate/evidence/CLOUDS/WP7/r3/WP7.3-metal-cycle.md) · [Executable checks](../../../.orchestrate/evidence/CLOUDS/WP7/r3/item-checks.log)
- The normal-window wind cadence still needs a capture-off run on the built-in 120 Hz panel. At 1920×1080 offscreen, tracing fewer texels reduced refused views in one trial. Host skips and capture omissions persisted in other trials, including the clouds-absent control. [Timing and conditions](../../../.orchestrate/evidence/CLOUDS/WP7/r3/1080-diagnosis.json)

## WP7 normal-window cadence retake

- 2026-11-25 On the built-in 120 Hz panel at 1920×1080 with pixel capture off, the rebased Metal release submitted 276 of 492 refresh targets at 40 m/s and 60×. The longest interval was 91.667 ms. The clouds-absent control submitted 490 of 492 under a different host load. The earlier build also missed targets in the four live steady-wind settings. [Window cadence and controls](../../../.orchestrate/evidence/CLOUDS/WP7/WP7.4-panel-retake.md)
- The bounded window's GPU timestamp queries materially affected cadence even with the pixel tap off. At 40 m/s and 60×, a same-binary comparison submitted 334 of 492 targets with timestamps and 492 without. The longer timestamp-free takes hit all 2,412 targets in still air and at 10 m/s. Other wind cases still missed targets. The opt-in wind observation records the ordinary 1920×1080 window without forced timing queries or scripted camera edits. [Timing comparison and sustained takes](../../../.orchestrate/evidence/CLOUDS/WP7/r7/timestamp-cadence.md)
- The passive observer recorded at least one uninterrupted 20-second interval for each wind condition in the 1920×1080 Metal window on the built-in 120 Hz panel. At 40 m/s and 1×, two separate takes reached every target. The matching 1080p originals show the cloud relief and shade moving together and staying in place across the 90° wind change. Earlier missed refreshes and the first turn take's readback losses are retained alongside the qualification. [Window and visual proof](../../../.orchestrate/evidence/CLOUDS/WP7/r8/WP7.4-metal-window-and-wind.md)

## WP7 ordinary window timing policy

- 2026-11-25 The normal bounded window prints its report with GPU timestamps disabled while the frame profiler is closed. Explicit report and pan qualification still request timestamps. Wind qualification inherits the window policy. The shared-GPU retake recorded untimed samples and missed refreshes. The subsequent panel run was fully occluded at the lock screen. [Window evidence](../../../.orchestrate/evidence/CLOUDS/WP7/r9/README.md)
- The fresh 1080p Metal T2 wind take retained 2,519 verified submitted images over 22 seconds at 40 m/s and 60×. Successive images around 14 seconds show the cloud relief and shade moving together. [Submitted-image evidence](../../../.orchestrate/evidence/CLOUDS/WP7/r9/visual-fast/verified.json)

## WP7 ordinary release display proof

- 2026-11-25 The normal V1 window runs until closed and leaves GPU timestamps off with the
  profiler closed. Its exit report counts successive surface submissions for future
  display targets. The built-in 120 Hz panel held all five wind conditions for at least
  20 consecutive seconds, including 29.57 seconds across the 90° turn and 41.37 seconds
  at 40 m/s and 60×. Capture and observers were off. Loaded trials also missed
  refreshes. [Release build and display receipts](../../../.orchestrate/evidence/CLOUDS/WP7/r11/README.md)

## WP7 diagnosis

- 2026-11-25 Planner, after four failed reviews, three on WP7.4. The plan was at fault. WP7.4 held two claims
  as one. Every review passed the clouds' own motion; the one no was whether the whole frame reaches the display
  at every refresh, and for that the item gave no protocol, no pass rule and no instrument. Each review measured
  a different run and each repair answered the last one. On one binary the builder's eighteen display receipts
  served 46% to 99.7% of their refreshes; in both ordered series the early runs held and the late ones did not,
  whatever the wind; the exclusive GPU lock did not change that; and the window's report counted targets sampled
  before encoding, which overstated one run and reported 0.125 s for a still-air run that presented 5,400 frames
  in 5,414 callbacks. [The evidence read](../../../.orchestrate/evidence/CLOUDS/WP7-diagnosis.md)
- WP7.4 before: "... clouds drift in world space at the wind's speed with their shade under them, by even steps
  from frame to frame at the display's refresh, and no texture slides inside a cloud's shape. A change of wind
  direction turns the drift and does not move a cloud." Now: the same drift, shade and turn, with the steps
  judged by each frame's display target (equal over one refresh, in proportion across a skipped one), as a
  numbered list. Held refresh moved, unweakened, to WP20.
- WP19 and WP20 are new, after WP7. WP19: the window's account of every refresh opportunity, placed by the time
  of the present call, with where each loss happened; a wind series with a clouds-absent run on each side of
  every cloud run, taken before any repair; the clouds' causes fixed and the window's proposed to
  `PLATFORM-RUNTIME-MAC`. WP20, `"alone": true`: the series once with no other agent working, each cloud run
  holding the refresh as the clouds-absent runs beside it do, in the release game with no frame limit or cap.
  WP12.10 is new: the series' two commands leave the cost tool when the module closes.
- Review 4's second finding, the governor qualification losing GPU timestamps, needs no item here:
  `APPLE-PRESENTATION` fixed it at `d7d9d5ddf` by asking for timing in its own proof.
- The design gains [Holding the display's refresh while clouds drift](DESIGN.md#holding-the-displays-refresh-while-clouds-drift-wp19-and-wp20);
  "no timing figure decides an item" keeps one exception, WP20, on the lead's words in this task: "Keep the
  bar: the player's ordinary game holds the display's refresh with clouds drifting; don't weaken it to a bounded
  or capped run."
- PLAN.md: WP18's and WP6's notes shrank to their review links, both being Done. WP18's assumption, "1 second
  below the existing 10-second timeout is enough headroom" for its focused parallel run, left the open list with
  the work package Done on it.

## Design review of 25 November

- 2026-11-25 Planner, after [the design review at `cbd6873e4`](../../../.orchestrate/evidence/CLOUDS/design-review.md):
  five Must Fix findings, all on the proof of held refresh. No mechanism of the sky and no interface changed.
- Must Fix 1, WP20 could pass while the player saw a repeated frame. A lost refresh is now counted from the
  OS-reported presented times, by `PLATFORM-RUNTIME-MAC`'s witness, on every run of the wind series, the turn
  included. The window's account only says where the game let a refresh go, with a sixth place, after the
  present call. WP20.1, WP19.2 and WP19.3 are written again on that and are attended setup. The design no
  longer says WP12 holds the broken and overcast skies to the refresh: WP7.8, WP12.5 and WP12.6 record and
  investigate, and the plan carries that as an assumption. No WP12 item changed.
- Must Fix 2, WP19 could send a cloud regression to the window owner. Attribution now asks the whole frame
  what the cloud run loses that its clouds-absent neighbours do not: more than the neighbours is the clouds',
  whatever the cloud passes take by themselves, and only a loss the neighbours share is the game's own. A
  diagnostic run may end as not reproduced or not distinguished, and that ends nothing `BLOCKED`. A proposal
  in `PLATFORM-RUNTIME-MAC`'s log no longer stops WP19.
- Must Fix 3, WP7.4 could reject correct drift on a late frame. Its steps are judged by the display time
  between two frames, from each frame's tick and display phase. Equal steps are expected only of frames one
  refresh apart in display time. Across the turn the list asks for continuity while the cloud keeps travelling.
- Must Fix 4, WP20 made a count in a clouds-absent run an admission gate. That count fails nothing now. The
  record gives each cloud run's result beside what its neighbours lost, the losses go to
  `PLATFORM-RUNTIME-MAC`'s log, and only a cloud run that loses more than one refresh beyond the worse of its
  neighbours rejects the sky. The plan no longer ends `BLOCKED` on three series with poor controls; a series
  is "not a measurement" only for a missing or occluded run or presented times that do not cover the interval.
- Must Fix 5, WP19.1 kept every opportunity of a run in memory. The account keeps counts for the whole run and
  rows only inside a measured interval a launch asks for, and its test gains a sixth case for that bound.
- Suggestions, all three taken: the builder handoff and risk 13 point at the scheduled `exact-images` check;
  the two `git show` examples name `M3-handoff.md` again, as Git stored it; WP19.1's test runs as unit tests
  of `thrum-app`, a new test target being against the one-binary rule.
- WP19 and WP20 are renamed to what they now claim, and PLAN.md has WP19 build the `hold` command so the
  sitting only runs it.

## Second design review of 25 November

- 2026-11-25 Planner, after [the design review at `93fbf8f49`](../../../.orchestrate/evidence/CLOUDS/design-review.md):
  one Must Fix, on the proof of held refresh. No mechanism of the sky and no interface changed. This was the
  second review in a row to ask for changes, so no reviewer reads this revision before building.
- Must Fix 1, equal loss totals could pass a freeze at the wind turn and give it to the window. A cloud run
  now holds by its count and by its longest stall of consecutive lost refreshes, and a stall that stands out
  is never the game's own. WP20.1, WP19.2 and WP19.3 are written again
  ([detail](../../../.orchestrate/evidence/CLOUDS/design-revision-2026-11-25-stalls.md)).
- New item WP19.4: the `hold` command gives the known answer on five scripted cases, the reviewer's among
  them, so the rule is proved before WP20's one attended sitting. It is deleted with the command in WP12.10.
- Suggestion taken: the design says the witness needs its joined submission and request
  records across both markers (841 joined drawables qualify, the 488 inside the markers fail coverage), apart
  from the account's rows, which stay bounded by the measured interval.
- Assumption extended (WP20's line in PLAN.md): the stall limit, and only a run's longest stall is compared,
  not how many it has. WP19.4 and the witness's records are in the design, not in WP19's note.

## WP7 cloud motion at display time

- 2026-11-25 Five realtime Metal takes verify WP7.4 under the revised inspection. The cloud and its shade
  hold in still air, travel together in wind, and bend without a jump through the 90° turn. At 60× the outline
  travels about twice as far across a double display interval as across adjacent single intervals. The
  three WP7 item commands passed on Metal. The previously inspected appearance, transitions and lifetimes
  retain their Metal and Chrome evidence. [Builder evidence](../../../.orchestrate/evidence/CLOUDS/WP7/display-time/README.md)

## WP19 refresh accounting checkpoint

- The Mac window accounts for present calls against display-link refresh intervals. Its ordinary exit report
  has whole-run counts, and an opt-in measurement retains rows. Nine refresh-account tests and the five
  scripted hold cases pass. [CPU evidence](../../../.orchestrate/evidence/CLOUDS/WP19/refresh-account-tests.log)
- The scoped Instruments recorder and the platform witness covered 20 seconds in each of the first five
  1920×1080 wind-series runs on the built-in panel. The display slept before the sixth run and set the
  console-lock flag. The first two paired cloud runs showed excess loss under shared load. The required
  diagnostics and the later series remain open at this checkpoint.
  [Partial series](../../../.orchestrate/evidence/CLOUDS/WP19/pre-repair-scoped/series.json) ·
  [Resume record](../../../.orchestrate/evidence/CLOUDS/WP19-handoff.md)

## WP19 paired cadence record

- Two complete eleven-run series on the built-in 120 Hz panel account for every lost refresh. The queue
  fix assigns pending present calls as soon as their interval becomes known. All ten refresh-account tests
  pass, including the regression from the rejected first attempt at the later series.
- The corrected accounting covers six entries from the first series. The five-refresh stall in the 10 m/s run
  did not recur. Four first-series excess places recurred and their cause remains unresolved. One entry went
  to the platform proposal. The cloud workload is unchanged.
  [Device evidence](../../../.orchestrate/evidence/CLOUDS/WP19/review-fixed-accounting/README.md)

- The merged cadence check rejected a clouds-absent run after one occluded Metal surface acquisition.
  AppKit reported zero window occlusion events and the panel stayed awake and unlocked. The default cadence
  command now keeps each rejected attempt and starts a whole new series after occlusion or an Instruments
  recording timeout, up to three attempts in total. The CPU tests cover fresh restarts, the retry limit
  and propagation of other failures.
  [Retained arm replay](../../../.orchestrate/evidence/CLOUDS/WP19/occlusion-replay.log)

- The review replay joins each lost refresh to the next shown drawable and its present call. The original
  loss counts and stall times are unchanged. A missing OS observation that can affect the measured interval
  withholds the cadence judgement. A later loss at a place establishes recurrence even when the paired count
  is within the neighbours' range. The recorder verifies a writable descriptor's path and file identity
  before it models or removes the scratch. [Review replay](../../../.orchestrate/evidence/CLOUDS/WP19/review-fixed-accounting/reviewer-replay.json)
- A refresh was lost just before a call that still served the display link's interval. The measured
  rows now export the drawable's own request outcome so the analyser can place that loss before the call.
  The ten refresh-account tests pass on the rebased build.
  [CPU proof](../../../.orchestrate/evidence/CLOUDS/WP19/review-refresh-account-tests.log)
- The rebased cadence check recorded all eleven arms and ran two timing diagnostics. Every recording has
  a verified scratch identity and cleanup receipt.
  [Device audit](../../../.orchestrate/evidence/CLOUDS/WP19/cadence-20261125T182649.431879Z/audit.json)

- An attended rerun reached an awake built-in panel, but the first game process exited with zero presents.
  Replays of the same build under the GPU lock measured AppKit occlusion and thousands of refused drawable
  acquisitions. A window-order sample showed another application above the cadence window during a run.
  The cadence launch now focuses its normal window once after preparation and writes the window conditions
  even when the game exits without a frame. The collector uses observed occlusion to reject and restart the
  whole default series, and it cycles the display during setup if a wake alone leaves the idle flag set.
  [Window order](../../../.orchestrate/evidence/CLOUDS/WP19/merged-retry-window-order.log) ·
  [Rejected check](../../../.orchestrate/evidence/CLOUDS/WP19/merged-cadence-retry.log)
- Once the panel was available, the next check recorded three valid arms. The fourth had no occlusion,
  but drawable 1473 had a game submission and no OS presented-time row inside the measured interval.
  This incomplete witness now restarts the whole default series. The rejected trace and its unmatched
  submission remain available for review.
  [Incomplete witness](../../../.orchestrate/evidence/CLOUDS/WP19/cadence-20261125T195654.460494Z/03-10ms-1x/witness.json)
- The reviewer found another before-call loss that took its place from a later request. The analyser now
  uses the joined drawable's request outcome when the request had already been dispatched. An elapsed
  opportunity can still supply evidence of a skipped callback, late request or refusal. The first, later
  and reviewer series replays preserve every OS loss time and stall length. The first-series accounting
  reaches the same six decisions, and the Mac platform rows carry the corrected places.
  [Replays and accounting](../../../.orchestrate/evidence/CLOUDS/WP19/request-attribution/README.md)

## WP20 quiet wind series

The release V1 game on `bea780010a58` held by both paired limits in all five
cloud conditions on the M3 Max built-in 120 Hz display. The OS presented-time
witness covered all eleven measured intervals. The independent audit
reproduced each lost refresh and stall. The 10 m/s run lost one isolated
refresh between loss-free controls. The other four cloud runs lost none.
The four clouds-absent losses are in the
[Mac runtime log](../PLATFORM-RUNTIME-MAC/LOG.md#refreshes-lost-with-clouds-absent-in-clouds-wp20).
[Series and audit](../../../.orchestrate/evidence/CLOUDS/WP20/hold-20261125T224406.456981Z/README.md)

## WP8 World light

The optional overlay carries cloud RGB in-scatter and transmittance into ATMOSPHERE's
diffuse and reflection products. Clouds keep borrowing the clear sky for their own
ambient light. The incident meter now uses the Sun path at its metering point when
World light is declared. The display smooths gain through the clear handover while
the meter continues to report physical illuminance. Legacy aerial transport scales
its solar in-scatter where the scene has no view volume.

Metal and Chrome fixed views reproduce neutral overcast daylight and directional
reflections under broken weather. The Metal retake resolves the exposure steps in
the coverage sweep. The apparent hard edge follows the authored lookout geometry.
The paired 1440p Metal record compares World light against Sky alone and the WP7
reference and times the overlay, diffuse projection and reflection mips by name.
[Qualification and limits](../../../.orchestrate/evidence/CLOUDS/WP8/README.md)

### WP8 review rework

A transparent overlay now retains the original diffuse and reflection arithmetic.
The matched Metal noon valley images agree for every pixel. The incident meter keeps
local shadow and lux, while display gain uses an open horizontal reference with a
bounded diffuse correction. The 1× Metal retake shows the shadow and haze moving
across the valley without the exposure rebound. Fixed daylight and broken reflections
were inspected again on Metal and Chrome.

The visible layer and overlay share the clear-sky hemisphere integral. Coarse
procedural and placed samples use the profile's detail mean through the shared
erosion functions. A settled overlay sleeps with its diffuse and reflection
passes, then refreshes on a changed camera in the Metal timing test.

The 1440p record now includes Off, Sky and World light at the same view. The
validator checks the retained record and fresh timings, and requires an investigation
for each triggered pass. The repeat series documents timing variability.
[Rework qualification](../../../.orchestrate/evidence/CLOUDS/WP8-rework/README.md)

### WP8 clear sunset

A transparent overlay follows the ordinary incident meter as the Sun changes.
It carries exposure history only while finishing a handover from cloud cover.
The small Metal scene compares same-submission exposure words and pixels over
three light levels. In the cooked valley's realtime sunset, Sky and World light
reach the same exposure and identical endpoint pixels, with adaptation inactive
throughout both runs.
[Metal sunset and coverage evidence](../../../.orchestrate/evidence/CLOUDS/WP8/clear-exposure/README.md)

## WP9 Weather controls and sky motion

The native panel, Chrome and parameter files share a typed control declaration.
Weather targets enter through the authoritative tick. The Internal field overlay
reads the prepared weather, placement, Sun-path and layer fields. At the same
held tick the sky returned exactly after switching the overlay off on both
targets. The 1440p Metal record pairs all four presets at noon and a 10° Sun
with clouds absent and documents the triggered cost comparisons.

Both ends of eleven available sky knobs passed on Metal with the same
authoritative state and byte-identical weather columns, and without creating GPU
resources. WP9.7 also includes two cadences for cloud Ground, whose producers
belong to WP14. WP16.7 owns their knob proof. Stationary controls at the same
yaw as the low-Sun pan show much of the same lower cloud detail on both targets.
Chrome shows extra smearing at cloud edges during the pan, and the motion items
remain open. The work stays on the builder branch. [Target evidence](../../../.orchestrate/evidence/CLOUDS/WP9/README.md)

The planner revision on main limits WP9.7 to the eleven sky knobs. The
post-rebase Metal check passed both endpoints for each knob. Cloud history
applied its declared reset, the weather state and physical columns stayed
identical, and the GPU creation ledger recorded no new work.
[Metal evidence](../../../.orchestrate/evidence/CLOUDS/WP9/revised-knobs-metal.log)

The cloud composite now filters sky transport at render pixel centres.
Low-Sun pan comparisons show smaller differences at the newly revealed
edge on both targets. The six TAAU retakes per target passed visual review
and both cuts and profile changes reset for one submission. The controls,
fixed field views and 1440p cost replay were repeated. Chrome and the
same-endpoint Metal readback restored the ordinary sky byte for byte.
[Retake evidence](../../../.orchestrate/evidence/CLOUDS/WP9/filtered/README.md)

The cloud resolve interpolates current sparse samples and reduces history
weight where the projected position or opacity changes. A turn across a
refresh block or a coverage or type update uses a full trace. Portable Metal
TAAU retakes at requested strides 2 and 4 preserved cloud contours through
the pan. The preset filled in through intermediate frames. The cut and layer
profile change each reset history for one submission. The 1440p range replay
passed. [Sparse refresh evidence](../../../.orchestrate/evidence/CLOUDS/WP9/sparse-refresh/README.md)

World light now bounds the exposure residual around the current meter.
The seven Metal control sweeps retained daylight detail. Field diagnostics
draw after reconstruction. The first restored Chrome submission matched
the ordinary sky byte for byte, and the held-host Metal check also passed.
In settled views, sparse resolve carries a texel through its refresh cycle.
Metal captures at 960×540 and 1920×1080 preserved the detached cloud at
the return to stride 4. [Target evidence](../../../.orchestrate/evidence/CLOUDS/WP9/review-fixes/README.md)

Rapid solar edits now refresh the whole World light cube in one
submission. The cube carries absolute in-scatter, so mixing strips lit
at different times made the foreground flash at dusk. The fair and
broken portable Metal sweeps retain the dusk transition across
consecutive submissions. Valid Sky and Shapes profiles can also be
replaced during setup. Their public descriptors match those of a fresh
contribution using the replacement profiles.
[Solar sweeps and setup proof](../../../.orchestrate/evidence/CLOUDS/WP9/dusk/README.md)

## WP10 noon relief parked

- 2026-11-26 Lead, after WP10 stopped at the three-look limit on weak noon relief (the congestus core and the
  mediocris twin turrets, on Metal and in Chrome): park, don't push. "The bar is the look of shipped high-end
  games, and a high Sun flattening broad lobes is plausible." Planner: WP10.1 and WP10.4 excuse the weakness by
  name, so WP10 closes on the light retained at `63f4e85794fe`. [Risk 14](DESIGN.md#risks) records it and what
  WP10 ruled out. New WP21, after WP17 and before WP12, takes the relief up. No mechanism or interface changed.
- 2026-11-26 Design review of the parking: four Must Fix, all confirmed by the lead. WP21 moved last, after
  WP12, with new WP21.5 for the finished views; the waiver names two shapes at noon; WP21.4 records cost at the
  noon Sun, before and after; WP21.1 and WP21.3 judge by the cues alone. [Risk 14](DESIGN.md#risks) carries
  all four. [What changed for each finding](../../../.orchestrate/evidence/CLOUDS/design-revision-2026-11-26-noon-relief.md)
- 2026-11-26 Second design review of the parking: three Must Fix, all settled; last round, so no review follows.
  WP21 is out of the done-when file and the plan, so the module finishes at WP12, and
  [the noon-relief follow-up](DESIGN.md#the-noon-relief-follow-up) holds its work for the lead to schedule, with
  a motion check under MetalFX and a cost question asked of each Sun. No mechanism or interface changed.
  [What changed for each finding](../../../.orchestrate/evidence/CLOUDS/design-revision-2026-11-26-noon-relief-2.md)
- 2026-11-26 WP10 ended `NEEDS_CONTEXT`: its finished gallery shows two more shapes flat at noon only, the
  upper left lobe of congestus-forked and the merged head and middle lobe of fair-head-tail, outside the two
  named exceptions. Lead: the same cause as the core and the twin turrets (diffuse sunlight into broad lobes
  at a high Sun), not four defects; excuse noon relief on the broad lobes of any shape whose 10° view passes,
  and have the follow-up (the lead's "WP21") fix all four and any other shape with the cue to the bar of
  shipped games, without holding up WP12 or the module's close. Planner: [risk 14](DESIGN.md#risks), WP10.1,
  WP10.4 and WP12.9 now go by the cue, and [the follow-up](DESIGN.md#the-noon-relief-follow-up) answers for
  every shape WP10's evidence names. WP10.2 to WP10.5 stand as proved. No mechanism or interface changed.
- 2026-11-26 Design review after the widening: one Must Fix, settled in
  [the follow-up's line 1](DESIGN.md#what-the-follow-up-must-show). Moving congestus-forked and fair-head-tail
  into line 1 had left the affected shapes judged at noon by relief and edges only, so a remedy could restore
  a crease and lose the shape. Line 1 now judges its noon stills by two lists, the relief cues and lines 1 to
  4 of WP10.1's list (recognisable with its whole outline, in one light with its family, base on the shared
  level, no box, seam, voxel step or filtering band), and Chrome's line 3 asks for both. Took the suggestion:
  lines 1 and 2 link WP10's complete 48-view gallery, `WP10/parked-metal/`. No item, mechanism or interface
  changed, and the lead's decision of 26 November stands.
## WP10 Library use distance

The held gallery and fair-sky views now exercise the accepted library and growth track through the ordinary cloud
trace. Full placed share removes procedural heaped cells. A local Sun-column march restored low-Sun relief on
Metal. The independent noon retake still found curved bands on the mediocris ridge and weak relief on the
congestus core and mediocris twin turrets. The sky remains on the builder branch.

The matched 1440p Metal device profiles counted thirteen admitted volume forms throughout the timed arms and
recorded R8 and BC4 bytes for both stacks. [WP10 target evidence](../../../.orchestrate/evidence/CLOUDS/WP10/README.md)
Documented review observations and remaining work are in the [handoff](../../../.orchestrate/evidence/CLOUDS/WP10-handoff.md).

The lead reopened placed-cloud lighting inside WP10 on 26 November. Tighter steps and a temporally rotated
blue-noise offset both left the noon ridge bands in the same places. Separate Metal captures located the bands
in the Sun-driven diffuse term. The retained light uses the broader Sun-path column for high-Sun diffuse
transport and keeps the local beam. A filtered density probe keeps surface relief. Near a low Sun, the shaded
flank now agrees with the neighbouring clouds while the forward rim remains visible.
[Metal lighting controls](../../../.orchestrate/evidence/CLOUDS/WP10/lighting-controls/README.md)

The broader column alone still gave flat lower lobes. The filtered density correction and the final stronger
local-depth response also left weak noon relief on the congestus core and mediocris twins. The one-sided
probe gave brighter relief but added a chain of pale marks across the core. The independent retake found no
regular bands in the final local-depth trial and rejected its flat lower faces. The retained checkpoint keeps
the filtered response and the lower-Sun flank correction. The three-look limit leaves WP10.1 and WP10.4 open.
[Noon comparisons](../../../.orchestrate/evidence/CLOUDS/WP10/lighting-controls/daylight-comparison.png)

The growth retake qualified both speeds and all ten key boundaries. The settled 8× sky, the six mixed stills,
the four full-share stills and the shade control passed independent inspection. The paired 1440p Metal record
observed all thirteen volume forms throughout both visible arms. WP6's focused Metal and Chrome checks passed,
and independent comparison found its accepted fixed lighting, shade, overlap and tower cues.
[Growth](../../../.orchestrate/evidence/CLOUDS/WP10/qualification-growth.md) ·
[Fair sky](../../../.orchestrate/evidence/CLOUDS/WP10/qualification-fair.md) ·
[Cost](../../../.orchestrate/evidence/CLOUDS/WP10/qualification-cost.md)

With risk 14 in force, the retained light was checked on rebuilt Metal and Chrome captures. WP10.4 passed
the current checklist. Eighteen Chrome WebGPU submissions reported both R8 stacks resident and agreed with
Metal in the shape, mixed sky, full-share and shade views.
[Chrome proof](../../../.orchestrate/evidence/CLOUDS/WP10/qualification-chrome-parked.md)

WP10.1 has two noon findings outside risk 14's named shapes. At both heights, the upper-left face of
congestus-forked reads as even grey despite the visible crease between the forks. The head and adjoining
middle lobe of fair-head-tail merge into one grey face. The 10° Sun separates both shapes' lobes.
The plan holds the retained light, so the findings go back to the lead before the gallery can pass.
[Forked crops and receipts](../../../.orchestrate/evidence/CLOUDS/WP10/qualification-forked.md) ·
[Head-tail crops and receipts](../../../.orchestrate/evidence/CLOUDS/WP10/qualification-head-tail.md)

The revised risk 14 rule covers all four noon findings in the retained 48-view Metal gallery. At the same
eye heights, the 10° views distinguish those lobes, and all twelve shapes pass the full low-Sun checklist.
The noon views pass the outline, shared light, base and edge checks. The retained Chrome proof meets the
same rule on its three representative shapes. Growth, fair-sky and cost evidence carry forward from
their target runs. [Metal gallery proof](../../../.orchestrate/evidence/CLOUDS/WP10/qualification-gallery.md) ·
[Chrome proof](../../../.orchestrate/evidence/CLOUDS/WP10/qualification-chrome-parked.md)

The review repair keeps a hash-selected cell's palette fixed by its authored profile and uses the model's
per-slab scale limit for both march bounds. In the reported full-share ramp the same cell and source survive
the return to full share, with the optical-depth change halving as the step halves. The shallow-profile retest
encloses the occupied top at 3,281.20 m with a ceiling of 3,419.24 m. Fresh Metal and Chrome stills, Metal
growth and fair-sky motion, and the paired Metal cost check cover the original five items on the repair.
[Review repair and target receipts](../../../.orchestrate/evidence/CLOUDS/WP10/review-repair/README.md)

## WP14 broken-cloud shaft parked

- 2026-11-26 Lead, after WP14 ended `BLOCKED` with `needs_context` on the shaft below broken cloud (WP14.6,
  WP14.9): park the one cue, as WP10's noon relief was, and move it to a later work package that blocks
  nothing. Planner: WP14.6 and WP14.9 name the limitation, [risk 15](DESIGN.md#risks) records it, and new
  WP22, last, after WP12, takes it up. No mechanism or interface changed.
  [What changed](../../../.orchestrate/evidence/CLOUDS/design-revision-2026-11-26-broken-cloud-shaft.md)
- 2026-11-26 Design review of the parking: four Must Fix, all settled. New WP22.8 holds a changed broken
  preset to WP7.1's broken sky; WP14.6 and WP14.9 judge the kept broken-preset views by every line but the
  shaft's; WP22.1's controls are judged by what each takes away, and WP22.4 claims lines 1 to 4; WP22.2
  follows the shaft in motion under TAAU in Chrome. No mechanism or interface changed.
  [What changed for each finding](../../../.orchestrate/evidence/CLOUDS/design-revision-2026-11-26-broken-cloud-shaft-2.md)
- 2026-11-26 Second design review of the parking: three Must Fix, all settled; last round, so no review
  follows. WP22.8 judges every preset whose cloud WP22 changes by WP7.1's words (overcast and fair as well as
  broken, since type-table rows are shared), and judges broken's banks whole, not their tops and bases. WP22.2
  lets a moving shaft fade or end while its opening stays open where the frames show the cause, and WP22.4
  judges MetalFX motion by the same four lines. Took the suggestion: risk 15 calls WP14's six views the
  selected stock views. WP14's items are unchanged. No mechanism or interface changed.
  [What changed for each finding](../../../.orchestrate/evidence/CLOUDS/design-revision-2026-11-26-broken-cloud-shaft-3.md)
## WP14 Valley mist, fog and low cloud

The V1 valley supplies the measured terrain surface to Ground. The ground band extends the Sun's path
through condensate and keeps surface visibility separate. The near coefficient pass lends the view volume
extinction and light at its jittered sample. The far trace and sky overlay use the same density and solar
kernel. The cloud transport model now uses the beam and two streams.
[Metal correctness probes](../../../.orchestrate/evidence/CLOUDS/WP14/CORRECTNESS.md) ·
[Metal 1440p timings](../../../.orchestrate/evidence/CLOUDS/WP14/COST.md)

The Metal and Chrome captures show mist pooling below the ridge and the polished sphere losing its blue
reflection through the fog edge. The broken-cloud takes show a bright pool and broad glow, with the shaft
reaching into the mist still unproved on either target. The accepted target evidence and open findings
are recorded in the [visual inspection](../../../.orchestrate/evidence/CLOUDS/WP14/VISUAL.md).

A full two-minute mist sweep passed on Metal, along with the 100 m walk, ten-second fog pan and mover
crossing. The elapsed-time previews retain their engine submission times.
[Motion receipts](../../../.orchestrate/evidence/CLOUDS/WP14/motion/selected.json)

The broken-cloud follow-up replayed the same Metal pose with Shade, Sky, World Light and Ground. The
diagonal in the haze becomes less distinct when the upper clouds are drawn, before mist contributes.
A renderer diagnostic kept the clouds' extinction and removed their source radiance, exposing the
shaft in that pose. The ordinary broken-cloud image still needs a legible shaft reaching the mist.
[Matched captures and diagnostic controls](../../../.orchestrate/evidence/CLOUDS/WP14/shaft-follow-up/README.md)

The matched scene-linear Metal captures show that removing upper-cloud radiance raises contrast across
the edge in the sky and changes little below the ridge. Four fresh 1440p pairs on Metal and Chrome
show the same diagonal glow, cast light and pooled mist. The builder and a fresh reader could follow
no distinct shaft into the mist. The maturity record now includes the completed Metal motion series.
[Linear samples](../../../.orchestrate/evidence/CLOUDS/WP14/shaft-follow-up/linear-luminance.json) ·
[Target inspection and inputs](../../../.orchestrate/evidence/CLOUDS/WP14/shaft-target-pairs/README.md)

Closer 16° evening views show a distinct diagonal from broken cloud reaching the mist's upper edge
on Metal and Chrome. Chrome controls remove the diagonal with clear sky and expose its lit terrain
footprint with clear Ground. The diagonal still dissolves into the pooled light at the mist's upper
edge. Seven deeper and more diffuse mist profiles raised the lit pool on Metal and left that cue
unresolved. Localizing the bank made the footprint brighter and the narrowest takes developed a
blocky edge. The stock profile gave the clearest view.
[Closest target stills and mist controls](../../../.orchestrate/evidence/CLOUDS/WP14/shaft-target-pairs/README.md)

Metal controls reduced the ground band's node spacing from 80 to 20 m horizontally and from 12
to 4 m vertically. Mean absolute luma change at the shaft and landing stayed below one SDR level
in both poses. Suppressing the mist's diffuse solar component darkened the surrounding pool and
left its broad shape. Staggering two library clouds along the Sun direction narrowed competing
bright patches. Adding a local bank concentrated the landing without preserving the diagonal
through the mist. These controls used scratch builds.
[Sampling and aperture comparisons](../../../.orchestrate/evidence/CLOUDS/WP14/shaft-target-pairs/README.md)

A depth-clipped Metal probe sampled the local mist source before view integration. The close view's
adjacent source peaks occurred at different distances, 4.05 km at image row 710 and 2.25 km at row
760. Row 660 had no occupied mist samples in the probed range. Eight ordinary captures framed the
far peak directly and still showed a shallow band of light. The stock render and authored selections
are in use.
[Scene-linear probe and peak views](../../../.orchestrate/evidence/CLOUDS/WP14/shaft-target-pairs/README.md)

A scene-linear Metal probe mapped mist extinction and Sun-path optical depth at nine altitudes
for seven Sun positions. Some stock openings crossed occupied mist over several successive
heights. Fourteen ordinary Metal captures aimed at the ranked openings still showed pools or
horizontal bands. The probe shader stayed in scratch.
[Atlas, pose selection and target captures](../../../.orchestrate/evidence/CLOUDS/WP14/shaft-target-pairs/world-atlas/README.md)
Three fixed Chrome WebGPU captures reproduced the Metal pool and band shapes at matching poses.

WP14.6 and WP14.9 pass the revised inspection. The six retained broken-preset pairs
preserve light in shaded ground and smooth edges in the lit mist. A ridge shaft remains visible
where framed. The bright pools and strips inside the mist under cloud openings are the
documented risk 15 limitation assigned to WP22.
[Metal and Chrome inspection](../../../.orchestrate/evidence/CLOUDS/WP14/VISUAL.md)

The WP14 review repair extends the ground band's height to clear all configured fog, including
fog on terrain above the receiver. Metal agrees with the headless query through the original
96 m boundary and the actual top, and the near medium distinguishes deeper fog above 96 m.
The revised paired cost record includes the four view-volume history textures and both upper
phase buffers, with the shared coefficient texture counted once.
[Sun-path proof](../../../.orchestrate/evidence/CLOUDS/WP14/REVIEW-SUN-PATH.md) · [Cost](../../../.orchestrate/evidence/CLOUDS/WP14/COST.md)

## WP15 Far mist and low cloud

Ground marches once per 2×2 block and stores a transport stop at every render pixel's depth.
The late effect has scene depth and lacks a scene-motion lease, so Ground uses the design's
full-trace fallback. The low deck shares the weather columns, shape erosion, Sun term and
distance weights between the trace and view volume.

Metal verifies the bank depth matrix, the mover and cut, and byte equality for the retained
no-Ground scene. Metal and Chrome show clean dawn-mist edges and complete the 240 m handover.
The 5° mesa controls show ridge shade entering the far mist.
[Correctness](../../../.orchestrate/evidence/CLOUDS/WP15/CORRECTNESS.md) ·
[Visual inspection](../../../.orchestrate/evidence/CLOUDS/WP15/VISUAL.md) ·
[Metal costs](../../../.orchestrate/evidence/CLOUDS/WP15/COST.md)


The surveyed 40 m view at 5° on Metal shows cloud shade darkening the mist. Matching
fair, clear and dry controls trace the cloud shadow across the valley floor. The mesa
clear control preserves the separate ridge shadow in the mist.
[Shadow inspection](../../../.orchestrate/evidence/CLOUDS/WP15/continuation/SHADOW.md)


The low deck reads Sun depth on the cloud height grid and adds only the ground band's fog
depth. This removes the repeated summit bands. The new V1 profile retains the level base
and gives the lit deck visible relief. Metal and Chrome show the mesa's soft cloud margin
and a continuous climb through the base and top. Inside, surveyed surface texture fades
over roughly 20 to 53 m. The cost pair was rerun at the same 1440p Metal dawn camera.
[Metal deck](../../../.orchestrate/evidence/CLOUDS/WP15/continuation/DECK.md) ·
[Chrome mist and deck](../../../.orchestrate/evidence/CLOUDS/WP15/continuation/CHROME.md)


The WP15 review repair keeps elevated-cloud samples in the trace with active ground rows.
Metal replays at three altitudes preserve the upper sky when mist is enabled, and the
no-Ground control matches the starting frame byte for byte. The 65×37 resize probe reads
all 2,405 pixel stops. Ground declares its fixed layer scale. A combined low deck and
placed-volume probe checks the query, trace and sky overlay under portable Metal limits.
The mist-far validator covers the retained and fresh three-arm cost records and rejects
missing arms, trace data and investigations.
[Review repair evidence](../../../.orchestrate/evidence/CLOUDS/WP15/review-repair/REPAIR.md)

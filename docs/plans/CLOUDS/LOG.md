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

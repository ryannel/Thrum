# PUDDLES log

- 2026-11-14 The owner, about the `SEA` module: "Please make sure the sea work is actually thinking about water
  as a whole. So puddles, rivers, water falls, streams, lakes, and more. We want something that's built
  cohesively." The lead's task for this plan: "puddles and wet ground as part of the one water system, at the
  quality of shipped high-end games".
- 2026-11-14 Planned from [the module list's brief](../ARCHITECTURE.md#rivers-and-puddles) and
  [WATERS' design](../WATERS/DESIGN.md#what-the-later-modules-build-on). Seven milestones: two on the CPU after
  `WATERS:M1`, three on the island after `WATERS:M8`, the V1 valley after `SHOWCASE:M6`, and a close after
  `WATERS:M10`, `M14` and `M16`.
- 2026-11-14 The seam question the brief carried, how puddles that overflow merge and part, decided: all the
  hollows of a stretch of ground are one body in the map, a hollows body, whose level at a point is the level
  of that point's hollow. A tree of hollows is prepared from the bed (Barnes, Callaghan and Wickert's
  depression hierarchy), water volumes are stepped on the CPU by fill, spill and merge, and the map reads a
  label and a level. Nothing is rebuilt at a merge, so the body's identity, its query and its rings hold. Not
  chosen: a basin for each hollow, one flood level over a mask, and a live shallow-water solver.
  [Reproductions 1 to 6](../../../.orchestrate/evidence/PUDDLES/design-proof/README.md) were run for it.
- 2026-11-14 Replaced, from `WATERS`' design and the interface table: "a puddle is a basin whose stage rain
  raises" and "`PUDDLES`' drops are the same record". A bed 3.2 m square holds 20 hollows whose fills overlap
  in 10 pairs, and a square metre takes thousands of drops a second. Rain rings are two wave layers that are a
  function of time, summing each scheduled drop's own wave train; a footfall is the game's impulse record.
- 2026-11-14 Rings from a footfall, decided: `WATERS`' interaction operator on a third, fine level of 2 cm
  cells with a ladder from 5 mm and surface tension. Its near level's cell is a quarter of a puddle. The fine
  level steps one tick at a time: steps of eight ticks lose up to 42% of a wave's height a second (part 6).
- 2026-11-14 Wet ground, decided: a function of prepared fields and stepped stores, not a field of state. Two
  stores for each ground class, surface water and pores, at three exposures; darkening from the light the film
  returns to the ground; and water standing in the ground's relief drawn as `WATERS`' film, so wet paving
  mirrors scenery. Not chosen: one global wet level, and a wetness texture stepped on the GPU.
- 2026-11-14 Shelter, decided: prepared content, a canopy grid from the triangles a game says shelter, because
  it sets how fast a puddle fills and that may not depend on a camera. Rain falls straight.
- 2026-11-14 [Seven rulings](../ARCHITECTURE.md#seam-rulings-for-puddles) settle what is built in `WATERS`'
  crates, the forward shader and the two games. No item of another module changes.
- 2026-11-14 Read as summarised by fetch: Lagarde's Water drop 2b, 3a and 3b, and the method of Fill–Spill–Merge.
  Abstract only: Le Méhauté 1988; the Far Cry 6 weather session's description. Not found: any talk on the rain
  or wet ground of The Last of Us Part II, Red Dead Redemption 2, Forza Horizon, Spider-Man, Alan Wake 2 or
  Cyberpunk 2077. From memory: Marshall and Palmer's drop sizes, Atlas's fall speed and the form of Lekner and
  Dorf's model, which part 4 derives again and which gives 0.55 where Lagarde quotes about 0.68 for asphalt.
- 2026-11-14 Design review asked for changes: [five Must Fix findings](../../../.orchestrate/evidence/PUDDLES/design-review.md).
  All five are resolved in the design, the items and [the rulings](../ARCHITECTURE.md#seam-rulings-for-puddles),
  with [reproductions 7 to 10](../../../.orchestrate/evidence/PUDDLES/design-proof/README.md).
- 2026-11-14 The mirror (finding 1). A hollows body is no longer one plane at the mean of its ground, which
  cut the foot off everything standing in a low puddle. New ruling 8: it gives the mirror its levels in
  bands no taller than 5 cm, each cut at its lowest water, ranked with every other level; a lookup refuses
  a sample below its own surface. Part 7: one plane fails whichever height it takes. Declared: a strip
  behind a thing standing in water above its band's plane, and two views for however many bands. M7.2 is
  restated and M7.12 added. Not chosen: a cut for each thing at its own water, which ties a view to scenery.
- 2026-11-14 Rain that stops (finding 2). The rain state keeps the rate of each tick of the last second and
  a drop is born by the rate at its own tick, so rings run out their second after a stop and none is born
  in a dry spell (part 9). The bands' passes stop when the history is empty. M2.6, M4.1, M4.2 and M7.1 are
  restated to it, and M4.12 looks at a stop and a restart.
- 2026-11-14 First-hit receivers (finding 3). The canopy grid holds two shelters in a column and exposure is
  the product of those above a point; a third is refused (part 10). Rain on a puddle is the puddle's; the
  ground's stores describe uncovered ground only, with entries in the account when water covers ground and
  when it leaves. M1.5 has the crown under the roof; M1.13 and M2.11 judge each receiver alone.
- 2026-11-14 Saturation (finding 4). A ground class has a soak rate into its pores and a percolation rate out
  of them. Earth takes a sheen once its pores are full, 1,080 s into the review's shower, and only ground
  that passes water down faster than the rain never does (part 8). M1.4, M2.1 and M5.5 and cue P6 say so.
- 2026-11-14 The fine interaction level (finding 5) moved out of M2 and M4 into a new milestone, M8, built
  after M4 and waiting for `WATERS:M6`, whose reference it extends. M2 and M4 no longer need it.
- 2026-11-14 Suggestions taken, both. The damp band has a state rule: a fall adds, a rise takes away, and it
  decays at the pores' rate, so it is judgeable when a level stops, a puddle empties or drying is off
  (M2.3). A GPU bound measured first is accepted only at a tenth of what a named wrong evaluator makes on
  the same case (M4.1, M5.2, M8.3).
- 2026-11-14 Not run by the planner: any GPU check, the bands' ranking as a camera moves, and the stores with
  the covering transfers in a full fill step. Part 1's account was run without stores.
- 2026-11-14 `SCENE-VIEWS`' third design review: a view's air may start only at a level plane. A band's
  plane is its lowest water, which is level, so ruling 8 and M7 stand. No text, item or bound here changed.
- 2026-11-14 Dependency audit ([the dependency audit](../../../.orchestrate/evidence/dependency-audit-2026-11-14/README.md)). M6 now waits for `SHOWCASE:M5` in place of `SHOWCASE:M6`: it needs
  the exit scene and its props, which M5 delivers, and `SHOWCASE` M6 changes no pixel of it. No design text or
  item changed; one assumption names the new milestone.
- 2026-11-16 M1 built (code 78c4af12b). Verified: thrum-rain-model (tree of hollows, fill step, account, canopy,
  rain state) and hollows bodies in thrum-water-model; all M1 tests pass, clippy clean, water-model tests unchanged.
  Cost record: 640,000 cells prepared in 0.082 s into 624 hollows; a fill step takes 71 µs on 1 thread. M1.12 is left
  to the main-branch check exact-images. Fixed in building: a component meeting the edge-joined one spills into the
  hollow across, not off the rectangle. Suspected: the threaded step loses to one thread at this size (spawn per tick).
- 2026-11-16 M1 review fix (code 2ae13f69c). Verified: rain on water now goes to its first hit at the water's
  surface; each shaded cell keeps its exposure by height. receivers.rs checks every receiver on every tick
  as a puddle rises past a low slab, plus the review's covered floor; both tests fail on the old code. All
  rain-model tests pass, clippy clean; the cost record was taken again (fill step 71 µs on 1 thread). Byte
  counts are now labelled payload. Not done: thread reuse in the fill step (review suggestion).
- 2026-11-16 M2 built (code 61ffd6b7f). Verified: ground stores per class at three reaches read by RainState with
  cover and uncover transfers; damp bands on hollows and shores; the wet look; ring tables and two rain bands
  summed in thrum-water-model; conformance reports ignored exposure. All M2 tests and water-model tests pass,
  water-model tests unchanged, workspace clippy clean. M2.10 is left to the main-branch check exact-images.
  Not verified here: `./dev check` hit the 30-minute gpu hold in other crates' GPU tests under load; no failure
  before that. Suspected: the stores' interpolation between reaches is the largest wetness error (0.23 of the pores).
- 2026-11-16 M2 review fix (code a0aea615f). Verified: no water is made when a puddle can't pay for ground it
  covers or leaves; the shortfall is owed by the hollow (`RainState::owed`), paid first from its next water, and
  owed pores soak nothing. `ground_held` counts the debt; `Account::unpaid` is removed. New tests run the review's
  dry flat floor (worst residual 3.7e-13 of the rain) and check every entry, donor and receiver per tick. All
  rain-model tests pass; clippy clean. Suspected: an owed hollow reads its ground as the class's (wetness ignores
  the debt); a planner may want the design to name the debt.
- 2026-11-16 M2 second review fix (code 4fd75c5e8). Verified: an owing hollow's ground no longer spends water it
  doesn't hold. Its drying and percolation take only the reading's real share, the debt fades by the rest, and
  `hollow_ground` never goes below zero. The review's 12 short-shower cases pass through 3 h of drying, ending dry
  with nothing owed, and so does an owing account under the crown through merges. Its probe gives min_ground 0.
  Suspected: when an empty puddle uncovers ground, the film it couldn't leave is a debt, not water on that ground;
  the design's text assumes the puddle has it. A planner may want the design to name the debt.
- 2026-11-16 Merge check failed in `v1-game` `cooked_assets_gpu::discarded_frames_leave_no_trace_on_cooked_uploads`
  ("part of the edit was committed"). Verified: v1-game links neither M2 crate; on main f41ebc97b, under the gpu lock,
  the test passed 3/3 alone and the whole file 13/13 three times. Suspected: a load-dependent flake (the assert needs
  one frame to commit upload bytes) while other builders share the GPU; it belongs to that test's owning module.
- 2026-11-16 M2 failed its third review on M2.11, the finding of the second again: an emptied puddle left no film
  on ground it uncovered and the model owed it. The builder ended Blocked without changing code
  ([handoff](../../../.orchestrate/evidence/PUDDLES/M2-handoff.md)): the design's covering rule did not say what happens when a
  puddle holds less than the pores it rises over or the film it must leave, and on ground with deep pores that is
  the ordinary case.
- 2026-11-16 Re-planned by the lead's task: "Settle a conserving rule (water is never invented or silently owed),
  rewrite the design section and M2.11 to match, and say which existing tests change." Decided: the pores under a
  puddle take its water at the class's soak rate, not at once; each hollow keeps a mark, ground changes hands only
  in its class's state, and ground the puddle cannot pay for is held until it can
  ([design](DESIGN.md#the-mark-and-ground-that-changes-hands)). Not chosen: a debt named in the design, a named
  source, ground left uncovered until its pores are paid for, and the old rule limited to shallow pores.
- 2026-11-16 [Part 11](../../../.orchestrate/evidence/PUDDLES/design-proof/README.md) runs the rule on one hollow: 3,024 cases close
  to 4e-13 with nothing below none, and ground is held in 75, on a floor half under a crown and in a dish as
  shallow as its relief. Not run: the Rust model, and hollows that share a puddle. Read for the soak rate:
  hydrology course texts on ponded infiltration, as summarised by one search.
- 2026-11-16 Items: M2.11 rewritten as the receivers and the bed's take, M2.12 added for the mark and held ground,
  both on a new test file, `tests/marks.rs`; M2.1's last sentence names the pores under the marks. Tests that
  change: the three M2.11 cases in `tests/receivers.rs` and the four coupled account cases in `tests/stores.rs`
  are replaced; `tests/state.rs` and `tests/margins.rs` follow the state's fields. M1 shrinks to its review link.
- 2026-11-17 Design review of the re-plan asked for changes:
  [one Must Fix](../../../.orchestrate/evidence/PUDDLES/design-review.md). Rain on held ground was given to the puddle because
  the hollow keeps that ground's account, and held ground stands above the water, so C09's first hit is the
  ground. Its probe: on part 11d's dish at 1 mm/h the puddle was given 5.1e-8 m³ a tick where 2.8e-9 falls on
  water. The conserving transfers, the mark and the hold stand as approved.
- 2026-11-17 Decided: a mark says only whose account keeps a stretch of ground's water. Rain is assigned from
  the bed, the shelters and the level at the tick's start: to the puddle on water, by the exposure at the
  water's surface; to held ground by the exposure at the ground, where its share of the pores under the mark
  takes it at the class's soak rate, then down at the percolation rate, and the rest is runoff to the puddle;
  to open ground elsewhere. Held ground stands no film, a declared approximation. Not chosen: a film store for
  each hollow's held ground, a third store with its own drying and handover for ground held in 90 of 3,528 cases.
- 2026-11-17 Items: M2.11 rewritten as the three receivers, each summed from the bed and the level and never
  from a mark, with the review's dish at 1 and 100 mm/h and a floor held with room in its pores. The bed's take
  moved from M2.11 to a new M2.13. M2.12 no longer says held ground's rain is the puddle's, and its held case
  has rain on it. All three stay on `tests/marks.rs`. No dependency, lock or other milestone changed.
- 2026-11-17 Suggestions taken, all three. M2.1 names both rates for ground that takes no film, with the
  review's case of a class that percolates faster than it soaks. M2.3 says a rise takes its own height from
  the band and gives the steady fall's range; the recurrence stays the authority. Part 11's sketch takes the
  percolation rate over full pores (11g), M2.13 holds both orders of the two rates, and the sweep has a class
  that soaks 1 mm/h and percolates 15.
- 2026-11-17 [Part 11](../../../.orchestrate/evidence/PUDDLES/design-proof/README.md) run again with the rule: 3,528 cases close
  to 4e-13 with nothing below none; ground held in 90, with rain on the held ground in 6. 11f sums each
  receiver from the bed, the exposure and the level and matches the step. The review's probe still reproduces
  its witness state on the sketch; its last assertion, that the sketch takes the soak rate over full pores,
  now fails, as intended. Not run: the Rust model, hollows that share a puddle, and M2's item checks other
  than `--test marks`, which fails for want of the file, as it should.
- 2026-11-16 M2 third review: Blocked on M2.11, no code changed ([handoff](../../../.orchestrate/evidence/PUDDLES/M2-handoff.md)).
  Verified on the review's witness: the floor's pores have room for 4.8e-4 m³ and the whole shower is 2.1e-4 m³,
  so the puddle can't fill the pores it rises over and holds nothing when it uncovers. Proposal for a planner:
  the design's covering rule must say what happens when a puddle holds less than a transfer. Suspected: no
  shortfall arises without unfilled pores (three no-pore showers showed none).
- 2026-11-17 M2 rebuilt to the mark (code 2ec287003). Verified: `tests/marks.rs` works every entry of every tick
  from the cells, the stores and the state before it, and the model matches on the review's twelve floors, a
  dish that holds ground for 1,645 ticks, a floor held in the rain and two hollows through a merge and a parting;
  the account closes to 1.7e-13 on the design's bed. `owed` is gone, and a class may percolate faster than it
  soaks. All rain-model tests pass; clippy clean. [Evidence](../../../.orchestrate/evidence/PUDDLES/M2-evidence.md). Suspected:
  nothing else reads the saved rain state, whose format changed (`THRRAIN3`).

- 2026-11-17 Human: "Ok let's lower the bar." The most plausible result as fast as possible, not physical accuracy; references become
  diagnostics. Plan: dropped M4.3, M8.1, M8.2 and M8.5 (a ring's place and the fine level's CPU reference). M4.1 and M5.2 keep the
  GPU against the model with a tolerance the test states and their wrong evaluators; M3.3, M3.4 and M6.2 name `WATERS` M4.1's
  tolerance; M5.3 and M8.3 state the look's rule without a reference. Five cost records pass on their check alone. No item of a Done
  milestone changed.

- 2026-11-18 Design review: changes requested, one Must Fix. Dropping M8.1 on 17 November also dropped the only check that the
  near level gives up the waves under 1 m when a profile names the fine level; a 0.64 m wave fits both grids and two copies still
  look like a ring. Plan: new item M8.9 tests it on the GPU's own level state, with and without the fine level, and has the
  reviewer read the test; M8.3 is as it was. Design: the footfall section states the split (complementary filters crossing at
  1 m, by wavelength wherever the source lies, so a source outside the fine window raises no wave under 1 m). Suggestion taken:
  the module handoff and ruling 2 now name `WATERS:M6`'s GPU interaction step, not a CPU reference. M2's note left the plan, as
  M2 is Done.
- 2026-11-20 Planner, from the cleanup review (L8-07): M7.12's ask named the design proof under `docs/evidence`, a path that no longer exists; it now names the copy in the module's evidence folder. M7 was Not Started.
- 2026-11-26 WP3 partial checkpoint `bdb8cdccb`. Verified on Metal: unchanged island body and stage tests, the
  hollows point test at five fills across a merge, and surface pipeline submission in an engine fixture.
  The yard, float and device costs remain unproved. The yard specifies multiple ground classes and water
  types inside one hollows body, while the model steps one ground class per body and the renderer binds one
  optical type per contribution. Proposal: separate paving and packed-earth hollows bodies and authorize
  per-body optics. [Handoff](../../../.orchestrate/evidence/PUDDLES/WP3-handoff.md)
- 2026-11-26 Authored the yard's paving, packed-earth ruts, roof, posts and wall beside the lake. Verified the
  baked mesh triangles share the paving-track edge, the 15 cm paving pit joins the shallow dip at a sill below
  the outlet, both ruts retain water, and the roof covers four posts. The fifth post crosses the pit floor.
  [Geometry and cooker records](../../../.orchestrate/evidence/PUDDLES/WP3/yard.md)
- 2026-11-26 Connected the yard mesh, canopy, rain model, level uploads and small float to the selected island
  world. The yard moved from (146, 42) to (144, 61) after full-stage map preparation caught the old plot in
  the lake's allowed shore margin. The model test now observes the paving depressions fill separately,
  join and part, the float's sampled level, dry ground under the roof and stable body identity. This
  checkpoint uses one uniform ground class and one optical profile across the yard.
- 2026-11-26 Paving and track now keep separate ground stores behind their raised divider while
  publishing one hollows rectangle with stable labels. Optional per-hollow sediment optics let the
  track hide its bed and leave the paving on the ordinary clear-water path. The selected route uses
  the common water gather and tapers only the thin compositing contact after transmission.
- 2026-11-26 WP3 connects the rain selection to one hollows body on the island. Paving and track have
  separate ground stores and share stable labels and display levels. The rendered opaque bed now
  supplies the optical contact depth. The lower paving pit reaches the sill before the upper
  hollow. Metal checks cover map, submitted depth, float and history through a merge and a parting.
  Chrome WebGPU checks the selected yard and the float. The evidence includes both motion takes,
  native-size waterline crops and the paired cost record. [WP3 evidence](../../../.orchestrate/evidence/PUDDLES/WP3/README.md)
- 2026-11-26 WP3 review repair. The ordinary GPU map reader samples hollows, and query snapshots
  retain level-only changes. The yard float consumes actual body-query replies and the two dry
  columns are checked through the same query. The surface submits the wet tiles and their margins
  from a prepared index buffer. Submerged paving keeps visible grout with enough tint to follow
  the sill from overhead. [Repaired Metal, Chrome and cost evidence](../../../.orchestrate/evidence/PUDDLES/WP3-repair/README.md)
- 2026-11-26 WP4 adds the two GPU rain bands and their filtered moments to the water surface. The
  schedule uses 256 drops per square metre per second at full share so 10 and 50 mm/h retain distinct
  ring counts. The unresolved slope integral keeps the model's steady value and uses a smoother age
  envelope at the first and last part of each ring's life. Metal checked band values, canopy and
  depth gain, repeat and endpoint capture, and the tick query at two display cadences. Chrome WebGPU
  drew the yard and lake and skipped the passes after decay. The paired 1440p record covers the authored
  island at the yard and lake and includes interaction region labelling. [Evidence](../../../.orchestrate/evidence/PUDDLES/WP4/README.md)

- 2026-11-26 WP4 repair. The tick query filters rain by its footprint and follows the prepared current
  tracks, including their renewal and height-rate correction. Clearing a prepared map's rain model clears
  the visible field and skips rain compute. The rain planes share the cascade array and the canopy is a
  sampled texture. The cost check investigates passes against WP3's matching yard-brim arms as well as
  against the current controls. [Repair evidence](../../../.orchestrate/evidence/PUDDLES/WP4/review-repair.md)
- Proposal for the planner, WP4.4. At sparse rates an individual birth in a periodic field repeats at that
  field's period, even when the second field gives the combined pattern a longer period. Give the scheduled
  patterns a world-space repeat distance beyond the 6 m inspection span. Recheck crest sharpness, stop and
  restart, and the memory and dispatch costs before settling the physical profile and resolution.

- 2026-11-26 WP4 schedule repair, approved by the lead. The periods are 6.4 m and 6.4 × √(5/4) m
  (about 7.155 m). Both exceed the 6 m inspection span and their ratio is irrational. At lower rain rates,
  each of the earlier periods could repeat an individual ring even though the combined field had a long
  period. Keep the two 512² patches and the rate history. Bin periodic images in 64² tiles, reject tiles
  outside each drop's reach, and filter its normalized ring table in hardware. [Schedule repair evidence](../../../.orchestrate/evidence/PUDDLES/WP4/schedule-repair.md)

  The fixed world-space footprint thresholds keep longer periods from extending fine sampling. At 50 mm/h
  the rain base pass measured 0.95–0.98 ns per output texel on M3 Max Metal, down from 1.12–1.16 ns. All
  five executable items passed. The native captures and independent inspection cover sparse births, three
  ages of one ring, the heavy shower and shelter, the distant reflection, and both stop and restart views.

- 2026-11-26 Design review of the longer rain periods asked for changes:
  [one Must Fix](../../../.orchestrate/evidence/PUDDLES/design-review.md). The game's rain queries drew the
  drop schedule again for every sample of every station's footprint, 30 to 57 ms for the example's 54
  stations at the new periods where a tick is 16.7 ms, and no cost record covered it.
- 2026-11-26 Must Fix 1, decided: the design's [rain rings](DESIGN.md#rain-rings) now says what a query
  reads. The rain's share is added in the drain for the use tick, from one list of the live drops a tick
  shared by the batch and a prepared footprint, a ring's mean over the station's disc. Not chosen: a share
  read from the bands' textures, and one list a tick with the footprint still integrated on the tick.
  [What changed, the candidates and their times](../../../.orchestrate/evidence/PUDDLES/design-revision-2026-11-26.md)
- 2026-11-26 Items: WP4.10 written again, so its record also holds the drain of the example's 54 stations
  in rain and dry, with a wire at a tenth of the tick. WP4.13 added: the model's footprint read against a
  direct average over the disc at three radii, and the mailbox against the read, because WP4's test compared
  them only at a point and at a 3 m hull, which a read with no footprint in between would pass. Suggestions
  taken, both: the handoff names the tracked provenance record, and WP8.9 opens with each wave counted once
  in place of "no wave is on two interaction levels", which the crossing at 1 m contradicts. Not run: WP4's
  checks, whose commands are unchanged.

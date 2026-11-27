# RIVERS log

- 2026-11-14 The owner, about the `SEA` module: "Please make sure the sea work is actually thinking about water
  as a whole. So puddles, rivers, water falls, streams, lakes, and more. We want something that's built
  cohesively." The lead's task for this plan: "streams, rivers and waterfalls as part of the one water system,
  at the quality of shipped high-end games".
- 2026-11-14 Planned from [the module list's brief](../ARCHITECTURE.md#rivers-and-puddles) and
  [WATERS' design](../WATERS/DESIGN.md#what-the-later-modules-build-on). Seven milestones: three on the CPU
  after `WATERS:M1`, three on the island after `WATERS:M9`, and a close after `WATERS:M10` and `M16`.
- 2026-11-14 The solve, decided: the steady state of the 2D shallow-water equations with Manning friction over
  the map's bed, marched in pseudo-time on wet cells from filled pools, stored as a river file with a key. Not
  chosen: a spline, routing by steepest slope (no level pool, no jump), an offline particle simulation, and a
  live solver. [Reproductions 1 to 7](../../../.orchestrate/evidence/RIVERS/design-proof/README.md) were run for it.
- 2026-11-14 Found in the reproduction and written into the design as properties of the scheme: Audusse's
  reconstruction stands a thin sheet on steep ground 23% too deep, where Chen and Noelle's is within 5%; and
  friction taken at the new discharge inside a pseudo-time step shifts the steady state with the step; and a
  lip modelled as a drop to a dry cell draws the water above it down to 0.22 of its critical depth, which
  would throw a sheet 3.2 times too far, where a face that passes the flow at its critical depth does not.
- 2026-11-14 Replaced, from `WATERS`' design: "a reach keeps its prepared flow whatever its source's stage".
  Over 0.6 m of lake level an outlet's discharge changes 80 times (part 3), so a river is solved at source
  levels 0.3 m apart and the map reads between them by a gauge.
- 2026-11-14 Replaced, from `WATERS`' design: "roughness from the flow is a gain on the shortest cascades". A
  gain on the wind's waves gives a stream nothing in a calm. The flow has two wave layers of its own, at a gain
  from the bed's friction: the chop's rms height is twice the depth times the friction slope, capped at the
  depth limit.
- 2026-11-14 The chart's loss, decided: the power that breaks the surface, head lost beyond bed friction plus
  friction's power beyond the chop's cap. Foam births read it by the shallows' own constant.
- 2026-11-14 A fall, decided: a brink is a property of the bed alone; the sheet follows the water's free path
  or the rock's face; its pattern is read by the time each part left the lip, so streaks fall and stretch with
  the water; its plunge gives births from the fall's power and impulses from the unsteady share of its
  momentum. Not chosen: a card with a scrolling texture, and particles. Mist is not planned.
- 2026-11-14 One mean current for an interaction window, judged: it slides rings in a pool 4.7 m in 5 s beside
  a riffle (part 7). Each region's mean is weighted by its waves.
- 2026-11-14 [Seven rulings](../ARCHITECTURE.md#seam-rulings-for-rivers) settle what is built in `WATERS`' crates.
  No item of `WATERS` or `SEA` changes.
- 2026-11-14 Read whole: the Uncharted 4 rapids slides and notes, and pages 26 to 36 of the Horizon Forbidden
  West water notes. Not read: the Far Cry 5 water talk, any account of Red Dead Redemption 2's rivers or
  falls, and measurements of a falling sheet's breakup. Horeni's length, the weir relation and the surface's
  turbulent energy are from memory.
- 2026-11-14 Design review asked for changes: [three Must Fix findings](../../../.orchestrate/evidence/RIVERS/design-review.md).
  All three are resolved in the design, the items and [rulings 2 and 5](../ARCHITECTURE.md#seam-rulings-for-rivers).
- 2026-11-14 Dry support. A dry cell's level always lies under its bed, and between two samples each cell
  has a prepared wetting gauge: the least gauge at which wet cells join it to the lower sample's water. Below
  it the cell has no water, wave support or margin, so a hollow behind a ridge is dry until the ridge wets.
  A bank cell's answer is the linear mix it was. [Part 8](../../../.orchestrate/evidence/RIVERS/design-proof/README.md)
  reproduces the review's 0.80 m of invented water and its removal. M1.6, M3.2 and M4.3 judge the wet
  cells as well as depth and current; a river file gains a seventh value.
- 2026-11-14 Rings on running water. The weighted mean is replaced by a current ladder in the interaction
  step: a few carriage currents for each region, each cell blending the two that bracket its own, as the
  depth ladder does. Part 9: two ring sets at once on a pool and a riffle stay within 0.10 m of their own
  water, where any one current leaves each 3.25 m off. New item M6.10 judges it with a second source in
  the same region; M6.3 keeps the plunge. Declared: 0.2% to 1.3% of a midway wave's energy lost a second.
- 2026-11-14 M1.6 no longer asks for the solver's row inside a join. The stored chart equals the solve; the
  map equals it where the join's weight is nothing, and `WATERS`' joined row, backwater included, elsewhere.
- 2026-11-14 Suggestion taken in part: M6.4 looks at the fall from 2 m and 200 m as well, and the design and
  M7.11's ledger say that only the island's fall of about 4 m is judged by eye. Short and tall falls for
  the looks alone were not added to the island: they are scenery no cue of the island needs.
- 2026-11-14 Not run by the planner: the ladder in plan or on the GPU, and wetting gauges on a solved valley.
- 2026-11-14 Second design review asked for changes: [one Must Fix finding](../../../.orchestrate/evidence/RIVERS/design-review.md).
  A reach whose lower sample is wholly dry had no wet cell to flood from, so none of its cells took a
  wetting gauge and it never wetted.
- 2026-11-14 Wetting restated: water spreads from where it is. The flood starts from the lower sample's
  wet cells and from each inflow place the lower sample leaves dry, a spring's cell or where water arrives
  from the reach above, keyed at the gauge that inflow starts; reaches are flooded from the sources down.
  A wet cell passes water to a neighbour when its own level stands over that neighbour's bed. A part of a
  fall that only the upper sample runs starts at its lip cell's wetting gauge.
  [Part 10](../../../.orchestrate/evidence/RIVERS/design-proof/README.md): the review's 32 cells all take the lip's gauge,
  and part 8's answers are unchanged. Not chosen: an extra flow sample at the gauge a lip starts, which
  costs solves and still needs the seed.
- 2026-11-14 Replaced in that rule: a cell's own crossing, taken from its two stored levels. Above a wet
  channel it held a newly fed branch dry until 0.98 of the way, and in a dry reach it depended on the
  margin a dry level is stored at (part 10). The solve now writes the gauges; the map still only reads them.
- 2026-11-14 Items. M3.2 keeps the mix and says its depth and speed bound is for cells both samples wet
  (suggestion 1, taken: the design already said so). New M3.8 holds the wetting: the hollow, and a reach
  that goes from dry to running. M3.3 and M4.3 gain the lip that starts to run and the dry reach on the GPU.
- 2026-11-14 Suggestion 2 taken, in the design only: carriage currents are vectors chosen so their span
  holds a region's currents, not picked from its cells. Part 11: round a 90° bend at 1.5 m/s, three of the
  cells' own leave cells 0.11 m/s outside the span, 0.57 m after 5 s; rest and three on the bend's tangents
  hold them all. Ruling 5 says so. M6.10 is unchanged and still measures a turning window.
- 2026-11-14 Not run by the planner: the solver's own samples with a lip that starts to run, wetting gauges
  on the GPU, and waves blended between three carriage currents.
- 2026-11-15 M1 built ([evidence](../../../.orchestrate/evidence/RIVERS/M1.md)). Verified: the valley holds discharge to 0.139%,
  pool level to 5.3 mm at 1.73 critical depths over the sill; closed forms within the items' bounds; WATERS
  takes the chart; one and eight threads give the same bytes; 1.31 s on one thread, 9,798 updates a wet cell.
  By construction, not measured: the energy account closes because the face-based head loss telescopes;
  friction power is computed apart in the test. M1 limits: first order in space; a mouth must be one straight
  row or column. Suspected, not tested: the active-span cost stays flat for rectangles much larger than 2×.
- 2026-11-16 M1 review fix (M1.5, the step budget): the solver's last batch now stops at the budget, so a
  solve is refused at exactly its budget or converges inside it. Verified: `tests/report.rs` covers an
  interval longer than the budget and partial final batches; the reviewer's probe now refuses budget 1 at
  step 1 for intervals 10 and 10,000 ([output](../../../.orchestrate/evidence/RIVERS/M1-budget-probe-after-fix.txt)).
- 2026-11-16 M2 built ([evidence](../../../.orchestrate/evidence/RIVERS/M2.md)). Verified: the lip stands at 1.112 hc where the flow is slow and 1.017 of normal depth where it is fast;
  discharge across the fall is equal to 5 digits; throw against the closed form within 1.5e-12; plunge rms equals the design's figure. Declared choices:
  air share 0.5; a brink end is the `BRINK` body sentinel, because existing `End` literals can't gain a field; a brink is drowned where the body below
  stands at or above the lip's bed; a path may travel over a receiver's box beyond the rectangle; RIVER_FORMAT and SOLVER_VERSION are 2.
  The sheet pattern is a caller-supplied trait (`SheetPattern`), so the game owns its texture. Suspected, not tested: landing lag of
  one step and per-check recomputation of landings could oscillate on a path that grazes a sill.
- 2026-11-16 M2 review fix (M2.5): a path running down dry rock into the pool now meets the water at the shoreline,
  bisected on the stored chord, not at the first point under water. Verified: a new test over three rock faces and five
  stages with the real map gives the surface's level exactly and the place within 5.1e-5 m of a scan; it fails on the old code.
- 2026-11-16 M3 built ([evidence](../../../.orchestrate/evidence/RIVERS/M3.md)). Verified: the mix between samples is within 5.2 mm and 7.7% of a fresh solve;
  the hollow and the terrace wait for their wetting gauges and rise from nothing; fall records are the samples' mix to 4e-15; the chop
  follows the law to 2e-14; with no bands the water model's answers are identical in bits to before. Fixed: a reach falling into a body
  also got a mouth across the cliff, and its 2 m join left the last 1.6 m above the lip dry in the map; such an end is now a half-cell seam.
  Declared: flow bands have no choppiness; the surf model's long-wave swash does not count the flow bands. M1.1's riffle is
  1.5%, half the design's: its chop is 28× the pool's on average but 19× by rms over each. Suspected, not tested: a landing between a wet
  and a still-dry cell gives no plunge for the first millimetres of gauge (bilinear water under bilinear ground).
- 2026-11-17 M3 after its first review ([evidence](../../../.orchestrate/evidence/RIVERS/M3.md#after-the-first-review-a-chart-as-the-source-body-m32)).
  Built: `Gauge::Chart { body, place }`, a source body that is another river's chart, read as that chart's own level at the place.
  Verified: a lake moves the river above and, through its pool's level, the gauge, charts and falls of a solved river below it in the one
  map; that river's mix is within 2.2 mm and 4.6% of a fresh solve. Not shown: the two rivers joined in space, which needs the river
  above solved with the branch as a way out (a scene for M6). For M4: the GPU must read this gauge too.
- 2026-11-17 M3 after its second review ([evidence](../../../.orchestrate/evidence/RIVERS/M3.md#after-the-second-review-a-refused-river-leaves-the-map-as-it-was)).
  Fixed: a river refused for its banks left the map with support from the refused sample; `add_river` now rebuilds before that error.
  Verified: the review's probe as a test, 3,721 answers unchanged after two kinds of refusal; it fails at 167 without the fix.

- 2026-11-17 Human: "Ok let's lower the bar." The most plausible result as fast as possible, not physical accuracy; references become
  diagnostics. Plan: M5.1, M6.1, M6.2, M6.3, M6.10, M7.5 and M7.11 are rewritten without a bound measured first or an f64 reference, and
  M6.3 and M6.10 no longer run `thrum-water-model`'s interaction test, which `WATERS` M6 no longer builds. M4.3 names the tolerance
  `WATERS` M4.1's test states. Three cost records pass on their check alone. No item of a Done milestone changed.
- 2026-11-18 Design review after the bar was lowered: [changes requested](../../../.orchestrate/evidence/RIVERS/design-review.md), three Must Fix, no
  Suggestions. Plan: M5.10 is new, the query's answers over a windless riffle against the model's flow bands at `WATERS` M5.1's
  tolerance. M6.10 is rewritten so a ring on blended water must keep half its energy and move with that water, with the half from
  [part 12](../../../.orchestrate/evidence/RIVERS/design-proof/README.md) (88.7% at worst); its limits and bit identity move to M6.11. The design's
  open question on the wave share and ruling 5 no longer assign a measured bound or an f64 reference of the ladder.
- 2026-11-18 Design review of the revision: [changes requested](../../../.orchestrate/evidence/RIVERS/design-review.md), one Must Fix. Design: a
  query's demand runs the flow bands on the frame that answers, whatever the cadence and the camera. Plan: M5.6 is rewritten. The
  Suggestion on part 12's 88.7% is taken. [What changed and why](../../../.orchestrate/evidence/RIVERS/design-replan-2026-11-18.md).

- 2026-11-23 M1–M3 planning archive. In the [solver probe](../../../.orchestrate/evidence/RIVERS/design-proof/river_proof.py),
  part 1 gives M1.2’s bounds, parts 2 and 6 give the section and valley, part 3 gives M3.2’s bounds and part 4
  gives M2.4’s closed forms. M3.8’s wetting-gauge cases are in parts 8 and 10.
  [Revision probe](../../../.orchestrate/evidence/RIVERS/design-proof/revision_proof.py) and
  [wetting probe](../../../.orchestrate/evidence/RIVERS/design-proof/wetting_proof.py).

## M8 workspace timing baseline

- 2026-11-23 These are the cases from the passing `747909d1c` workspace check, with seconds from that run.

| Seconds | Test |
|---:|---|
| 9.272 | `thrum-river-model::it flow_bands::with_no_wind_the_riffle_has_the_chop_s_height` |
| 8.708 | `thrum-river-model::it closed_forms::a_slow_flow_with_friction_holds_its_made_depth` |
| 8.611 | `thrum-river-model::it fall_gauge::a_part_only_the_upper_sample_runs_rises_from_nothing_at_its_lip_s_wetting` |
| 8.596 | `thrum-river-model::it fall_gauge::no_running_part_lands_on_a_dry_reach_and_the_terrace_waits_for_its_lip` |
| 8.477 | `thrum-river-model::it gauge::a_spring_fed_river_s_gauge_is_its_own_stage` |
| 8.235 | `thrum-river-model::it fall_gauge::a_part_both_samples_run_is_their_mix_and_grows_with_the_gauge` |
| 8.042 | `thrum-river-model::it gauge::every_chart_of_a_river_takes_the_one_gauge` |
| 7.966 | `thrum-river-model::it report::doubling_the_rectangle_round_the_valley_barely_changes_its_work` |
| 7.892 | `thrum-river-model::it flow_bands::a_sea_a_basin_and_a_chart_that_is_no_river_s_lie_at_their_level` |
| 7.647 | `thrum-river-model::it flow_bands::without_the_bands_every_answer_is_what_it_was` |
| 7.579 | `thrum-river-model::it report::a_solve_stops_on_its_tolerances_and_reports_its_work` |
| 7.514 | `thrum-river-model::it energy::in_the_valley_the_head_lost_beyond_friction_lies_round_the_rock_and_below_the_narrows_and_ledge` |
| 7.435 | `thrum-river-model::it store::a_river_file_decodes_to_the_same_bytes_and_the_same_chart` |
| 7.428 | `thrum-river-model::it valley::the_valley_keeps_its_discharge_fills_its_pool_to_the_weir_and_quickens_in_the_narrows` |
| 7.302 | `thrum-river-model::it chart::the_stored_chart_is_the_solve_and_its_wet_cells_are_the_solvers` |
| 7.251 | `thrum-river-model::it gauge::the_mixed_row_is_near_a_solve_at_the_level_between_and_one_sample_raised_is_not` |
| 7.216 | `thrum-river-model::it chart::the_map_takes_the_chart_and_joins_it_to_the_lake_at_both_ends_of_its_stage` |
| 7.070 | `thrum-river-model::it samples::a_spring_fed_stream_is_solved_at_its_declared_discharges_and_gauged_at_its_spring` |
| 6.716 | `thrum-river-model::it chop::the_chop_follows_the_friction_slope_and_is_far_rougher_in_the_riffle_than_the_pool` |

- 2026-11-23 M8.1 built. The stopping, file and discharge checks use a short spring-to-pool reach.
  The valley retains its terrain features at 0.5 m spacing, and the sampling checks use prepared charts
  and fall records. Verified on the M3 Max with default Nextest. All 19 cases passed in isolation and as
  one focused batch, with a maximum of 0.957 s in each run. The padding control held the declaration,
  interior bed and wet mask fixed while doubling the rectangle with dry rows.
  [Per-case coverage and timings](../../../.orchestrate/evidence/RIVERS/M8/M8.1.md)

- 2026-11-23 M8.1 review fix. The upper-only lips and their dependent cells wet at 3.58 m, 30 mm above
  the sample midpoint. The common fall measures sheet thickness at a point strictly inside its path.
  Verified on the M3 Max. Both tests caught the review's regressions when reintroduced. All 19 cases
  passed again in isolation and in the focused batch.
  [Review fix evidence](../../../.orchestrate/evidence/RIVERS/M8/review-fix/M8.1.md)

- 2026-11-24 The chart-source gauge and one-versus-eight-thread byte tests were outside M8's nineteen
  cases. Both hit the ten-second limit in the workspace batch and alone at `299376222cb1`. Both
  already hit it in the batch at `747909d1cd25` and retried in 5.362 s and 5.746 s respectively.
  The failure revision changes capture code outside these tests. M9 removes the oversized setup
  and keeps both guards in the fast tier.

- 2026-11-24 M9 built. A prepared four-cell upper pool drives a compact solved lower reach.
  The gauge checks both sample intervals and all seven refusals. Its off-sample comparison
  covered 28 unblended wet cells within 1.4 mm in depth and 2.8% in speed.
  A 12 × 94 reach crossed the parallel threshold and gave identical bytes in two serial
  solves and one eight-thread solve. Both tests passed on the first attempt alone and together
  on the M3 Max, with focused batch times of 0.032 s and 0.266 s.
  [Gauge evidence](../../../.orchestrate/evidence/RIVERS/M9/M9.1.md)
  [Determinism evidence](../../../.orchestrate/evidence/RIVERS/M9/M9.2.md)
- 2026-11-26 From `WATERS`' planner, for this module's planner: `WATERS` no longer carries a cascade by two
  cross-faded samples, which turn a wave's phase back. [Its design](../WATERS/DESIGN.md#carriage-by-the-current)
  now carries one in three ways, and a solved river, sheared across its width, takes the third: long ripples
  travel at a run's mean speed and the pattern shears between runs. The flow bands' carriage and WP5.2's
  "within a fifth of the current" rest on it. If it does not serve a solved reach, propose the remedy in
  `WATERS`' log.

## WP10 workspace timing

- 2026-11-26 Main's `workspace-tests` check failed at `ba611f39c2a7` with nothing asserted wrong: on a loaded
  Mac `chop::each_sample_of_a_river_file_carries_its_own_gains` was stopped at ten seconds and again at 30 s
  alone. No commit broke it: the test was outside WP8's and WP9's lists and still solves the 0.25 m valley
  twice on the stepper's threads. New WP10, ahead of WP4, makes it small and keeps it in the fast tier.
  [Measurements](../../../.orchestrate/evidence/RIVERS/WP10/workspace-timing.md).
- 2026-11-26 For the lead, not planned in WP10: five more of this package's tests, three of them `wetting::`,
  passed alone at 13 s to 29 s in that run.
  [The five](../../../.orchestrate/evidence/RIVERS/WP10/workspace-timing.md#for-the-lead-not-planned-in-wp10).

- 2026-11-26 WP4 built the opt-in island valley, keyed spring stream and lake outlet, and GPU sample mixing in
  the shared water map. Metal matched the interior depths and gated wetting to the model and measured seam
  residuals up to 4.8 µm over the lake and tide extremes. The two floats reached the lake and sea. Hardware
  Chrome drew both rivers and read the two packaged keys. Submitted Metal stage sweeps showed both waterlines
  rising and falling. The paired device record includes file sizes, sample times and each tripped comparison.
  The scene outlet uses a 0.006 m/s level-rate stopping bound and 0.004 imbalance bound.
  [Target evidence](../../../.orchestrate/evidence/RIVERS/WP4/README.md)

- 2026-11-26 WP10.1 uses the short spring reach on the calling thread. Both gains for all 16 wet
  footprint cells per sample matched their own solve exactly after decoding, and the slope gain
  rose at the whole discharge. On the M3 Max, isolated wall time fell from 4.29 s to 0.03 s.
  The timer reported 6.64 s of CPU before and rounded both CPU components to 0.00 s after.
  [Timing and check evidence](../../../.orchestrate/evidence/RIVERS/WP10/WP10.1.md)

- 2026-11-26 WP4 review fixes. Sampled mouths retain transport current through backwater and blend wind waves
  into the receiving sheet. The river surface refines the submitted depth along the camera ray. Metal checked
  72 interior ray hits within 40 µm, all three joins in swell and both wetting mutations. At the default stages
  with continuous Metal submissions, the outlet float was afloat in the sea by 36 s in both winds. The spring
  float reached the lake by 100 s in calm and 60 s in trade.
  [Target evidence](../../../.orchestrate/evidence/RIVERS/WP4/repair/README.md)

- 2026-11-26 Lead: WP4.3 failed review twice on the depth the water writes inside a river (up to 1.4 mm
  from the model, then 5.2 mm, against 0.040 mm). The lead ruled that bound not meaningful for a pixel. WP4.3 now holds the
  GPU map's rows and the step across each end's edge at `WATERS` WP4.1's tolerance, and the written depth
  along each river within 0.010 m; [the design](DESIGN.md#proof-and-builder-handoff) has the reasoning. No
  other WP4 item changed.
  [First review](../../../.orchestrate/evidence/RIVERS/WP4-review-1.md) ·
  [second review](../../../.orchestrate/evidence/RIVERS/WP4-review.md)

- 2026-11-26 WP4 review fixes. The shared map supplies one bulk current through backwater. The river surface
  intersects the pixel ray with the sampled chart and swell. Wind-wave motion blends into the receiving sheet.
  Metal sampled both rivers every 1.5 m at four stages, with up to 19.5 µm depth error and 1.9 µm at the joins.
  The opt-out source matched the pre-WP4 assembly byte for byte. The target record holds the rerun float courses,
  Chrome, captures and paired cost.
  [Target evidence](../../../.orchestrate/evidence/RIVERS/WP4/final-fixes/README.md)

- 2026-11-26 Design review, Must Fix 1 (WP4.3 no longer said the hollow and the dry reach are *drawn* dry):
  WP4.3 is rewritten in three parts, the map, the draw and the joins. The draw now also holds in the wetting
  scene: at gauges either side of each wetting, at the hollow's middle and along the reach, the water writes
  nothing above the ground while the map has the place dry and is within 0.010 m once it is wet. Its `ask`
  adds the reviewer's case, a right map with the hollow or the reach drawn from the upper sample below its
  wetting. [The design](DESIGN.md#proof-and-builder-handoff) gives the reason. The map's rows, the 0.010 m
  along the island's rivers and the joins' bound are unchanged, and no other item changed.
  [Review](../../../.orchestrate/evidence/RIVERS/design-review.md)

- 2026-11-26 WP4.3 now draws the wetting scene as well as sampling its map. The fall reach starts
  dry, so its first-sample bounds supplied no mesh. Sampled chart bounds now cover the declared
  footprint for all gauges. The Metal depth test covers the hollow and four stations along the
  fall reach at seven gauges. Forcing either feature to draw from the upper sample below its
  wetting fails the depth test while the map test passes.
  [Target evidence](../../../.orchestrate/evidence/RIVERS/WP4/wetting-draw/README.md)

- 2026-11-26 WP4.3 review fix. River depth and plain receiving-sheet ownership now read the
  world column at the ray intersection. The column holds its map gain while it inverts the
  cached horizontal swell displacement and reads the active short waves at the same column.
  The Metal transects retain the example's choppiness and cover stored and interpolated gauges.
  A second Metal guard compares a submitted lower-outlet column to a body query from the same
  field time with the full trade spectrum. [Target evidence](../../../.orchestrate/evidence/RIVERS/WP4/choppy-depth/README.md)

- 2026-11-26 WP4.3 review fix. At stage-controlled mouths, body queries use the same receiving wind-wave
  phase as submitted depth. The prepared join selects the receiving phase. Bulk current continues to come from
  the map. Metal compares full-trade submitted depth and live query at 143 stations per gauge setting, over
  four settings, and across 180 successive frames at the reported tidal column. The largest difference
  was 0.231 mm against 10 mm. The f64 choppy-swell, map, wetting and join tests also pass.
  [Target evidence](../../../.orchestrate/evidence/RIVERS/WP4/receiving-phase/README.md)

- 2026-11-26 WP4 integration with WATERS WP15. The shared island material now takes a vertical datum.
  The river scene supplies its zero datum through the same material constructor as the ordinary island.

- 2026-11-26 Lead: WP4.3 failed a fifth review, on 19.13 mm from a bank camera; the fourth review's 0.42 m
  phase fault at the tidal mouth is fixed. The depth inside a river is now read from two named cameras at named
  places and times: 0.010 m from above, the greater of 0.010 m and 4 pixels from the bank, and a new inspection
  of the floats and the surface. The map's rows and the step at each end keep their tolerance.
  [Fifth review](../../../.orchestrate/evidence/RIVERS/WP4-review.md) · [the bounds and why](DESIGN.md#proof-and-builder-handoff)
- 2026-11-26 WP4 review fix. The river depth solver keeps the sign of the measured slope along the
  camera ray and brackets a crossing once it finds one. At the two-metre tidal bank, the old solver
  missed the live query by 19.10 mm. The compact Metal regression covers ticks 380–400 in full
  trade and measured at most 0.531 mm with the fix. The four full-length transects, wetting and
  join checks also passed. Hardware Chrome drew both rivers and read both packaged keys with
  zero solves. [Target evidence](../../../.orchestrate/evidence/RIVERS/WP4/oblique-depth/README.md)

- 2026-11-26 WP4.3 proof follows the revised camera and time bounds. The four-pixel bank allowance is
  evaluated at each reconstructed ray hit. Metal passed 143 wet stations per camera at each of four
  gauges. The full 420-pair tidal runs measured at most 0.0770 mm from above and 0.5302 mm from
  the bank. The maximum step residual across the three ends was 1.9 µm. The complete seven-second
  runs belong to the main tier. Short observations and the transects stay in the merge tier.
  [Target evidence](../../../.orchestrate/evidence/RIVERS/WP4/revised-depth/README.md)

- 2026-11-27 WP4.3 review fix. The sampled-river map now interpolates opposing directions in its
  bounded wave track separately. The body query uses the same calculation. On Metal, moving
  highlights cross the former spring-pool seam in the bank view. At the two seven-second tidal
  stations, the maximum depth differences were 0.0701 mm from above and 0.5057 mm from the bank.
  [Target evidence](../../../.orchestrate/evidence/RIVERS/WP4/pool-seam/README.md)

- 2026-11-27 WP4.3 review fix. The sampled wave reference belongs to its shared track.
  A track that includes a one-sample chart keeps the steady reference so joined reaches
  agree on phase. Ordinary chart draws still read their material-space row on mixed maps.
  The Metal regression compares the same chart with and without samples on a separate chart.
  [Target evidence](../../../.orchestrate/evidence/RIVERS/WP4/one-sample/README.md)

- 2026-11-27 WP4 review fix. An ordinary body keeps its material-space row on a mixed map.
  The coarse sea rings use world-position ownership when a declared sampled river joins the sea.
  On Metal, the disconnected-basin regression reduced 870 above-bed coverage changes over three
  matched frames to zero. The one-sample chart guard, regular river checks, both 420-tick tidal
  comparisons, unchanged channel checks, hardware Chrome and paired cost record passed.
  [Target evidence](../../../.orchestrate/evidence/RIVERS/WP4/basin-shoreline/README.md)

- 2026-11-27 WP4 review fix. The hollows vertex return initializes shared fields by name,
  so its optional sheet data has the correct zero value in either surface layout. The
  Metal guard reproduced the original validation error and now reads the expected submitted
  depths from both the chart and its separate puddle. The puddle reading is identical in
  both arms. The three existing hollows tests and the configured WP4 item checks passed.
  [Target evidence](../../../.orchestrate/evidence/RIVERS/WP4/mixed-hollows/README.md)

- 2026-11-27 WP5. The flow profile adds two bands to the shared water field and keeps the
  chart's standing level. It uses the prepared friction gain at `c = 2`. The display
  can skip the band's producer, and the game's query refreshes the same field when its
  dispatch needs it. Metal measured 21.60 mm and 20.45 mm RMS in running water and
  1.304 mm in the quiet pool. The mailbox agreed with the model for both bands and
  returned 320 bit-identical answers across cadence and camera changes.
- 2026-11-27 WP5. Chart loss feeds the existing whitewater stocks at 0.0011 m²/J.
  This is a water-type constant for the river profile. It was chosen against the
  valley's measured loss to retain whitewater at ledges and rock shoulders while
  keeping the pool and glide below visible cover from their small local losses.
  Metal reads the birth rates and transported stock at all eight valley stations.
  Hardware Chrome shows the riffle, quiet pool and thinning ledge foam.
  [Target evidence](../../../.orchestrate/evidence/RIVERS/WP5/README.md)
- 2026-11-27 WP5. The Metal inspection followed the headwater riffle, pool and narrows in
  both winds and both Sun elevations. Aligning the right-bank rocks with the solved shoulder
  made the stationary rise and moving ripples readable at the same point. The downstream
  camera follows whitewater from the rock's lee into thinning lace. The gauge take shows the
  wider foam band at the upper sample. The paired 1440p Metal record retains the WP4
  comparison and the cost investigations. [Target evidence](../../../.orchestrate/evidence/RIVERS/WP5/README.md)

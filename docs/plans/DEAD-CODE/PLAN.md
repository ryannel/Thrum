# DEAD-CODE plan

Engine code that nothing calls is gone, and small APIs stop asking for stubs. [Design](DESIGN.md) ·
[done-when file](DONE.json), which owns the criteria · [log](LOG.md) ·
[module list](../ARCHITECTURE.md#track-6-the-cleanup-of-20-november)

Read [Mechanisms](DESIGN.md#mechanisms), [the files each milestone touches](DESIGN.md#files-each-milestone-touches)
and [Exact baselines](../ARCHITECTURE.md#exact-baselines) before any milestone. Every file here is another module's,
and most have a builder at work: keep each commit small, rebase often, and change nothing you are not deleting
or converting. The sites of each finding are listed in the cleanup review's `findings/` and `verified/` folders, at
the path the design gives. Main moves, so search for callers again on the commit you start from.

## Open for the human

- Assumption: the simulation's incremental capture is deleted, about 330 lines: one-tick deltas that roll a saved capture forward. Nothing outside `thrum-core`'s own tests creates or applies one, and `SESSION`'s design rules out whole-world rollback. Full capture and undo stay. The other choice keeps it for a later rollback or replay feature.
- Assumption: the simulation keeps no public query for what changed. M1 removes the three that read the four-tick change history, and M3 the one that reads each slot's change tick. Nothing calls any of them, but the simulation decision lists a change journal among the core's responsibilities. The other choice keeps the per-slot query, `visit_with_changes`, as the engine's change detection, which costs nothing per tick, and removes only the history.
- Assumption: the serial transaction visits are deleted, about 190 lines. They are not copies of the chunked visit: they give a callback a mutable context in slot order and can visit a query that is not indexed. Nothing uses either ability. The other choice keeps one ordered mutable visit.
- Assumption: the game's state projection, about 650 lines, and the crate `thrum-content-runtime`, about 180, are deleted once `HOST` M3 has removed the host they sit in. The projection copies every committed page's changes for a reader that was never built, and the crate has no producer. The other choice keeps either for a reader you still expect.

## M1 The simulation no longer writes a change history every tick, and the sea no longer builds a buffer ladder every frame, for readers that do not exist

In `thrum-core`, `finish_tick` only writes the ring, so it goes with its calls, and `begin_tick` loses the
parameter that fed it, at about ten call sites. `visit_changes_with_ticks3` reads the ring and goes here.
`visit_with_changes` reads the per-slot ticks, not the ring, through `change_ticks3_at`: both have no other
caller and are M3's, so leave them. The maturity ledger's row `bounded_structural_growth` speaks of a "stable-ID
change journal": find out whether that is this ring, and reword the row as retired only if it is. In the sea, `forward_cache_gpu`, which
`WATERS` M2.10 ran, already checks all five planes of every filtered-texture mip; `SEA` M2.3 is Done and its text
stays. `crates/thrum-water/src/records/tests.rs` checks the sea's `Moments` layout against a struct in
`surface.wgsl` that nothing binds, so it fails if only the sea changes. Two functions are called
`surface_accesses`: the dead one is `Sea::surface_accesses`, and `Filtered::surface_accesses`, which water uses
every frame through `filtered_accesses`, stays. The cost priors in `examples/water/tests/fixtures/` and under
`docs/evidence/WATERS/` keep their "sea moments mip" rows: don't edit them. The stepped hour
(`examples/v1-game/tests/exit_scene.rs`) and the gale (`examples/water/tests/floating_gpu.rs`) are ignored long
tests by the time this starts, so `./dev check` runs neither. M1.5 passes from the shared run on main, where a
failure takes hours to trace to its commit, so run both yourself before you merge, as `./dev long-tests` runs
them.

## M2 A scene contribution writes only the hooks it uses

The trait's defaults, the stub removal and the probe test in `contribution::list` are on main, from `b57808ace`
and `8956287dd`, which started from `f5d7c5bba`. A review of them found the work sound and failed only an item
that named a run which skips one binary, so this milestone is most likely proved on main's tip as it stands:
change code only where a check fails. The trait is in `crates/thrum-render/src/contribution.rs`, and the one
wrapper outside tests is `SceneContributions` in `contribution/list.rs`. The fixtures that forward are `Fault` in
the game's ray-tracing proof tests, `Observed` in `crates/thrum-clouds/src/shade/tests.rs` and `PoolWitness` in
the mirror's `view_knobs_gpu` tests. M2.5 runs the game's `mesh_stage_gpu` one test at a time because the
main-tier list marks it not-run: run whole, 4 to 6 of its 10 `metal::` tests fail on state they share, which is
`MESH-SHADING`'s to fix. Leave those tests, their support file and the list's not-run line as they are. In each
item's evidence, name the commit on main that the check ran on, not a branch commit that a rebase replaces.

## M3 The engine's public API holds no simulation query, capture, device opener, serialiser or shader copy that only its own tests reach

The serial visit's unit test is `refusing_visit_refreshes_the_written_rows_active_bit`; the chunked kernel test
indexes only the read column today, so it does not yet cover that property. `g2_runtime.rs` has two legs that use
the incremental capture, and a unit test in `registered_world.rs` uses it while also checking undo and the active
layout: rewrite those with a full capture. `PUDDLES` M1.1 is Done and says "the same bytes": leave its text.

## M4 The water scene reads its settings in one place, so a proof captures the scene a person opens

The two parsers are `examples/water/src/main.rs` and the proof tests' support. The proof also hard-codes
`lake = false` for its bodies scene: decide it as the design decides the others, and write each decision in the
log. The window reads its variables through a helper, `option(key, "THRUM_WATER_X")`, and the tests read theirs
with `std::env::var`. Leave `Fixture` in `tests/support.rs` as it is: the gale and the foam's moored run build
their hosts through it, and they run only on main. If the parse cannot land without changing it, write the
proposal in the log and end `BLOCKED`, because the milestone would then need the `long-tests` item. This milestone comes before M5 and M6 because `WATERS` M7, which it waits for, is the
nearest of the three waits to clearing; if that changes, the lead asks a planner to reorder them.

## M5 The game's unread state projection and the content crate with no producer are gone

`HOST` M3 will have removed the forwarding host by the time this starts, so begin by reading what it left. When
the projection's capture goes, `recover_page` must still call `published_page()`, which republishes. The
engineering skill's package table names `thrum-content-runtime` at about line 42.

## M6 Bind-group layout entries and graph declarations are each written through one helper

Convert one crate a commit and run that crate's GPU tests before the next. Three entries have a count or a
dynamic offset and keep their literals. `REPO-RULES` M3 later renames and splits `synthetic.rs`, so this lands
before it.

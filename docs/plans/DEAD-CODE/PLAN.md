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
- Assumption: the game's state projection, about 650 lines, and the crate `thrum-content-runtime`, about 180, are deleted once `HOST` M3 has removed the host they sit in. The projection copies every committed page's changes for a reader that was never built, and the crate has no producer. The other choice keeps either for a reader you still expect.

## M1 The simulation no longer writes a change history every tick, and the sea no longer builds a buffer ladder every frame, for readers that do not exist

In `thrum-core`, `finish_tick` only writes the ring, so it goes with its calls, and `begin_tick` loses the
parameter that fed it, at about ten call sites. `visit_with_changes` and half of `visit_changes_with_ticks3` read
the per-slot ticks, not the ring, but have no caller either: they are M3's. In the sea, `forward_cache_gpu`, which
`WATERS` M2.10 ran, already checks all five planes of every filtered-texture mip; `SEA` M2.3 is Done and its text
stays. `crates/thrum-water/src/records/tests.rs` checks the sea's `Moments` layout against a struct in
`surface.wgsl` that nothing binds, so it fails if only the sea changes.

## M2 A scene contribution writes only the hooks it uses

The trait is in `crates/thrum-render/src/contribution.rs`. The wrappers that forward to other contributions are
the list of contributions, the lent one and the ray-tracing replacement. `RAY-TRACING` M3 is building in
`contribution.rs`: tell its builder before the trait change lands.

## M3 The engine's public API holds no simulation query, capture, device opener, serialiser or shader copy that only its own tests reach

The serial visit's unit test is `refusing_visit_refreshes_the_written_rows_active_bit`; the chunked kernel test
indexes only the read column today, so it does not yet cover that property. `g2_runtime.rs` has two legs that use
the incremental capture, and a unit test in `registered_world.rs` uses it while also checking undo and the active
layout: rewrite those with a full capture. `PUDDLES` M1.1 is Done and says "the same bytes": leave its text.

## M4 The water scene reads its settings in one place, so a proof captures the scene a person opens

The two parsers are `examples/water/src/main.rs` and the proof tests' support. The proof also hard-codes
`lake = false` for its bodies scene: decide it as the design decides the others, and write each decision in the
log.

## M5 The game's unread state projection and the content crate with no producer are gone

`HOST` M3 will have removed the forwarding host by the time this starts, so begin by reading what it left. When
the projection's capture goes, `recover_page` must still call `published_page()`, which republishes.

## M6 Bind-group layout entries and graph declarations are each written through one helper

Convert one crate a commit and run that crate's GPU tests before the next. Three entries have a count or a
dynamic offset and keep their literals. `REPO-RULES` M3 later renames and splits `synthetic.rs`, so this lands
before it.

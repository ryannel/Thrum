# Dead code: engine code that nothing calls is gone, and small APIs stop asking for stubs

20 November 2026. [Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) ·
[module list](../ARCHITECTURE.md#track-6-the-cleanup-of-20-november) ·
[exact baselines](../ARCHITECTURE.md#exact-baselines) ·
[patterns](../../../.agents/skills/thrum-engineering/references/patterns.md)

The findings this module answers are cited by id, such as C1-01. They are in the cleanup review of 20 November,
which is kept in the main checkout at `/Users/ryannel/Workspace/thrum/tmp/cleanup-2026-11-20/`: `REPORT.md` ranks
them, `results.json` holds each finding with its verifier's note, and `findings/` and `verified/` hold the
write-ups, with every site listed. Where a verifier corrected a finding, the verifier is right.

## The request

The owner, 20 November 2026: "We want to do a code cleanup and consolidation for Thrum." The lead's task for the
plan: make Thrum cheaper to change, with "no tools, records or code that nothing uses", and without changing
"what the engine draws, simulates or sends".

## Purpose and who it is for

About 4,000 lines of engine code have no caller, and some of it runs: the simulation writes a change history on
every tick that nothing has read since 5 November, and the sea builds a nine-level buffer ladder every frame that
only a test reads. Elsewhere a trait makes 35 implementations write about 140 empty methods, and 103 bind-group
layout entries are spelled out by hand. This module's users are builders, who read and copy this code, and a
game's author, who reads the public API to learn what the engine offers. A player sees nothing change.

## Mechanisms

**1. What dead means here.** Code is dead when nothing reaches it but its own definition and its own tests: no
caller in `crates`, `examples` or `tools` on main, none in the games repository, and no design that plans a
caller. Public API is held to the same test, because a game's author takes a public item as an offer. Each
deletion is made by removing the item and letting the compiler name what became unused, then removing that too,
until the build is clean. A test that covered only deleted code goes with it. A test that also covered something
kept is rewritten to keep covering it, and the design names those below.

**2. Per-tick and per-frame work for no reader goes first** (WP1).

- *The change-frame ring* (C1-01). `RegisteredWorld` keeps a four-tick ring of change bitsets and updates it on
  every write and spawn. Its only readers are world queries with no caller since publication moved to the
  snapshot ring. The ring, its maintenance, its readers and the step that only wrote it go. The per-slot change
  ticks and the structural tick stay, because publication reads them, and so does everything `finalize_activity`
  reads. No state, digest or published page changes: the simulation's determinism tests are the proof, with no
  expected value edited. With the ring go the public queries that answer what changed over an interval, and WP3
  removes the last change query, which reads the per-slot ticks. The simulation decision names a change journal
  as the core's, so removing every change query takes away something a design offers: it is the owner's call,
  and an assumption in [the plan](PLAN.md).
- *The sea's moment ladder* (E1-01). The sea writes a moments buffer and runs eight reduce passes into a
  nine-level chain of about 8.4 MB each frame. Water samples the filtered textures, which carry their own moments
  plane and mips; only one test oracle reads the buffers. The buffers, the reduce pipeline, their bind group and
  records, and the write that feeds them go, so the sea's graph declares fewer passes. That every mip of the
  filtered textures carries the slope and Jacobian moments on all five planes is already held by the forward-cache
  test, which carries the proof the oracle gave. Water's record test stops checking the dead record. The recorded
  cost priors keep their "sea moments mip" rows, because a prior is the record of the run that made it and the
  comparison does not fail on a row that is gone; the sea's README stops describing the ladder. No pixel changes.

Each removal is on a path that only a long test runs for long, and no merge check runs a long test once
`GATE-TIME` WP1 has landed. The exit scene's stepped hour steps the simulation for an hour of ticks through the
tick path the ring was written on, and it is the one test of a long run. The ten-minute gale renders the sea's
graph every frame. So WP1 waits for `GATE-TIME` WP1, which declares the main-branch check `long-tests`, and is Done
only when that check passes on main with WP1's commit in.

**3. A scene contribution writes only the hooks it uses** (WP2; L7-01, X1-01, R2-02). `SceneContribution` gives
default bodies to five hooks: `computes` (false), `encode_compute`, `encode_after_compute`, `draw` and
`submitted` (nothing). Every stub equal to its default is deleted, about 140 bodies in about 29 files. `passes`,
`rebuild` and `encode_upload` stay required: most implementations give them real bodies, and a default for
`passes` would let a contribution compile while declaring nothing. Defaults hide two faults, so the trait's
documentation states both and a test holds the first. A wrapper that stands in front of other contributions must
forward every hook, and a default lets one forget. Outside tests there is one such wrapper, the list of
contributions (`SceneContributions`), which the game's lent browser path also wraps its contributions in; a probe
behind it sees every hook arrive. The game's lent diagnostic is a contribution of its own and forwards to
nothing, so it gets no probe. Three test fixtures forward: the ray-tracing replacement, which keeps all eight
forwards written out, and the clouds' observed contribution and the mirror's pool witness, which forward only the
hooks their tests need. None of the three gets a probe. And `computes` returning false means `encode_compute` is
never called, while `submitted` is where work staged by `encode_upload` is committed.

**4. API that only its own tests reach goes** (WP3).

- In `thrum-core` (L5-07, C1-02, C1-03): seven runtime methods with no caller; the serial transaction visits,
  where every consumer uses the chunked visit; and the incremental capture (`RuntimeDelta` and what serves only
  it), a third rollback shape that nothing outside the crate creates or applies. The serial visits are not
  copies of the chunked one: they hand a callback a mutable context in slot order, and can visit a query that is
  not indexed. Nothing uses either ability, and both go with them, which is the owner's call and an assumption
  in the plan. [SESSION](../SESSION/DESIGN.md)'s
  design rules out whole-world rollback, so no caller is planned. One property is covered only by a serial
  visit's test: a visit that refuses a row still refreshes the written row's active bit. It is moved to the
  chunked visit, with the written column indexed. `RuntimeError::NoCommittedTick` stays, because the game matches
  on it.
- Three public device openers in `thrum-render`'s capabilities that only call each other (R2-01). The device
  recipe is the one way to open a device.
- The rain model's tree and catchment serialisers, which exist for one determinism test (WP1-03). The types
  compare as values, which is as strict, and the test asserts on that. `Tree::bytes`, which the rain cost
  example uses, stays.
- A test-only copy of the shadowed aerial shader, byte-identical to the two production pieces joined (R1-01).
  The copy goes, the test that read it reads the production pieces, and the test that only compared the copy
  with them goes. The loop the camera and secondary views share is not merged: that would rewrite the main
  view's shader source and risk a pixel.

**5. The water scene's settings are parsed once** (WP4; L3-09). The window and the proof tests each build the
scene from the `THRUM_WATER_*` variables that choose it, and they have drifted, so a proof can capture a scene a
person cannot open. One typed settings value is parsed from the environment or from a page's query, and the
window, the page and the proof tests all use it. The proof harness's own variables are not scene settings and
stay with the tests: where a proof writes, and the commit, route and recipe it records (`THRUM_WATER_PROOF`,
`_OUTPUT`, `_COMMIT`, `_ROUTE` and `_RECIPE`). The scene builders stay separate, because the proofs build
paired arms the window never does. Where the two parsers disagree, the window's reading stands: a floating
scene's heading is beam, `THRUM_WATER_STONE` is on only when it is `1`, and `THRUM_WATER_WAKE_BODIES` is
honoured. A meaning only a test needs, such as the island camera index that reuses `THRUM_WATER_VIEW`, gets a
variable of its own. Neither `water_cost.py` nor the browser check sets a variable whose meaning changes, so
recorded evidence stands.

**6. Two leftovers the host rewrite frees** (WP5; G1-01, A1-01). The game's state projection copies every
committed page's changes for a reader that never came: [SESSION](../SESSION/DESIGN.md) chose a world image in
`thrum-core`. `thrum-content-runtime` has no producer, and one test builds its product. Both sit in the game's
forwarding host, which [HOST](../HOST/DESIGN.md) WP3 removes, so this waits for it and deletes what is left: the
projection and its test legs, and the crate with its registration, with that one test moved to `thrum-render`'s
own environment API on the same numbers. A recovered page is still republished.

**7. One helper each for two things written longhand** (WP6).

- *Bind-group layout entries* (L6-02). `thrum_render::binding` is a small public module with one constructor
  for each standard kind: uniform buffer, storage buffer (read-only or not), sampled texture, sampler, storage
  texture, acceleration structure. The 103 literals in 41 non-test files become calls, one crate a commit, and
  the six local helper sets go. An entry with a count or a dynamic offset keeps its literal, and files that use
  automatic layouts are left alone. Each call must give the entry its literal gave: a wrong size or read-only
  flag is a validation error at pipeline creation, which every GPU test would hit, not a changed pixel. The
  module is new public API in `thrum-render`; it is accepted because five crates and the examples already
  write these entries against `thrum-render`'s shaders.
- *Graph declarations* (R3-07). Sixteen `Access` literals equal `Access::new`, and eight sites rebuild a
  texture's graph descriptor from the `wgpu` texture by hand. One function declares a graph resource from a
  texture: always a 2D texture, with the texture's own layers, mips and samples, never a cube, because every
  site declares 2D today. The compiled graph is unchanged, and the graph snapshot tests hold that.

## Interfaces

| Interface | For |
|---|---|
| `SceneContribution`'s five defaulted hooks, with `passes`, `rebuild` and `encode_upload` required | Every contribution: `ATMOSPHERE`, `CLOUDS`, `WATERS`, `RAY-TRACING`, `RENDER-2D`, cooked assets, the examples |
| `thrum_render::binding`: a layout entry for each standard kind | Every crate and example that creates a bind-group layout |
| Declaring a graph resource from a texture | Every graph contribution in `thrum-render`, and `RENDER-2D`'s sprite graph |

Nothing is removed that a module's design offers to another module, and no deletion removes API that a design
names as an interface, with one exception the plan carries to the owner: the change queries, which the simulation
decision counts among the core's responsibilities.

## Files each work package touches

The lead confirms these against the builders at work before it starts a work package. A work package touches nothing
outside its row; a builder that finds it must writes the proposal in the log and ends `BLOCKED`.
Three files are everyone's: `.config/shape-baseline.json`, `docs/design/maturity.json` and `Cargo.lock`. Change
them when your landing moves them, whatever your row says; a landing that meets a conflict there rebases and
makes the same edit again.

| Work package | Files | Waits for | Who else is near |
|---|---|---|---|
| WP1 | `crates/thrum-core/src/sim/` (`registered_world.rs`, `registered_world/image.rs`, `runtime.rs`, `transaction.rs`, `publication/page_tests.rs`) and `crates/thrum-core/tests/g2_runtime.rs`; one line of `docs/design/decisions/G2-GENERIC-SIMULATION.md`, and the row `bounded_structural_growth` of `docs/design/maturity.json` if it describes this ring; `crates/thrum-sea/src/` (`spec.rs`, `ambient.wgsl`, `prepare.rs`, `records.rs`, `lib.rs`), `crates/thrum-sea/tests/support.rs` and the sea's README; `crates/thrum-water/src/surface.wgsl` and `src/records/tests.rs` | `GATE-TIME` WP1, which declares the main-branch check `long-tests`: the stepped hour and the gale run only there, on the paths this work package changes | `SESSION` builds in `thrum-core`'s runtime files: land the ring as small commits and tell its builder. `WATERS` builds in `thrum-water` |
| WP2 | `crates/thrum-render/src/contribution.rs` and each file with an `impl SceneContribution`, about 32 across `crates` and `examples`, tests among them | `GATE-TIME` WP6: some of these files hold tests behind a Cargo feature, which no merge compiled before it. The gale and the field flythroughs render through edited contributions too | The busiest file in the renderer: `RAY-TRACING`, `WATERS`, `CLOUDS`, `HOST` and `RENDER-2D` all implement the trait. Land the trait's defaults alone first, then the stubs, in one quick pass |
| WP3 | `crates/thrum-core/src/sim/` (`runtime.rs`, `registered_world.rs`, `schedule.rs`) and `src/sim.rs`, with `tests/g2_runtime.rs`, `g2s_chunked_kernel.rs` and `profile_resources.rs`; `crates/thrum-render/src/capabilities/hardware.rs`; `crates/thrum-rain-model/src/tree.rs`, `tree/encode.rs`, `tree/catchment.rs` and `tests/tree.rs`; the shadowed aerial shader copy in `crates/thrum-render/src/` with `atmosphere/model/tests.rs` and `synthetic/secondary/tests.rs` | Nothing | `SESSION` in `thrum-core`. `REPO-RULES` WP3 later splits these runtime files, so this lands before it |
| WP4 | `examples/water/src/main.rs` and `lib.rs`; `examples/water/tests/proof_gpu.rs`, `realtime.rs` and `support.rs`; the water example's README | `WATERS` WP7, which is building in these files | `PUDDLES` WP3 and `RIVERS` WP4 add scenes through the parse afterwards. `GATE-TIME` WP1 and WP3 edit other tests of the same package |
| WP5 | `examples/v1-game/src/projection.rs`, `host/world.rs`, `host.rs`, `host/scene_source.rs`, `lib.rs`; `examples/v1-game/tests/host_gpu.rs` and `cooked_rows_gpu.rs`; `crates/thrum-content-runtime/`; the workspace list in the root `Cargo.toml`, `Cargo.lock`, the game's manifest and `.config/architecture.toml`; the package table of `.agents/skills/thrum-engineering/SKILL.md`, which names the crate; one row of `docs/design/maturity.json` | `HOST` WP3, which removes the game's forwarding host | `HOST` WP4 follows in the same files. `GATE-TIME` WP5 edits the `[profile]` tables of the same root manifest |
| WP6 | A new `binding` module and `graph.rs` in `crates/thrum-render/src/`; every non-test file that writes a `BindGroupLayoutEntry`, in `thrum-render`, `thrum-asset-gpu`, `thrum-clouds`, `thrum-water`, `thrum-sea`, `examples/mirror` and `examples/v1-game`; `visibility.rs`, `visibility/casters.rs`, the atmosphere's `night.rs`, `cloud.rs`, `graph.rs` and `gpu/aerial_view.rs`, `synthetic/secondary.rs` and `sprite/graph.rs` | `REVIEW-RENDER` WP3, which rewrites the atmosphere's graph accesses; `GATE-TIME` WP6, because the game's ray-tracing and mesh-stage code behind a feature writes these entries too, as do the files the gale and the field flythroughs render through | The widest set in the module: `synthetic.rs` had 24 commits in three days. `CLOUDS`, `WATERS`, `RAY-TRACING`, `REVIEW-RENDER` and `RENDER-2D` are in these files |

## Not built here

- **The state-delta and state-ack codecs in `thrum-network`** (N1-01, L4-01). Nothing sends state, but
  [SESSION](../SESSION/DESIGN.md)'s design says they stay, and the contracts select them as the parked wire model
  of the 3D game. They stay unless that design changes.
- **The engine's `projected_changes` and `projection_lost_ticks` report fields**, which are always zero once the
  projection is gone. Removing them changes a report that scripts read.
- **Merging the shadowed aerial loop of the camera and secondary views**, which risks a pixel.
- **The larger consolidations**, each of which needs a design: shader variants made by text replacement, the two
  camera draw routes, 2D carried through the 3D host, the CPU-fed cloud field, integer graph ids, and the four
  height-lattice types. [The module list](../ARCHITECTURE.md#not-planned-in-track-6) names each.

## What it must not do

Change a pixel, a simulation state or digest, a wire byte, a cooked or saved format, or what a query answers.
Remove or weaken an assertion about code that is kept. Remove API that a design names as an interface, or that
the games repository uses. Edit a registered exact baseline, a Done item or another module's design. Reformat or
reorder code it is not deleting or converting.

## Proof

A deletion is proved by the compiler and by a search for the deleted names, and its harmlessness by the tests
that were already there: simulation determinism and conformance with no expected value edited, the sea's and
water's GPU tests, and the exact baselines on main. A reviewer answers whether anything kept lost its test. The
trait's defaults are proved by the probe behind the list of contributions and by a reviewer's list of
implementations. A work package that changes what a long test runs, or edits a file holding tests behind a Cargo
feature, is proved there by the main-branch check `long-tests`, because no merge check runs those tests: WP1 for
the stepped hour and the gale, WP2 and WP6 for the feature tests and for the long tests that render through what
they edit. That check skips a binary the main-tier list marks not-run, which today is the game's `mesh_stage_gpu`:
run whole, its tests fail on state they share, a fault that is `MESH-SHADING`'s to fix and was listed before WP2
began. WP2 edits a contribution in that binary and WP6 converts layouts its tests draw through, so each carries a
second item whose check runs the binary one test at a time with a GPU required, on main. That shows the edit
broke none of its tests. It does not show the binary is sound run whole, and neither work package changes those
tests or the list. WP3 and WP5 delete only what nothing calls on those paths, and WP4 leaves the fixture the gale and the
foam's moored run build their hosts through as it is, so none of the three carries that item. The settings parse is proved by a reviewer's table of each setting's reading before and after. The two helpers are
proved by every GPU test passing, by graph snapshots that were not edited, and by the exact baselines.

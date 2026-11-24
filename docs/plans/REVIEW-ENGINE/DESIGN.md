# Review fixes outside the renderer: the simulation, the sea, cooked assets and the cooker

14 November 2026, revised 15 November after [the first design review](../../../.orchestrate/evidence/REVIEW-ENGINE/design-review.md).
[Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) ·
[module list](../ARCHITECTURE.md#track-4-the-code-review-and-the-games-separation) ·
[the code review](../../../.orchestrate/evidence/code-review-2026-11-14/CODE-REVIEW.md) ·
[patterns](../../../.agents/skills/thrum-engineering/references/patterns.md)

## The request

The owner asked for the review's fixes to be planned and driven. The lead's order: "confirmed bugs first (H01, H09,
M41, H11, H14), then typed Rust-WGSL records and typed pass keys (T1, T2), … error convention (T3), performance
(T12)", each fix given to the module that owns the code, "or a new module where none does", without rewriting
files that running builders hold.

## Scope

This module takes the review's findings in crates where no builder is at work, so it can start at once:
`thrum-core`, `thrum-network`, `thrum-sea`, `thrum-sea-model`, `thrum-asset`, `thrum-asset-gpu`, `thrum-cook`,
`thrum-controls`, and the one-line device request in the iPhone app. The renderer's findings are
[REVIEW-RENDER](../REVIEW-RENDER/DESIGN.md)'s and the shells' are [HOST](../HOST/DESIGN.md)'s. The code stays its
owners': each change is built here under [the rulings](../ARCHITECTURE.md#seam-rulings-for-track-4) and belongs to
its owner once built. `thrum-cloud-model` and `thrum-water-model` have builders at work and are not touched.

Its users are the builders who extend these crates next (`WATERS`, `RIVERS` and `PUDDLES` in the sea, `CONTENT-LOOP`
in the loader, `SIM-2D` and `SESSION` in core) and anyone who runs the game on an iPhone.

## Mechanisms

**1. The iPhone device request (H01, first aid).** `examples/v1-game-ios/src/ios/render.rs` asks for timing
features only and for `downlevel_defaults` limits. So cooked textures never take the ASTC profile on the one
platform whose profile it is, and a stage gets four storage buffers where `thrum-sea` refuses fewer than eight.
The request takes every content feature the adapter offers and the adapter's own limits, as the Mac window's
does. This is the contained fix; `HOST` WP1 then replaces every host's request with one recipe.

**2. One publication model (H14).** `GameRuntime` supports a page the game fills and a page the engine fills, and
branches on which in every publication method. The G2S decision gives layout and page capture to the engine, and
the only production consumer uses the engine's page: the game path runs in tests alone, and the rewind patch the
V1 game passes never runs. The engine's page is the one model. The game-authored capture, the state only that
path used and the rewind patch callbacks are removed, and the tests that used them are rebuilt on the engine's
page. What the engine's own path keeps for a deferred publication stays: its undo preimages, the skipped-with-undo
state and the previous poses, which a later publication and WP5's replay fix rely on. What must
stay true is the behaviour, so it gets a test it never had: after a rewind and replay the published page equals
a fresh capture of the same world. No planned module (`SIM-2D`, `SESSION`, `SWARM`) uses a game-authored page.

**3. The sea: keys, records, upload (H17, T1, H16).**

- Buffers, pipelines, bind groups and passes are named by typed keys, and sizes, usages, bind groups, dispatch
  counts and graph accesses are derived from one table. The test that holds it compares, for every pass, what the
  bind group binds with what the graph declaration names.
- Each record a shader reads is a `Pod` struct written whole, and a reflection test parses the WGSL and fails for
  a bound struct with no equal Rust record. Reflection, and not a hand-kept list, so a new struct cannot be
  missed.
- The spectrum's modes change only when the spectrum changes, and the GPU already evolves them from time. So the
  mode buffer is written on a spectrum change, and a frame writes the parameter block, the long-wave phases and
  the queries. A weather transition blends into an existing spectrum.

The sea's output is held by `SEA`'s own tests against the CPU model, unedited.

**4. Cooked meshes load from the mapping (H18, H19).** The cooker writes each meshlet's triangles into the index
section in meshlet order; the loader rebuilds the same bytes on the CPU and uploads its copy. The loader uploads
the section itself and takes meshlet ranges from the meshlet table by a prefix sum. The cooked format does not
change, so no product, content address or baseline moves. `examples/asset-loop`, a second loader that the cooker's
edit-to-visible test and tripwire measure, goes: the test and bench drive `thrum-asset-gpu`'s loader, and the
profile preference both copies held becomes one function in `thrum-asset`. The tripwire then measures other code,
so its history breaks once: the old and new figures are recorded side by side and the new one is the baseline.
This work package starts before [REPO-RULES](../REPO-RULES/DESIGN.md)' rule engine exists, so that the cooker reaches
no example is proved here by a search of the cooker's manifest and sources; `GAME-SPLIT` WP2 later holds it, with
every other reach, by rule.

**5. Errors (T3).** Each public boundary in these crates gets one enum with `Display` and `Error`, as
`thrum-asset`'s is. A compile-time test in each crate names its public error types. Core's `FrameError` (an input
frame) and the renderer's (a drawn frame) get different names; core's is renamed for what it is, an input frame's
error.

**6. Waste, not tuning (T12).** Three changes remove work whose result is discarded, each shown by a counter:

- replay copies the whole undo on every deferred tick and uses only the last;
- the cooker decodes and mips a texture once for each profile;
- the sea runs eleven complex transforms a band where its own design says packed transforms, and holds about 75
  MB where the design's account says about 15.

The packed transform changes the sea's bits within `SEA`'s bounds; nothing exact reads the sea yet.

**7. Corrected replay in both test tiers (G2S).** The fast CPU test uses the smallest mover population that
dispatches multiple production kernel chunks, including a partial chunk. It keeps slot churn and exercises
serial execution, requested Rayon with serial fallback, and forced parallel dispatch, with counters that show
which route ran. For each of the three original refusals, compare complete authority immediately before and
after the rejected step, before any further step or replay. Compare at the kernel refusal, at the Resolve
refusal after tentative kernel writes, and at the refusal inside the corrected replay window. The retained
input amendment must change the endpoint. After replay, compare complete authority and the published page
with the straight corrected run, retaining the published-page assertions for page identity and each live
row's stable identity, position, velocity and visual.

The original 100,000-entity test moves intact to the main tier and keeps the G2S qualification at scale.
Its four production-sized mover chunks and widely separated live and reused slots stress undo, publication
and partitioning. The fast test's timing proof uses the workspace runner's stable toolchain, nonincremental
build and default nextest profile, then filters which tests run. It records the compact case's per-test time
alone and under ordinary builder load. The isolated run takes under 6 s, the cutoff used to identify the
slow cases, with about 2 s as the planning target. The loaded run passes the existing 10 s limit on its
first attempt.

## Parked, with the reason

- **M71, one indirect draw for each meshlet on Metal.** The fix is a new Apple culling recipe. `CULLING`'s plan
  leaves native recipes to the performance phase the brief puts after the exit.
- **The mip filter's odd sizes and alpha (in M67).** It changes cooked bytes, so it would move the exact
  baselines; it needs a refresh ruling of its own.
- **M51, M53, M61** (page capture, touched sets and byte codecs written several times) are theme T11, which the
  review's order of attack does not schedule.

## What it must not do

Change the cooked format; edit an assertion, bound or reference in a test another module left; touch
`thrum-cloud-model` or `thrum-water-model`; add a numeric engine performance budget. Cost is recorded as paired rows.

## Proof

Each fix has a test that fails without it. Unchanged behaviour is held by the owning modules' own tests, unedited,
and by the exact-baseline verifier. iPhone function is shown on the simulator, which proves function only.

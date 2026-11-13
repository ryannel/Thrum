# SEA plan

An open sea from waterline to horizon, with floating bodies, wakes and foam, finished in Thrum.
[Design](DESIGN.md) · [done-when file](DONE.json), which owns the criteria · [log](LOG.md) · [brief](BRIEF.md) ·
[module list](../ARCHITECTURE.md)

Before any milestone, read [Proof and builder handoff](DESIGN.md#proof-and-builder-handoff) and
[Exact baselines](../ARCHITECTURE.md#exact-baselines). The limits there hold for every milestone: Broadsides is
read and never changed, nothing is copied from it before the owner names the licence, game content stays in
`examples/sea`, and another module's exact test or baseline is never edited to make a change pass.

## Open for the human

- Assumption: the owner's words of 13 November, "use the fable planner like normal but you can bring that across when it makes sense", mean planning starts now and this plan decides when the Broadsides sea is brought across. M1 and M2 can start as soon as the design is approved, and later milestones start when the milestones they depend on are Done, which can be before G7. This sets aside the water decision's line that implementation follows G7.
- Assumption: those words do not name a licence. Broadsides code is marked `UNLICENSED`, so nothing is copied or translated from it until the owner says under what licence it enters this repository. Until then builders write from published sources and the design, and read Broadsides as a record of what was tried. The other choice lets them lift its maths and kernels through review, which is faster.
- Assumption: the craft, the floats and the foam pattern of the example scene are made in this repository, original or public domain, and no Broadsides asset is used.
- Assumption: a finished sea excludes inland water, coasts, shoaling, surf and tides, and the three cues the 10 November spikes left open (spray release at chase distance, the attached sheet at the hull, a visibly rising gas cloud). They stay parked spikes.
- Assumption: "persists after the hull stops or turns" is met inside windows that follow the craft, as the water decision sizes them. Wake waves shorter than about 12 m are carried to about 30 m behind the craft and longer ones to about 250 m; the foam trail reaches further. A wake that persists everywhere the craft has been needs water state that stays in the world, which is not planned.
- Assumption: full refresh is claimed on the Mac, as the brief's cost sentence says. Chrome is held to the same look and behaviour, and its frame intervals are recorded, not gated.
- Assumption: until the showcase's own extent exists, cost is recorded at 2560×1440, the extent Broadsides was measured at. M10 repeats it at the showcase's extent.
- Assumption: the module closes only after `APPLE-PRESENTATION`'s governor and MetalFX milestones (M7 and M8 in its drafts), which are not planned yet. Everything a visitor sees is finished by M9; M10 waits.

## M1 The sea's waves are computed on the CPU alone, from glassy calm to gale, and agree with wave theory

Nothing exists yet. This milestone creates `thrum-sea-model`, with no dependency on the renderer, and registers
it in `.config/architecture.toml`. The reproductions in `docs/evidence/SEA/design-proof/` show the long-wave
phase wrap the evaluator must use. The wake and foam references come with M6 and M7, not here.

## M2 The GPU computes the same sea as the CPU model, tick after tick, through pauses, hitches and a replaced device

This creates `thrum-sea` as an effect on `EFFECT-GRAPH`'s seam, which that module's M2 built:
`crates/thrum-render/src/contribution.rs` holds the trait and `examples/v1-game/src/effect` a working effect to
read. Only `simulate`-stage passes are built here; nothing is drawn. The seam is `EFFECT-GRAPH`'s. If the two
tick banks cannot be declared with it as built, write the proposal in the log and end `BLOCKED`. M2.2's bound is
measured first and then asserted. The recorded Chrome limits are in `docs/evidence/EFFECT-GRAPH/M1/`.

## M3 An open sea in a trade wind draws from the waterline to the horizon under the engine's own sky, and what it costs is on record against the display's refresh

This creates `examples/sea` (package `sea-scene`) as a game on `thrum-app`'s shell with its own ticking world;
`examples/v1-game` is the working model for the shell, the tick page and the capture tap, and the sky kit's
world never ticks. The surface draws in `after-opaque` and writes scene depth under
[ruling 1](../ARCHITECTURE.md#seam-rulings-for-sea), which this builder adds in `EFFECT-GRAPH`'s files without
changing that module's items or tests. Ruling 3 covers anything the shell or the capture tap still assumes of
the V1 game; `CREATOR` M2 and M3 edit `thrum-app` too, so integrate on top of what has landed. Light comes only from `ATMOSPHERE`'s lent library. This milestone creates
`scripts/sea_cost.py`, and its first record is the baseline later milestones are read against. If the frame does
not hold the refresh after the three designed remedies in the design's first risk, end `BLOCKED` with the
record: the design is re-planned before M5.

## M4 The same sea draws in Chrome from one command

`scripts/check_starter_browser.mjs` and `scripts/check_v1_browser.mjs` show how a page is launched and checked;
this milestone creates `scripts/check_sea_browser.mjs`. The web shell is `thrum_app::web`, and ruling 3 covers
what the page or the packaging scripts still assume of the V1 game or the kit. A Chrome launch needs a run
outside the Codex sandbox.

## M5 A float and a small craft ride the waves they sit on, from rest to 15 m/s

The engine side is the query, its ring and its counters. The craft, the floats, their hydrostatics, drag, thrust
and rigid-body step are game code in `examples/sea`. The world runs on its own lane, so body records and query
points travel in the tick page and answers come back through a mailbox; nothing may wait on the GPU on either
side.

## M6 The craft leaves a wake that spreads behind it, outlives it, and lifts a float it crosses

`docs/evidence/SEA/design-proof/wake_proof.py` is a working sketch of the two banded levels, the border and the
Fourier-space source, and its output gives the figures M6.1 and M6.2 are bounded from. The engine tests use a
generic pressure patch, never the example's hull. The body's own dent is added back in the query; filtering the
craft out of the wake is the wrong fix for a craft that bounces on its own waves.

## M7 Whitecaps form as the wind rises, and the craft leaves a foam trail that drifts, spreads and fades

The foam pattern is the example's cooked product, lent by a `CONTENT-LOOP` lease;
`examples/v1-game/src/lent` is a working lease consumer. The model and the spike's verdicts are in the design.
The look is tuned against M7.5's and M7.6's lists with a fresh opinion before review. After three looks that
fail the same cue, record them in the log and end `BLOCKED`.

## M8 The sea reflects the sky, the Sun and the craft, glitters, darkens with depth and dims under a cloud's shade, from dawn to night

The gather pass reads scene colour and depth under [ruling 2](../ARCHITECTURE.md#seam-rulings-for-sea), added
in `EFFECT-GRAPH`'s files as ruling 1 was. The cloud shade uses `ATMOSPHERE`'s provider registration with a
stand-in provider in the example, labelled as one; `examples/v1-game/src/environment.rs` has the diagnostic
provider to read.

## M9 A visitor takes the sea from glassy calm to gale and from dawn to night with the controls, and it holds up in motion, on the Mac and in Chrome

TAAU is `APPLE-PRESENTATION`'s. A defect that is the reconstructor's and not the sea's inputs goes to that
module's log as a proposal. The knobs are declared through `EFFECT-GRAPH`'s list; no governor is needed here.

## M10 On the Mac's Apple recipe the sea runs with MetalFX and the governor at the display's full refresh, and the module is closed

Run M10.1's check first. If it fails, end `BLOCKED` with `needs_dependency` and build nothing: the governor and
MetalFX are `APPLE-PRESENTATION`'s to build. M10.6 needs the owner's attended sitting; do every other item
first.

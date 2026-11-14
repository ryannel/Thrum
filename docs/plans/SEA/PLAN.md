# SEA plan

An open sea from waterline to horizon, with floating bodies, wakes and foam, finished in Thrum.
[Design](DESIGN.md) · [done-when file](DONE.json), which owns the criteria · [log](LOG.md) · [brief](BRIEF.md) ·
[module list](../ARCHITECTURE.md)

Before any milestone, read [Proof and builder handoff](DESIGN.md#proof-and-builder-handoff) and
[Exact baselines](../ARCHITECTURE.md#exact-baselines). The limits there hold for every milestone: Broadsides is
never changed, a file copied from it is listed in the provenance file, game content stays in `examples/sea`, and
another module's exact test or baseline is never edited to make a change pass.

## Open for the human

- Assumption: the owner's words of 14 November about clouds, "it should start from first principles", govern the sea too. The 13 November design, the brief and the port decision were judged as inputs: two mechanisms changed, and the rest are kept for reasons the design gives.
- Assumption: "you can copy that code out and reuse as is if it's useful" is the owner's licence for what Broadsides' authors wrote, so it enters this repository on this repository's terms. Material that came into Broadsides from elsewhere, such as its foam texture from Crest under MIT, keeps its own licence, and none that is GPL or share-alike is copied. Licensing is the owner's decision; builders work to this reading.
- Assumption: "bring that across when it makes sense" (13 November) means this plan decides when. Milestones start when their dependencies are Done, which can be before G7, setting aside the water decision's line that implementation follows G7.
- Assumption: every part of the sea beyond the open sea (query, wake, whitewater, body optics) is a capability a scene can leave out at no cost. The owner raised flexible capabilities for clouds, not the sea. It costs one item, M9.12.
- Assumption: a finished sea excludes inland water, coasts, shoaling, surf and tides, and the three cues the 10 November spikes left open (spray release at chase distance, the attached sheet at the hull, a visibly rising gas cloud). They stay parked spikes.
- Assumption: "persists after the hull stops or turns" is met inside windows that follow the craft, as the water decision sizes them. Wake waves shorter than about 12 m are carried to about 30 m behind the craft and longer ones to about 250 m; the foam trail reaches further. A wake that persists everywhere the craft has been needs water state that stays in the world, which is not planned.
- Assumption: the full-refresh cost account is recorded on the Mac only, as the brief's cost sentence says. Chrome is held to the same look and behaviour, and its frame intervals are recorded. On neither does a timing figure decide an item or stop a milestone (brief decision 3).
- Assumption: "screen-space reflection" means the sea reflects the bodies a scene registers, the craft and floats here, within a reach of 64 m and on water tilted up to 20°, fading to the sky beyond. Other opaque geometry is not reflected. Mirroring everything in view needs a whole-scene copy or ray query; neither is planned.
- Assumption: until the showcase's extent exists, cost is recorded at 2560×1440, where Broadsides was measured. M10 repeats it at the showcase's extent.
- Assumption: the module closes only after `APPLE-PRESENTATION`'s governor and MetalFX milestones (M7 and M8 in its drafts), which are not planned yet. Everything a visitor sees is finished by M9; M10 waits.

## M1 The sea's waves are computed on the CPU alone, from glassy calm to gale, and agree with wave theory

This milestone creates `thrum-sea-model`, with no dependency on the renderer, and registers it in
`.config/architecture.toml`. The 14 November revision changed none of its items. The wake and foam references
come with M6 and M7, not here.

## M2 The GPU computes the same sea as the CPU model at every frame's time, after a day of running, through pauses, hitches and a replaced device

This creates `thrum-sea` as an effect on `EFFECT-GRAPH`'s seam, which that module's M2 built:
`crates/thrum-render/src/contribution.rs` holds the trait and `examples/v1-game/src/effect` a working effect to
read. Only `simulate`-stage passes are built here, all of them every-frame passes that keep nothing between
frames; nothing is drawn, and the seam's state protocol is first used in M6. This milestone also changes
`thrum-sea-model`, which M1 built: cascade bins take whole-cycle loop frequencies (M2.9), and the model gives each
cascade's rate of change. Broadsides' `ocean_evolve.wesl` and `ocean_fft.wesl` already do the loop phase and the
256-point transform ([what Broadsides offers](DESIGN.md#what-broadsides-offers)). M2.2's bound is measured first
and then asserted. The recorded Chrome limits are in `docs/evidence/EFFECT-GRAPH/M1/`.

## M3 An open sea in a trade wind draws from the waterline to the horizon under the engine's own sky, and what it costs is on record against the display's refresh

This creates `examples/sea` (package `sea-scene`) as a game on `thrum-app`'s shell with its own ticking world;
`examples/v1-game` is the working model for the shell, the tick page and the capture tap, and the sky kit's
world never ticks. The surface draws in `after-opaque` and writes scene depth under
[ruling 1](../ARCHITECTURE.md#seam-rulings-for-sea), which this builder adds in `EFFECT-GRAPH`'s files without
changing that module's items or tests. Ruling 3 covers anything the shell or the capture tap still assumes of
the V1 game; `CREATOR` M2 and M3 edit `thrum-app` too, so integrate on top of what has landed. Light comes only
from `ATMOSPHERE`'s lent library. This milestone creates `scripts/sea_cost.py`, and its first record is the
baseline later milestones are read against. A frame over the refresh interval is investigated here with the
remedies in the design's risks 1 and 9, and stops no milestone.

## M4 The same sea draws in Chrome from one command

`scripts/check_starter_browser.mjs` and `scripts/check_v1_browser.mjs` show how a page is launched and checked;
this milestone creates `scripts/check_sea_browser.mjs`. The web shell is `thrum_app::web`, and ruling 3 covers
what the page or the packaging scripts still assume of the V1 game or the kit. A Chrome launch needs a run
outside the Codex sandbox.

## M5 A float and a small craft ride the waves they sit on, from rest to 15 m/s

The engine side is the query, its ring and its counters. The craft, the floats, their hydrostatics, drag, thrust
and rigid-body step are game code in `examples/sea`, where Broadsides' hydrostatics may serve. The world runs
on its own lane, so body records and query points travel in the tick page and answers come back through a
mailbox.

## M6 The craft leaves a wake that spreads behind it, outlives it, and lifts a float it crosses

The seam's state protocol and the two endpoints start here.
`docs/evidence/SEA/design-proof/wake_proof.py` is a working sketch of the two banded levels, the border and the
Fourier-space source, and its output gives the figures M6.1 and M6.2 are bounded from. Part A of
`revision_proof.py` beside it sketches the step of several ticks that M6.10 holds. The engine tests use a
generic pressure patch, never the example's hull. The body's own dent is added back in the query; filtering the
craft out of the wake is the wrong fix for a craft that bounces on its own waves.

## M7 Whitecaps form as the wind rises, and the craft leaves a foam trail that drifts, spreads and fades

The foam pattern is the example's cooked product, lent by a `CONTENT-LOOP` lease;
`examples/v1-game/src/lent` is a working lease consumer. Broadsides' ranked pattern may be the source, with its
MIT notice. The look is tuned against M7.5's and M7.6's lists with a fresh opinion before review. After three looks that
fail the same cue, record them in the log and end `BLOCKED`.

## M8 The sea reflects the sky, the Sun and the craft, glitters, darkens with depth and dims under a cloud's shade, from dawn to night

The gather pass reads scene colour and depth under [ruling 2](../ARCHITECTURE.md#seam-rulings-for-sea), added
in `EFFECT-GRAPH`'s files as ruling 1 was. Its conversion to radiance uses `ATMOSPHERE`'s lent aerial lookup and
exposure scale as they are. Parts C and E of `docs/evidence/SEA/design-proof/revision_proof.py` sketch the
receiving rectangles and the conversion. The cloud shade uses `ATMOSPHERE`'s provider registration with a
stand-in provider in the example, labelled as one; `examples/v1-game/src/environment.rs` has the diagnostic
provider to read.

## M9 A visitor takes the sea from glassy calm to gale and from dawn to night with the controls, and it holds up in motion, on the Mac and in Chrome

TAAU is `APPLE-PRESENTATION`'s. A defect that is the reconstructor's and not the sea's inputs goes to that
module's log as a proposal. The knobs are declared through `EFFECT-GRAPH`'s list; no governor is needed here.

## M10 On the Mac's Apple recipe the sea runs with MetalFX and the governor, its cadence on the built-in display is measured, and the module is closed

Run M10.1's check first. If it fails, end `BLOCKED` with `needs_dependency` and build nothing: the governor and
MetalFX are `APPLE-PRESENTATION`'s to build. M10.6 needs the owner's attended sitting; do every other item
first.

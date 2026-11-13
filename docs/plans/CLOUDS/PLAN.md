# CLOUDS plan

A weather-driven sky from clear to overcast, with placed cloud volumes, that shades and lights the world,
finished in Thrum. [Design](DESIGN.md) · [done-when file](DONE.json), which owns the criteria · [log](LOG.md) ·
[brief](BRIEF.md) · [module list](../ARCHITECTURE.md)

Before any milestone, read [Proof and builder handoff](DESIGN.md#proof-and-builder-handoff),
[the seam rulings](../ARCHITECTURE.md#seam-rulings-for-clouds) and
[Exact baselines](../ARCHITECTURE.md#exact-baselines). The limits there hold for every milestone: Broadsides is
read and never changed, nothing is copied from it before the owner names the licence, game content stays in the
games, a ruling changes another module's code and never its items, tests, bounds or references, and with the
cloud selection off each game is what it was.

## Open for the human

- Assumption: the owner's words of 13 November, "use the fable planner like normal but you can bring that across when it makes sense", mean planning starts now and this plan decides when the Broadsides cloud work is brought across. M1 and M2 can start as soon as the design is approved, and later milestones start when the milestones they depend on are Done, which can be before G7. This sets aside the port decision's line that the owner schedules each port.
- Assumption: those words do not name a licence. Nothing is copied or translated from Broadsides, code or cloud library, until the owner says under what licence it enters this repository. Until then builders write from published sources and the design, placed volumes are built and proved with stand-in shapes made here (M6), and the twelve accepted volumes with their growth track wait in M10, which also holds M11 and M12. The other choice, naming the licence now, lets M10 run in its turn and lets builders lift the weather maths and shaders through review.
- Assumption: "the sky over the V1 valley and the sea scene" is a cloud selection inside `examples/v1-game` and `examples/sea`, off by default, and not a new cloud scene. The V1 exit scene is unchanged unless someone turns the selection on.
- Assumption: finished clouds excludes the ten-genera, squall and warm-front scope of the Broadsides game, and the consolidation the owner asked for is the port decision's hybrid of weather layers and placed volumes.
- Assumption: a finished sky keeps the camera and all solid geometry below the lowest cloud base, as the brief's 2–40 m cameras do. Mountains that rise into cloud, light shafts in haze and a camera in cloud are not planned.
- Assumption: the time-lapse control runs from 1× to 60×, and "continuous to the horizon" means to the geometric horizon of the cloud tops, about 200 km.
- Assumption: the full-refresh cost account is recorded on the Mac only, as the brief's cost sentence says. Chrome is held to the same look and behaviour, and its frame intervals are recorded. On neither does a timing figure decide an item or stop a milestone (brief decision 3).
- Assumption: until the showcase's own extent exists, cost is recorded at 2560×1440, the extent Broadsides was measured at. M12 repeats it at the showcase's extent.
- Assumption: the module closes only after `APPLE-PRESENTATION`'s governor and MetalFX milestones (M7 and M8 in its drafts), which are not planned yet, and its sea milestone waits for `SEA` M8. Everything a visitor sees over the valley, except the accepted library shapes, is finished by M9.

## M1 The weather over any place and time is computed on the CPU alone, from a clear sky to overcast, and repeats exactly

Nothing exists yet. This milestone creates `thrum-cloud-model`, with no dependency on the renderer, and registers
it in `.config/architecture.toml`. In `docs/evidence/CLOUDS/design-proof/`, `cloud_field_proof.py` is a working
sketch of the cell process, the deck and the Sun-path reference, and `revision_proof.py` of a placed shape's
footprint and the coarse density; their constants are a first profile and their shapes are not the look. The optics oracle is `scripts/reference/cloud/`, which needs NumPy.

## M2 Cloud shade crosses the valley in step with the weather, with no cloud drawn

This creates `thrum-clouds` as an effect on `EFFECT-GRAPH`'s seam: `crates/thrum-render/src/contribution.rs`
holds the trait and `examples/v1-game/src/effect` a working effect to read. Only `simulate`-stage passes are
built here. The shade path is `ATMOSPHERE`'s: `SunOpticalDepth` in `crates/thrum-render/src/atmosphere/cloud.rs`,
`cloud.wgsl` beside it, and the diagnostic provider in `examples/v1-game/src/host/scene_source/cloud.rs`.
Ruling 1 extends that path in those files; `ATMOSPHERE` M6 edits the same module, so integrate on top of what
has landed. `examples/v1-game` is `SHOWCASE`'s: add the selection and leave its generator, routes and exit
configuration alone. M2.2's and M2.3's bounds are measured first and then asserted. This milestone creates
`scripts/cloud_cost.py`, whose `check` the design's cost account defines. If the seam cannot declare the Sun-path product's time-sliced levels as built, write
the proposal in the log and end `BLOCKED`.

## M3 Fair-weather cumulus stands over the valley under the engine's own sky, and what it costs is on record against the display's refresh

The trace, the resolve and the compose are `after-opaque` passes; `examples/v1-game/src/effect` has a working
after-opaque effect once `EFFECT-GRAPH` M3 is Done. Ruling 2 is built in `ATMOSPHERE`'s aerial lookup. The look
is tuned against M3.1's to M3.3's lists with a fresh opinion before review; after three looks that fail the same
cue, record them in the log and end `BLOCKED`. A whole frame over the refresh interval is investigated here
with the remedies in the design's first risk and written up in the record; it does not stop the milestone.

## M4 The same sky and the same shade draw in Chrome

`scripts/check_v1_browser.mjs` is shared by every module with a Chrome case: add cases and change none. A Chrome
launch needs a run outside the Codex sandbox.

## M5 Clouds are lit through the whole day: bright sides and shadowed bases at noon, a silver lining against the Sun, sunset colour, and dark shapes against the stars

M5.1's bound is measured first and then asserted. If low-Sun relief fails M5.3 with three local samples, try the
design's fallbacks for its fourth risk before changing anything else, and record what each cost.

## M6 A cloud made as content stands in the weather's sky, lit and shading like the clouds around it, and one grows through its staged keys

`examples/v1-game/src/lent` is a working lease consumer, and `thrum-cook`'s demo writes a `TVOL` source. The
stand-in shapes and track are made here; no Broadsides file is opened for content. If the scene's residency
cannot take the volumes as a second source folder, write the proposal in `CONTENT-LOOP`'s log; do not add them
to the showcase valley's source without a ruling.

## M7 The sky runs from clear through fair and broken to overcast and back, and its clouds drift, form, grow and thin

## M8 Under a clouded sky the land is lit by the clouds: an overcast valley is grey and soft, haze dims with it, and polished surfaces reflect the clouds

Rulings 3 and 4 are built in `ATMOSPHERE`'s reflection and diffuse producers and its aerial decoder. Their
storage is relative to the Sun; the rulings state behaviour, not storage. If the producers cannot compose the
overlay within their existing bounds, write the proposal in `ATMOSPHERE`'s log and end `BLOCKED`.

## M9 A visitor changes coverage, cloud type, wind and time of day with the controls, and the sky holds up in motion, on the Mac and in Chrome

TAAU is `APPLE-PRESENTATION`'s. A defect that is the reconstructor's and not the clouds' inputs goes to that
module's log as a proposal. The knobs are declared through `EFFECT-GRAPH`'s list; no governor is needed here.

## M10 The twelve accepted cloud volumes and the growing congestus stand in the sky

Run M10.1's check first. If it fails, end `BLOCKED` with `needs_context` and copy nothing. The lead adds the line
it looks for when the owner answers: `- <date> Owner licence for the cloud library: "<the owner's words>"`. The
library is at Broadsides `79637f789` under `assets/clouds/heaped-fair/` (Git LFS; density is the PNG's red
channel, 128³), with `factory/clouds/heaped-fair/B3-HANDOFF.md` for axes, placement and hashes, and the growth
track's keys are described under `factory/clouds/heaped-fair/review/m2-authored-growth-v2/`.

## M11 Clouds drift over the open sea, shade it and show in its reflections

`examples/sea` is `SEA`'s: add the selection and change nothing else of that module's.

## M12 On the Mac's Apple recipe the sky runs with MetalFX and the governor, its cadence on the built-in display is on record, and the module is closed

Run M12.1's check first. If it fails, end `BLOCKED` with `needs_dependency` and build nothing: the governor and
MetalFX are `APPLE-PRESENTATION`'s to build. M12.6 needs the owner's attended sitting; do every other item first.

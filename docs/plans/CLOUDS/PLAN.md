# CLOUDS plan

[Design](DESIGN.md) · [done-when file](DONE.json), which owns the criteria · [log](LOG.md) ·
[brief](BRIEF.md) · [module list](../ARCHITECTURE.md)

Before any milestone, read [Proof and builder handoff](DESIGN.md#proof-and-builder-handoff),
[What Broadsides offers](DESIGN.md#what-broadsides-offers),
[the seam rulings](../ARCHITECTURE.md#seam-rulings-for-clouds) and
[Exact baselines](../ARCHITECTURE.md#exact-baselines). Their limits hold for every milestone.

## Open for the human

- Assumption: the owner's words of 14 November, "it should start from first principles", govern. The 13 November design, the brief and the port decision were judged as inputs; the design lists what changed and why the rest is kept. "Atmospheric system" is read as clouds, weather and how they shade and light the world; `ATMOSPHERE`'s own sky, which is in the V1 exit, is not reopened.
- Assumption: "a flexible set of cloud capabilities" is taken up. The engine has no sky types: a sky of weather alone, of cooked shapes alone or of both is the game's data, and shade, the visible sky, the world's light and shapes are each left out at no cost. It costs two items, M6.12 and M8.9.
- Assumption: "visible clouds off releases presentation resources and keeps shade" keeps a shape library loaded. A scene that shows no sky but wants the shade of cooked shapes holds their volumes, 27 MiB for the example's twelve, because a footprint cannot give a shape's shade under a slanting Sun.
- Assumption: a game may place up to 16 shapes itself, so a demo can compose its sky. The owner did not ask for this; a sunset demo needs it, and a cloud-formation demo would build on it. It costs three items: M1.12, M6.9 and M6.10.
- Assumption: cloud light takes its first-order phase from the droplets, not the retained reference's lobe. Near the Sun and away from it the sky will differ from Broadsides' accepted stills: a tighter silver lining, and brighter clouds with the Sun behind the viewer.
- Assumption: the owner's words of 14 November are the licence for what Broadsides' authors wrote, the twelve volumes and growth track included. They enter this repository as the example's content with their licence record, which grants nothing to third parties; their terms if Thrum's source is published are the owner's to say. Third-party material keeps its own licence, and nothing GPL or share-alike is copied.
- Assumption: the owner's words of 13 November, "you can bring that across when it makes sense", mean this plan decides when the work starts: M1 and M2 once the design is approved, later milestones when their dependencies are Done, which can be before G7.
- Assumption: "the sky over the V1 valley and the sea scene" is a cloud selection inside `examples/v1-game` and `examples/water`, off by default, not a new scene. The V1 exit scene is unchanged unless the selection is turned on.
- Assumption: finished clouds excludes the Broadsides game's ten genera, squall and warm front, a cloud a simulation writes, and a cloud held in place against the wind.
- Assumption: a finished sky keeps the camera and all solid geometry below the lowest cloud base. Mountains that rise into cloud, light shafts in haze and a camera in cloud are not planned. The time-lapse control runs from 1× to 60×, and "continuous to the horizon" means to the horizon of the cloud tops, about 200 km.
- Assumption: the full-refresh cost account is recorded on the Mac only, at 2560×1440 until the showcase's own extent exists (M12 repeats it there). Chrome is held to the same look and behaviour, and its frame intervals are recorded. No timing figure decides an item or stops a milestone (brief decision 3).
- Assumption: the module closes only after `APPLE-PRESENTATION`'s governor and MetalFX milestones, which are not planned yet, and its sea milestone waits for `WATERS` M8. Everything a visitor sees over the valley is finished by M10.

## M1 The weather over any place and time is computed on the CPU alone, from a clear sky to overcast, and repeats exactly

This creates `thrum-cloud-model`, with no dependency on the renderer, and registers it in
`.config/architecture.toml`. `docs/evidence/CLOUDS/design-proof/` holds working sketches of the field, the Sun's
path, placed shapes and the phase with its disc average and table; their constants are a first profile and
their shapes are not the look. Broadsides' `scatter.rs` holds the point-Sun phase and its fit; its Sun-disc
`spread` loses energy and is not the design's. The optics oracle is `scripts/reference/cloud/`, which needs
NumPy.

## M2 Cloud shade crosses the valley in step with the weather, with no cloud drawn

This creates `thrum-clouds` as an effect on `EFFECT-GRAPH`'s seam: `crates/thrum-render/src/contribution.rs`
holds the trait and `examples/v1-game/src/effect` a working effect to read. The shade path is `ATMOSPHERE`'s: `SunOpticalDepth` in `crates/thrum-render/src/atmosphere/cloud.rs`,
`cloud.wgsl` beside it, and the diagnostic provider in `examples/v1-game/src/host/scene_source/cloud.rs`.
Ruling 1 extends that path in those files; `ATMOSPHERE` M6 edits the same module, so integrate on top of what
has landed. `examples/v1-game` is `SHOWCASE`'s: add the selection and leave its generator, routes and exit
configuration alone. `scripts/cloud_cost.py` is
created here. If the seam cannot declare the Sun-path
product's time-sliced levels as built, or cannot give a frame that shows a tick again its interpolation alpha,
write the proposal in the log and end `BLOCKED`.

## M3 Fair-weather cumulus stands over the valley under the engine's own sky, and what it costs is on record against the display's refresh

`examples/v1-game/src/effect` has a working after-opaque effect once `EFFECT-GRAPH` M3 is Done. Ruling 2 is built in `ATMOSPHERE`'s aerial lookup. After three
looks that fail the same cue of M3.1 to M3.3, record them in the log and end `BLOCKED`. A whole frame over the
refresh interval is investigated here with the design's first risk's remedies and written up; it does not stop
the milestone.

## M4 The same sky and the same shade draw in Chrome

`scripts/check_v1_browser.mjs` is shared by every module with a Chrome case: add cases and change none. A Chrome
launch needs a run outside the Codex sandbox.

## M5 Clouds are lit through the whole day: bright sides and shadowed bases at noon, a silver lining against the Sun, sunset colour, and dark shapes against the stars

If low-Sun relief fails M5.3, try the design's fallbacks for
its fourth risk first and record what each cost. If the aureole fails M5.8, its seventh risk says where the fix
lies.

## M6 A cloud made as content stands in the sky where the weather or the game puts it, lit and shading like the clouds around it, and one grows through its staged keys

`examples/v1-game/src/lent` is a working lease consumer, and `thrum-cook`'s demo writes a `TVOL` source. The
twelve volumes are Git LFS objects at
Broadsides `79637f789`; `git show` piped through `git lfs smudge` reads them and leaves that checkout as it is.
The growth track's eleven key volumes are not in Git: they are in the Broadsides folder at
`tmp/evidence-store/cloud-route-2026-11-08/m2-authored-growth-v2/source-keys-replay.tar.gz`, with each key's hash
in the track's manifest, and `factory/clouds/heaped-fair/coherent_growth.py` makes them again. If
neither gives keys that match the manifest, end `BLOCKED` with `needs_context`. Engine tests cook their own
shapes. If the scene's residency cannot take the volumes as a second source folder, write the proposal in
`CONTENT-LOOP`'s log; do not add them to the valley's source without a ruling.

## M7 The sky runs from clear through fair and broken to overcast and back, and its clouds drift, form, grow and thin

## M8 Under a clouded sky the land is lit by the clouds: an overcast valley is grey and soft, haze dims with it, and polished surfaces reflect the clouds

Rulings 3 and 4 are built in `ATMOSPHERE`'s reflection and diffuse producers and its aerial decoder. Their
storage is relative to the Sun; the rulings state behaviour, not storage. If the producers cannot compose the
overlay within their existing bounds, write the proposal in `ATMOSPHERE`'s log and end `BLOCKED`.

## M9 A visitor changes coverage, cloud type, wind and time of day with the controls, and the sky holds up in motion, on the Mac and in Chrome

TAAU is `APPLE-PRESENTATION`'s. A defect that is the reconstructor's and not the clouds' inputs goes to that
module's log as a proposal. No governor is needed here.

## M10 The twelve accepted cloud volumes and the growing congestus each read as themselves in the finished sky

The library has been in the game since M6. `factory/clouds/heaped-fair/B3-HANDOFF.md` at Broadsides `79637f789`
names each shape's intended read and its accepted weaknesses, which are not this milestone's to fix.

## M11 Clouds drift over the open sea, shade it and show in its reflections

`examples/water` is `WATERS`': add the selection and change nothing else of that module's.

## M12 On the Mac's Apple recipe the sky runs with MetalFX and the governor, its cadence on the built-in display is on record, and the module is closed

Run M12.1's check first. If it fails, end `BLOCKED` with `needs_dependency` and build nothing: the governor and
MetalFX are `APPLE-PRESENTATION`'s to build. M12.6 needs the owner's attended sitting; do every other item first.

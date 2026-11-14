# CLOUDS plan

[Design](DESIGN.md) · [done-when file](DONE.json), which owns the criteria · [log](LOG.md) ·
[brief](BRIEF.md) · [module list](../ARCHITECTURE.md)

Before any milestone, read [Proof and builder handoff](DESIGN.md#proof-and-builder-handoff),
[the seam rulings](../ARCHITECTURE.md#seam-rulings-for-clouds) and
[Exact baselines](../ARCHITECTURE.md#exact-baselines). Their limits hold for every milestone.

## Open for the human

- Assumption: the owner's "it should start from first principles" (14 November) governs.
- Assumption: "taking into account mist / fog" and "top quality atmosphere" (the owner, 14 November) are met by mist, fog and low cloud as the clouds' own droplets in this module's one weather field. `ATMOSPHERE`'s view volume draws them near the camera (its M7 and M8, after G3s closes) and this module's trace beyond. It adds M13 to M16 and three items to M11, and replaces the assumption that the camera and all geometry stay below the cloud base.
- Assumption: "changing with the weather and time of day" means the mist itself forms after dusk and burns off after sunrise, as well as taking the day's light. The narrower reading leaves every amount to the game and drops M13.4 and M16.1.
- Assumption: "low cloud the camera can enter" includes coming out above it on a summit. The camera stays 2 to 40 m over the ground; flying is not planned. Only a layered deck on the hills is inspected.
- Assumption: "a flexible set of cloud capabilities" is taken up: shade, the visible sky, the world's light, shapes and ground condensate are each left out by a scene at no cost.
- Assumption: a scene with no visible sky that wants the shade of cooked shapes keeps their volumes loaded (27 MiB).
- Assumption: a game may place up to 16 shapes itself, so a demo can compose its sky. The owner did not ask for this; it costs M1.12, M6.9 and M6.10.
- Assumption: cloud light takes its first-order phase from the droplets, so the sky will differ from Broadsides' accepted stills near the Sun.
- Assumption: the owner's words of 14 November are the licence for what Broadsides' authors wrote, the cloud library included; its terms if Thrum's source is published are the owner's. Third-party material keeps its own licence, and nothing GPL or share-alike is copied.
- Assumption: "bring that across when it makes sense" (13 November) means milestones start when their dependencies are Done, which can be before G7.
- Assumption: clouds are shown through a selection inside `examples/v1-game` and `examples/water`, off by default; the V1 exit scene is unchanged.
- Assumption: finished excludes the Broadsides game's ten genera, squall and warm front, rain, a cloud a simulation writes, a cloud held against the wind, and lamps glowing in fog.
- Assumption: time-lapse runs from 1× to 60×, and "continuous to the horizon" means about 200 km.
- Assumption: cost is recorded on the Mac only, at 2560×1440 until the showcase's extent exists; no timing figure decides an item (brief decision 3).
- Assumption: the module closes only after `APPLE-PRESENTATION`'s unplanned governor and MetalFX milestones. Fog waits for `ATMOSPHERE` M8, and the sea for `WATERS` M8.

## M1 The weather over any place and time is computed on the CPU alone, from a clear sky to overcast, and repeats exactly

This creates `thrum-cloud-model`, with no dependency on the renderer, and registers it in
`.config/architecture.toml`. `docs/evidence/CLOUDS/design-proof/` holds working sketches; their constants are a first profile and
their shapes are not the look. Broadsides' `scatter.rs` holds the point-Sun phase and its fit; its Sun-disc
`spread` loses energy and is not the design's. The optics oracle is `scripts/reference/cloud/`, which needs
NumPy. Ground condensate is M13's: build nothing for it here.

## M13 Mist, fog and low cloud over any ground are computed on the CPU alone, and a game can ask how far it can see

This extends M1's crate and changes none of its answers where no surface field or fog table is given.
`whole_atmosphere_proof.py` in the design-proof folder sketches it. Tests make their own relief.

## M2 Cloud shade crosses the valley in step with the weather, with no cloud drawn

This creates `thrum-clouds` as an effect on `EFFECT-GRAPH`'s seam (`crates/thrum-render/src/contribution.rs`;
`examples/v1-game/src/effect` is a working effect). The shade path is `ATMOSPHERE`'s:
`crates/thrum-render/src/atmosphere/cloud.rs`, `cloud.wgsl` and the diagnostic provider in
`examples/v1-game/src/host/scene_source/cloud.rs`. Ruling 1 extends it there; integrate on top of what
`ATMOSPHERE` has landed. In `SHOWCASE`'s `examples/v1-game` add the selection only. `scripts/cloud_cost.py` is
created here. If the seam cannot declare the Sun-path product's time-sliced levels as built, or cannot give a
frame that shows a tick again its interpolation alpha, write the proposal in the log and end `BLOCKED`.

## M3 Fair-weather cumulus stands over the valley under the engine's own sky, and what it costs is on record against the display's refresh

Ruling 2 is built in `ATMOSPHERE`'s aerial lookup. Write the trace over
[the interval a ray marches](DESIGN.md#the-visible-layer), though here that interval is the cloud band. After
three looks that fail the same cue of M3.1 to M3.3, record them in the log and end `BLOCKED`. A frame over the refresh interval is investigated with the design's first risk's remedies and does not stop
the milestone.

## M4 The same sky and the same shade draw in Chrome

`scripts/check_v1_browser.mjs` is shared by every module with a Chrome case: add cases and change none.

## M5 Clouds are lit through the whole day: bright sides and shadowed bases at noon, a silver lining against the Sun, sunset colour, and dark shapes against the stars

For M5.3 and M5.8 the design's fourth and seventh risks give the fallbacks; record what each cost.

## M6 A cloud made as content stands in the sky where the weather or the game puts it, lit and shading like the clouds around it, and one grows through its staged keys

`examples/v1-game/src/lent` is a working lease consumer, and `thrum-cook`'s demo writes a `TVOL` source.
[What Broadsides offers](DESIGN.md#what-broadsides-offers) says where the volumes and the growth keys are; read
the LFS objects with `git show` piped through `git lfs smudge`, which leaves that checkout as it is. If no source
gives keys that match the track's manifest, end `BLOCKED` with `needs_context`. If the scene's residency cannot take the volumes as a second source folder, write the proposal in
`CONTENT-LOOP`'s log.

## M7 The sky runs from clear through fair and broken to overcast and back, and its clouds drift, form, grow and thin

## M8 Under a clouded sky the land is lit by the clouds: an overcast valley is grey and soft, haze dims with it, and polished surfaces reflect the clouds

Rulings 3 and 4 are built in `ATMOSPHERE`'s reflection and diffuse producers and its aerial decoder. If `ATMOSPHERE`'s view volume has
landed, a scene that declares it takes no haze scale. If the producers cannot compose the overlay within their
existing bounds, write the proposal in `ATMOSPHERE`'s log and end `BLOCKED`.

## M9 A visitor changes coverage, cloud type, wind and time of day with the controls, and the sky holds up in motion, on the Mac and in Chrome

TAAU is `APPLE-PRESENTATION`'s; a defect of the reconstructor's goes to that module's log. No governor is
needed here.

## M10 The twelve accepted cloud volumes and the growing congestus each read as themselves in the finished sky

`B3-HANDOFF.md` at Broadsides `79637f789` names each shape's intended read and its accepted weaknesses, which
are not this milestone's to fix.

## M14 Mist lies in the valley's hollows and fades the land near the camera, in the land's own light

`ATMOSPHERE` owns the view volume, its history and the lent aerial operation; this milestone writes the medium
coefficients and the ground band it reads and replaces the labelled test medium in `examples/v1-game`. A defect
in the volume itself goes to `ATMOSPHERE`'s log. After three looks that fail
the same cue of M14.5 or M14.6, record them in the log and end `BLOCKED`.

## M15 Mist and fog banks reach the horizon, low cloud hides the hilltops, and a climbing camera passes into the cloud and out above it

Take M15.3's record before choosing between the front-distance rule and the eleventh risk's fallback. If the
handover shows in M15.4, the eighth risk gives the fixes in order.

## M16 Through a day mist forms after dusk and burns off in the morning, haze runs from crisp to murky, and a visitor sets them, on the Mac and in Chrome

## M11 Clouds drift over the open sea, shade it and show in its reflections, and a fog bank drifts ashore

`examples/water` is `WATERS`': add the selection and its surface field and change nothing else of that module's.

## M12 On the Mac's Apple recipe the sky runs with MetalFX and the governor, its cadence on the built-in display is on record, and the module is closed

Run M12.1's check first. If it fails, end `BLOCKED` with `needs_dependency` and build nothing: the governor and
MetalFX are `APPLE-PRESENTATION`'s to build. M12.6 needs the owner's attended sitting; do every other item first.

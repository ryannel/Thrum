# CLOUDS plan

[Design](DESIGN.md) · [done-when file](DONE.json) · [log](LOG.md) ·
[brief](BRIEF.md) · [module list](../ARCHITECTURE.md)

Before any work package, read [Proof and builder handoff](DESIGN.md#proof-and-builder-handoff),
[the seam rulings](../ARCHITECTURE.md#seam-rulings-for-clouds) and
[Exact baselines](../ARCHITECTURE.md#exact-baselines).

## Open for the human

- Assumption: "taking into account mist / fog" and "top quality atmosphere" (14 November) are met by mist, fog and low cloud as the clouds' own droplets in this module's one weather field.
- Assumption: "changing with the weather and time of day" means the mist itself forms after dusk and burns off after sunrise, as well as taking the day's light.
- Assumption: "low cloud the camera can enter" includes coming out above it on a summit. The camera stays 2 to 40 m over the ground. Only a layered deck on the hills is inspected.
- Assumption: a headless game asking whether it can see through a placed cloud gets a marked estimate unless it gives the cloud's voxels.
- Assumption: "a flexible set of cloud capabilities" is taken up: shade, the visible sky, the world's light, shapes and ground condensate are each left out by a scene at no cost.
- Assumption: a scene with no visible sky that wants the shade of cooked shapes keeps their volumes loaded (27 MiB).
- Assumption: a game may place up to 16 shapes to compose its sky.
- Assumption: cloud light takes its first-order phase from the droplets, so the sky will differ from Broadsides' accepted stills near the Sun.
- Assumption: for light scattered many times the design keeps its flux solution as a starting point. Nothing holds it to photon transport, and a builder may tune it by eye.
- Assumption: the owner's words of 14 November are the licence for what Broadsides' authors wrote, the cloud library included; Third-party material keeps its own licence, and nothing GPL or share-alike is copied.
- Assumption: clouds are shown through a selection inside `examples/v1-game` and `examples/water`, off by default; the V1 exit scene is unchanged.
- Assumption: finished excludes the Broadsides game's ten genera, squall and warm front, rain, a cloud a simulation writes, a cloud held against the wind, and lamps glowing in fog.
- Assumption: time-lapse runs from 1× to 60×, and "continuous to the horizon" means about 200 km.
- Assumption: cost is recorded on the Mac only, at 2560×1440 until the showcase's extent exists; no timing figure decides an item (brief decision 3), except in WP20 and WP25, by [their rule](DESIGN.md#holding-the-displays-refresh-while-clouds-drift-wp19-and-wp20).
- Assumption: what the game loses with clouds absent stops nothing here; it is listed for `PLATFORM-RUNTIME-MAC` and the performance phase.
- Assumption: only the fair preset's sky is held to the refresh. The broken and overcast skies' cost and the finished sky's presented cadence are recorded and investigated (WP12.5, WP12.6), and the rest is the performance phase's.
- Assumption: WP17 judges whether each procedural cloud holds up on its own with library shapes disabled.
- Assumption: near heaps are drawn half a second of play behind a query's weather, and past about 25 km a heap loses its overhangs.
- Assumption: the game queries committed weather. Clear samples throughout the contributing filter footprints at their retained states guarantee a clear drawn column. [Near and far sampling guarantees](DESIGN.md#the-weather-volume).
- Assumption: the weather volume uses a fixed fill cadence at every governor setting. [Weather volume](DESIGN.md#the-weather-volume).
- Assumption: WP25 measures the release V1 game in its 1920×1080 window, on the Apple recipe with TAAU under the fair preset, in one more sitting alone.
- Assumption: the module closes only after `APPLE-PRESENTATION`'s governor and MetalFX work packages.
- Assumption: the finished sky closes at WP12 with weak noon relief on broad lobes of library shapes as a listed weakness ([risk 14](DESIGN.md#risks)); [the noon-relief follow-up](DESIGN.md#the-noon-relief-follow-up) is no work package and waits for the lead.
- Assumption: WP14 to WP12 pass without a shaft of sunlight followed inside mist below broken cloud ([risk 15](DESIGN.md#risks)); WP22, last, takes it up.
- Assumption: a scene that declares Ground pays its full-refresh trace at every fog amount, zero included. With no mist it is held to the look of the same scene without Ground.
- Assumption: from inside upper cloud only the far-mist toggle is inspected, not how the cloud looks from within.

## WP1 The weather over any place and time is computed on the CPU alone, from a clear sky to overcast, and repeats exactly: done
[review](../../../.orchestrate/evidence/CLOUDS/WP1-review.md)

## WP13 Mist, fog and low cloud over any ground are computed on the CPU alone, and a game can ask how far it can see: done
[review](../../../.orchestrate/evidence/CLOUDS/WP13-review.md)

## WP2 Cloud shade crosses the valley in step with the weather, with no cloud drawn: done
[review](../../../.orchestrate/evidence/CLOUDS/WP2-review.md)

## WP3 Fair-weather cumulus stands over the valley under the engine's own sky, and what it costs is on record against the display's refresh

[Review](../../../.orchestrate/evidence/CLOUDS/WP3-review.md)

## WP4 The same sky and the same shade draw in Chrome

[Review](../../../.orchestrate/evidence/CLOUDS/WP4-review.md)

## WP5 Clouds are lit through the whole day: bright sides and shadowed bases at noon, a silver lining against the Sun, sunset colour, and dark shapes against the stars

[Review](../../../.orchestrate/evidence/CLOUDS/WP5-review.md)

## WP18 Small cloud products check Metal and portable limits with headroom

[Review](../../../.orchestrate/evidence/CLOUDS/WP18-review.md)

## WP6 A cloud made as content stands in the sky where the weather or the game puts it, lit and shading like the clouds around it, and one grows through its staged keys

[Review](../../../.orchestrate/evidence/CLOUDS/WP6-review.md)

## WP7 The sky runs from clear through fair and broken to overcast and back, and its clouds drift, form, grow and thin

[Review](../../../.orchestrate/evidence/CLOUDS/WP7-review.md)

## WP19 While clouds drift, the game can say where each lost refresh went, and the losses the clouds cause are fixed

[Review](../../../.orchestrate/evidence/CLOUDS/WP19-review.md)

## WP20 With clouds drifting, the player sees a new frame as steadily as in the same game without them, in its ordinary window on the Mac

[Review](../../../.orchestrate/evidence/CLOUDS/WP20-review.md)

## WP8 Under a clouded sky the land is lit by the clouds: an overcast valley is grey and soft, haze dims with it, and polished surfaces reflect the clouds

[Review](../../../.orchestrate/evidence/CLOUDS/WP8-review.md)

## WP9 A visitor changes coverage, cloud type, wind and time of day with the controls, and the sky holds up in motion, on the Mac and in Chrome

[Review](../../../.orchestrate/evidence/CLOUDS/WP9-review.md)

## WP10 The twelve accepted cloud volumes and the growing congestus each read as themselves in the finished sky

[Review](../../../.orchestrate/evidence/CLOUDS/WP10-review.md)

## WP14 Mist lies in the valley's hollows and fades the land near the camera, in the land's own light

[Review](../../../.orchestrate/evidence/CLOUDS/WP14-review.md)

## WP15 Mist and fog banks reach the horizon, low cloud hides the hilltops, and a climbing camera passes into the cloud and out above it

[Review](../../../.orchestrate/evidence/CLOUDS/WP15-review.md)

## WP16 Through a day mist forms after dusk and burns off in the morning, haze runs from crisp to murky, and a visitor sets them, on the Mac and in Chrome

[Review](../../../.orchestrate/evidence/CLOUDS/WP16-review.md)

## WP11 Clouds drift over the open sea, shade it and show in its reflections, and a fog bank drifts ashore

[Review](../../../.orchestrate/evidence/CLOUDS/WP11-review.md)

## WP23 The weather's own clouds take a new form, a cluster of rounded heaps, in what a game reads and in stills drawn from it on the CPU

Preserve the broken-deck opening and fit each puff inside its cell. Move WP23.1's witnesses with
the fit, then repeat WP23.2's stills. Inspect near clouds from below and look for repeated towers.
[Form and erosion](DESIGN.md#one-weather-field-in-the-air-frame) · [When to end `BLOCKED`](DESIGN.md#risks).

## WP24 The engine draws, shades and answers for the new cloud form on the Mac and in Chrome, and the old form is gone

Judge no look here.

## WP17 Each fair-weather cloud holds up on its own: a cluster of heaps of different heights, with no shelf, fold or repeated outline

Take WP17.1's stills first. For WP17.6, frame a heap across a face whose parts sample different stages
of the fair-to-clear change. [Tuning and remedies for a visible cut](DESIGN.md#risks).

## WP25 With the new clouds drifting, the player still sees a new frame as steadily as in the same game without them

By [WP20's protocol](DESIGN.md#holding-the-displays-refresh-while-clouds-drift-wp19-and-wp20): if a cloud run
does not hold, end `BLOCKED` with the series and build no repair.

## WP12 On the Mac's Apple recipe the sky runs with MetalFX and the governor, its cadence on the built-in display is on record, and the module is closed

Run WP12.1's check first. If it fails, end `BLOCKED` with `needs_dependency` and build nothing. WP12.6 needs
the owner's attended sitting; do every other item first.

## WP22 Below broken cloud, shafts of sunlight stand in the valley's mist

Follow [the broken-cloud shaft follow-up](DESIGN.md#the-broken-cloud-shaft-follow-up-wp22). Take
WP22.6's starting cost arms first, and note that commit for WP22.8.

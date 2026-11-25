# CLOUDS plan

[Design](DESIGN.md) · [done-when file](DONE.json), which owns the criteria · [log](LOG.md) ·
[brief](BRIEF.md) · [module list](../ARCHITECTURE.md)

Before any work package, read [Proof and builder handoff](DESIGN.md#proof-and-builder-handoff),
[the seam rulings](../ARCHITECTURE.md#seam-rulings-for-clouds) and
[Exact baselines](../ARCHITECTURE.md#exact-baselines).

## Open for the human

- Assumption: aim for about 2 s per fast test when run alone on a quiet Mac. The owner can revise this target.
- Assumption: the owner's "it should start from first principles" (14 November) governs.
- Assumption: "taking into account mist / fog" and "top quality atmosphere" (14 November) are met by mist, fog and low cloud as the clouds' own droplets in this module's one weather field. `ATMOSPHERE`'s view volume draws them near the camera (its WP7 and WP8, after G3s closes) and this module's trace beyond. It adds WP13 to WP16 and three items to WP11.
- Assumption: "changing with the weather and time of day" means the mist itself forms after dusk and burns off after sunrise, as well as taking the day's light. The narrower reading leaves every amount to the game and drops WP13.4 and WP16.1.
- Assumption: "low cloud the camera can enter" includes coming out above it on a summit. The camera stays 2 to 40 m over the ground. Only a layered deck on the hills is inspected.
- Assumption: a headless game asking whether it can see through a placed cloud gets a marked estimate unless it gives the cloud's voxels.
- Assumption: "a flexible set of cloud capabilities" is taken up: shade, the visible sky, the world's light, shapes and ground condensate are each left out by a scene at no cost.
- Assumption: a scene with no visible sky that wants the shade of cooked shapes keeps their volumes loaded (27 MiB).
- Assumption: a game may place up to 16 shapes itself, so a demo can compose its sky. The owner did not ask for this; it costs WP6.9 and WP6.10.
- Assumption: cloud light takes its first-order phase from the droplets, so the sky will differ from Broadsides' accepted stills near the Sun.
- Assumption: for light scattered many times the design keeps its flux solution as a starting point, because it brightens a cloud with nothing tuned. Under the owner's bar of 17 November nothing holds it to photon transport, and a builder may tune it by eye.
- Assumption: the lowered bar keeps tests where a game reads the weather (the GPU's map against the CPU query) and on the frame contract, and drops every test of how light was computed.
- Assumption: the owner's words of 14 November are the licence for what Broadsides' authors wrote, the cloud library included; Third-party material keeps its own licence, and nothing GPL or share-alike is copied.
- Assumption: "bring that across when it makes sense" (13 November) means work packages start when their dependencies are Done, which can be before G7.
- Assumption: clouds are shown through a selection inside `examples/v1-game` and `examples/water`, off by default; the V1 exit scene is unchanged.
- Assumption: finished excludes the Broadsides game's ten genera, squall and warm front, rain, a cloud a simulation writes, a cloud held against the wind, and lamps glowing in fog.
- Assumption: time-lapse runs from 1× to 60×, and "continuous to the horizon" means about 200 km.
- Assumption: cost is recorded on the Mac only, at 2560×1440 until the showcase's extent exists; no timing figure decides an item (brief decision 3), except in WP20 (the lead's task of 25 November): the weather sky loses at most one refresh in 20 seconds beyond the same game without clouds.
- Assumption: the fair cumulus's close-up form (shelves, folds, repeated caps) waits for WP17, just before the module closes, on the lead's decision of 18 November; every sky until then shows those flaws.
- Assumption: the module closes only after `APPLE-PRESENTATION`'s governor and MetalFX work packages. Fog waits for `ATMOSPHERE` WP8, and the sea for `WATERS` WP8.
- Assumption: the game WP20 measures is the release V1 game in its 1920×1080 window, on the Apple recipe with TAAU under the fair preset, as WP7's reviews ran it. WP20 comes before WP8 and pauses every other agent for one series.

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

WP7.4 was re-planned on 25 November ([the diagnosis](LOG.md#wp7-diagnosis)); the other seven items passed as
built. Change nothing in `thrum-app`'s window here.

## WP19 While clouds drift, the game accounts for every refresh it loses, and none is lost to the clouds

Read [Holding the display's refresh](DESIGN.md#holding-the-displays-refresh-while-clouds-drift-wp19-and-wp20)
first. Take the first wind series before any repair. End `BLOCKED` after three repairs that leave a cause
standing, or once a window cause has its proposal in `PLATFORM-RUNTIME-MAC`'s log. An occluded run voids a
series: wake the display for each run only.

## WP20 With clouds drifting, the game holds the display's refresh in its ordinary window on the Mac

Every other agent waits while this is built: run the check and end the session. After three
series that are not a measurement, end `BLOCKED` with what was running.

## WP8 Under a clouded sky the land is lit by the clouds: an overcast valley is grey and soft, haze dims with it, and polished surfaces reflect the clouds

Rulings 3 and 4 are built in `ATMOSPHERE`'s reflection and diffuse producers and its aerial decoder. If `ATMOSPHERE`'s view volume has
landed, a scene that declares it takes no haze scale. If the producers cannot compose the overlay within their
existing bounds, write the proposal in `ATMOSPHERE`'s log and end `BLOCKED`.

## WP9 A visitor changes coverage, cloud type, wind and time of day with the controls, and the sky holds up in motion, on the Mac and in Chrome

TAAU is `APPLE-PRESENTATION`'s; a defect of the reconstructor's goes to that module's log. No governor is
needed here.

## WP10 The twelve accepted cloud volumes and the growing congestus each read as themselves in the finished sky

`B3-HANDOFF.md` at Broadsides `79637f789` names each shape's intended read and its accepted weaknesses, which
are not this work package's to fix.

## WP14 Mist lies in the valley's hollows and fades the land near the camera, in the land's own light

Follow the [ground-condensate integration handoff](DESIGN.md#ground-condensate-integration-handoff).

## WP15 Mist and fog banks reach the horizon, low cloud hides the hilltops, and a climbing camera passes into the cloud and out above it

If the handover shows in WP15.4 or WP15.13, the eighth risk gives the fixes in order.

## WP16 Through a day mist forms after dusk and burns off in the morning, haze runs from crisp to murky, and a visitor sets them, on the Mac and in Chrome

## WP11 Clouds drift over the open sea, shade it and show in its reflections, and a fog bank drifts ashore

`examples/water` is `WATERS`': add the selection and its surface field and change nothing else of that module's.

## WP17 Each fair-weather cloud holds up on its own: a cluster of heaps of different heights, with no shelf, fold or repeated outline

[Risk 2](DESIGN.md#risks) holds what WP3 found and what its looks ruled out. The records are
`.orchestrate/evidence/CLOUDS/WP3-erosion/` and `WP3-type-plane/`, whose archives hold the opacity observer, the point
march and the capture scripts. Find each flaw's cause with those controls before any look. WP17.1 and WP17.2
judge a sky of weather alone, the placed share at 0: the sky WP3 stopped on.

## WP12 On the Mac's Apple recipe the sky runs with MetalFX and the governor, its cadence on the built-in display is on record, and the module is closed

Run WP12.1's check first. If it fails, end `BLOCKED` with `needs_dependency` and build nothing: the governor and
MetalFX are `APPLE-PRESENTATION`'s to build. WP12.6 needs the owner's attended sitting; do every other item first.

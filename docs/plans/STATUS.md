# Status

The orchestrate script writes this page from the plan files and records after each record it lands. Don't edit it, because the next landing overwrites it. `status --format md` prints the same page.

## Overview

3 of 20 modules done · 3 in progress · 14 to come

### Waiting on you

- CREATOR assumes: the starter gets a Chrome form now. The G6 decision left it as an owner question and recommended deferring it to G7's page; website demos, and the sea and cloud demo scenes, need a creator's scene in Chrome. ([plan](CREATOR/PLAN.md))
- CREATOR assumes: "profiler" in the V1 exit is met by readouts in the demo UI on both targets, fed by the frame host's completed samples and the shells' callback records, not by a separate tool. ([plan](CREATOR/PLAN.md))
- CREATOR assumes: the profiler is built after `APPLE-TILE:M3` and not beside it, so it waits on that module's attended items. The other order has the readouts built now on the positional GPU slots, with the switch added later. ([plan](CREATOR/PLAN.md))
- CREATOR assumes: the profiler shows encode and submit as one row, and the frame-callback interval in place of a display interval, because those are what the engine measures. Physical cadence stays with the Mac qualification. ([plan](CREATOR/PLAN.md))
- CREATOR assumes: the profiler toggle is off by default, so a kit scene and the V1 window run untimed unless the profiler is on or a frames report was asked for. ([plan](CREATOR/PLAN.md))
- CREATOR assumes: the fresh-agent proof is run by the milestone's reviewer, who builds the scene before reading engine source. ([plan](CREATOR/PLAN.md))
- CREATOR assumes: G6 closes without the clipping overlay, the shadow-cascade overlay and parameter files as a cooked product, which the G6 decision holds or parks. ([plan](CREATOR/PLAN.md))
- APPLE-PRESENTATION assumes: the Chrome witness covers the same rows as the Mac, the volume and phase-edit rows included. The review required only the invalid-history rows and the history identity in Chrome; the fixture is shared, and Chrome is where cloud and sunset demos will run. ([plan](APPLE-PRESENTATION/PLAN.md))
- APPLE-PRESENTATION assumes: ΔE is CIEDE2000 on the submitted 8-bit SDR output, since reconstruction is judged as displayed. The bound stays at 3 and binds until a plan revision changes it. This replaces the earlier assumption that the bound is "restated after M3's first run". ([plan](APPLE-PRESENTATION/PLAN.md))
- APPLE-PRESENTATION assumes: the per-frame history identity reaches Chrome through the game page's own report, so T2's capture record is not changed. A builder who needs a T2 record change proposes it here first. ([plan](APPLE-PRESENTATION/PLAN.md))
- APPLE-PRESENTATION assumes: seams 1–4 of the design are accepted as written, with the G4 builder making the motion-only edit in the atmosphere shaders (seam 4) and the third forward target across contributors (seam 3) itself. ([plan](APPLE-PRESENTATION/PLAN.md))
- ATMOSPHERE assumes: readiness work for the sea and cloud port (M5) proceeds now, ahead of the owner's decision on when the port itself starts; the port decision says to keep the foundation converging on readiness. ([plan](ATMOSPHERE/PLAN.md))
- ATMOSPHERE assumes: G3s closes with the west-lookout crossing (M3) still parked and named as a known defect, and with M4 deferred; neither is judged in M6. ([plan](ATMOSPHERE/PLAN.md))
- ATMOSPHERE assumes: the sky's closing review waits for TAAU, so it is judged once on the image the exit will show. ([plan](ATMOSPHERE/PLAN.md))
- ATMOSPHERE assumes: solar sky radiance and solar aerial transport stay unqualified against an all-orders reference when G3s closes. M5 and M6 prove that the lent lookups return the stored products in the right units and that the existing bounded referees still pass; the ledger names the gap. A full accuracy campaign would be a separate plan. ([plan](ATMOSPHERE/PLAN.md))
- ATMOSPHERE assumes: the terrain lip on the shipped Ground E pan is judged in M6.1 as a brightness step, and the held 17.4 h stop is judged; only the moving spur edge of the parked M3 crossing is left out. ([plan](ATMOSPHERE/PLAN.md))

### Now and next

- The existing frame lifecycle runs through one split graph (APPLE-TILE M2): Not Started · [plan](APPLE-TILE/PLAN.md)
- Every pixel of the valley carries correct motion under jitter, including the sky and stars (APPLE-PRESENTATION M1): Not Started · [plan](APPLE-PRESENTATION/PLAN.md)
- ATMOSPHERE's design: Design Ready for Review · [design](ATMOSPHERE/DESIGN.md) · [plan](ATMOSPHERE/PLAN.md)

### Recently done

- The valley selects a recipe and stores only actual future inputs (APPLE-TILE M1), 11 Nov · [review](../evidence/APPLE-TILE/M1-review.md)
- The starter scene runs in Chrome from one command, with its controls and live asset edits (CREATOR M1), 11 Nov · [review](../evidence/CREATOR/M1-review.md)
- A repeatable offscreen valley proves the comparison path (APPLE-TILE M0), 11 Nov · [review](../evidence/APPLE-TILE/M0-review.md)
- iOS simulator engine motion without desktop capture (T2-CAPTURE M2), 10 Nov · [review](../evidence/T2-CAPTURE/M2-review.md)
- Cooked scene draws on adapters without indirect execution, selected from the capability record (COOKED-DRAW M1), 9 Nov · [review](../evidence/COOKED-DRAW/M1-review.md)
- Scene and internal render size follow the oriented iOS surface (PLATFORM-RUNTIME-IOS M7), 9 Nov · [review](../evidence/PLATFORM-RUNTIME-IOS/M7-review.md)

## All modules

| # | Module | What it's for | Needs | Status |
|---|---|---|---|---|
| 1 | PLATFORM-RUNTIME-IOS | The game runs on the iPhone simulator, through touch, rotation, backgrounding and graphics loss |  | Done 9 Nov |
| 2 | PLATFORM-RUNTIME-MAC | The game recovers from graphics loss on the Mac, and its motion is proved smooth on the built-in 120 Hz display | T2-CAPTURE M0, APPLE-TILE M3 | M2 of 4 · Blocked |
| 3 | T2-CAPTURE | Agents record exactly what the engine drew, as stills and motion, with nobody at the screen | PLATFORM-RUNTIME-MAC M1, COOKED-DRAW M1 | Done 10 Nov |
| 4 | COOKED-DRAW | Cooked scenes draw on devices that cannot run GPU-driven draws, such as the iPhone simulator | PLATFORM-RUNTIME-IOS M7 | Done 9 Nov |
| 5 | APPLE-TILE | Apple devices run their own frame recipe beside the portable one, with a native Metal step inside the frame | T2-CAPTURE M0 and M1, PLATFORM-RUNTIME-MAC M1, COOKED-DRAW M1 | M2 of 7 · Not Started |
| 6 | CREATOR | A newcomer starts a scene with one command, tunes it with controls and sees where frame time goes, on Mac and in Chrome | APPLE-TILE M3 | Waiting on APPLE-TILE:M3 |
| 7 | APPLE-PRESENTATION | Sharp, stable images from upscaling, highlights brighter than SDR white, and a governor that holds the frame rate | APPLE-TILE M0, M2, M3, M11 and M12 | M1 of 3 · Not Started |
| 8 | ATMOSPHERE | One physical sky, Sun, haze, night and stars, which every other part of the engine reads | APPLE-PRESENTATION M3 | Design Ready for Review |
| 9 | CULLING | Only what the camera can see is drawn, decided on the GPU | APPLE-TILE M2 | Not planned yet |
| 10 | EFFECT-GRAPH | Sea, clouds and sky run their own simulation and drawing steps inside the one frame | APPLE-TILE M2 | Not planned yet |
| 11 | CONTENT-LOOP | Edited meshes, textures and volumes appear in the running game without a stall | APPLE-PRESENTATION M2 | Not planned yet |
| 12 | SHOWCASE | The V1 exit: the showcase valley on Mac and in Chrome, reviewed, with its closing record | The last milestone of modules 1 to 9 and 11 | Not planned yet |
| 13 | SEA | An open sea from waterline to horizon, with floating bodies, wakes and foam | ATMOSPHERE M5, EFFECT-GRAPH, APPLE-TILE M3, APPLE-PRESENTATION M3 and M7, CONTENT-LOOP; the owner's port timing | Not planned yet |
| 14 | CLOUDS | A weather-driven sky from clear to overcast, with placed cloud volumes that shade the world | ATMOSPHERE M5, EFFECT-GRAPH, APPLE-TILE M3, APPLE-PRESENTATION M3 and M7, CONTENT-LOOP; the owner's port timing | Not planned yet |
| 15 | SIM-2D | Thousands of 2D bodies step identically on every device, and find neighbours and hits cheaply | SHOWCASE | Not planned yet |
| 16 | TRANSPORT | Two processes exchange messages over real internet protocols, one of them a browser | SHOWCASE | Not planned yet |
| 17 | RENDER-2D | Thousands of crisp sprites, drawn without paying for anything 3D | SHOWCASE, APPLE-TILE, CONTENT-LOOP | Not planned yet |
| 18 | SESSION | Two to four players share one run and keep seeing the same crowd | SIM-2D, TRANSPORT | Not planned yet |
| 19 | GAME-UI | Health bars, timers, menus and a join screen drawn in the game's own frame | RENDER-2D | Not planned yet |
| 20 | SWARM | The swarm-survival game itself: its name, rules, characters and art | SIM-2D, RENDER-2D, GAME-UI, TRANSPORT, SESSION | Not planned yet |

# Showcase: the exit scene and the V1 exit record (G7)

13 November 2026. [Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) ·
[module list](../ARCHITECTURE.md) · [the brief](../VISION.md). Governed by
[the route decision](../../design/decisions/G7-SHOWCASE-ROUTE.md), which built the launch and the six-stop route,
by C04, C05, C07 and C12 in [CONTRACTS](../../design/CONTRACTS.md), and by
[exact baselines](../ARCHITECTURE.md#exact-baselines). Revised on 18 November 2026 after WP5's first review:
[Motion on a shared machine](#motion-on-a-shared-machine) says how motion is judged before WP8. Revised again that
day for [the design review](../../../.orchestrate/evidence/SHOWCASE/design-review.md) of that revision: how each target produces
a tick-by-tick sequence, what build a timing record is of, how the exit baseline is replaced, and what WP7 does
with a late frame it cannot explain. Revised a third time for that review's second round: a timing record's
digest also covers the scripts that launch, drive and observe its arms. Revised on 19 November 2026 for
[the ruling on the valley's lookout rim and sunrise stop](../ARCHITECTURE.md#ruling-on-the-valley-fix), which
adds WP9: [The lookout's rim and the sunrise stop](#the-lookouts-rim-and-the-sunrise-stop). Revised the same
day for the design review of WP9: culling is compared again on the changed terrain, a diagnostic control is drawn
at its stop's hour, the bare valley's check cannot pass without its baseline, and what WP9 may change in another
module's files is said file by file. Revised for that review's second round: the root pins are every route file
that names an old root when WP9 starts, WP6's `exit-fast-yaw.json` among them. Revised on 20 November 2026 after
[WP9's review](../../../.orchestrate/evidence/SHOWCASE/WP9-review.md): WP9's landing also moved the retained references
into test fixtures, and [what moves with the terrain](#the-lookouts-rim-and-the-sunrise-stop) says which
paths follow them.

## The request

The human's words weren't passed on; this design answers the lead's task of 13 November 2026, to plan row 13 of
the module list. What the module must deliver is in [the brief](../VISION.md), the owner's page. Its text is
carried from the build plan and is not the owner's own words, except decision 3.

- **The V1 exit** (decision 1): "one reusable engine powers `examples/v1-game`, a single-player showcase on this
  M3 Max and its built-in 120 Hz ProMotion display, and the same scene in Chrome. It has physical sky, Sun and
  aerial perspective with time controls; cooked meshes/textures; live camera, lights and cascaded shadows; GPU
  culling; MetalFX, EDR and a quality governor; display-link interpolation; hot reload, profiler and
  native/browser demo UI. Keep 10,000 entities and 1,000–2,000 movers as the representative scene."
- **Gate G7:** "Assemble the exit scene on Mac Apple tile and portable and on Chrome portable. The fixed route
  passes correctness and agent visual review; record whole frames and visible stutter, startup and hitches. Native
  and web share scene and controls. Close with the visual digest, functional and platform status and the device
  runs owed."
- **Decision 2:** a gate closes when "its outcome runs in the game on the named target, correctness, lifecycle and
  required oracle checks pass, `./dev check` is green, and independent code, evidence and agent visual review
  pass."
- **Decision 3** (the owner, 6 November 2026): "Numeric performance and allocation figures are baselines, not
  gates."
- **Decision 4:** visual work "closes on a clean-context reviewer's pass. Acceptance never blocks other work."

## Purpose and scope

Every other track 1 module proves its part on the scene it had. This module puts the parts in one scene, shows
that they hold together on the three rows the gate names, and writes the record that closes V1. Its consumers are
the owner, who runs the showcase, and track 3, which starts when this module is Done.

**Three names used throughout.** The **exit scene** is the valley with its 10,000 entities. The **exit
configuration** is the one file that says how each row runs it. A **row** is one of: the Mac on its Apple recipe,
the Mac on the portable recipe, Chrome on the portable recipe.

**Built before this plan** and kept: the launcher `scripts/run_v1_showcase.py` (`mac`, `chrome`, `check`,
`startup`, `moving`), the six stop presets, the parity metric with its fixed calibration rule and controls, and the
valley's terrain and ground material ([route slice record](../../../.orchestrate/evidence/SHOWCASE/ROUTE-SLICE-RECORD.md)). Its
work packages were numbered M1 to M4, so this plan starts at WP5.

**What the valley is today.** `THRUM_SCENE=valley` draws the cooked terrain, one sphere and one rock. The
simulation already steps 10,000 entities with 1,500 movers, but they are cubes in a grid 1.8 m across near the
origin, about 240 m under the lookout, so none is seen. The representative scene exists as a workload and not as
a picture.

**Commissioned here:**

1. WP5: the valley is populated. 10,000 entities draw as cooked mesh rows, 1,500 of them moving.
2. WP6: the populated valley is culled on the GPU with no pixel changed, and the default mode is set by
   `CULLING`'s rule.
3. WP7: one command runs the exit scene on each row, and each run reports what it resolved, with frame records.
4. WP8: the visual review, the records taken on the built-in display, and the V1 exit record.
5. WP9, built after WP6 and before WP7: the lookout's ground runs to its rim without a bright band at sunset, and
   the sphere and the rock cast long shadows at the sunrise stop. Culling is compared with Direct again on the
   terrain that ships.

**Not built here:** level of detail for entities; entities that turn or animate; textures on entities
(`CONTENT-LOOP` keeps them on the scene's material table); the iPhone simulator as a row; a live culling switch;
sea or clouds; anything published to the website. Performance tuning belongs to the phase the brief puts after
the exit.

## Cues

| Cue | Where it must read | Range | Plausible wrong substitute, and what rejects it |
|---|---|---|---|
| **S1 A populated place** | The six stops and the exit flythrough, Mac and Chrome | Dawn to night; near ground to the far slopes | 10,000 entities that exist and are not seen, as today. Rejected by in-view counts at each stop and inspected frames |
| **S2 Things move** | A held view and the flythrough, tick by tick, and in real time at the close | 1,500 movers; a run of hours; pause and resume | Movers that drift out of the valley after ten minutes, or sit too far off to read. Rejected by an hour stepped in a test, a capture of every tick, and WP8's real-time review |
| **S3 One scene, three rows** | Each stop on each row | Apple and portable recipes; Chrome | A row that quietly ran a fallback, or a comparison that cannot see a missing prop. Rejected by the game's own report and a no-entities control |
| **S4 Culling is never seen** | The flythrough with movers moving | Frustum and Occlusion; both recipes; Chrome | Equality on a scene with nothing hidden. Rejected by a non-empty prepass and hidden slots in the rows |
| **S5 Smooth** | The built-in 120 Hz panel, at full screen on the Apple recipe; Chrome's window on it | The flythrough, a full day cycle, startup | A smooth offline movie, or 120 Hz held in a small window or on the bare valley. Rejected by capture-off frame records, and by OS-reported presented intervals of the populated flythrough with the drawable extent and cooked root the game resolved |
| **S6 One picture** | Stops and motion on each row | Low Sun, noon, night; fast yaw | Props lit or hazed differently from the terrain. Rejected by a clean-context review of crops |

The bar is a shipped high-end game's vista: scatter as in Red Dead Redemption 2's New Austin or Horizon Forbidden
West's desert, balloons adrift as over Forza Horizon 5's Mexico, time of day as `ATMOSPHERE` names it, smooth at
the display's full refresh. It is judged in words, never against a target image.

## Research and selection

- **Scatter from data, deterministic.** Horizon Zero Dawn places over 100,000 objects from authored 2D density
  maps turned into points by dithering, so placement is data-driven, deterministic and locally stable
  ([van Muijden, GDC 2017](https://www.guerrilla-games.com/media/News/Files/GDC2017_VanMuijden_GPUBasedProceduralPlacementInHorizonZeroDawn.pdf);
  I read the slides' summary, not the talk). Thrum's valley is small and fixed, so placement runs once, offline, in
  the generator that already authors the terrain. Density by distance and by asset size is the part carried over.
- **Frame records, not averages.** One 40 ms frame is a hitch a one-second average hides, so reviewers publish
  frame-time traces and low percentiles, with a warm-up run and the same content each time
  ([NVIDIA FrameView guide](https://help.bestware.com/hc/en-gb/articles/37725286171293-Analysing-frametime-consistency-and-1-Low-FPS-using-NVIDIA-FrameView)).
  A percentile says pacing is bad and not why, so the record here lists each late frame with its cause.
- **Read from this repository:** the route decision and its record, `examples/v1-game/src/{scene,content,showcase}.rs`,
  `scripts/run_v1_showcase.py`, and the designs of `CULLING`, `CONTENT-LOOP`, `APPLE-TILE`, `APPLE-PRESENTATION`,
  `CREATOR` and `PLATFORM-RUNTIME-MAC`.
- **What would change the selection.** If entities gain orientation from the tick page, movers may turn and the
  concept below widens. If the frame cannot hold the refresh interval with any measured prop set, entity level of
  detail becomes a `CULLING` proposal.

## Mechanisms

### The exit scene is new content; nothing that exists changes

`APPLE-TILE`'s exact baselines pin `THRUM_SCENE=valley`, the grid content and a fresh cook of
`examples/v1-game/assets/showcase` at root `70d2baf8…`, and `examples/v1-game/routes/moving-v1.json` pins that
root. So:

- The exit scene is a third game content beside `Grid` and `Field`, chosen by name. With it unselected, every
  existing configuration prepares, steps and draws as before.
- The valley's source folder and its cooked root do not change, except once, in
  [WP9](#the-lookouts-rim-and-the-sunrise-stop). The exit's props are separate sources, and the
  exit's cook source is the valley's files plus the props. The launcher already cooks a copy of the source; the
  copy step can assemble it. The builder may assemble it another way that keeps the valley's root.
- The six stop presets are reused as they are; WP9 changes the hour of one. The exit gets its own flythrough
  route file, pinned to the exit store's root.

### One generator authors the terrain, the props and the paths

`assets/showcase/generate.py` holds the only height function. The game has none, and must not gain a second copy.

- The generator writes a **layout**: for each of the 10,000 entities its mesh, material, position, turn and
  scale, and for each mover its loop. The layout is checked in and embedded by the native game and the Wasm.
- Still props are placed on the terrain surface from a density that falls with distance from the lookout and the
  flythrough. Each kind is placed only out to the distance at which it still reads; the builder sets those
  distances from captures.
- The generator refuses a prop inside the lookout or the camera's swept path, and a mover whose loop leaves the
  valley or comes near the ground.
- The generator is deterministic: it reproduces the checked-in layout byte for byte.

### Props are cooked mesh rows, sized by measurement

Entities draw whole mesh rows on `CULLING`'s bucket route, and `CONTENT-LOOP:WP2` supplies cooked meshes as rows.
The route allows 64 buckets, three per mesh, so the scene uses at most 21 meshes. Turn and scale come from each
slot's basis, fixed when the scene is prepared, so a few meshes give varied scatter. Entities are coloured by
material and carry no texture.

There is no entity level of detail, so triangle count is the cost lever. **Sizing rule:** before the final meshes
are made, sweep triangles per prop on the exit scene at 1920×1080 with Sun shadows, at the stop with the most
entities in view, on both Mac recipes, in two separate runs. Ship the richest measured set whose whole-frame GPU
p95 stays under the panel's 8.33 ms interval in both. If none fits, ship the lightest and record the gap. Chrome's
rows are recorded and size nothing. The chosen rows are this module's cost baseline.

### Movers translate on closed loops under one registered rule

The tick page carries an entity's position and material, not its orientation, and the existing rule adds a
constant velocity and never turns back, so any mover leaves the valley in time.

- One new registered rule moves each mover along its authored loop as a pure function of committed state and the
  tick. It is bounded by construction, so the scene looks the same after an hour as after a minute.
- It joins the game's corrected-run replay test with complete authority-state equality, as G2S requires. The four
  peers stay registered.
- Movers are things that need no turning: radially symmetric, or all facing one way. The plan's assumption picks
  hot-air balloons. A concept that needs turning is a proposal to the extraction owner, not built here.
- The GPU already interpolates positions between tick endpoints, so motion is smooth at any refresh.

### The exit configuration: one file, three rows, reported and not echoed

Reconstruction, EDR, the governor and the Apple recipe are selections their owners expose, several of them off by
default so that exact baselines hold. No one place says which of them the showcase turns on.

- One checked-in file lists, for each row, every selection that differs from the game's bare defaults, and the
  value the game must report for it. The launcher and every check read it. Nothing else holds a copy, as with the
  presets.
- The launcher translates the file into each target's existing inputs: environment variables natively, the page's
  query in Chrome. It adds no second way to select anything.
- The game reports what it **resolved**: the scene's cooked root, recipe, reconstructor, display mode, governor
  state and culling mode. A check compares the report with the file. The route slice learned that a field echoing
  the request proves nothing. Where an engine module does not yet tell the game a resolved value, the
  builder writes the proposal in the log and does not infer the value from the request
  ([resolved selections](../ARCHITECTURE.md#interfaces)).
- "The same scene" means the same cooked publication, layout, route and controls. Texture variant, reconstructor
  and display range differ by row, and the record says how.

The file's entries for MetalFX, EDR and the governor are written from `APPLE-PRESENTATION`'s closing record, so
WP7 depends on that module's paired proof, WP11, and WP7.1 checks the five work packages WP7 builds on in the
record files. WP7 does not wait for `APPLE-PRESENTATION` WP17.

### The lookout's rim and the sunrise stop

WP9, added on 19 November 2026. `ATMOSPHERE` WP6 judges the valley's sky and sunlight against a list of defects
earlier reviews recorded, and its builder stopped on two whose cause is the authored valley. The module list's
[ruling](../ARCHITECTURE.md#ruling-on-the-valley-fix) gives the valley's source and stop presets to this
module and the fix to WP9. The numbers below are in
[the design proof](../../../.orchestrate/evidence/SHOWCASE/design-proof/lookout-rim.md), which is arithmetic on the generator
and not a GPU run.

**The band along the rim.** At the `sunset-clear-sun` stop, and through a pan there, a bright band lies along
the far edge of the lookout's ground, on the Mac and in Chrome. The lookout's top is flat to 55 m west of
the camera and then rolls over into a face that drops 160 m. The mesh has a column of vertices at 40 m and the next
at 60 m. So one strip of triangles covers 20 m of ground that the camera sees and that slopes 4°, while its far
vertices store a normal turned 33° toward the west, taken from the fall beyond them. Shading interpolates
between the two, and at the 17.4 h Sun the strip climbs from an N·L of 0.13 to 0.63. The ground itself, sampled
from the height function, keeps N·L between 0.07 and 0.15 everywhere the camera can see, and the part that
turns to the Sun is hidden behind the edge. Beyond 500 m the stored normals agree with the triangles to 0.13 or
better, so the fault is the rim's alone.

**The mechanism.** The mesh resolves the rim: wherever the lookout's camera can see the top's edge, at any yaw,
each triangle it sees carries the normals of the ground that triangle covers, and none carries a normal that
belongs to the face beyond. Two forms give that. A crease puts a row where the top ends, with the top's normal
on it, and gives the face rows of its own; it leaves no band. More rows across a smooth roll-over shrink the
band to the last metre or two and leave a rise of about a half there, which may still read as a rim. The
builder chooses from stills, and may reshape the rim so that it reads as a rock edge, where a straight
featureless edge reads as a stripe. The height function stays the generator's only one. Two things were tried
before and are not the fix: tapering the shelf to the west halved the step and moved it, and shading with face
normals in the shader cut the step and added a second contour
([the record](../../../.orchestrate/evidence/v1-workload/G3S-HANDOFF.md)). The renderer does not change here.

**What is and is not the defect.** The defect is a band of brighter ground on a surface that is one piece.
Where the lookout's ground ends, the valley floor beyond is nearly 2 km farther off, lit at another angle and
later in shadow while the lookout is still lit. A difference there is a depth edge, as in any view from a
ledge, and passes when it reads as the lookout's edge. Between 17.4 and 17.8 h the west ridge's shadow
crosses the lookout, and its edge on the ground is a shadow and not the band. How that edge moves is
`ATMOSPHERE`'s parked crossing, which no work package here judges.

**The sunrise stop.** At 6.1 h the Sun is about 1° up and the east ridge, 7 km away, hides its whole disk from
the lookout. The far ridges are lit and the lookout is in shade, so the sphere and the rock can cast no shadow,
and the list's entry "missing object shadows at sunrise" can never pass there. The stop moves later, to the
first hour at which both props stand in full sunlight with the ridge's penumbra past them, found by stepping
the preset's hour on the controls' 0.01 h grid. The Sun is then still low and the shadows several times as long
as the props are tall. The preset file is the stop's one authority, and
[the route decision](../../design/decisions/G7-SHOWCASE-ROUTE.md)'s table takes the new hour in the same
landing. The ridge is not lowered: that would reshape distant land other stops see, to keep one hour.

**What moves with the terrain.** The valley's cooked root and the exit store's root change. So, in the one
landing:

- The layout is regenerated on the new terrain and passes the layout check. A prop keeps its place where the
  ground under it did not change.
- **Root pins.** Every file in `examples/v1-game/routes/` names the cooked root it was made for. The rule is
  by what a file pins, not by a list: each one that names the old valley root or the old exit root when WP9
  starts takes its new root, and its generation if that moved, and changes in nothing else. Keys, phase and
  checkpoints stay. The valley's root is pinned by `moving-v1.json`, `culling-fast-yaw.json`,
  `effect-state.json` and `effect-after-opaque.json`, which are other modules'. The exit's is pinned by this
  module's `exit-flythrough.json` and `exit-held.json`, and by `exit-fast-yaw.json`, which WP6 adds for its
  frame-edge inspection. A route left on an old root stops before its first frame with `runtime route cooked
  root mismatch`, and nothing WP9 runs plays the fast yaw, so WP9.7 counts the files at WP9's starting commit. The
  launcher's calibrated root is the only other pin today. The rule "only the pin" is about a file that holds
  a pin. It says nothing against the next bullet.
- **`APPLE-TILE`'s references.** The retained references that moved, images and manifests, are regenerated by
  that module's own scripts, so their bytes change. In the registry the `pin` and the `refresh` of each
  baseline whose reference moved change. `APPLE-TILE`'s log gains the entry the ruling asks for. All three are
  the refresh the ruling requires, and a reviewer expects to see them in the diff.
- **Where the references are kept.** The same landing moves every retained reference out of `docs/evidence/`
  into the game's test fixtures, on the lead's ruling of 20 November 2026
  ([the module list](../ARCHITECTURE.md#ruling-on-relocated-references)). Each file keeps its name, and one
  that was not regenerated keeps its bytes. Only the paths that say where the references are follow them: the
  registry's `reference` and `covered_paths`, and the folder `scripts/apple_tile_m1.py` reads the native
  references from. Every pin changes with its path. A reviewer expects these in the diff too.
- **What stays.** What each comparison compares and how it decides. The registry's other fields and every
  other line of the files it lists as comparisons do not change.
- **This module's two thresholded baselines** are calibrated again, each once, for its new root, by the route
  decision's fixed rule, with every control failing as it must. That is a first calibration for a new root and
  not [the refresh](#the-route-check-sees-a-missing-prop), which keeps a root. No threshold is tuned.
  [What a route check must have compared](#what-a-route-check-must-have-compared) says what the two checks
  gain in this landing.
- **Culling.** The occluding mesh, the layout and the exit root are new, so
  [WP6's comparison is made again](#culling-on-the-assembled-scene) on them.

**How it is proved.** By eye on the Mac, in fixed captures with TAAU on, which is how the gate that asked for
the fix will judge it: the sunset stop, views 20° either side of it, and that view held from 17.2 to 17.8 h,
where the earlier sweep found the step largest; and the sunrise stop. The route checks compare the Mac with
Chrome at all six stops, so Chrome gets no inspection of its own here; `ATMOSPHERE` WP6 judges it there, with
motion. Culling is the exception to "by eye": an object wrongly hidden as it clears the new lip would vanish
alike on both targets and from both baselines, so it is proved by equality with Direct. While the terrain is
open the builder looks at the six stops for any other entry of
[`ATMOSPHERE`'s list](../ATMOSPHERE/DESIGN.md#closing-g3s) whose cause is the terrain's mesh, fixes it in the
same regeneration, and records in the log the ones whose cause is not, for WP6. The bar is that module's: time
of day as in Red Dead Redemption 2 or Horizon Forbidden West.

### The route check sees a missing prop

The parity metric averages each frame to a 32×18 grid. Its maximum cell catches a missing rock at the lookout, but
a thin scatter of small props could vanish on one target and stay under the thresholds. So the exit check gains a
third control beside the diagnostic scene and the retinted texture: the same store drawn with no entities must
fail at every stop that has entities in view. If it does not, the metric goes back to design (a finer grid) and is
not tuned to pass. Thresholds are set once for the exit store's root by the route decision's fixed rule. The exit
has its own baseline file; the bare valley's check and baseline are untouched until WP9.

<a id="what-a-route-check-must-have-compared"></a>

**What a route check must have compared (WP9).** Two gaps in the checks as built would let WP9's recalibration
pass on less than it claims. WP9 closes both in the launcher, and
[the route decision](../../design/decisions/G7-SHOWCASE-ROUTE.md) takes the same two changes with the new hour.

- **A control is drawn at its stop's hour and pose.** The negative control is the diagnostic scene, the page
  without the cooked store, and it shows the metric would catch a fallback. It shows that only when nothing
  else differs. Today five stops have one and each is at its stop's hour, yaw and pitch. The sunrise control is
  the checker's `phase-scene-sunrise`, which presses the page's Sunrise button and expects 6.1 h, so once the
  preset moves, a tenfold difference there could come from the sunlight alone. So each check reads, for every
  control frame, the hour, yaw and pitch the page applied, writes them in its record beside the stop's
  preset, and fails when they differ by more than the controls' steps, as the route frames are already held
  to. The sunrise control becomes a diagnostic frame at the preset's hour and pose, which the page's controls
  can set, as the checker's midnight phases show. The `sky-reflections` case keeps its nine phases and what
  they expect, and the Sunrise button keeps 6.1 h: other modules' evidence rests on both. The
  `sunset-clear-sun` stop still has no control.
- **The bare valley's canonical check fails unless it compared.** The exit check already refuses a baseline of
  another identity. The bare valley's does not: its comparison returns `other identity` or `absent` before
  reading a grid, and the check counts a baseline failure only when the status is `compared`. A baseline
  whose frame-source version differed would pass unread. The canonical check is `check` with the checked-in
  source and baseline and without `--recalibrate` or `--save-baseline`. From WP9 it fails unless the status is
  `compared` and all six stops on each target are under the thresholds. A run on another source or a scratch
  baseline, `--recalibrate` and `--save-baseline` behave as before: there another identity is information, or
  the thing being replaced.

**Replacing the exit baseline.** `check --exit --save-baseline` is the first calibration and nothing else: it
refuses to run while a baseline exists, and with the file deleted it would compute the thresholds again. A
reviewed change that moves a stop, as `ATMOSPHERE` WP10's shadow correction may, needs a refresh, which the
launcher gains the first time one is needed. The refresh requires the existing baseline and refuses a run whose
identity (comparator, frames, cooked root and presets) differs from the saved one. It judges parity and the
three controls by the saved thresholds, and replaces the grids only when all pass. It keeps the identity and the
thresholds, and writes into the baseline file a record of the refresh: the cause, the builds before and after,
and for each target and stop how far the grid moved, marking those over the thresholds. The cause and the old
and new file hashes go in the log. From WP7 the same refresh serves each row's baseline.

**Three rows, one encoding (WP7).** The rows are compared as 8-bit sRGB submitted output after tone mapping,
before display: T2's `sdr` source in Chrome and on the Mac's portable row. The Apple row keeps EDR selected and
is captured at headroom 1, where
[`APPLE-PRESENTATION`'s EDR transform](../APPLE-PRESENTATION/DESIGN.md#the-edr-transform-and-its-h--1-identity-wp6)
is the SDR curve term for term, and its submitted float frame is encoded to 8 bits for the comparison, within
one code of the SDR path's. What that row shows above headroom 1 is compared by nothing here: the exit record
cites `APPLE-PRESENTATION`'s own EDR evidence for it. The comparison's record names each row's source and
headroom. If a row cannot give this encoding, that is a proposal in the log, not a second metric.

### Culling on the assembled scene

`CULLING` closed on the scenes that existed and left this rerun here. WP6 runs its equality command on the exit
scene: Frustum and Occlusion with Sun-caster culling against bucket-ordered Direct, at every submitted frame of
the flythrough with movers moving, on both recipes, and at checkpoints in Chrome. The default is set by
`CULLING`'s rule from paired rows in two runs. This module changes no culling code.

**The comparison is of one cooked root.** Equality says nothing was wrongly hidden by the geometry it ran on.
The Metal oracle is an ignored test that only the launcher's `culling` command runs, so `./dev check` does not
repeat it, and the command refuses a store whose root is not the one the flythrough pins. WP9 changes the
occluding mesh, the layout and the exit root, and neither a layout check nor a comparison of the Mac with
Chrome compares a culled draw with Direct. So WP9 runs WP6's two equality commands again, on both Mac recipes
and in Chrome, with its own receipts beside WP6's. WP6's receipts stay as they are, the record for the root
they name. Nothing else of WP6 is repeated: the default is a cost choice already made, and motion on the changed
valley is judged in `ATMOSPHERE` WP6 and in WP8. The same holds for any later landing that changes the exit
root, such as content regenerated to fit the frame in WP7. The exit record's culling clause cites the receipts
whose cooked root is the one the claimed commit cooks, and WP8.8 asks for that.

### Frame records, startup and hitches

- **Unattended (WP7).** With capture off, the frame host's completed samples over the flythrough and one full day
  cycle, per row: CPU stages, GPU passes by name and the callback interval, with build and load. Every frame that
  missed its opportunity is listed with its duration, its place on the route or in the day and what its
  investigation found. Startup to the first exit-scene frame is a labelled proxy.
- **What an investigation can end in.** WP7's records are taken while other builders work. The investigation
  reads the late frame's own stage and pass times, its neighbours and the load. Where a stage or pass of the
  engine's or the page's accounts for the delay, that is its **cause**, and it is fixed or proposed to its owning
  module. Where the record shows the delay lay outside the game, it says how. Where the frame's own work was
  ordinary and the time went between callbacks, the record shows the gap and cannot say whose it was: the frame
  is listed as **unresolved under load**, with the evidence read, and carried to WP8. Load is never a cause by
  itself. It is the reason a cause could not be established. WP8.13 answers every carried frame from the sitting
  with the other builders paused: it did not recur at that place, or it recurred and gets a cause there.
- **On the built-in display (WP8, attended setup).** The Mac's presented-time witness, as `PLATFORM-RUNTIME-MAC`
  qualifies it, over the flythrough at full screen on both recipes with capture off, and windowed startup on
  both. [The full refresh at full screen](#the-full-refresh-at-full-screen-on-the-populated-exit) says what the
  Apple recipe's takes must show. For Chrome, the page's frame-callback intervals in a visible window. No
  presented-time witness exists for Chrome, and the record says so.
- **Tripwire.** A row about a quarter slower than WP5's baseline at the same stop is investigated (brief decision
  3). No figure is a gate.

### The full refresh at full screen, on the populated exit

Added 26 November 2026, after [the design review](../../../.orchestrate/evidence/SHOWCASE/design-review.md).
`APPLE-PRESENTATION` WP17 holds the display's full 120 Hz at full screen on its own route, and its full-screen
driver runs the bare valley over `moving-v1.json`. The exit scene is another cooked root with 10,000 props on
it, and its route file declares a 1280×720 render extent for the route check. So WP17 Done and a smooth exit
flythrough in a window would not together show the populated showcase smooth at full screen. WP8.6 claims that
directly, in the sitting WP8 already takes with the other builders paused.

- **The run.** The exit configuration's Apple recipe row, launched as WP7 launches it, with the window at full
  screen and nothing else changed: capture off, the release game, the exit flythrough from its first tick to its
  last. The route file's render extent is for the route check's captures and sizes no frame of this run: the
  governor chooses the render scale, as the row's entry says.
- **What each take records, from what the game resolved and never from the request.** The scene's cooked root,
  which is the exit's. The drawable's pixel extent, which is the panel's. The
  [requested frame rate](../APPLE-PRESENTATION/DESIGN.md#the-requested-frame-rate-at-full-screen-wp11), which is
  the panel's 120 Hz with no launch setting forcing it, as it is once WP17 is Done. Where the Mac host does not
  yet tell the game one of these, that is a proposal to its owning module, as for
  [the exit configuration](#the-exit-configuration-one-file-three-rows-reported-and-not-echoed).
- **The judge.** The presented-time witness `PLATFORM-RUNTIME-MAC` qualified: an OS-reported presented interval
  for every submission, in multiples of the display's refresh period. `APPLE-PRESENTATION`'s refresh account
  counts the engine's present calls. This counts when frames reached the panel.
- **What holds means.** Two takes. In each, every presented interval is one refresh period. An interval that is
  longer is listed with its place on the route, and is excused only when the record shows the delay lay outside
  the game and no long interval sits at that place in the other take. A long interval the frame's own stages or
  passes account for, or one that recurs at its place, is the game's: it is fixed and both takes are made again.
  A proposal to another module does not answer it. Every full-screen take of the sitting is kept and listed with
  the host's load, so two that hold are not two chosen from many without that being seen.
- **If it does not hold.** The first remedy is content, by regenerating, as for
  [risk 1](#risks). A cause in another module's code is handled as
  [defects found in another module's code](#defects-found-in-another-modules-code) says. Accepting less than the
  full refresh at full screen changes the vision's bar, which is the owner's: the builder ends `NEEDS_CONTEXT`
  with the record, and WP8.6 stays open.
- **The portable recipe and startup (WP8.14).** The portable row runs the same flythrough at full screen with
  the same record and witness. It is the Mac's fallback recipe and no work package brings it to 120 Hz at the
  panel's pixel extent, so its record says what it held, and each missed refresh is listed with a cause that is
  fixed, proposed to its owning module, or shown to lie outside the game. Startup to the first exit-scene frame
  is recorded windowed on both recipes, apart from the full-screen takes, so neither waits on the other.

### Motion on a shared machine

WP5's first review failed its two motion items on real-time takes: a held image for 49.81 ms and a step after
an 86.31 ms gap on the Mac, and a 92.8 ms gap in Chrome, every submitted image retained
([review](../../../.orchestrate/evidence/SHOWCASE/WP5-review.md)). The follow-up found two different things
([diagnostics](../../../.orchestrate/evidence/SHOWCASE/WP5-motion-diagnostics.md)).

- **A shadow that blinks on, in the renderer.** A dark strip appears on the Mac between committed ticks 233
  and 234 with every tick submitted. It is `ATMOSPHERE`'s, fixed by its M10
  ([what the caster height map holds](../ATMOSPHERE/DESIGN.md#what-the-caster-height-map-holds)), which WP5
  waits for.
- **Whole-frame gaps, from the machine.** Complete takes still held gaps: 121.8 and 176.1 ms in Chrome, 54 and
  70 ms on the Mac, and gaps with capture off as well. Every arm was taken while other builders compiled and
  tested, at load averages of 4 to 55. Nothing on a shared machine separates the engine's share of such a gap
  from the scheduler's, and [the project settings](../../../ORCHESTRATE.md) make a timing inside a work package a
  tripwire for that reason.

So motion is proved in two parts, and the project's rule for timing decides which part can fail a work package.

1. **What the engine draws at each tick is judged, and gates.** WP5.8 and WP5.9 judge tick-by-tick sequences.
   On the Mac every world tick is submitted exactly once. In Chrome a due tick can be deferred to the next
   callback under load, so a judge reads the source ticks and judges the runs that advance one at a time, as
   **Chrome** below says. What such a sequence draws at a tick is the same whatever the machine is doing, so a
   pop, a jump, a mover through terrain or a shadow that blinks is the engine's and is a finding.
2. **How evenly real time delivers those frames is recorded, and does not gate before WP8.** WP5.11 keeps
   real-time takes on both targets with capture-off arms back to back, their timelines and the load. Its
   check fails on a record of another build or of other capture scripts, on a missing arm or field, on an arm
   whose world stopped early,
   and on a median interval more than twice that of the retained arm it repeats. Those arms held the requested
   60 Hz: medians of 16.8 and 16.7 ms on the Mac and 16.7 ms in Chrome. A long interval in the record is
   looked into when the take's own timeline points at the engine, and is never a finding by itself.

**How each target produces a tick-by-tick sequence.**

- **Mac.** T2's `fixed` runner, `capture_v1_runtime.py --mode fixed`, cannot give one. Its loop ends after 480
  opportunities, four seconds, and it advances the world from elapsed host time, so a slow readback changes
  which ticks are submitted. The native route recorder can (`examples/v1-game/src/bin/window/route.rs`, started
  with `THRUM_CAPTURE_ROUTE`). It gives the world a synthetic time one tick on from the last, at a chosen phase,
  reads T2's submitted-image tap, and refuses a step that did not submit exactly one fresh tick. It logs every
  routed tick with its submission, and today keeps pixels only at a route's named checkpoints.
- **Windows.** WP5 extends the recorder to keep the submitted image of every tick inside **windows**, runs of
  consecutive routed ticks that the caller names, each image with the identity a checkpoint has. A take's span
  comes from its log: one line for each routed tick, in order, at one phase. A judge names its own windows, so
  no stretch of a take is the builder's choice alone. The route slice built this recorder and other modules'
  scripts drive it for registered checkpoint bytes, so a run that names no window records exactly as before.
- **The two takes.** The flythrough is `exit-flythrough.json`, 1,350 ticks. The held view is a second route
  file, pinned to the exit root, that holds the pose of the `exit-drift` case for at least 1,200 ticks. Both fit
  the recorder's 1,800-tick limit for an exit route.
- **Phase.** A route file's phase is one half. The strip was reproduced at route ticks 202 to 203, committed
  ticks 233 to 234, at phase 0.9999, which `THRUM_CAPTURE_PHASE` sets
  ([record](../../../tests/fixtures/showcase/motion-diagnostics.json), `fixed_shadow_diagnostic`). That window is
  judged at both phases, because a fault that shows near the end of a tick can hide at its middle.
- **Chrome.** `capture_v1_browser.mjs --mode fixed` gives the page a synthetic 60 Hz time at each callback, at
  phase one half, and retains every submission with its source tick. Under load the page can defer a due tick
  to the next callback ([ruling](../ARCHITECTURE.md#a-fixed-capture-that-stalls)): one record repeats a tick
  and the next carries two. That is a step of the page's clock and not of the scene. A judge reads a run's
  source ticks first, judges on runs whose ticks advance one at a time, and does not count a deferred tick as a
  jump. A deferral accounts for how far the scene moves between those frames and for nothing else: a mover or
  prop that is missing, new or changed in them is judged like any other.

**What a timing record is of.** A record that names a build proves nothing if the check never looks: a change
to a frame driver after the arms were taken would leave the record, the launcher and the check's result as they
were. The same holds for how the arms are run. A capture script decides which route the launcher starts, how a
frame is driven and how many submissions the receiver keeps, with the game's binary unchanged: Chrome's receiver
refused the flythrough's record 1,024 until `4ffb33052` raised its limit, a change to two files under `scripts/`
and to nothing else. So `motion` refuses a tree with uncommitted changes, builds both programs from it, and takes
all six arms from that one commit. The record holds the commit, each program's hash, and one digest of every
tracked file there that builds, launches, drives or observes an arm: the files under `crates/`,
`examples/v1-game/` (which holds the routes and the capture page), `scripts/` and `.cargo/`, and `Cargo.toml`,
`Cargo.lock` and `rust-toolchain.toml`. `scripts/` is taken whole and not as a list of today's helpers, because
the launcher and the two capture drivers import across it and a list would miss the next helper. `motion --check`
computes that digest again from the checkout it runs in, without building, and fails when it differs. WP5.11's
`reads` lists exactly those paths and the two records the check compares, so a pass is not reused once one of
them changes: the digest and the reuse of a pass cover the same files. Other modules land changes to those paths
most hours, and a record cannot outlive one. So the arms are the work package's last act, taken on a branch that
already holds main, and landed with nothing else. If another landing gets in first, the check fails on main and
the six arms, a few minutes of takes, are taken again. Nothing else is proved twice.

In WP5, WP6 and WP7 a reviewer who sees a step in a real-time take reads the take's timeline first. A step the
elapsed time between two submissions accounts for is a recorded gap. A step it does not account for, or
content that differs from the tick-by-tick capture of the same tick, is a finding. A late frame in WP7's frame
records is handled as [Frame records, startup and hitches](#frame-records-startup-and-hitches) says. Stutter at
1x is judged in WP8.2 and WP8.5, and on the built-in display in WP8.6, WP8.7 and WP8.14. WP8 is this module's last work package,
so its real-time takes and frame records, the three attended ones included, are made in the sitting the project
settings give a last work package, with the other builders paused.

### Defects found in another module's code

Assembly finds defects no single module could. The builder fixes one in place when the fix keeps that module's
design, tests and exact comparisons true, and notes it in both logs. A fix that changes an interface, a design or
an exact comparison is a proposal in this log, and the builder ends `BLOCKED needs_dependency`.

### The exit record

One page, `.orchestrate/evidence/SHOWCASE/V1-EXIT.md`: each clause of the V1 exit and of gate G7 with its evidence at the
claimed commit, the rows it ran on and its status as implementation, correctness, visuals and device, kept apart;
the visual digest; platform status, the iPhone simulator's taken from the modules that proved it; known defects;
and the physical-device runs owed. The maturity ledger and the README are brought to match it. The brief is the
owner's page and is not edited.

**What the exit waits for from `APPLE-PRESENTATION`.** Its WP11 ends with a full-screen window on the built-in
display presenting at a steady 60 Hz, an interim rate, and its WP17 is Done when that window holds the display's
full 120 Hz ([its design](../APPLE-PRESENTATION/DESIGN.md#reaching-the-full-refresh-at-full-screen-wp17)). The
exit is judged "smooth at the display's full refresh" on that display, so WP8 depends on WP17 and starts after
it, and WP8.12 lists WP17 among the work packages the exit rests on. An open WP17 keeps the exit open. Taking
WP17 out of the exit changes the exit, which is the owner's call. WP17 proves the rate on the bare valley, so the
exit record's claim for the showcase cites WP8.6's two takes of the populated flythrough, with their drawable
extent, and not WP17 alone
([the full refresh at full screen](#the-full-refresh-at-full-screen-on-the-populated-exit)).

### Check that it can work

- **Cost.** `CULLING` WP2 measured the mixed-mesh field, 100,513 instances with Sun shadows at 1920×1080: whole-GPU
  p50 48.7 ms with Frustum in the open and 108 ms Direct, contended
  ([record](../../../.orchestrate/evidence/CULLING/WP2.md)). A tenth of the instances at that mesh mix predicts about 5 ms, close
  to the 8.33 ms interval before terrain, sky and reconstruction. So the sweep comes first, on stand-in meshes.
- **Size on screen.** At 1080p and a 46° field of view a pixel spans about 0.74 mrad. A 1 m prop is under two
  pixels beyond 700 m; a 20 m balloon is still five pixels at 5 km. Small props stay near, and movers can fill the
  far valley.
- **Bounded movers.** A loop that is a function of the tick cannot drift. A test steps one simulated hour.
- **Baselines.** With the exit content unselected nothing in the covered configurations changes, and the
  main-branch check `exact-images` proves it after every landing.

## The system as it runs

| State | Owner |
|---|---|
| The exit content, its layout and generator, prop sources, the mover rule, the exit configuration, the flythrough, the exit baseline, the launcher and the exit record | This module (`examples/v1-game`, `scripts/run_v1_showcase.py`, `.orchestrate/evidence/SHOWCASE/`) |
| The bucket route, the mesh arena, culling modes and their rule | `CULLING` |
| Cooked products, mesh rows, the publication | `CONTENT-LOOP` |
| Recipes, the graph, the native segment | `APPLE-TILE` |
| Reconstruction, EDR, the governor, pipeline preparation | `APPLE-PRESENTATION` |
| Sky, Sun, aerial perspective | `ATMOSPHERE` |
| Controls, the profiler, the app runner | `CREATOR` |
| Engine capture; the presented-time witness | `T2-CAPTURE`; `PLATFORM-RUNTIME-MAC` |

**A launch, in order.** The launcher builds, assembles and cooks the exit source, reads the row's entries from the
exit configuration and starts the game. The game prepares the exit content from the embedded layout and the
store's mesh rows, resolves its selections and writes its report. The launcher compares the report with the file
before it trusts any frame.

## Exact baselines

- No registered exact baseline moves, except in WP9. There the terrain changes, so all five registered sets are
  expected to move, and WP9 refreshes them in its own landing under
  [the ruling](../ARCHITECTURE.md#ruling-on-the-valley-fix), and moves them into the game's test fixtures
  under [the relocation ruling](../ARCHITECTURE.md#ruling-on-relocated-references). In every other work package the valley's source
  root, the grid and field contents and the default output stay as they are.
- Every landing here changes covered paths. WP7 and WP8 carry no verifier item: the scheduled main-branch
  check `exact-images` proves each landing on main after the merge
  ([why](../ARCHITECTURE.md#proofs-run-once-on-main)). WP5 and WP6 are Done and each carried one, which passed
  from that check. No builder runs the verifier before a merge.
- This module owns two thresholded baselines: the bare valley's, unchanged, and the exit's, created in WP5 and
  extended to one per row in WP7. `ATMOSPHERE` WP10 takes a balloon's false strip off the far ground, so the
  exit's may be replaced once in WP5 for that cause, by
  [the refresh](#the-route-check-sees-a-missing-prop), which keeps its identity and thresholds. WP9 calibrates
  both again for the new roots, and from then the bare valley's canonical check fails without its baseline.

## Limits for every work package

- **Nothing that exists changes.** With the exit scene unselected, every existing configuration prepares, steps
  and draws exactly as before, because exact baselines pin them. If one does not, stop and fix that first. WP9
  is the one exception, for the terrain and the sunrise hour alone.
- **Another module's comparison is not edited.** If a change fails an exact test, a baseline or a witness
  another module owns, write the proposal in the log and end `BLOCKED`
  ([rule 1](../ARCHITECTURE.md#exact-baselines)). The path a comparison reads its references from, moved under
  [the relocation ruling](../ARCHITECTURE.md#ruling-on-relocated-references), is not such an edit.
- **Exact images are proved on main,** by the scheduled check `exact-images`. No item of WP7 or WP8 asks for
  the verifier. Do not run it before a merge.
- **The game gets no height function.** Anything that needs the ground's height is decided in the generator,
  so that two copies cannot disagree.
- **Entities stay as V1 has them.** A mover that needs to turn, or a prop that needs a texture or a level of
  detail, is a proposal in the log and not work here.
- **WP6 and WP9 change no culling code or comparison.** If equality fails, stop and diagnose. Never widen a
  comparison. A landing that changes the exit root runs the two equality commands on the new root and keeps
  earlier receipts as they are.
- **No parity threshold is tuned.** If a control does not fail as it must, the metric comes back to this design.
- **A tripwire that trips is investigated.** A record is of the commit being claimed, with its load.
- **No target stands in for another.** If Chrome or Instruments will not start, leave the item open and say so.

## Risks

1. **The frame does not fit.** Early sign: WP5's sweep. Fallback: lighter meshes or fewer props near the lookout,
   by regenerating; then a level-of-detail proposal to `CULLING`.
2. **Distant props shimmer under reconstruction.** Early sign: WP5's flythrough crops. Fallback: shorter placement
   distances per kind.
3. **Props are lit or hazed unlike the terrain.** Early sign: WP5's stills at low Sun. It is an integration defect
   for `ATMOSPHERE` or the forward pass, handled as above.
4. **A row falls back silently.** Mitigation: the resolved report and a control with one selection removed.
5. **Shared files move under the builder.** `scripts/run_v1_showcase.py` and the game's sources are edited by
   other modules. Integrate sequentially. After a rebase, rerun only what the script says failed.
6. **The built-in display is not free when WP8 is ready.** Its three attended items are done in one sitting, batched
   with other modules' attended items.

## Proof and builder handoff

- **Targets.** `mac-metal` on both recipes, and `chrome-webgpu`. The iPhone simulator is not a row.
- **Capture.** T2 engine capture on the Mac and in Chrome: `fixed` for stops, checkpoints and WP5's tick-by-tick
  motion, which on the Mac comes from the native route recorder; `realtime` for WP5.11's record and for motion
  from WP6 on. [Motion on a shared machine](#motion-on-a-shared-machine) says how each is produced and read.
  Frame records are taken with capture off.
- **Reaching the targets.** A Mac GPU run holds the `gpu` lock and a run that starts Chrome holds `chrome`. The
  project runs Codex with `danger-full-access`, so Chrome and Instruments start without an approval. A reviewer
  takes its own captures and never judges the builder's in place of them. If a reviewer cannot start a target,
  the review moves to Claude.
- **Attended setup.** Three items in WP8 need the built-in display unlocked and awake. The agent runs and reviews;
  the owner does nothing else. Console state is recorded and is never a pass condition elsewhere.
- **Storage.** Scratch in `tmp/showcase-<work-package>/`; selected evidence in `.orchestrate/evidence/SHOWCASE/`.

## Open questions

- Whether Occlusion or Frustum is the exit scene's default. WP6's paired rows settle it.
- Whether the frame holds the refresh interval under the full exit configuration. WP7's records settle it
  first, and WP8.6's takes at full screen; the remedy is content, by regenerating.
- How long a day-cycle sweep runs, and what its captures may store. The exit scene has a solar-time control and
  no sweep yet. WP7 builds the sweep for its capture-off records, which store no images. WP8's real-time day-cycle
  takes run longer than T2's four-second case, so before the first one its builder writes the storage record
  [T2's design](../T2-CAPTURE/DESIGN.md) asks of a longer session: extent, duration, expected peak from the
  bytes WP5's takes measured for each frame, the excerpts kept and the expiry.

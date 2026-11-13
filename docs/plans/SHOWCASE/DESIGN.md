# Showcase: the exit scene and the V1 exit record (G7)

13 November 2026. [Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) ·
[module list](../ARCHITECTURE.md) · [the brief](../BRIEF.md). Governed by
[the route decision](../../design/decisions/G7-SHOWCASE-ROUTE.md), which built the launch and the six-stop route,
by C04, C05, C07 and C12 in [CONTRACTS](../../design/CONTRACTS.md), and by
[exact baselines](../ARCHITECTURE.md#exact-baselines).

## The request

The human's words weren't passed on; this design answers the lead's task of 13 November 2026, to plan row 13 of
the module list. What the module must deliver is in [the brief](../BRIEF.md), the owner's page. Its text is
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
valley's terrain and ground material ([route slice record](../../evidence/SHOWCASE/ROUTE-SLICE-RECORD.md)). Its
milestones were numbered M1 to M4, so this plan starts at M5.

**What the valley is today.** `THRUM_SCENE=valley` draws the cooked terrain, one sphere and one rock. The
simulation already steps 10,000 entities with 1,500 movers, but they are cubes in a grid 1.8 m across near the
origin, about 240 m under the lookout, so none is seen. The representative scene exists as a workload and not as
a picture.

**Commissioned here:**

1. M5: the valley is populated. 10,000 entities draw as cooked mesh rows, 1,500 of them moving.
2. M6: the populated valley is culled on the GPU with no pixel changed, and the default mode is set by
   `CULLING`'s rule.
3. M7: one command runs the exit scene on each row, and each run reports what it resolved, with frame records.
4. M8: the visual review, the records taken on the built-in display, and the V1 exit record.

**Not built here:** level of detail for entities; entities that turn or animate; textures on entities
(`CONTENT-LOOP` keeps them on the scene's material table); the iPhone simulator as a row; a live culling switch;
sea or clouds; anything published to the website. Performance tuning belongs to the phase the brief puts after
the exit.

## Cues

| Cue | Where it must read | Range | Plausible wrong substitute, and what rejects it |
|---|---|---|---|
| **S1 A populated place** | The six stops and the exit flythrough, Mac and Chrome | Dawn to night; near ground to the far slopes | 10,000 entities that exist and are not seen, as today. Rejected by in-view counts at each stop and inspected frames |
| **S2 Things move** | A held view and the flythrough, in real time | 1,500 movers; a run of hours; pause and resume | Movers that drift out of the valley after ten minutes, or sit too far off to read. Rejected by an hour stepped in a test and a real-time capture |
| **S3 One scene, three rows** | Each stop on each row | Apple and portable recipes; Chrome | A row that quietly ran a fallback, or a comparison that cannot see a missing prop. Rejected by the game's own report and a no-entities control |
| **S4 Culling is never seen** | The flythrough with movers moving | Frustum and Occlusion; both recipes; Chrome | Equality on a scene with nothing hidden. Rejected by a non-empty prepass and hidden slots in the rows |
| **S5 Smooth** | The built-in 120 Hz panel; Chrome's window on it | The flythrough, a full day cycle, startup | A smooth offline movie. Rejected by capture-off frame records and OS-reported presented intervals |
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
- The valley's source folder and its cooked root do not change. The exit's props are separate sources, and the
  exit's cook source is the valley's files plus the props. The launcher already cooks a copy of the source; the
  copy step can assemble it. The builder may assemble it another way that keeps the valley's root.
- The six stop presets are reused as they are. The exit gets its own flythrough route file, pinned to the exit
  store's root.

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

Entities draw whole mesh rows on `CULLING`'s bucket route, and `CONTENT-LOOP:M2` supplies cooked meshes as rows.
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

The file's entries for MetalFX, EDR and the governor are written from `APPLE-PRESENTATION`'s closing record. Its
M4 to M11 have no done-when entries yet, so M7 cannot be built until they are planned and Done. M7 carries a check
that fails until then.

### The route check sees a missing prop

The parity metric averages each frame to a 32×18 grid. Its maximum cell catches a missing rock at the lookout, but
a thin scatter of small props could vanish on one target and stay under the thresholds. So the exit check gains a
third control beside the diagnostic scene and the retinted texture: the same store drawn with no entities must
fail at every stop that has entities in view. If it does not, the metric goes back to design (a finer grid) and is
not tuned to pass. Thresholds are set once for the exit store's root by the route decision's fixed rule. The exit
has its own baseline file; the bare valley's check and baseline are untouched.

### Culling on the assembled scene

`CULLING` closed on the scenes that existed and left this rerun here. M6 runs its equality command on the exit
scene: Frustum and Occlusion with Sun-caster culling against bucket-ordered Direct, at every submitted frame of
the flythrough with movers moving, on both recipes, and at checkpoints in Chrome. The default is set by
`CULLING`'s rule from paired rows in two runs. This module changes no culling code.

### Frame records, startup and hitches

- **Unattended (M7).** With capture off, the frame host's completed samples over the flythrough and one full day
  cycle, per row: CPU stages, GPU passes by name and the callback interval, with build and load. Every frame that
  missed its opportunity is listed with a cause. Startup to the first exit-scene frame is a labelled proxy.
- **On the built-in display (M8, attended setup).** The Mac's presented-time witness, as `PLATFORM-RUNTIME-MAC`
  qualifies it, over the flythrough on both recipes with capture off, and windowed startup. For Chrome, the page's
  frame-callback intervals in a visible window. No presented-time witness exists for Chrome, and the record says
  so.
- **Tripwire.** A row about a quarter slower than M5's baseline at the same stop is investigated (brief decision
  3). No figure is a gate.

### Defects found in another module's code

Assembly finds defects no single module could. The builder fixes one in place when the fix keeps that module's
design, tests and exact comparisons true, and notes it in both logs. A fix that changes an interface, a design or
an exact comparison is a proposal in this log, and the builder ends `BLOCKED needs_dependency`.

### The exit record

One page, `docs/evidence/SHOWCASE/V1-EXIT.md`: each clause of the V1 exit and of gate G7 with its evidence at the
claimed commit, the rows it ran on and its status as implementation, correctness, visuals and device, kept apart;
the visual digest; platform status, the iPhone simulator's taken from the modules that proved it; known defects;
and the physical-device runs owed. The maturity ledger and the README are brought to match it. The brief is the
owner's page and is not edited.

### Check that it can work

- **Cost.** `CULLING` M2 measured the mixed-mesh field, 100,513 instances with Sun shadows at 1920×1080: whole-GPU
  p50 48.7 ms with Frustum in the open and 108 ms Direct, contended
  ([record](../../evidence/CULLING/M2.md)). A tenth of the instances at that mesh mix predicts about 5 ms, close
  to the 8.33 ms interval before terrain, sky and reconstruction. So the sweep comes first, on stand-in meshes.
- **Size on screen.** At 1080p and a 46° field of view a pixel spans about 0.74 mrad. A 1 m prop is under two
  pixels beyond 700 m; a 20 m balloon is still five pixels at 5 km. Small props stay near, and movers can fill the
  far valley.
- **Bounded movers.** A loop that is a function of the tick cannot drift. A test steps one simulated hour.
- **Baselines.** With the exit content unselected nothing in the covered configurations changes, and the verifier
  proves it at every landing.

## The system as it runs

| State | Owner |
|---|---|
| The exit content, its layout and generator, prop sources, the mover rule, the exit configuration, the flythrough, the exit baseline, the launcher and the exit record | This module (`examples/v1-game`, `scripts/run_v1_showcase.py`, `docs/evidence/SHOWCASE/`) |
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

- No registered exact baseline moves. The valley's source root, the grid and field contents and the default
  output stay as they are.
- Every landing here changes covered paths, so every milestone carries the verifier and the builder runs it
  before `land`.
- This module owns two thresholded baselines: the bare valley's, unchanged, and the exit's, created in M5 and
  extended to one per row in M7. Each replacement is logged with its cause and hashes.

## Risks

1. **The frame does not fit.** Early sign: M5's sweep. Fallback: lighter meshes or fewer props near the lookout,
   by regenerating; then a level-of-detail proposal to `CULLING`.
2. **Distant props shimmer under reconstruction.** Early sign: M5's flythrough crops. Fallback: shorter placement
   distances per kind.
3. **Props are lit or hazed unlike the terrain.** Early sign: M5's stills at low Sun. It is an integration defect
   for `ATMOSPHERE` or the forward pass, handled as above.
4. **A row falls back silently.** Mitigation: the resolved report and a control with one selection removed.
5. **Shared files move under the builder.** `scripts/run_v1_showcase.py` and the game's sources are edited by
   other modules. Integrate sequentially and rerun the checks after every rebase.
6. **The built-in display is not free when M8 is ready.** Its two attended items are done in one sitting, batched
   with other modules' attended items.

## Proof and builder handoff

- **Targets.** `mac-metal` on both recipes, and `chrome-webgpu`. The iPhone simulator is not a row.
- **Capture.** T2 engine capture: `fixed` for stops and checkpoints, `realtime` for motion, on the Mac and in
  Chrome. Frame records are taken with capture off.
- **Reaching the targets.** GPU runs hold the `gpu` lock. Chrome and Instruments start outside the Codex sandbox
  by an approval for each run. A builder refused one leaves the item open and says so. A reviewer refused one has
  the review moved to Claude; a reviewer never judges the builder's captures in place of its own.
- **Attended setup.** Two items in M8 need the built-in display unlocked and awake. The agent runs and reviews;
  the owner does nothing else. Console state is recorded and is never a pass condition elsewhere.
- **Storage.** Scratch in `tmp/showcase-<milestone>/`; selected evidence in `docs/evidence/SHOWCASE/`.

## Open questions

- Whether Occlusion or Frustum is the exit scene's default. M6's paired rows settle it.
- Whether the frame holds the refresh interval under the full exit configuration. M7's records settle it; the
  remedy is content, by regenerating.

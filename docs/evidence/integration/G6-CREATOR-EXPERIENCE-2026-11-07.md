# G6 creator experience — 7 November 2026

Governing guide: [creator experience](../../design/ENGINE-EXPERIENCE.md). Build-plan item: G6 in the
[build plan](../../BUILD-PLAN.md). Work runs on an Apple M3 Max (macOS 26.6, Metal) and headless Chrome 156 WebGPU on the
same machine, which other agents were also using. Slices 1 and 2 make no timing claims; slice 3's reload timings are
contended observations, not budgets.

## Slice 1: typed demo controls (native panel and browser page)

### What was built

- **`thrum-controls`** (engine crate, no game schema, no I/O):
  - a typed model of sliders, toggles, text fields and readouts with stable dotted keys (`sky.solar_time`), labels,
    canonical engine values and a `Display` mapping (day fraction shown as `HH:MM`, radians as degrees);
  - revisions per control, so consumers apply only what changed since they last looked;
  - people's edits snap, clamp and are counted; code edits and parameter files refuse invalid values with a
    message that names the control, the cause and a next step, including the nearest key for a typo;
  - parameter files (`apply_file`): a JSON object of keys to display values, valid entries applied, each refusal
    explained;
  - two views of the same model: an AppKit HUD panel (`macos::MacPanel`, `objc2-app-kit`, one narrow target/action
    class) and a DOM builder (`web/thrum-controls.js`).
- **V1 game binding** (`examples/v1-game/src/controls.rs`, game-owned): solar time, camera yaw and pitch, the world
  pause and a Sun-elevation readout. Edits travel through the game's existing checked setters and pause path; what the
  host shows is written back without being mistaken for an edit, and a pending edit survives an observation.
- **Wiring:** the Mac window creates the panel and its Q/E/R/F/[ ] keys edit the same model; Chrome's sliders and pause
  toggle are generated from the model with their previous element ids, and the presets edit the model.

### Checks

| Check | Result |
|---|---|
| `thrum-controls` unit tests and doctest | 12 pass: declaration refusals, code-edit refusals, snapping and clamping, revisions, repeated person edits, parameter files with suggestions, JSON in display units, grouping. |
| `v1-game` binding tests | 4 pass: edits reported once, observations are not edits, pending edits survive observation, keys wrap and stop, the readout follows clock and edits. |
| `panel_check` example (AppKit, offscreen) | The real `controlChanged:` action path moves solar time to 18:29, yaw to its 180° stop and sets pause; the model applies all three with no repairs. |
| Chrome `controls`, `sky-reflections`, `valley`, `valley-recovery`, `device-recovery`, `device-recovery-retry` | Pass on the built page (see below). Paused edits, presets, keyboard stepping on a focused slider, device replacement and the cooked valley all drive the generated controls. |

### Visual review

The native panel, rendered offscreen through AppKit before and after the simulated edits:

![Panel at noon](../render/G6-MAC-PANEL-INITIAL-2026-11-07.png)
![Panel after edits](../render/G6-MAC-PANEL-EDITED-2026-11-07.png)

The Chrome page, generated from the same model, on the cooked valley at the twilight preset (`valley` case, paused):

![Chrome controls at twilight](../render/G6-CHROME-CONTROLS-TWILIGHT-2026-11-07.png)

Review notes. Labels, units and grouping match the native panel. The readout reads −5° at 18:29, which matches the
day cycle at 45° latitude (asin(cos 45° · cos 97.2°) ≈ −5.1°); the first build showed 45° here because the readout
only refreshed on clock observations, and that was fixed before these runs. Groups follow first declaration, so the readout sits with its sky group; this fixed a repeated "Sky"
header in the first render. Values use tabular digits and the clock reads `HH:MM`. The HUD material renders grey
offscreen; on screen it is translucent dark.

### Limits

- The Mac window itself could not present during this session because the Mac was locked and the display link was idle.
  The panel was verified offscreen only; **on-screen use of the Mac panel remains open** (a person dragging it in the
  running window, with the scene responding).
- Field overlays are readouts only in this slice; a GPU overlay slot will be proposed before touching the graph.

## Slice 2: parameter files (game layer)

### What was built

A parameter file is one JSON object of control keys and display values, such as
`examples/v1-game/params/sunset.params.json`. It is applied through the same checked edits as the panel and the page:
- valid entries apply;
- each refused entry is explained with its key, its cause, the nearest key and a next step;
- a file that cannot be read or parsed leaves every control unchanged.

Reload is explicit, because G5's watcher does not carry parameter files yet. Whether it should is recorded for the
owner in the G6 boundary notes.
- **Mac:** `THRUM_PARAMS=<path>` is applied once rendering starts and again when you press `L`. A one-line summary
  (`1 changed, 0 unchanged, 2 refused`) and each problem print to the terminal.
- **Chrome:** `?params=<url>` is fetched after the controls mount. The summary and problems appear under the
  controls, with a **Reload parameters** button. Without `?params` these stay hidden.

### Checks

| Check | Result |
|---|---|
| `thrum-controls` and `v1-game` unit tests | The example file applies cleanly. A typo names the nearest key. An out-of-range pitch reports `-34–34` rather than f32 noise. |
| Mac window, `THRUM_PARAMS` with a typo and an out-of-range pitch | The window presented. The terminal showed `1 changed, 0 unchanged, 2 refused` with both explanations, and the run completed normally. |
| Mac window, paused apply | Applying `{"world.paused": true, "sky.solar_time": 17.9}` committed 2 world ticks over 240 presented frames. The example file without a pause committed 739 ticks over 240 frames. A paused apply holds the world clock. |
| Chrome `params` case (new) | The page loads with `?assets=…&params=case.params.json`, so the cooked valley is drawn as well. The served file moved solar time to 17:54 and paused the world, and the page explained the `camera.yw` typo. After the served bytes were rewritten, **Reload parameters** set 06:30, and the game clock matched within 1e-6. `#status` never showed a parameter problem. |
| Chrome `controls`, `sky-reflections`, `valley`, `valley-recovery`, `device-recovery`, `device-recovery-retry` | Pass on the same build. |

![Chrome parameter diagnostic](../render/G6-CHROME-PARAMS-DIAGNOSTIC-2026-11-07.png)

**Review notes.** The page shows the summary and the explanation as two lines under the reload button. The
Sun-elevation readout of 1° at 17:54 matches the day cycle at 45° latitude.

### Limits

- Parameter edits reload on request, not on save.

## Slice 3: cooked material hook (shader reload)

### What was built

A game-authored WGSL hook, `cooked_surface`, changes the cooked meshes' surface factors and reloads explicitly into
the running scene. The [G6 decision](../../design/decisions/G6-CREATOR-EXPERIENCE.md#shader-edits-the-cooked-material-hook-built)
records the API and the lifecycle.
- **Mac scenes:** the sky kit reads `THRUM_MATERIAL` when the window opens and again on `L`.
- **Chrome:** v1-game's page takes `?material=<url>` and has a **Reload material** button.

The page fix below also makes slice 2's claim true: the parameter row is now hidden without `?params`. Before, the
`.params` display rule overrode the `hidden` attribute.

### Checks

| Check | Result |
|---|---|
| No-hook pixels, v1-game `THRUM_CAPTURE_SCENE` | Plain, diagnostic-store and valley captures (13 files each) are byte-identical to captures from `ed64db0`. |
| No-hook and identity-hook pixels, starter `THRUM_CAPTURE_PNG` | The default capture and an identity-hook capture are byte-identical to the `ed64db0` starter. |
| Mac GPU tests, `examples/v1-game/tests/cooked_material_gpu.rs` (4) | **Valid reload:** the banded hook visibly changes only the cooked terrain. Motion history restarts on the promoting frame and continues on the next. **Refused hook:** a misspelt identifier is rejected at `hooks/misspelt.wgsl:3:21`, and the pixels do not change. **Superseded:** of three back-to-back requests, the first two are superseded and the last is drawn. **Empty hook:** refused with line 0. **Identity hook:** draws the built-in material with zero differing bytes. **Device replaced during a reload:** the outstanding request is retired, and the replacement device draws the accepted hook's pixels again. A new request then draws. **Requests before the first frame:** the newer one is prepared once the assets build their pipelines, and it draws. |
| Mac starter capture with a typo | Exits 1 with `hooks/typo.wgsl:17:21: no definition in scope for identifier: \`roughnes\``. |
| Chrome `material` case (new) | **Valid reload:** the page loads the valley with an identity hook. The **Reload material** button then draws the shipped moss hook, made unique per run, and changes 56% of the canvas pixels. **Refused hook:** the misspelt hook is refused as `case.material.wgsl:3:21: unresolved value 'roughnes'`, and the screenshot is byte-identical to the moss one. **Device destroyed with a request outstanding:** a request and `GPUDevice.destroy()` run in one JavaScript task. The request is retired, and the restored device's screenshot is byte-identical to the moss one. **Reload on the replacement device:** it draws. Passed 5 runs. A sixth failed on a check race (the restore button had not yet appeared), since fixed. |
| Chrome `controls`, `params`, `sky-reflections`, `valley`, `valley-recovery`, `device-recovery`, `device-recovery-retry` | Pass on the same build. |

![Chrome valley, built-in material](../render/G6-CHROME-MATERIAL-BUILT-IN-2026-11-07.png)
![Chrome valley, moss hook](../render/G6-CHROME-MATERIAL-MOSS-2026-11-07.png)
![Starter rock, moss hook](../render/G6-STARTER-MATERIAL-MOSS-2026-11-07.png)

**Review notes:**
- **Chrome moss hook.** The dunes turn green. The chrome sphere and the rock gain green tops where they face the sky.
  The sky, the shadows and the sphere's reflection band are unchanged.
- **Starter moss hook.** The rock's upper faces are green and its sides keep the sandstone texture. It sits on the
  kit's lit ground and casts its shadow there. This capture was retaken after the kit gained the ground.

### Reload timing (contended, observations only)

The figures come from a shared machine, and nothing here is a budget.
- **Mac.** `cargo bench -p v1-game --bench material_reload`: 1280×720, each frame waited for, six distinct hooks per
  run, three runs.
  - The compile thread took 0.66–2.2 s per hook, usually 0.7–1 s.
  - The request call took 0.02–0.35 ms.
  - The hook was drawn within 2 ms of becoming eligible.
  - Frames kept running during each reload, 37–127 per reload.
  - Quiet frames before the reloads had worst-case latencies of 22–39 ms across runs.
  - The worst frame during a reload was usually within that spread, but 3 of the 18 reloads had a 75–118 ms
    frame. This is not explained; it may be contention or interference from the compile.
- **Chrome.** The page's call took 0.3 ms. With a hook that had not been compiled on the machine before:
  - the moss hook became eligible after 577–880 ms;
  - one animation-frame interval during the compile was 300–750 ms (5 runs);
  - frames before and after stayed at a 16.8 ms worst.
  - **This is a stall.** Chrome creates the pipeline in its GPU process, and an asynchronous wrapper cannot move that
    work. Moving it would need `createRenderPipelineAsync`, which wgpu 30 does not expose.
- **Cached compiles.** A hook already compiled on the machine came back in 3–60 ms without a stall, on both
  platforms.

### Limits

- **Reload trigger.** Reload is explicit, not on save.
- **Compile cost.** Each hook recompiles the whole scene fragment shader.
- **Coupling.** The seam depends on one line of `thrum-render`'s scene shader. It is guarded by a count check that
  refuses with a diagnostic.
- **Temporal history.** Only the meshes' motion history restarts on promotion.
- **Inputs.** The hook gets no time input.
- **Starter size.** The kit wires the reload for any scene with cooked meshes, so it adds no game lines.

# G6 creator experience — 7 November 2026

Governing guide: [creator experience](../../design/ENGINE-EXPERIENCE.md). Build-plan item: G6 in the
[build plan](../../BUILD-PLAN.md). Work runs on an Apple M3 Max (macOS 26.6, Metal) and headless Chrome 156 WebGPU on the
same machine, which other agents were also using. No timing claims are made here.

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

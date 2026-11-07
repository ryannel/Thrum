# G6 decision: creator experience and demo controls

7 November 2026. This is the governing design for build-plan item G6.

**Status.** The typed controls and parameter files are selected and implemented, and they run in the V1 game on Mac and
Chrome. The app runner, the starter, shader reload and GPU field overlays are designed below but not built.

**Disclosure.** The controls and parameter slices were built before this record was written. The design was agreed
in the G6 boundary notes with the root agent first, and an independent review of the landed code prompted this record.
It sits under the [creator guide](../ENGINE-EXPERIENCE.md) and the [build plan](../../BUILD-PLAN.md), with ownership as in
[TOOLS-DELIVERY](../modules/TOOLS-DELIVERY.md) and [PLATFORM-RUNTIME](../modules/PLATFORM-RUNTIME.md). Evidence:
[G6 report](../../evidence/integration/G6-CREATOR-EXPERIENCE-2026-11-07.md).

## Commission

A creator tunes a running scene through named, typed controls that look the same on Mac and in Chrome. They start a
new scene with one command and about 100 lines of game code. Edits to assets, shaders and parameters reach the
running scene, and every mistake is explained by name, cause and next step.

## Typed controls: `thrum-controls`

There is one model per game: `Controls`, a list of sliders, toggles, text fields and readouts.
- **Keys and labels.** Each control has a stable dotted key (`sky.solar_time`) and a label.
- **Values.** Each control has a canonical engine value and a `Display` mapping. A clock shows a day fraction as
  `HH:MM`, degrees show radians, and a scaled display shows `engine × scale`.
- **The model is the only authority** for those values. The views and the game read it and never keep their own copy.

**Edits come from three sources and follow two rules:**
- **People** (a panel, the page or keys) snap to the step, clamp into range and are counted. A person's edit always
  advances the revision, even when it repeats the shown value, because the game's clock may have moved on.
- **Code** (`set`) and **parameter files** are refused when invalid. Files get a half-step tolerance at the range ends.
  The error names the control, the cause and the next step, and a mistyped key gets the nearest declared key by edit
  distance.

**Change tracking.** Each control has a revision, and `changed_since` lets each consumer apply only what moved. A
game binding (for example v1-game's `GameControls`) records the revision it last settled per control. That separates
an edit still pending from an observation of the host's state, and an observation never counts as an edit.

**Declarations are refused when they could admit invalid values:** a bad key or label, a duplicate key, a non-finite
or unordered range, a default outside the range, a negative step, or a display scale that is not finite and
positive.

**Two views of the same model:**
- **Mac:** `macos::MacPanel`, an AppKit HUD panel built with `objc2-app-kit`. It uses one narrow target/action class
  under `#[expect(unsafe_code)]`, and `render_png` renders it offscreen for agent review.
- **Browser:** `web/thrum-controls.js`, a DOM builder. It is driven by `describe_json` and `changes_json`, and groups
  are shown in first-declared order.

The crate has no game types, no I/O and no dependencies beyond `serde_json` and the macOS view.

## Parameter files

A parameter file is one JSON object of control keys and display values. `Controls::apply_file` applies the valid
entries and returns a `FileReport`: the counts, plus each refusal sorted by key. A file that is not a JSON object
changes nothing.
- **Mac:** `THRUM_PARAMS=<path>` is applied once rendering starts and again on `L`.
- **Chrome:** `?params=<url>`, which works alongside `?assets`, is applied on load and by a reload button.

Both apply through the same edit path as the panel, so a paused apply holds the world clock.

**Reload is explicit, because files do not ride the G5 watcher yet.** Making parameter files a fourth G5 product kind
is designed in the boundary notes and is an owner question while G5 extensions are parked.

## App runner and starter (designed; the move awaits root review)

- **The runner.** The macOS window stack moves out of v1-game into `crates/thrum-app`, unchanged in behaviour. It
  owns:
  - the window, device recipe and display link;
  - the render actor, pause, recovery and surface generations;
  - the controls panel, parameter loading and the bounded report.
- **What a game supplies.** It prepares a `thrum_render::runtime::FrameHost` (the scene-source and contribution
  hook), its controls, an edits type the render owner applies, and its keys.
- **Readback gating.** The app passes a render reply's readback to the game only when the reply is at least as new as
  the last request that carried edits. Otherwise an in-flight frame would snap the panel back.
- **The starter** uses a small kit with:
  - a still world;
  - a sky scene with a look-at camera;
  - a cooked mesh through `thrum-asset-gpu`;
  - a Sun slider;
  - `./dev starter`, which cooks with G5's cooker, runs G5's watcher for asset hot reload, and opens the window.
- **Native only.** The starter runs on the Mac. Its Chrome form is an owner question; the recommendation is to defer
  it to G7's shared demo page.

## Shader edits (proposed; needs a seam)

Every shader is compiled in. The smallest seam is an optional WGSL material snippet in the cooked mesh pipeline:
- the reload is explicit;
- the snippet is compiled under a validation scope before the swap;
- on failure the old pipeline keeps drawing, and the error names the file and line.

Shader reload is one of G6's own done criteria, and the parked G5 work covers only garbage collection, leases and
layered EAC. So this seam needs root's boundary review, not an owner decision.

## Field overlays

**Phase 1 is readouts** of engine fields in the panels, for example the Sun's elevation computed from the shown
clock.

**Phase 2 is a GPU false-colour field,** such as EV or the shadow cascade index. Its constraints:
- it needs a frame-graph slot after tone mapping, proposed to root first;
- it composites in encoded display space, undithered or with the same ±1-LSB dither as the display encode.

## Performance

Controls are only a few dozen values.
- **Per frame:** a view refresh costs one `changes_json` scan, which returns `None` when nothing changed, and Chrome
  calls it every frame.
- **Native:** the panel syncs only on edits and render replies.
- **Never in the frame path:** parameter files and the panel do not allocate there.
- **Shader rebuilds** happen at frame boundaries.

# G6 decision: creator experience and demo controls

7 November 2026. This is the governing design for build-plan item G6.

**Status.** The typed controls and parameter files are selected and implemented, and they run in the V1 game on Mac and
Chrome. The macOS app runner is built and runs the V1 game. The shader seam (a game-authored WGSL material hook in
the cooked mesh pipeline, reloaded explicitly) is built and runs on Mac and in Chrome. The starter (`./dev starter`,
`./dev scene <folder>` and the sky kit) is built and runs on the Mac. The first GPU field overlay, exposure zones,
is built in the shared display pass. The clipping field is held, and the shadow cascade index is not designed.

**Disclosure.** The controls and parameter slices were built before this record was written. The design was agreed
in the G6 boundary notes with the root agent first, and an independent review of the landed code prompted this record.
It sits under the [creator guide](../ENGINE-EXPERIENCE.md) and the [build plan](../../plans/BRIEF.md), with ownership as in
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
- **Code** (`set`) and **parameter files** are refused when invalid. Files get a half-step tolerance at the range ends,
  and a file value between steps snaps to the nearest step, as the slider would.
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
  are shown in first-declared order. Every element id a control emits (its own, and a slider's `<id>-value`) is
  reserved against the other controls and the page; a clash is refused through `onError` and that control is not
  drawn.

The crate has no game types, no I/O and no dependencies beyond `serde_json` and the macOS view.

## Parameter files

A parameter file is one JSON object of control keys and display values. `Controls::apply_file` applies the valid
entries and returns a `FileReport`: the counts, plus each refusal sorted by key. A file that is not a JSON object
changes nothing.
- **Mac:** `THRUM_PARAMS=<path>` is applied once rendering starts, again on `L`, and when a save settles (see
  [saved files reload](#saved-files-reload-built)).
- **Chrome:** `?params=<url>`, which works alongside `?assets`, is applied on load and by a reload button.

Both apply through the same edit path as the panel, so a paused apply holds the world clock.

**Parameter files stay out of the cooked asset schema.** On the Mac, a save reaches the scene through the app's own
file watch rather than the G5 watcher. Making parameter files a fourth G5 product kind is designed in the boundary
notes and is an owner question while G5 extensions are parked. Chrome still reloads by button.

## App runner and starter (built)

- **The runner.** The macOS window stack moves out of v1-game into `crates/thrum-app`, unchanged in behaviour. It
  owns:
  - the window, device recipe and display link;
  - the render actor, pause, recovery and surface generations;
  - the controls panel, parameter loading and the bounded report.
- **What a game supplies.** It prepares a `thrum_render::runtime::FrameHost` (the scene-source and contribution
  hook), its controls, an edits type the render owner applies, and its keys.
- **Edit flow.** Edits ride the next frame request. The app passes a render reply's readback to the game only when the
  reply is at least as new as the last request that carried edits; otherwise an in-flight frame would snap the panel
  back. A frame cancelled by device loss takes its edits with it, so recovery queues the game's whole desired state
  again. `app/edit_flow.rs` holds this as one pure state machine, tested across a GPU replacement.
- **The starter** uses a small kit with:
  - a still world;
  - a sky scene with a look-at camera;
  - a cooked mesh through `thrum-asset-gpu`;
  - a Sun slider;
  - `./dev starter`, which cooks with G5's cooker, runs G5's watcher for asset hot reload, and opens the window.
- **Native only.** The starter runs on the Mac. Its Chrome form is an owner question; the recommendation is to defer
  it to G7's shared demo page.

### Saved files reload (built)

Saving the parameter file or a game's own file reloads it in the running window, as `L` does, without a key press.
Root commissioned this on 8 Nov; root and G6 agreed the seam.

- **Watch.** `crates/thrum-app/src/file_watch.rs` is a polling thread with no new dependency, because `notify` is
  authoring-only by policy and `thrum-app` links into every scene.
  - Every 100 ms it reads each file's metadata (length, modification time, inode). When that changes, it reads the
    bytes.
  - It delivers once the bytes hold for a tick and differ from the last bytes delivered. A burst is one delivery;
    a touch or an identical rewrite is none.
  - A file missing for 500 ms is delivered so the reload reports it. Rename-saves, symlinks and files created later
    need no special case.
- **What is watched.** The parameter file and `Game::reload_files()`, a read-only addition to the trait that defaults
  to none. The sky kit names its `THRUM_MATERIAL` hook.
- **Delivery.** The watch sets per-kind bits (parameters, game files) and wakes the event loop with a payload-free
  `UserEvent::FilesSaved`. The event loop takes the bits and runs `file_watch::deliver`, the one body `L` also runs
  with both bits:
  - `load_params` only for the parameter bit (its report is tagged `(saved)`);
  - `Game::reload` only for the game bit, so a parameter-only save never compiles the material;
  - then a redraw.
  - All reads are on the event loop or the watch thread; none run in the display-link or render callbacks.
- **Lifecycle.** `App` starts the watch after the render worker and before the first load, so the baseline precedes
  the first load and no later save is lost. `Drop` stops it before `render.shutdown()`. A delivery after the
  renderer has stopped is dropped. If the thread cannot start, the app says so and `L` still works.
- **Witnesses.** Unit tests in `./dev check` cover:
  - the watch: in-place save, rename-save, burst, touch, delete-then-create, a lasting absence, a file created late,
    both kinds coalescing and stop without a tick;
  - the delivery body: per-kind routing, `L` doing both, and a save during shutdown being dropped.

  `scripts/check_starter_live_reload.py` runs the real starter in live capture mode, `THRUM_CAPTURE_LIVE` with
  `THRUM_CAPTURE_PNG` (`sky/live_capture.rs`). Live capture uses the same watch and delivery body offscreen and writes
  a PNG and report line per save. A newly drawn content generation, or an asset edit the loader refuses while the
  previous generation keeps drawing (`content: kept N (<reason>)`, also logged by the window), writes a `content`
  line; the script renames the starter's mesh away from its placement and back. It does not qualify the window's event path on glass; pause ordering through
  `controls_edited` is unchanged and shared with `L`, but has no new test, because `App` needs a main-thread event
  loop.

## Shader edits: the cooked material hook (built)

Every engine shader stays compiled in. Root approved one seam in the G6 boundary notes: an optional game-authored
WGSL material hook in the cooked mesh pipeline. `thrum-asset-gpu` owns it.

**The hook changes surface factors only.** A game writes one function:
`fn cooked_surface(surface: CookedSurface, at: CookedSurfacePoint) -> CookedSurface`.
- **Inputs:** `surface` holds the base colour, roughness, metalness and emission that the placement set. `at` holds
  the fragment's `uv`, `world` position and `normal`.
- **Glue:** clamps the result to valid factors and keeps the flags, texture layers and alpha cutoff. A factor the
  hook made infinite or NaN falls back to the placement's value, and emission is capped at the largest half-float,
  the scene target's format.
- **Where it applies:** only in the cooked mesh pipeline's copy of the scene shader, where the one read of the
  material record (`materials[input.material]`) goes through the glue. The scene's fragment shading then reads the
  returned factors, so lighting keeps one authority.
- **What does not change:** the material ABI, the bindings, the cull and shadow pipelines, the frame graph and asset
  publication.
- **Without a hook** the pipeline compiles exactly the source it always has. v1-game's `THRUM_CAPTURE_SCENE`
  captures before and after the change are byte-identical.

**API.** `CookedAssets::material()` returns a `MaterialReload` handle.
- `request(MaterialHook { name, source })` returns a serial.
- `take_outcomes()` reports each serial's outcome: `Promoted` with its timing, `Rejected` with diagnostics,
  `Superseded`, or `Retired`. Each request gets exactly one. `Dropped` answers no request (its `serial()` is `None`):
  the accepted hook could not be restored on a replaced device or surface, so the built-in material draws.
- `accepted()` names the accepted hook. `restoring()` says whether that hook is being validated again for a
  replaced device or surface, while the built-in material draws.
- `ready()` names the serial of a request that validated and is held for drawing but has no outcome yet: no submitted
  frame has recorded draws of a resident cooked mesh with it, because none is resident or the frame was skipped.
- `revision()` counts changes to what those report, so an idle caller can skip building a report.
- A request made before the assets first draw is prepared when they first build their pipelines.

**Lifecycle:**
- **Preparing.** The replacement is prepared outside frame recording.
  - **Natively,** one compile thread runs at a time per handle. A request made during a compile waits, and a newer
    one supersedes it before it compiles.
  - **In the browser,** which has no threads, the caller's task creates the module and pipeline. It uses WebGPU's
    synchronous methods, because pinned wgpu 30.0.1 does not expose `createRenderPipelineAsync`.
  - **Dev-only.** In the browser the reload is compiled only into the development page, behind v1-game's
    `dev-material` Cargo feature (`web/build.sh --dev`), which compiles into its own Cargo target folder so a
    concurrent default build cannot swap Wasm with it. The default page exports no material methods, so a
    `?material=` URL there starts no material work.
  - **A compile that cannot start** (the thread fails to spawn) is rejected with that error, and so is every job
    queued behind it. A queued restore that fails this way is reported `Dropped`, and the built-in material
    draws.
- **Eligibility.** A candidate becomes eligible only after its compilation messages and its validation error scope
  have both resolved.
- **Installing.** The next frame boundary draws an eligible candidate. The first submission whose frame recorded
  draws of a resident cooked mesh with it promotes it, so `Promoted` and its `drawn` time never precede resident
  content. That is an installation, not a visibility claim: the GPU cull may have drawn none of the mesh, and
  visible meshlets are reported separately and asynchronously (`CookedStats::visible_meshlets`). Until then it
  is held and `ready()`: a submitted frame with no cooked mesh, or an encoded frame that is skipped (recording is
  tentative), leaves it so, and a newer request or a rebuilt surface still replaces it.
- **On promotion,** motion history restarts once, and residency and bindings are kept.
- **On failure,** the previous pipeline keeps drawing. Diagnostics are translated to the hook's own file, line and
  character column; a message outside the hook has line 0.
- **Identity.** Each candidate carries its serial, device generation and surface epoch:
  - a newer request supersedes an older one;
  - a candidate prepared for a replaced device or surface is retired;
  - a rebuilt device or surface first builds the built-in pipelines and draws them. The last accepted hook is then
    queued as a restore job on the same single compile owner, behind any compile still running for the old device.
    That compile is retired when it finishes. The restore draws only once it has compiled and validated for the new
    target, and it is promoted by the submission that carries it. A hook that no longer validates is dropped with
    its diagnostics.

**Front ends.** Both reload explicitly:
- **Mac scenes:** the sky kit's `SkyGame::with_meshes` reads `THRUM_MATERIAL=<file>`. It reloads the file when
  the window opens and on `L`, after the parameter file. `./dev scene <folder>` points it at the folder's
  `material.wgsl` when there is one, and the starter ships one.
- **Chrome:** v1-game's development page has `?material=<url>` (with `?assets`) and a **Reload material** button.
  The shipped example is `examples/v1-game/materials/moss.material.wgsl`.

**Measured cost.** This is a contended shared machine, so the figures are observations, not budgets.
- **Compile cost.** A distinct hook compiles the whole scene fragment shader again: about 0.7–1 s on Mac (up to
  2.2 s under contention). The caller's call measured under 0.4 ms on both platforms. In the browser, though, the
  work it starts is synchronous WebGPU (see the stall below).
- **Mac:** frames keep running during the compile. In 18 reloads the worst frame was usually within the quiet
  spread, but 3 frames reached 75–118 ms; this is not yet explained.
- **Chrome stalls.** The page's animation frames stop for 300–750 ms during a cold compile. wgpu 30.0.1 calls the
  synchronous `createRenderPipeline`, and an async wrapper cannot move that work. `createRenderPipelineAsync`
  might, but wgpu 30 does not expose it. This stays open under G6 and is acceptable only because the control exists only on the development page.
- **Compiler cache.** A hook compiled earlier on the machine comes back in tens of milliseconds without a stall,
  because the system's compiler cache serves it.

**Open:**
- **Coupling to the scene shader.** The anchor is a line of `thrum-render`'s scene shader. A count check refuses a
  scene with zero or several reads, with a diagnostic. A named material-fetch function owned by `thrum-render`
  would remove the coupling.
- **Temporal history.** History restarts only for the contribution's meshes. Contributions cannot reset the
  scene's own temporal history.
- **Browser cold-compile stall.** This is open; see the measured cost above.
- **Inputs.** The hook gets no time input.

## Field overlays

**Phase 1 is readouts** of engine fields in the panels, for example the Sun's elevation computed from the shown
clock.

**Phase 2 is GPU false-colour fields.** The first is **exposure zones**, agreed with root in the G6 boundary notes.
- **Where it is drawn.** The existing SDR display pass draws it, so it needs no new pass, binding or graph resource.
  It sits beside the tone map in the per-view presentation policy:
  - `SyntheticView::with_display_overlay(DisplayOverlay)` selects it;
  - it is packed in bits 8–15 of the policy word whose bits 0–7 hold the `SdrToneMap`.
  - Because games build the view every frame from their own state, a replacement device draws the same overlay.
- **What it shows.** The pass reads the exposed scene colour at the pre-tone seam the tone maps already use
  (stored ÷ (storage scale × inverse camera gain)) and takes its Rec. 709 luminance. It writes one of 13 one-stop
  bands as exact undithered display codes:
  - −6 or darker, through to +6 or brighter;
  - grey at zero, which is the exposed luminance of an 18% Lambertian card lit at the metered illuminance with zero
    compensation, 0.18 ÷ (π × 1.2 × 0.4).
  The overlay is independent of the selected tone map and output transform.
- **Off.** With the overlay off, the policy word and every pixel are unchanged for every tone map.
- **How games reach it.** v1-game has a `display.exposure_zones` toggle on the Mac panel, in Chrome and in parameter
  files. The device-recovery replay carries it. The sky kit has `Sky::exposure_zones`, which the starter binds to the
  same toggle.
- **Held.** A post-tone clipping field cannot show where input or storage clipped, because the SDR operators bound
  their output to display white. It needs a signal from the stage that clamps.
- **Not designed.** The shadow cascade index needs forward-pass output.

## Performance

Controls are only a few dozen values.
- **Per frame:** a view refresh costs one `changes_json` scan, which returns `None` when nothing changed, and Chrome
  calls it every frame.
- **Native:** the panel syncs only on edits and render replies.
- **Never in the frame path:** parameter files and the panel do not allocate there.
- **Shader reloads** compile outside frame recording and install at frame boundaries; see the measured cost above.

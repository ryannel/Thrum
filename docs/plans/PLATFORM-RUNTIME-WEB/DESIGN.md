# Web shell: the engine carries the browser platform runtime (G3b)

8 November 2026. Moved from the web shell plan on 11 November 2026, when the plans took the orchestrate 1.7.0 shape. The web shell was designed, built and reviewed before the loop, so this module has no done-when file; [the record](../../evidence/PLATFORM-RUNTIME-WEB/M1-record.md) holds what was accepted and [the log](LOG.md) the reviews. The sections below are unchanged.

Family design: [PLATFORM-RUNTIME.md](../../design/modules/PLATFORM-RUNTIME.md#g3b-world-and-presentation-lifecycle--selected-partial-implementation),
contracts [C02](../../design/CONTRACTS.md#c02-time-and-committed-state--f1-coordinator-w1-world),
[C06](../../design/CONTRACTS.md#c06-gpu-execution-and-lifetime--v1) and
[C11](../../design/CONTRACTS.md#c11-input-audio-and-ui-lifecycle--p1a1u1-with-f1). Sibling: the
[iOS shell](../PLATFORM-RUNTIME-IOS/PLAN.md).
Commission: Thrum Core's routing of daily-review finding D-M3, 8 November (`tmp/g6-boundary/NOTES.md`, "Root routing
for the 8 November daily review"). Design and plan go to Thrum Core before any code.

## Problem

The macOS shell is engine code in `thrum-app`: a game supplies `Game` hooks and nothing else. The browser shell is not.
Of the 1,232 lines in `examples/v1-game/src/browser.rs` and `browser/recovery.rs`, most is platform runtime, not game:
- WebGPU instance, canvas surface and device request;
- the device-loss signal (`GraphicsFault`) and detached reconstruction (`Recovery`);
- surface configuration, resize, late acquisition and present;
- the clock policy: browser timestamps to the world clock, timestamp clamping, visibility and user pause composed, and
  overload recovery;
- frame samples and the 600-sample report.

A second game would have to copy all of it, and two copies of platform policy would drift (review D-M3). This plan
moves it into the engine. The page and what a player sees do not change. The checker changes once, before the move,
to witness the loss policy.

## Boundary

| Stays in the game (`examples/v1-game`) | Moves to the engine (`thrum_app::web`, wasm32 only) |
|---|---|
| The `#[wasm_bindgen]` facade: `create_browser_game`, `create_field_game`, `BrowserGame` and every method the page and checker call, with the same names, arguments and report text; the console panic hook, which is process-wide and belongs to the embedding application | `wgpu::Instance` for `BROWSER_WEBGPU`, the canvas surface, adapter and device request, the capability record |
| `index.html`, its RAF loop, DOM events and input mapping | The device-loss signal and the detached, generation-checked reconstruction with its weak mailbox and explicit retry |
| `GameControls`, parameter files, `apply_parameters`, `edit_control`, `controls_json`, and `demo.observe_time` before each `frame` call, as today | Clock policy: timestamp origin and validation, the backwards-timestamp clamp, visibility ∘ user pause, and when to pause and resume for overload |
| The world's lifecycle: `pause`, `resume_after_drain` and the drain state (`resume_pending`) in `WorldDriver`, exposed to the shell through `WebWorld` | The frame step: reconfigure on resize or `Suboptimal`, advance, encode before acquiring, late acquire, submit, present, poll completion |
| `GameSetup` (valley, field, culling, stars), the authored scene, and a constructor that returns the `Runtime` | Typed counters, status and bounded frame samples (see the report fields below) |
| Cooked valley fetch, material hooks (`dev-material`) and the capture oracle's image checks | The loss gates: capture start and finish, scene-contribution attach, and the `graphics_lost()` query that material reload checks |
| `report()`: every word of the report text, built from the shell's typed fields | |

The page's JS stays the only owner of `requestAnimationFrame`, `visibilitychange` and DOM input. The engine shell
receives timestamps and visibility as calls. It installs no listener and does not touch the DOM beyond the canvas
it is given. Offscreen capture consolidation (`bin/window/capture.rs`, `motion.rs` and `thrum-app/src/sky/capture.rs`)
is separate and not in this plan.

## Shell design

**Shape.** `thrum_app::web::WebShell<W, V>` owns a `thrum_render::runtime::FrameHost<W, V>`, the same runtime the macOS
shell drives, plus the surface, the device, the loss signal and the recovery state. The game builds the runtime once,
in a closure the shell calls with its first device:

```rust
// Sketch, wasm32 only. Not a committed API.
pub trait WebWorld: FrameWorld {
    /// Stop admitting wall time, keeping accepted ticks.
    fn pause(&mut self) -> Result<(), String>;
    /// Resume from `now`, or once the accepted ticks have drained on later drives.
    fn resume_after_drain(&mut self, now: Duration) -> Result<(), String>;
}

pub async fn create<W, V, F>(canvas: HtmlCanvasElement, generation_base: u64, prepare: F)
    -> Result<WebShell<W, V>, String>
where F: AsyncFnOnce(WebGpu<'_>) -> Result<FrameHost<W, V>, String>;

impl<W: WebWorld, V: SceneSource<W::Page>> WebShell<W, V> {
    pub fn frame(&mut self, timestamp_ms: f64) -> Result<(), String>;
    pub fn set_visible(&mut self, visible: bool, timestamp_ms: f64) -> Result<(), String>;
    pub fn set_paused(&mut self, paused: bool, timestamp_ms: f64) -> Result<(), String>;
    pub fn paused(&self) -> bool;
    pub fn graphics_fault(&self) -> Option<String>;
    pub fn graphics_lost(&self) -> bool;
    pub fn request_graphics_recovery(&mut self) -> Result<(), String>;
    pub fn recovering(&self) -> bool;
    pub fn recovery_error(&self) -> Option<&str>;
    // Edits: CPU state only, allowed while lost.
    pub fn host(&self) -> &FrameHost<W, V>;
    pub fn world_mut(&mut self) -> &mut W;
    pub fn source_mut(&mut self) -> &mut V;   // resets transform history, as FrameHost::source_mut does
    // GPU-touching: gated by the loss signal.
    pub fn set_scene_contribution(&mut self, c: Box<dyn SceneContribution>) -> Result<(), String>;
    pub fn begin_capture(&mut self) -> Result<PendingCapture, String>;
    pub fn try_finish_capture(&mut self, c: &mut PendingCapture) -> Result<Option<CapturedPixels>, String>;
    pub fn status(&self) -> WebStatus;
    pub fn samples(&self) -> &[FrameSample];
}
```

- **Lifecycle route: a shell-side trait, `WebWorld`.** `FrameHost` and `FrameWorld` have no pause or resume. Both live
  on the game's `WorldPort` and `WorldDriver` (`host/world.rs`). `WebWorld` is defined in `thrum_app::web`, and the
  game implements it for its inline `WorldPort` by forwarding to the existing `pause` and `resume_after_drain`. The
  drain state (`resume_pending`, and the deferred resume completed inside `drive` once the queue is empty) stays in
  `WorldDriver`, the one owner of world-clock state under C02. The shell owns only the policy around it:
  - visibility ∘ user pause decides when to call `pause` or `resume_after_drain`;
  - an overload report makes it call `pause` then `resume_after_drain`;
  - it keeps the counters.

  `FrameWorld` lifecycle methods were rejected: they would change a `thrum-render` API and bind the macOS and iOS
  worlds, which drive the lifecycle through `WorldControls` requests instead. Callbacks were rejected: they are the
  same contract, but untyped. This route changes no `thrum-render` API and touches neither native world.
- **No `host_mut`.** The game reaches the runtime only through `host()` and the CPU mutators above. Each game edit
  reads and compares through `host()` first, then takes one mutable borrow only when the value changes, as
  `host.rs` does today. `source_mut()` resets transform history, so an unchanged edit must not reach it. Every game edit
  today uses exactly `world()`, `world_mut()`, `source()`, `source_mut()` and `invalidate_history()` (`host.rs`,
  `host/scene_source.rs`), and all five are already public on the runtime. The runtime's GPU entry points (`encode_scene`,
  `advance`, `drive`, `resize_surface`, `poll_completed`, `device_prepare_inputs`, `publish_device`, `publish_visual`,
  `begin_capture`, `try_finish_capture`, `capture_pixels`, `set_scene_contribution`) are then reachable only through the
  shell. The type system enforces the loss policy.
- **No callback trait for the game.** The browser shell has no render thread and no hand-off, so it needs no `Send`
  bounds, `Edits` or `Readback`. The game calls the shell. It is called back only through the preparation closure and
  `WebWorld`. The macOS `Game` trait is unchanged.
- **The frame step's calls exist on the runtime:** `advance`, `advance_without_rendering`, `encode_scene`,
  `can_acquire_surface`, `resize_surface`, `poll_completed`, `pending_frames`, `device_prepare_inputs`, `publish_device`.
- **One threading model.** The world stays `WorldPort::Inline` on the page thread. The browser baseline needs no
  threads (PLATFORM-RUNTIME, browser constraints).
- **Naming.** The device bundle handed to `prepare` is `WebGpu`, so it does not shadow the macOS `thrum_app::Gpu`. The
  game passes its `DEVICE_GENERATION` base to `create`, and the shell numbers generations from it.

**Loss policy.** After loss, `frame` does only `advance_without_rendering`, as now: no poll, completion collection,
resize or encoding. PLATFORM-RUNTIME (G3b) requires observed loss to proceed without GPU calls. Each operation is
classified by what it touches:

| Operation while the device is lost | Policy | Today |
|---|---|---|
| Authority and view edits: solar time, camera yaw and pitch, player input, parameter files and overlay, user pause and visibility | Allowed. They change CPU state only. The replacement device draws that retained state | Allowed: `browser.rs` 390–416 and 604–640; the lost branch of the frame step at 511–520 |
| Capture start and finish | Refused with `graphics device is lost`. A capture outstanding at loss is dropped and reported, as now | The same: `begin_capture` 678–683 and `try_finish_capture` |
| Material reload (`dev-material`) | Refused by the facade with `graphics device is lost`, checked through `graphics_lost()`. A request outstanding at loss is still retired, and the accepted hook is restored on the replacement device, as now | **Changes.** Today the request compiles synchronously on the lost device (`material.rs` `request` → `Candidate::prepare`), then is retired at publication |
| Cooked valley attach | Deferred. While lost, `set_scene_contribution` holds the contribution (a later attach replaces it), and the shell attaches it right after publishing the replacement device, before its first encode. `publish_device` installs free slots, so the no-frames-in-flight check passes. A refusal there is reported in the cooked loader errors (`cooked_status`), where the page and checker already look for cooked failures | **Changes.** Today it rebuilds resources on the lost device at once (`runtime.rs` `set_scene_contribution`). That makes GPU calls while lost; the page attaches before RAF starts, so no refusal occurs today |

The two changes land first, on today's facade, with their checker witnesses (M1, first work item). The extraction
then carries them unchanged.

**Report fields.** `WebStatus` carries every field `report()` and the facade read today: adapter name, surface format,
configured width and height, device generation, recovery attempts and recovered, `last_frame` (time since the first
timestamp),
user pause, and the counters (callbacks, presented, unavailable, overload events, refused ticks, clamps). `samples()`
is bounded by `FRAMES` (600) and exposes that bound for `complete()`.

**Cost.** The frame step is the same code, moved and made generic, and is monomorphized for the one runtime. The two
`WebWorld` calls happen on play-state changes, not per frame. There is no new allocation, indirection or copy per
frame. The Wasm size change is recorded in M1, and should be near zero.

**Placement.** A `web` module in `thrum-app`, compiled only for `wasm32` (Thrum Core's ruling, 8 November). A game
author then finds both shells in one crate, they share the counter vocabulary, and the architecture
entry already permits the needed workspace crates (`thrum-render`, `thrum-core`). The clock policy is a pure submodule,
`web_clock` (crate-private, tested natively): a state machine over visibility, user pause and timestamps that returns pause or resume
actions, with no `web-sys`, `wgpu`, `WebWorld` or `FrameWorld` types. It is compiled for `any(target_arch = "wasm32", test)`, so its unit
tests run natively under `cargo test -p thrum-app` and need no wasm test runner. The shell records a play state only after the world
applies it, so a refused resume is asked again. Overload recovery is the shell calling `pause` then
`resume_after_drain` on `WebWorld`; it holds no clock state.

**Report ownership.** The shell exposes typed fields and samples. The game facade keeps the report text (ruling).

## Shared-file seam

Thrum Core reviews these as an exact diff before they land:

| File | Owner | Change |
|---|---|---|
| `crates/thrum-app/Cargo.toml` | Thrum Core (shared crate) | A `cfg(target_arch = "wasm32")` dependency table: `thrum-render`, `thrum-core`, `wgpu` (`webgpu`), `web-sys` (`HtmlCanvasElement`), `wasm-bindgen-futures`. The panic hook stays in the game (Thrum Core's extraction review): the shell must not overwrite an embedding application's process-wide hook. The versions are the ones `v1-game` already pins. The `description` stops saying "Native macOS window" only |
| `crates/thrum-app/src/lib.rs` | Thrum Core | `#[cfg(target_arch = "wasm32")] pub mod web;`, `web_clock` under `any(wasm32, test)`, and the crate doc's "empty on other targets" line rewritten |
| `.config/architecture.toml` | Thrum Core | The `thrum-app` comment names the web shell. The purpose and cost text for `web-sys` ("Browser V1 scene only") and `wasm-bindgen-futures` ("V1 startup only") covers the engine shell. `allowed_workspace_dependencies` is unchanged |
| `tools/xtask/src/main.rs` | Thrum Core | `check_web` also runs wasm32 Clippy on `--package thrum-app` |
| `scripts/check_v1_browser.mjs` | Thrum Core (shared checker) | The loss-policy witnesses (below). They land before the extraction, proven against today's facade |
| `docs/design/modules/PLATFORM-RUNTIME.md` | Thrum Core | One sentence in the G3b section naming the web shell and linking this plan |
| `examples/v1-game/Cargo.toml` | game | `thrum-app` in the wasm32 dependency table |
| `examples/v1-game/src/browser.rs`, `browser/recovery.rs` | game | The two loss-gate changes first; then the facade over `WebShell`, and `recovery.rs` moves into the engine |
| `examples/v1-game/src/host.rs`, `host/web.rs` (new, wasm32), `host/scene_source.rs` | game | `host/web.rs` holds the game's side of the shell: a constructor that returns `Runtime`, `WebWorld` for the inline `WorldPort`, and edit functions over the shell (`set_time_of_day`, `set_camera_*`, `set_scene_camera`, `set_display_overlay`, `set_controls`) with reads (`controlled_position`, `tick`, `camera_orientation`, `time_of_day`). The edit checks are factored out of `host.rs`'s setters so both paths share them: they compare through `&GameScene` and the shell's `source_mut` is taken only on a change. `GameScene::select_camera` is shared the same way. The native wrapper is unchanged and not cfg-gated: the `rg` check covers only the browser facade, which no longer names it |

Not changed: `examples/v1-game/web/index.html` (the page keeps RAF, DOM and input ownership), `web/build.sh`,
`scripts/run_v1_showcase.py`, any `thrum-render` API, the macOS and iOS worlds, and the macOS `Game` trait.

## Loss-policy witnesses

Today's `device-recovery` case already edits during loss and checks retained state. Held controls move the player while
lost, and solar time 18.48 is set while the replacement is pending. After recovery, the paused tick, player position and
time must equal the retained snapshot, while the new generation presents at least 12 frames. It does not change the
camera, and it compares no pixels: only the retry case saves phase screenshots, and it does not compare them. The
checker addition covers the gaps.

**Retained view**, in every recovery round of `device-recovery`, `device-recovery-retry` and `valley-recovery`:
1. While lost, before the restore click, turn the camera through the real `#camera-yaw` control, to a yaw distinct from
   the pre-loss one. Solar time keeps its existing 18.48 edit.
2. After the replacement presents and the world is paused at the retained tick, read yaw and time back from the game
   (`controls_json`, `time_of_day`), not the DOM, and require the edited values.
3. Take state A twice, as A and A′, then three control states:
   - B1: the pre-loss yaw with the edited time;
   - B2: the edited yaw with the pre-loss time;
   - C, then C′: the edited yaw and time again.
   Each state first presents until two consecutive canvas screenshots match (bounded at 30 presents, failing beyond),
   so every state is converged after the history reset its edit set. Then it records two images:
   - a canvas screenshot clipped to the canvas, the presented frame;
   - the capture oracle's readback (`begin_capture`, `try_finish_capture`), whose `generation` must be the new one,
     taken after those presents, so `history_reset_pending` is clear for every state.
4. Require, for both image kinds:
   - A == A′ and C == C′. This controls for determinism, including the transform-history reset the edits set.
   - A == C.
   - A ≠ B1, so the camera is retained.
   - A ≠ B2, so the time is retained.

   If either screenshot repeat fails (A ≠ A′ or C ≠ C′), the screenshot comparison is reported and not claimed. The
   oracle comparison still stands, under the same rule for its own repeats.

**No GPU calls while lost.** Before each destroy, the checker wraps the destroyed device's `create*` methods and its
queue's `submit`, `writeBuffer` and `writeTexture` through CDP and counts calls. Between the loss and the replacement's
publication the count must be zero. This discriminates the two gate changes from today's behaviour, which makes such
calls without error.

**Material reload after loss**, in the `material` case on the development page: after an actual destroy, a fresh
`reload_material` is refused with `graphics device is lost`, starts no compile (`material_report` shows no new
serial), and makes no call on the lost device. After recovery, a reload is accepted and promoted on the replacement device. The existing outstanding-across-loss
step is kept.

**Attach after loss**, a new `valley-late-attach` case with its own navigation, included in `--case all`:
1. Hold the cooked valley's fetch with the CDP `Fetch` domain before navigating.
2. Once `create_browser_game` has returned, destroy the device and wait for `graphics_fault()`, since the lost callback
   is asynchronous. Then release the fetch.
3. The page's `attach_cooked_valley` returns while lost. Require no GPU call on the lost device, and `cooked_status`
   with no draw and no error.
4. When the page reports `Running`, recover through the real restore button. Require the valley drawn on the
   replacement device, with one swap and frames submitted.

## Follow-ups

Neither is planned; each needs its own commission.

- **A second game in Chrome.** Parked by Thrum Core's ruling, not part of this plan. The proof that the shell is reusable is a game other than v1-game drawing in Chrome through `thrum_app::web` alone. The candidate is `examples/starter`, whose macOS build already uses `thrum-app`. It gets its own commission.
- **One device authority.** Agreed with G3t and not built: when `SceneSurface.device` becomes `&DeviceSite`, the shell drops its raw `wgpu::Device` fields and reads the device through the published capabilities' site. [The log](LOG.md) has the entry.

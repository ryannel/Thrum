# Platform runtime web shell plan (G3b)

Design: [PLATFORM-RUNTIME.md](PLATFORM-RUNTIME.md#i1-phone-shell-lifecycle-slice--selected-target-partial-implementation),
contracts [C02](../CONTRACTS.md#c02-time-and-committed-state--f1-coordinator-w1-world),
[C06](../CONTRACTS.md#c06-gpu-execution-and-lifetime--v1) and
[C11](../CONTRACTS.md#c11-input-audio-and-ui-lifecycle--p1a1u1-with-f1). Sibling: the
[iOS shell plan](PLATFORM-RUNTIME-IOS-PLAN.md).
Commission: Thrum Core's routing of daily-review finding D-M3, 8 November (`tmp/g6-boundary/NOTES.md`, "Root routing
for the 8 November daily review"). Design and plan go to Thrum Core before any code.
Reviews: none yet.
Milestone: M1 Engine web shell carries the V1 page unchanged · Updated: 2026-11-08
Now: revised to Thrum Core's 8 November ruling (placement, report ownership, loss policy, M2 parked)
Next: independent review of the design and plan, then the checker addition and the extraction
Blocked: none

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
moves it into the engine. The page, the checker and what a player sees do not change.

## Boundary

| Stays in the game (`examples/v1-game`) | Moves to the engine (`thrum_app::web`, wasm32 only) |
|---|---|
| The `#[wasm_bindgen]` facade: `create_browser_game`, `create_field_game`, `BrowserGame` and every method the page and checker call, with the same names, arguments and report text | `wgpu::Instance` for `BROWSER_WEBGPU`, the canvas surface, adapter and device request, the capability record |
| `index.html`, its RAF loop, DOM events and input mapping | The device-loss signal and the detached, generation-checked reconstruction with its weak mailbox and explicit retry |
| `GameControls`, parameter files, `apply_parameters`, `edit_control`, `controls_json` | Clock policy: timestamp origin and validation, the backwards-timestamp clamp, visibility ∘ user pause, overload pause and drain |
| `GameSetup` (valley, field, culling, stars) and the authored scene | The frame step: reconfigure on resize or `Suboptimal`, advance, encode before acquiring, late acquire, submit, present, poll completion |
| Cooked valley fetch and attach, material hooks (`dev-material`), the capture oracle and its readback | Typed counters (callbacks, presented, unavailable, overloads, refused ticks, clamps) and bounded frame samples |
| Game edits through its existing free functions over `Runtime` (`host::set_time_of_day` and so on) | The capture start gate (see the loss policy) |
| `report()`: every word of the report text, built from the shell's typed counters and samples, so the checker's parsers are untouched | |

The page's JS stays the only owner of `requestAnimationFrame`, `visibilitychange` and DOM input. The engine shell
receives timestamps and visibility as calls. It installs no listener and does not touch the DOM beyond the canvas
it is given. Offscreen capture consolidation (`bin/window/capture.rs`, `motion.rs` and `thrum-app/src/sky/capture.rs`)
is separate and not in this plan.

## Shell design

**Shape.** `thrum_app::web::WebShell<W, V>` is generic over the engine runtime
`thrum_render::runtime::FrameHost<W, V>`, the same runtime the macOS shell drives. It owns the runtime, the surface,
the device, the loss signal and the recovery state. The game builds the runtime once, in a closure the shell calls
with its first device:

```rust
// Sketch, wasm32 only. Not a committed API.
pub async fn create<W, V, F>(canvas: HtmlCanvasElement, prepare: F) -> Result<WebShell<W, V>, String>
where F: AsyncFnOnce(Gpu<'_>) -> Result<FrameHost<W, V>, String>;

impl<W: FrameWorld, V: SceneSource<W::Page>> WebShell<W, V> {
    pub fn frame(&mut self, timestamp_ms: f64) -> Result<(), String>;
    pub fn set_visible(&mut self, visible: bool, timestamp_ms: f64) -> Result<(), String>;
    pub fn set_paused(&mut self, paused: bool, timestamp_ms: f64) -> Result<(), String>;
    pub fn graphics_fault(&self) -> Option<String>;
    pub fn request_graphics_recovery(&mut self) -> Result<(), String>;
    pub fn recovering(&self) -> bool;
    pub fn recovery_error(&self) -> Option<&str>;
    pub fn host(&self) -> &FrameHost<W, V>;
    pub fn host_mut(&mut self) -> &mut FrameHost<W, V>;                  // allowed while lost, see below
    pub fn begin_capture(&mut self) -> Result<PendingCapture, String>; // refused while lost
    pub fn counters(&self) -> WebCounters;
    pub fn samples(&self) -> &[FrameSample];
}
```

- **No trait.** The browser shell has no render thread and no hand-off, so it needs no `Send` bounds, `Edits` or
  `Readback`. The game calls the shell. It is not called back, apart from the one preparation closure. This keeps the
  macOS `Game` trait unchanged.
- **Everything the frame step calls already exists on the generic runtime:** `advance`, `advance_without_rendering`,
  `encode_scene`, `can_acquire_surface`, `resize_surface`, `poll_completed`, `pending_frames`, `pause`,
  `resume_after_drain`, `device_prepare_inputs`, `publish_device` and `begin_capture`. The v1-game wrapper only
  forwards these today. The move therefore changes no `thrum-render` API.
- **One threading model.** The world stays `WorldPort::Inline` on the page thread. The shell drives it through the
  runtime exactly as `browser.rs` does now. The browser baseline needs no threads (PLATFORM-RUNTIME, browser
  constraints).
- **Loss policy.** After loss, `frame` does only `advance_without_rendering`, as now: no poll, completion collection,
  resize or encoding. Edits are classified by what they touch, and the classification is today's facade behaviour,
  kept exactly:

  | Operation while the device is lost | Policy | Today |
  |---|---|---|
  | Authority and view edits through `host_mut`: solar time, camera yaw and pitch, controls (player input), parameter files and overlay, user pause and visibility | Allowed. They change CPU state only: the world, the clock or the view the next encode reads. The replacement device presents that retained state | `browser.rs` `set_time_of_day`, `set_camera_*`, `set_controls`, `set_paused` (390–416, 604–640); the frame step's lost branch (511–520) |
  | Capture start | Refused by `WebShell::begin_capture` with `graphics device is lost`. An outstanding capture's readback is dropped and reported by the game's oracle, as now | `begin_capture` (678–683), `try_finish_capture` |
  | Material reload (`dev-material`) | Not gated by the shell. The game's `MaterialReload` owns its device lifetime: a candidate prepared for a lost device is retired, and the accepted hook is compiled again on the replacement device before it draws | `thrum-asset-gpu` `material.rs`; the checker's `material` case covers a request outstanding across an actual destroy |
  | Cooked valley attach | Allowed. It sets a CPU scene contribution, which every new device stages again | `attach_cooked_valley` (350–368); the `valley-recovery` case |

  `host_mut` therefore stays unrestricted. It does expose the runtime's public GPU entry points (`encode_scene`,
  `begin_capture`, `publish_device`, `device_prepare_inputs`, `poll_completed`). Calling those is the shell's job
  alone, by contract: only its frame step and its gated `begin_capture` do. M1's `rg` check makes this a tripwire, since
  `examples/v1-game/src` must call none of them on the wasm path. A narrower edit view would enforce it in the type
  system, but would need a `thrum-render` API change for one caller, so it waits for a second game. Authority, controls and the world survive reconstruction, as the G3b lifecycle requires.

**Cost.** The frame step is the same code, moved and made generic, and is monomorphized for the one runtime. There is
no new allocation, indirection or copy per frame. Sample storage keeps its `FRAMES` (600) bound. The Wasm size change
is recorded in M1. It should be near zero, since the code moves rather than grows.

**Placement.** A `web` module in `thrum-app`, compiled only for `wasm32` (Thrum Core's ruling, 8 November). Today the
crate is "the application shell shared by Thrum games" but empty off macOS. With `web`, a game author finds both
shells in one crate, they share `percentile` and the counter vocabulary, and the architecture entry already permits
the needed workspace crates (`thrum-render`, `thrum-core`).

**Report ownership.** The shell exposes typed counters and samples. The game facade keeps the report text (ruling).

## Shared-file seam

Thrum Core reviews these as an exact diff before they land:

| File | Owner | Change |
|---|---|---|
| `crates/thrum-app/Cargo.toml` | Thrum Core (shared crate) | Add a `cfg(target_arch = "wasm32")` dependency table: `thrum-render`, `thrum-core`, `wgpu` (`webgpu`), `web-sys` (`HtmlCanvasElement`, `console`), `wasm-bindgen-futures`. The versions are the ones `v1-game` already pins. |
| `crates/thrum-app/src/lib.rs` | Thrum Core | `#[cfg(target_arch = "wasm32")] pub mod web;`, and rewrite the crate doc's "empty on other targets" line |
| `.config/architecture.toml` | Thrum Core | Update the `thrum-app` comment to name the web shell. `allowed_workspace_dependencies` is unchanged |
| `scripts/check_v1_browser.mjs` | Thrum Core (shared checker) | The retained-view addition to `qualifyDeviceRecovery` (M1, first work item). It lands before the extraction, proven against today's facade |
| `docs/design/modules/PLATFORM-RUNTIME.md` | Thrum Core | One sentence in the G3b section naming the web shell and linking this plan |
| `examples/v1-game/Cargo.toml` | game | Add `thrum-app` to the wasm32 dependency table |
| `examples/v1-game/src/browser.rs`, `browser/recovery.rs` | game | The facade over `WebShell`; `recovery.rs` moves into the engine |

Not changed: `examples/v1-game/web/index.html` (the page keeps RAF, DOM and input ownership), `web/build.sh`,
`scripts/run_v1_showcase.py`, any `thrum-render` API, and the macOS `Game` trait. The checker changes only by the
retained-view addition, before the extraction.

**Retained-view witness.** Today's `device-recovery` case already edits during loss and checks retained state: held
controls move the player while lost, solar time 18.48 is set while the replacement is pending, and after recovery
the paused tick, player position and time must equal the retained snapshot while the new generation presents at least
12 frames. It does not change the camera during loss, and it never shows that the recovered pixels reflect the
retained view: its phase screenshots are saved, not compared. The addition closes both gaps, in every recovery round:
1. While lost, before the restore click, also turn the camera through the real `#camera-yaw` control (a yaw distinct
   from the pre-loss one). Solar time keeps its existing 18.48 edit.
2. After the replacement presents and the world is paused at the retained tick, take capture A through the game's
   capture oracle (`begin_capture`, readback), and require its `generation` to be the new one.
3. Set the pre-loss yaw and time and take capture B. Set the edited yaw and time again and take capture C.
4. Require A and C byte-identical, A different from B, and the page's yaw and time readings equal to the edited
   values. A is then the edited view drawn by the replacement device, not the pre-loss view and not a stale frame.

## M1 Engine web shell carries the V1 page unchanged: planned

Done when:
- [ ] `thrum_app::web` owns the instance, surface, device request, loss signal, reconstruction,
      clock policy, frame step, counters and samples. `examples/v1-game/src` has no `wgpu::Instance`,
      `set_device_lost_callback`, `get_current_texture`, `queue.present`, `encode_scene`, `publish_device`,
      `device_prepare_inputs`, `poll_completed` or runtime `begin_capture` call on the wasm path (a `rg` check).
- [ ] A camera and time changed during device loss are retained, and the recovered device presents them: the
      retained-view witness passes in `device-recovery`, `device-recovery-retry` and `valley-recovery`, first on
      today's facade and then on the extracted shell.
- [ ] Every other edit listed as allowed in the loss policy still works while lost, and capture start is still refused
      while lost (the existing loss and recovery cases).
- [ ] The Chrome checker passes, run as `check_v1_browser.mjs --case all` plus the separate `material`,
      `material-disabled`, `field`, `field-direct`, `field-occlusion`, `field-oracle` and `route` cases. Pause,
      visibility, tabs, params, device loss, late loss, recovery, retry, valley and valley-recovery are included. The
      checker is not edited after the retained-view addition.
- [ ] The G7 route check passes against the committed baseline, and the Chrome stop frames are byte-identical to a
      pre-change run of the same commit's content.
- [ ] `report()` text is unchanged in form. The default-page Wasm size, before and after, is recorded.
- [ ] `./dev check` passes, and an independent review agrees. Its brief and verdict go on the Reviews line.

Work:
- [ ] The retained-view addition to the checker, run on the base commit's facade, posted to Thrum Core as a seam diff
      with its evidence and landed first (serves the retained-view item).
- [ ] Pre-change reference: the full checker and route check on the base commit, recorded under `tmp/g3b-web-shell/`
      (serves the checker, route and report items).
- [ ] `thrum_app::web` with the shell, recovery, loss signal and capture gate moved from the game, and wasm32 unit
      tests for the clock policy (clamp, visibility ∘ pause, overload) (serves the ownership, loss-policy and checker
      items).
- [ ] The v1-game facade rewritten over `WebShell`, `recovery.rs` removed, and the `rg` check (serves the ownership
      item).
- [ ] Checker, route and Wasm-size runs; review brief and review (serves the remaining items).

## Reuse follow-up: a second game in Chrome (parked)

Parked by Thrum Core's ruling, not part of this plan. The proof that the shell is reusable is a game other than
v1-game drawing in Chrome through `thrum_app::web` alone. The candidate is `examples/starter`, whose macOS build already
uses `thrum-app`. It gets its own commission.

## Log

- 2026-11-08 Plan drafted from Thrum Core's D-M3 routing, docs only. It was submitted with the shared-file seam and two
  open questions before any code.
- 2026-11-08 Revised to Thrum Core's ruling. Placement is the wasm-only `thrum_app::web` module; report text stays in
  the game facade over typed shell counters and samples; the second starter page is parked as a reuse follow-up. The
  blanket `host_mut` refusal is withdrawn: it would have refused solar time, camera and input edits that today's facade
  intentionally keeps usable while lost. The loss policy now classifies each operation, the shell gates only capture
  start, and M1 gains a retained-view checker addition, landed first on today's facade, proving the recovered device
  presents a camera and time changed during loss.

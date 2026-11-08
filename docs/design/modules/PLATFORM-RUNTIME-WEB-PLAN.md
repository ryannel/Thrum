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
Now: design and plan drafted, submitted to Thrum Core with the shared-file seam
Next: Thrum Core's ruling on the seam and the two open questions, then an independent review of the design and plan
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
| Cooked valley fetch and attach, material hooks (`dev-material`), the capture oracle | Counters (callbacks, presented, unavailable, overloads, refused ticks, clamps) and bounded frame samples |
| Game edits through its existing free functions over `Runtime` (`host::set_time_of_day` and so on) | A report builder over those counters and samples. The game chooses the text, so the checker's parsing is unchanged |

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
    pub fn host_mut(&mut self) -> Result<&mut FrameHost<W, V>, String>;  // refused while the device is lost
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
- **Loss gating moves with the shell.** After loss, `frame` does only `advance_without_rendering`, as now. `host_mut`
  and capture start refuse while the device is lost, so a game edit cannot reach a lost device. Authority, controls
  and the world survive reconstruction, as the G3b lifecycle requires.

**Cost.** The frame step is the same code, moved and made generic, and is monomorphized for the one runtime. There is
no new allocation, indirection or copy per frame. Sample storage keeps its `FRAMES` (600) bound. The Wasm size change
is recorded in M1. It should be near zero, since the code moves rather than grows.

**Placement.** The recommendation is a `web` module in `thrum-app`, compiled only for `wasm32`. Today the crate is
"the application shell shared by Thrum games" but empty off macOS. Adding `web` means:
- a game author finds both shells in one crate;
- they share `percentile` and the counter vocabulary;
- the architecture entry already permits the needed workspace crates (`thrum-render`, `thrum-core`).

The alternative is a new `thrum-web` crate: a cleaner per-platform split, but one more package to register and keep
narrow. Thrum Core decides (open question 1).

## Shared-file seam

Thrum Core reviews these as an exact diff before they land:

| File | Owner | Change |
|---|---|---|
| `crates/thrum-app/Cargo.toml` | Thrum Core (shared crate) | Add a `cfg(target_arch = "wasm32")` dependency table: `thrum-render`, `thrum-core`, `wgpu` (`webgpu`), `web-sys` (`HtmlCanvasElement`, `console`), `wasm-bindgen-futures`. The versions are the ones `v1-game` already pins. |
| `crates/thrum-app/src/lib.rs` | Thrum Core | `#[cfg(target_arch = "wasm32")] pub mod web;`, and rewrite the crate doc's "empty on other targets" line |
| `.config/architecture.toml` | Thrum Core | Update the `thrum-app` comment to name the web shell. `allowed_workspace_dependencies` is unchanged, or a new package entry is added if open question 1 chooses a crate |
| `docs/design/modules/PLATFORM-RUNTIME.md` | Thrum Core | One sentence in the G3b section naming the web shell and linking this plan |
| `examples/v1-game/Cargo.toml` | game | Add `thrum-app` to the wasm32 dependency table |
| `examples/v1-game/src/browser.rs`, `browser/recovery.rs` | game | The facade over `WebShell`; `recovery.rs` moves into the engine |

Not changed: `examples/v1-game/web/index.html`, `web/build.sh`, `scripts/check_v1_browser.mjs`,
`scripts/run_v1_showcase.py`, any `thrum-render` API, and the macOS `Game` trait.

**Open questions for Thrum Core.**
1. `thrum_app::web` module or a new `thrum-web` crate (see placement)?
2. Should the report text stay game-owned, as recommended so that the checker's parsers are untouched, or should
   the engine own a standard report that the game prefixes?

## M1 Engine web shell carries the V1 page unchanged: planned

Done when:
- [ ] `thrum_app::web` (or the crate chosen) owns the instance, surface, device request, loss signal, reconstruction,
      clock policy, frame step, counters and samples. `examples/v1-game/src` has no `wgpu::Instance`,
      `set_device_lost_callback`, `get_current_texture` or `queue.present` on the wasm path (a `rg` check).
- [ ] The Chrome checker passes unchanged, run as `check_v1_browser.mjs --case all` plus the separate `material`,
      `material-disabled`, `field`, `field-direct`, `field-occlusion`, `field-oracle` and `route` cases. Pause,
      visibility, tabs, params, device loss, late loss, recovery, retry, valley and valley-recovery are included, with
      no checker edit.
- [ ] The G7 route check passes against the committed baseline, and the Chrome stop frames are byte-identical to a
      pre-change run of the same commit's content.
- [ ] `report()` text is unchanged in form. The default-page Wasm size, before and after, is recorded.
- [ ] `./dev check` passes, and an independent review agrees. Its brief and verdict go on the Reviews line.

Work:
- [ ] Pre-change reference: the full checker and route check on the base commit, recorded under `tmp/g3b-web-shell/`
      (serves the checker, route and report items).
- [ ] `thrum_app::web` with the shell, recovery and loss signal moved from the game, and wasm32 unit tests for the
      clock policy (clamp, visibility ∘ pause, overload) (serves the ownership and checker items).
- [ ] The v1-game facade rewritten over `WebShell`, `recovery.rs` removed, and the `rg` check (serves the ownership
      item).
- [ ] Checker, route and Wasm-size runs; review brief and review (serves the remaining items).

## M2 A second game runs in Chrome through the engine shell: proposed

The proof that the shell is reusable, which is the point of D-M3: a game other than v1-game draws in Chrome using
only `thrum_app::web` and its own scene. The candidate is `examples/starter`, whose macOS build already uses
`thrum-app`. Thrum Core decides whether this is in scope for G3b or later.

Done when:
- [ ] The starter's page draws its scene in Chrome with no copied platform code. A minimal checker case proves the
      draw, pause and one device-loss recovery.

## Log

- 2026-11-08 Plan drafted from Thrum Core's D-M3 routing, docs only. It was submitted with the shared-file seam and two
  open questions before any code.

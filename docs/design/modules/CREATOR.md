# Creator experience: closing G6

10 November 2026. Design for the `CREATOR` module in [the module list](ARCHITECTURE.md).
[Plan](CREATOR-PLAN.md) · [Done when](CREATOR-DONE.json) ·
[G6 decision](../decisions/G6-CREATOR-EXPERIENCE.md) · [Creator guide](../ENGINE-EXPERIENCE.md) ·
[C11](../CONTRACTS.md#c11-input-audio-and-ui-lifecycle--p1a1u1-with-f1) ·
[C12](../CONTRACTS.md#c12-budgets-evidence-and-acceptance--t2-with-all-owners)

## The owner's request

From the 10 November planning task, word for word: "The owner wants work lined up continuously toward the V1 exit
in docs/BUILD-PLAN.md, so builders always have a next milestone ... each module that can start next has its
design, plan and done-when file, so a builder can start it".

The V1 exit this module serves, from [the build plan](../../BUILD-PLAN.md#goal-and-exit-rule): "hot reload,
profiler and native/browser demo UI" and "A fresh agent builds a small scene from public docs alone." Gate G6
adds: "One command produces a lit scene, sky, cooked mesh, camera and slider ... Keep that scene to about 100 game
lines, report game lines and zero engine game types."

## Purpose and scope

A creator starts a scene with one command, tunes it through named controls that are the same on the Mac and in
Chrome, sees where frame time goes, and has every mistake explained. Thrum's first outlet is teaching demos on
a website, so a creator's scene has to reach Chrome, not only the V1 game's page.

**Already built** under the G6 decision, and governed by it: the typed controls model (`thrum-controls`) with its
AppKit panel and DOM view; parameter files; the macOS app runner, sky kit and starter; saved-file reload;
the cooked material hook; the exposure-zones overlay; three fresh-agent scenes on the Mac in 114, 76 and 74 game
lines. This design does not reopen those mechanisms.

**Commissioned here**, to close G6:
1. A kit scene runs in Chrome from one command (M1).
2. A profiler in the demo UI on both targets (M2).
3. A fresh agent builds a new scene for both targets from the public docs alone, and the gate closes (M3).

**Not commissioned:** the clipping and shadow-cascade overlays, parameter files as a cooked product kind, a
browser-hosted editor, shader hot reload on the public page, iOS. The G6 decision lists them as held or parked and
they stay so.

## Cues

| Cue | Where it must hold | Bar |
|---|---|---|
| U1 One command, one URL: the starter's lit ground, sky and cooked mesh draw in Chrome with their sliders | Chrome on hardware WebGPU on this Mac; `examples/starter` and a copied scene folder | As direct as serving a Bevy example with trunk or opening a three.js example |
| U2 Same controls: keys, groups, display values and refusals match the Mac panel; a parameter file applies the same way | Every control the kit declares; valid, invalid and misspelt keys | One model, two views; no control exists on one target only |
| U3 Edits reach the page: a saved asset is re-cooked and drawn without a page reload, and a refused edit keeps the last good generation and says why | Mesh and texture edits; a rename that breaks a placement | The existing Mac behaviour, in Chrome |
| U4 Where the time goes: display interval, CPU stages and GPU passes are readable in the panel and the page, and an unavailable number says so | The starter and the V1 valley, Mac and Chrome | Unreal's `stat unit` and `stat gpu`: after an edit a creator can tell which pass got slower |
| U5 A first scene from the docs alone: about 100 game lines, no engine game types, no reading of engine source | Mac and Chrome; a scene the docs do not already contain | A new user's first hour with Bevy's or three.js's quick start |
| U6 Mistakes are explained by object, cause and next step | A misspelt control key, a missing or renamed asset, a material compile error, a missing asset store, a lost device | The existing Mac diagnostics, on both targets |

## Techniques and evidence

### A kit scene in Chrome

**Mechanism.** The sky kit (`thrum_app::sky`) gains a web runner on the engine's web shell (`thrum_app::web`),
which already carries the V1 page: canvas surface and device, loss gating and reconstruction, clock policy and
the frame step ([web shell plan](PLATFORM-RUNTIME-WEB-PLAN.md#boundary)). That plan parked "a second game in
Chrome" with the starter as its candidate; this is that commission.

- **The kit is the embedding application.** The web shell's boundary leaves the `wasm-bindgen` facade, the page,
  its animation-frame loop and DOM input with "the game". For kit scenes the kit supplies all of those once, so a
  scene's own source is the same file on both targets and gains no web lines. The shell's rule stands: the engine
  shell installs no listeners; the kit's page does.
- **One controls model.** The page renders the DOM view `thrum-controls` already has and applies edits through
  the same checked path as the Mac panel and parameter files. Nothing about a control is declared twice.
- **Cooked assets come from G5's dev server.** The page fetches the Portable products of the scene's cook through
  the existing Wasm client, and a new generation swaps in at a frame boundary, as the V1 page already does. The
  kit attaches cooked content through `thrum-asset-gpu` as on the Mac.
- **One command.** The existing `./dev scene <folder>` and `./dev starter` grow a web form that builds the scene
  for `wasm32-unknown-unknown`, cooks and watches its assets, serves the page and the store on loopback, and
  prints the URL. Its spelling is the builder's choice; the docs and the check name it.

**Alternatives considered.** A separate web starter with its own page and facade would double the game code and
fail U2 by construction. Deferring to the V1 page (the G6 decision's earlier recommendation) leaves every later
demo scene without a browser path, and `SEA` and `CLOUDS` need one. Compiling the Mac runner to the web is not
possible: it owns an AppKit window and display link.

**Check that it can work.** Every part already runs in Chrome for the V1 game: the shell, the DOM controls,
`?params`, the fetched cooked valley and live asset edits. The starter already depends only on `thrum-app`. What
is new is the kit's facade and page, and gating the kit's Mac-only pieces (`file_watch`, the AppKit panel,
offscreen capture) by target. The material hook stays a development-page feature, as the G6 decision ruled.

### The profiler

**Mechanism.** A profiler group in the controls model, shown by both views, fed only by measurements the engine
already takes:
- the display interval and missed opportunities from the shell's frame samples (the data behind the Mac
  profiler title and the `THRUM_REPORT` frames report);
- CPU time of the world tick, extraction, encode and submit from the same samples;
- GPU time per named graph pass from `thrum-render`'s timestamp queries (`GpuTimings`).

Readouts show a median over a stated window and refresh a few times a second, off the frame path. **Unavailable
is a value:** where the adapter lacks timestamp queries, or a frame was discarded, the row says unavailable and
why, never zero or the last number. **Off is free:** with the profiler off no timestamp query is written and no
readout work runs.

**Alternatives considered.** A separate overlay drawn by the engine would need a text renderer the engine does
not have and would differ between targets. External tools (Instruments, Chrome's tracing) stay the deep tools;
the V1 exit asks for a profiler a creator can read in the demo UI.

**Check that it can work.** The Mac title already shows a throttled frame profile and the render crate already
resolves per-pass timestamps. Chrome exposes `timestamp-query` to WebGPU with coarsened resolution; the row
states the resolution it got.

### The fresh-agent proof

The reviewer of M3 is the fresh agent. Before reading engine source or other design documents, it follows only
the README, the creator guide and the starter's README, and builds a new scene on both targets. This puts the
proof with an agent that has not seen the work, which a builder cannot arrange for itself. Blocking friction
(a step that needs engine source) fails the item. Other friction is recorded and either fixed or listed as a
known limit.

## The system as it runs

| State | Owner |
|---|---|
| Control declarations, values, revisions and refusals | `thrum-controls` (this module) |
| Parameter and material files and their reload | `thrum-app` (this module) |
| The scene shell on Mac (`app`) and web (`web`), and the kit's runners and page | `thrum-app`; the shell's frame, clock and loss policy stay as the platform plans define them |
| Cooked products and the dev server | `CONTENT-LOOP` |
| Frame samples and GPU pass timings | `thrum-render` and the shell; this module only reads them |
| A scene's game code, assets and parameter file | The creator |

A web frame is the V1 page's frame: the page's animation-frame callback passes a timestamp to the shell, which
steps the world, encodes, acquires late, submits and presents. The kit adds nothing per frame except the
existing controls change scan, which returns nothing when no control changed.

**Creator journey.** Copy the starter folder, edit `main.rs`, the assets and the parameter file, run the scene
command for the Mac window or the web form for a URL, tune with the panel or the page, turn the profiler on to
see what an edit cost.

## Risks

| Risk | Early sign | Wrong substitute to refuse |
|---|---|---|
| The kit's web runner needs scene-specific web code | `cfg(target_arch)` appears in `examples/starter/src` | A second starter for the web |
| Chrome draws a fallback instead of the cooked scene | The page reports a drawn generation but the mesh is absent | Passing on status text alone; the check must read pixels |
| The profiler measures itself | Frame time shifts when it is turned on | Hiding the cost; the paired on/off run records it |
| Coarse browser timestamps read as zero | GPU rows show 0.0 ms in Chrome | Showing zero; the row states resolution or unavailable |
| The fresh run passes because the reviewer already knows the engine | The friction log is empty | Reading `crates/` first; the item forbids it and the log must name what was read |

## Performance

Controls are a few dozen values. The profiler adds one timestamp pair per graph pass while on, and a bounded
readout refresh off the frame path; its own cost is recorded by a paired on/off run on the valley. The web
runner adds no per-frame work beyond the V1 page's. No budget is set. Startup of the web form (build, cook,
first cooked draw) is recorded as a tripwire.

## Open questions

- Whether saved parameter files should reach the page without the reload button. Not needed to close G6; the
  page keeps `?params` and the button.
- Whether the profiler should also report memory. Left until a consumer asks.

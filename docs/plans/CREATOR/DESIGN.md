# Creator experience: closing G6

10 November 2026, revised 11 November after
[the design review](../../evidence/CREATOR/design-review.md). Design for the `CREATOR` module in [the module list](ARCHITECTURE.md).
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
2. A profiler in the demo UI on both targets (M2). It needs two bounded additions to the frame runtime, so M2 is
   built after `APPLE-TILE:M3`; [the frame timing feed](#the-frame-timing-feed) says why.
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
| U4 Where the time goes: the frame-callback interval, CPU stages and GPU passes are readable in the panel and the page; every number is current, and a row without one says why | The starter on Mac and Chrome and the V1 valley on Mac; runs longer than the first 600 frames; the profiler switched on and off while frames run; a lost and replaced device | Unreal's `stat unit` and `stat gpu`: after an edit a creator can tell which pass got slower |
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

**Mechanism.** A profiler group in the controls model, shown by both views: one toggle, off by default, and one
table of rows. The profiler times nothing itself. CPU and GPU numbers come from one authority, the frame host's
completed frame sample (`FrameSample`, from `FrameHost::poll_completed`). Interval rows come from the callback
record each shell already keeps.

| Row | Source | What the label says |
|---|---|---|
| Callback interval | The timestamps a shell is handed for its admitted callbacks: the display link on the Mac, the animation frame in Chrome | "callback". It is when the shell was asked for a frame, not when the display showed one |
| Presented and missed | The shell's counts of callbacks that ended in a present call and of those that did not | "present calls", not displayed frames |
| World tick, Extract, Upload | `tick_cpu`, `extract_cpu`, `upload_cpu` | A still scene has no world, and its World tick row says so |
| Encode + submit | `encode_cpu`, which the runtime measures as one span across encoding, finish, submit and completion registration | One row with the combined name |
| One row per GPU pass | The pass-name timing of the same samples, as `APPLE-TILE:M3` delivers it, with the present pass | The graph's pass name |

Physical display cadence is not a profiler row. It stays with `PLATFORM-RUNTIME-MAC`'s attended protocol. Submit is
not split from encode: the split would add a timer to the frame path for a number no cue needs. A creator case
where submit dominates would reopen that, as a proposal to the frame runtime's owner.

Each number is a median over a stated window of the most recent completed frames, refreshed a few times a second,
off the frame path.

**Unavailable is a value.** A row without a current number shows its reason, never zero and never an old number:
the adapter has no timestamp queries, the profiler is off, the frame's timing batch faulted, or no frame has
completed lately (a paused or hidden page, a lost device).
[Chrome coarsens timestamps](https://developer.chrome.com/blog/new-in-webgpu-121) to 0.1 ms steps unless its
developer flag is set, and the page cannot ask which it got. So the readout takes the step from the values in its
window and shows a pass below one step as "< 0.1 ms".

**The table is a new control kind.** A readout today is one number declared at startup. GPU rows are named by the
graph and change with the scene, and a row must be able to hold a reason. So `thrum-controls` gains a read-only
table: an ordered list of label and text rows that the app replaces whole. Both views draw it, and a person's edit
or a parameter file is refused by key as for a readout.

#### The frame timing feed

The review found three things the existing interfaces cannot do. Each is settled here; the interface is also in
[the module list](ARCHITECTURE.md#interfaces).

1. **Fresh samples.** The web shell keeps only the first 600 completed samples of a device generation, for the V1
   page's report and its capture records; the Mac shell keeps a run history. Both keep those meanings. At the
   point where each shell drains completed samples, every sample also goes once, in submission order, to a
   **profile window** in `thrum-app`: one bounded type for both shells that holds the most recent frames and
   knows how old its newest sample is. The table reads only this window. The window is fed only while the
   profiler is on, and it is emptied when the profiler is switched or the device is replaced, so a number never
   outlives the frames it came from.
2. **Timing demand.** The frame host gains one switch that says whether frames are timed. It is on by default,
   which is today's behaviour, so tests, benches, the iOS shell, capture and offscreen runs change nothing.
   - *Off.* The next encoded frame writes no timestamp and submits no resolve or readback. Its sample still
     completes, and says GPU time was not requested. That is a third reason beside "unsupported" and "faulted".
   - *Frames in flight at the switch* finish as they were encoded: a timed frame still resolves and reports.
     Nothing is cancelled, so off is complete once the frames in flight have drained.
     [`wgpu-profiler`](https://github.com/Wumpf/wgpu-profiler) handles the same switch the same way.
   - *On.* The next encoded frame is timed. Rows say they are waiting until its sample arrives.
   - *Device replacement.* The switch is the host's state, not the device's, so it carries over. Batches of the
     lost generation are discarded as today.
   - *Prepared, not created.* Each slot keeps its query set and two small buffers whether timing is on or off,
     as today, so switching on creates no GPU object during play.
3. **Who asks.** Only the window and page shells in `thrum-app` turn demand off, and only for an app that
   declared the profiler group, while its toggle is off, in a run that writes no frames report. A run that
   writes one (`THRUM_REPORT`, the Mac qualification command, the V1 page's 600-sample report) keeps demand on
   throughout. The Mac qualification protocols therefore measure the path they measure today, which does
   strictly more work than the default. The window title's GPU part says timing is off instead of showing zeros.

**Ownership and order.** The switch and the "not requested" reason are the only changes this module makes in
`thrum-render`. M2's builder makes them, by the seam ruling in the module list, once `APPLE-TILE:M3` is Done.
`APPLE-TILE:M2` reschedules encode, acquire, submit and completion, which are the functions the switch touches,
and `APPLE-TILE:M3` replaces the positional GPU slots with pass names in every consumer. Building earlier would
put a second writer in those functions while protected work is in flight, and would hand `APPLE-TILE:M3` one more
consumer to migrate. Once built, the switch belongs to the frame runtime and its owner. Tests in `./dev check`
hold its behaviour, so later runtime work keeps it. Graph passes take their timestamps through the one optional
batch the switch controls, so they obey it without knowing the profiler exists; timing gathered another way, such
as a native segment's, is not gathered while the switch is off. This module owns the profile window, the group
and the rows.

**The valley.** The kit declares the group for kit scenes. The V1 game opts in with one call in
`examples/v1-game`, which is the only edit this module makes there. M2 proves the valley on the Mac; its page is
proved on `SHOWCASE`'s route.

**Alternatives considered.** A separate overlay drawn by the engine would need a text renderer the engine does
not have and would differ between targets. External tools (Instruments, Chrome's tracing) stay the deep tools;
the V1 exit asks for a profiler a creator can read in the demo UI. A profiler that only hides its rows fails "off
is free", because the runtime would keep writing and reading back timestamps. Building the readouts now on the
positional slots, and the switch later, would be migrated once and reviewed twice.

**Check that it can work.** The Mac title already formats these samples, and the render crate already resolves
timestamps per pass. Every timed pass takes an optional timing batch, so an untimed frame is an existing code
path (a device without timestamp queries runs it today). The frame host survives device replacement as one
object, so its switch does too.
[Bevy's render diagnostics](https://docs.rs/bevy_render/latest/bevy_render/diagnostic/struct.RenderDiagnosticsPlugin.html)
and `wgpu-profiler` both report per-pass GPU time from timestamp queries as an opt-in that degrades to CPU time
alone.

**Reading a change.** GPU passes share one device, and some rows hold more than one job: the forward pass
includes the sky draw and a scene contribution's work. So a change is read against the row's own spread across
repeated runs with no change, not against zero. These rows are coupled and may move together: the forward pass
with any material or mesh edit; every GPU row when the machine's GPU is shared or warm; Encode + submit with the
number of passes. M2's inspection uses two changes whose row is known beforehand, and both run today:
pausing the valley's world (`world.paused`) lowers World tick, and an edit to the starter's material hook that
adds fragment work raises the pass that draws the cooked mesh.

### The fresh-agent proof

The reviewer of M3 is the fresh agent, and the exercise comes before the rest of its review. Until its scene runs
on both targets it may have read only the review skill, the project settings, the graded examples, M3's items,
and the three public documents (the README, the creator guide and the starter's README) with whatever those tell
a creator to run or copy. It logs every file it opens, in order. This design, the plan, engine source and other
design documents come after. This puts the proof with an agent that has not seen the work, which a builder cannot
arrange for itself. A reviewer that read those first is not fresh: it says so and gives Could Not Judge, and the
next review starts with a new agent. Blocking friction (a step that needs engine source) fails the item. Other
friction is recorded and either fixed or listed as a known limit.

**The on-glass check** (M3.4) needs a person only for setup: the built-in display unlocked and awake, with Screen
Recording and Accessibility permission already granted to the process the agent runs. The agent then launches the
starter, drags the slider with synthetic pointer events, saves the files, and captures the real window and panel
by window id. An offscreen frame does not stand in for the window. Without the Accessibility grant a person does
the drag and nothing else. Agents never change a grant.

## The system as it runs

| State | Owner |
|---|---|
| Control declarations, values, revisions and refusals | `thrum-controls` (this module) |
| Parameter and material files and their reload | `thrum-app` (this module) |
| The scene shell on Mac (`app`) and web (`web`), and the kit's runners and page | `thrum-app`; the shell's frame, clock and loss policy stay as the platform plans define them |
| Cooked products and the dev server | `CONTENT-LOOP` |
| Frame samples, GPU pass timings and the timing-demand switch | The frame runtime in `thrum-render`. M2 adds the switch there under the seam ruling; this module then only reads samples and sets the switch from its shells |
| The profile window, the profiler group and its rows | `thrum-app` and `thrum-controls` (this module) |
| Callback timestamps and present counts | Each shell, as the platform plans define them; the profiler reads them |
| A scene's game code, assets and parameter file | The creator |

A web frame is the V1 page's frame: the page's animation-frame callback passes a timestamp to the shell, which
steps the world, encodes, acquires late, submits and presents. The kit adds nothing per frame except the
existing controls change scan, which returns nothing when no control changed. With the profiler on, each completed
sample is also copied into the profile window, and the table is rebuilt a few times a second.

**Creator journey.** Copy the starter folder, edit `main.rs`, the assets and the parameter file, run the scene
command for the Mac window or the web form for a URL, tune with the panel or the page, turn the profiler on to
see what an edit cost.

## Risks

| Risk | Early sign | Wrong substitute to refuse |
|---|---|---|
| The kit's web runner needs scene-specific web code | `cfg(target_arch)` appears in `examples/starter/src` | A second starter for the web |
| Chrome draws a fallback instead of the cooked scene | The page reports a drawn generation but the mesh is absent | Passing on status text alone; the check must read pixels |
| The profiler measures itself | Frame time shifts when it is turned on | Hiding the cost; the paired on/off run records it |
| A readout shows the first 600 frames for ever | A row does not move after a load change late in a run | Reading the shell's report buffer; the check changes the load after frame 600 |
| Off only hides the rows | Resolve submissions continue with the profiler off | A UI flag; the switch is on the frame host and tests count the timestamp work |
| A switch mid-run strands a frame | A frame slot never frees after a toggle | Draining or waiting on the GPU at the switch; frames in flight finish as encoded |
| An interval row is read as display cadence | A label says "display", "refresh" or "fps" | Renaming the callback interval; the row names its source |
| Coarse browser timestamps read as zero | GPU rows show 0.0 ms in Chrome | Showing zero; the row states resolution or unavailable |
| The fresh run passes because the reviewer already knows the engine | The friction log is empty | Reading `crates/` first; the item forbids it and the log must name what was read |

## Performance

Controls are a few dozen values. While on, the profiler adds one timestamp pair per graph pass, one resolve and
readback per frame (today's cost on every frame), one sample copy per completed frame, and a bounded table
rebuild off the frame path. While off it adds one branch per frame. Its cost is recorded by a paired on/off run
on the valley, alternating order, with the spread of repeated identical runs beside it. The web
runner adds no per-frame work beyond the V1 page's. No budget is set. Startup of the web form (build, cook,
first cooked draw) is recorded as a tripwire.

## Open questions

- Whether saved parameter files should reach the page without the reload button. Not needed to close G6; the
  page keeps `?params` and the button.
- Whether the profiler should also report memory. Left until a consumer asks.
- Whether the V1 page should write its 600-sample report only on request, so its default run is untimed.
  `SHOWCASE` settles it when it plans the exit route.

# Effect graph: engine modules run their own passes inside the one frame

11 November 2026. [Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) · [module list](../ARCHITECTURE.md) ·
[the brief](../VISION.md). C02, C04, C06, C07 and C12 in [CONTRACTS](../../design/CONTRACTS.md), the
[graph decision](../../design/decisions/G3-EXECUTING-RENDER-GRAPH.md) and
[the port decision](../../design/decisions/SEA-AND-CLOUD-PORT.md) govern. Revised 16 November 2026: a fourth
work package gives effects [the display phase](#the-display-phase-wp4). Revised again that day after
that revision's review (finding MF5; `git show 8ed45fc5b:.orchestrate/evidence/EFFECT-GRAPH/design-review.md`): the
phase describes the target the clock consumes, and a prediction is carried a tick at a time. Revised a third
time that day, after WP4's first review ([review](../../../.orchestrate/evidence/EFFECT-GRAPH/WP4-review-1.md)): which
[pair a frame shows](#the-frames-pair) on a world on its own thread. Revised 17 November, after WP4's second
review ([review](../../../.orchestrate/evidence/EFFECT-GRAPH/WP4-review.md)): [the timing model](#the-timing-model), which
states every rule about a frame's time as one set, with display time on the world's time.

## The request

The owner's planning task of 11 November 2026, word for word:

> Plan the modules CULLING and EFFECT-GRAPH from docs/plans/ARCHITECTURE.md: write each one's DESIGN, PLAN and
> done-when file. Both depend on APPLE-TILE WP2, which is in its second review now, so builders can start them once
> it passes. Everything else is waiting on APPLE-TILE WP2 or WP3, so these are the next work. Keep each first
> work package small enough to start right after APPLE-TILE WP2 lands. Also: APPLE-PRESENTATION WP1's jitter change broke
> APPLE-TILE's exact baseline comparisons, and nothing caught it before it merged
> (.orchestrate/evidence/APPLE-TILE/WP2-review.md MF1). Say in ARCHITECTURE.md or the affected designs how a module's change
> that alters another module's exact baselines gets caught before landing.

What the module is for comes from [the brief](../VISION.md), decision 6: "Sea and clouds are finished as Thrum
modules and are not part of the V1 exit." [The port decision](../../design/decisions/SEA-AND-CLOUD-PORT.md#readiness-6-november)
names the graph condition this module meets: 3D and cube resources, cadence, and async-compute eligibility as a
declared seam. The module list's brief adds the rest: mipped storage arrays, persistent state advanced by
tick-derived time, ping-pong and world-anchored wrapping histories, compute before the fragment passes that read
it, and module passes after opaque geometry that read scene depth and a refraction source.

The lead's planning task of 16 November 2026, after `CLOUDS` WP2's builder ended `BLOCKED needs_dependency`:

> CLOUDS WP2 needs something no plan provides yet. The builder wrote: The planner must settle and assign the shared
> display-time interface change before WP2.7's uneven 60× shade motion can be completed.

The builder's diagnosis (`.orchestrate/evidence/CLOUDS/WP2-temporal-blocker.md` on branch `build/clouds` at
`0756d0adce44`) asked that the change go to
"HOST/EFFECT-GRAPH, with their clock/platform consumers", that it "preserve enough target information and
endpoint history, or explicitly define a bounded prediction coordinate for presentation-only advection", and that
authoritative weather stay unchanged. The time a frame gives an effect is this module's interface, so the change
is [WP4](#the-display-phase-wp4) here. `HOST` waits on seven work packages and changes none of this.

This design reconciles the code on main; it is not a blind study. Read for it: `graph.rs`, `graph_record.rs`,
`contribution.rs`, `synthetic/{scene_graph,contributed}.rs`, `atmosphere/graph.rs`, the [SEA](../SEA/BRIEF.md) and
[CLOUDS](../CLOUDS/BRIEF.md) briefs, and the designs of the three modules whose files it shares. For the revision
after [the design review](../../../.orchestrate/evidence/EFFECT-GRAPH/design-review.md), also read: the history scope in
`graph.rs`, the encoded-frame token in `runtime/tokens.rs`, `timing.rs`, and `APPLE-PRESENTATION`'s WP2 and WP3 items.

## Purpose and scope

An engine module that lives outside `thrum-render`, such as `SEA` or `CLOUDS`, declares its own simulation and
drawing passes and its own persistent GPU state, and the one executor orders, allocates, times and submits them
with everything else. It replaces what Broadsides did with private pass lists and target pools.

**Consumers:** `SEA`, `CLOUDS`, and `ATMOSPHERE` if it later has a measured reason to move. `RENDER-2D` declares
its passes through the same seam after V1; a second way to declare passes is never built.

**What exists today.** The graph declares buffers and 2D textures with layers and mips, in transient, imported,
persistent and history scopes, compiled once per device generation and variant and split at surface acquisition.
A scene holds at most one `SceneContribution`, with fixed slots: an upload copy pass, one timed compute pass, a
draw at the end of the forward pass and a draw into the Sun cascades. The graph sees a contribution's resources
only as opaque tokens. The atmosphere stores its aerial volume as a 33-layer 2D array and its reflection cube as
six 2D layers.

**Commissioned:** the mechanisms below, proved by one diagnostic effect on `mac-metal` and `chrome-webgpu`.

**Not built:** a general plugin framework; passes a game writes; any sea or cloud pass; moving the atmosphere's
existing passes or descriptors (they keep their exact output); real async-compute scheduling; a governor.

## What a consumer must be able to do

| Need | Range it must hold over | Plausible wrong substitute, and what rejects it |
|---|---|---|
| **N1 Declare 3D, cube and mipped storage-array textures** | Transient, persistent and history scopes; the portable recipe's limits and the Mac's | A descriptor the graph accepts but allocates as 2D. Rejected by exact readback of every texel of each kind on Metal and in Chrome |
| **N2 Keep simulated state that advances with the world, not with the display** | 1 to several display frames per tick; hitches of many ticks; discarded and capture frames; pause; a tick that goes backwards; a new timeline | State that advances once per drawn frame, or that resets when a tick repeats. Rejected by comparing GPU state exactly with expectations written from [the state table](#state-time-and-the-tentative-frame-wp2) after a scripted mix of those frames |
| **N3 Run a pass less often than every frame, by a runtime value** | Every frame to once in many; changed while running; two histories at different cadences | Recompiling the graph on change, or a skipped pass whose readers get an older side. Rejected by creation counters across a change and by exact readback of a consumer's output on every run and skip boundary |
| **N4 Draw after opaque geometry, from scene depth and the scene behind** | Both native recipes and Chrome; a depth-only effect and a refracting one | A pass that reads last frame's depth or colour. Rejected by a moving occluder: the effect is hidden behind it in the same frame |
| **N5 Feed reconstruction's inputs** | Motion, reactive coverage and reset on cuts, replacement and effect reset | Zero motion on a moving effect. Rejected by reading motion from submitted frames against the known displacement |
| **N6 Survive resize and device replacement** | Mid-run, with state in flight | Reusing an old generation's texture. Rejected by generation checks and exact state after re-seed |
| **N7 Cost nothing when absent** | Every existing scene and variant | A seam that adds an empty pass or a copy. Rejected by identical compiled pass lists and the exact baselines |
| **N8 Show motion that is a function of time at the display's time** (WP4) | Consumed targets up to 50 ms ahead of the observation; expired and clamped platform targets; a display faster or slower than the tick; a tick whose commit lands late; a frame on a world on its own thread that polls after a tick due later than its observation was committed, once or frame after frame; a clock that starts at any host time on any tick; pause, overload, the hold after a resume, queued ticks and a rewind, each of which leaves stopped time out of display time; capture frames | The interpolation alpha, which stops at the committed tick: the motion then takes a short step and a long one in turn. Rejected by display times worked out from scripted observations and leads, across a tick's commit and on frames whose alpha is 1. Or a phase taken at the platform target while alpha is taken at the consumed one. Rejected by alpha being the phase held to 1 in every frame, and by a frame whose target had expired. Or a tick due after the frame's observation, which no phase from 0 up describes. Rejected by a delayed frame's display time, and by a run of delayed polls that must keep advancing. Or display time written as a host's time, which holds until the clock first stops. Rejected by frames after a pause partway through a tick, held to due times a script gives |

This is internal work: its outcomes are properties measured through the diagnostic effect, with inspected items
where a picture is the evidence.

N5 stops at the inputs. This module proves that an effect's motion, reactive coverage and reset reach the targets
reconstruction reads. How an effect looks after reconstruction, with no shimmer or ghost, is accepted in `SEA`'s
and `CLOUDS`' own work packages on `APPLE-PRESENTATION:WP3`'s reconstructor; the diagnostic slab is not a product and
this module does not wait for that reconstructor.

## Research and selection

- **Persistent resources live outside the graph and are imported.** Unreal's
  [RDG](https://dev.epicgames.com/documentation/en-us/unreal-engine/render-dependency-graph-in-unreal-engine)
  registers external textures and queues extractions so a history outlives one graph, and flags passes as
  `AsyncCompute` or `NeverCull`. Unity's
  [render graph](https://docs.unity3d.com/Packages/com.unity.render-pipelines.core@14.0/manual/render-graph-fundamentals.html)
  says the same: persistent resources are created outside, imported, tracked for dependencies and not owned by the
  graph. Thrum's persistent and history scopes already have this shape, so effects use them and no new lifetime
  class is added.
- **Both of those graphs are rebuilt every frame; Thrum's is compiled once.** They express "not this frame" by not
  adding the pass. Thrum compiles at preparation so that no graph or pipeline work happens in play (C06), so
  cadence needs its own rule, [below](#cadence). The precedent is already in the tree: the atmosphere's validity
  bits and a contribution's `computes()` both record nothing in a declared pass.
- **WebGPU's limits shape the portable recipe.** From [the specification](https://www.w3.org/TR/webgpu/)'s limits
  table (read 11 November): 8 storage buffers and 16 sampled textures per stage, 3D textures to 2,048. Read-write
  storage textures arrived in [Chrome 124](https://developer.chrome.com/blog/new-in-webgpu-124) for a few
  single-channel formats, and wider read-write access needs the optional
  [tier-2 feature](https://developer.chrome.com/blog/new-in-webgpu-142). So a simulation that reads and writes the
  same texels in a multi-channel float format must ping-pong on the portable recipe. The storage-texture count per
  stage, the view dimensions a storage binding accepts and the one-mip-per-binding rule were not re-read from the
  specification today; WP1 records them from the device's capability record in Chrome and the graph validates
  against the record, never against this paragraph.
- **What would change the selection.** If the cadence rules cannot keep readers defined for some consumer, that
  consumer's pass gets prepared graph variants (as day and night have) and the rule narrows. If a second consumer
  needs a stage this design lacks, the stage list grows here, with its consumers, before anyone builds it.

## Mechanisms

### Resource kinds (WP1)

The texture descriptor gains a dimension. The flattened range unit stays "one whole mip of one layer".

| Kind | Layers | Written by | Read as |
|---|---|---|---|
| 2D and 2D array, with mips | Any | Attachment, or storage one mip at a time | 2D or array |
| 3D, with mips | Always 1; depth is an extent | Storage, one whole mip at a time | 3D |
| Cube, with mips | 6 | Storage or attachment through a 2D-array view of its faces | Cube |

The compiler refuses, at preparation: a 3D texture with layers or samples above 1; a cube that is not square with
six layers; a storage write to a cube view; an attachment on a 3D texture; a mip or layer range outside the
descriptor; and, against the capability record of the generation being compiled, a format used for storage that
the record does not allow for that access, more storage textures in one pass than the record's limit, or an extent
above the record's limit. Transient reuse stays exact: only identical descriptors, dimension included, may share
an allocation. A module asks the record which formats a recipe allows; it never probes the device.

### Effects and stages (WP2, WP3)

An **effect** is a `SceneContribution` that also declares passes and resources. The existing trait and its fixed
slots stay as they are, so `thrum-asset-gpu` does not change. A scene holds an ordered list of contributions in
place of one; the game sets the order, and it is the tie-break wherever accesses do not order two passes.

At preparation an effect declares, once per generation:

- **resources:** descriptor, scope, and for persistent and history scopes an owner identity inside the effect's
  own namespace and an epoch;
- **passes:** a name, a stage, its accesses, its pipeline key, a cadence, and whether it is async-eligible.

The scene graph inserts them and the executor calls the effect back to record each pass. Stages are a closed,
ordered set:

| Stage | Kind | Where in the frame | Status |
|---|---|---|---|
| `upload` | Copy | Before anything reads the effect's inputs | Exists |
| `simulate` | Compute | After upload, before the Sun passes and forward | Generalises the one compute slot |
| `shadow` | Draw | Inside the Sun cascade passes | Exists |
| `opaque` | Draw | End of the forward opaque pass | Exists |
| `after-opaque` | Render or compute | After opaque geometry, sky and stars; before the pre-tone input | New (WP3) |

Compute in `simulate` therefore always precedes the fragment passes that read it, by graph order, not by
convention. All of an effect's pipelines are prepared in `rebuild`, before readiness. Its passes are timed under
their own names.

### State, time and the tentative frame (WP2)

Effect state lives in persistent or history resources the effect owns. Three rules make it advance with the world.

**1. The tick drives it.** Each frame gives every effect the frame's source tick, timeline, tick duration and
interpolation alpha (C02). The effect's state carries the timeline and tick it was last stepped to. What a frame
does to the state depends only on how the two compare:

| The frame's tick, against the state's | What the effect does | The state's tick after the frame is submitted |
|---|---|---|
| Same timeline, equal | Nothing: the committed state is kept exactly. This is the ordinary frame, since a 120 Hz display shows each 60 Hz tick twice, and it is every frame of a pause | Unchanged |
| Same timeline, ahead by *n* | Advances by min(*n*, *b*) ticks, where *b* is a bound the effect declares, and counts any ticks beyond *b* as dropped | The frame's tick |
| Same timeline, behind | Resets: it re-seeds from its declared reset. State is never un-stepped | The frame's tick |
| Another timeline | Resets | The frame's tick and timeline |
| First frame of an epoch | Seeds, from its CPU authority or its declared reset | The frame's tick and timeline |

A capture frame reads the state and neither steps nor resets it, whatever its tick.

An advance of several ticks is one execution of the effect's passes, not several: the frame carries the count and
the effect's rule takes it as a parameter, as a larger step or a loop inside the shader. The seam never runs a
producer twice in one frame, which is what lets one compiled graph serve every frame.

**2. Recording is tentative.** Everything recorded for a frame may be discarded before submission. The effect's
state tick, the side of each history and the wrapping anchor commit only in `submitted`. This is the contribution
seam's existing rule, extended to state. It rests on one fact about the host: the encoded-frame token borrows the
host exclusively, so at most one frame is ever recorded and not yet submitted or discarded. When a frame is
recorded, every earlier frame has committed or been dropped, and "the committed state" has one meaning. In-place
persistent writes are safe for the same reason, and because submitted frames execute in queue order.

**3. A new generation starts clean.** Device replacement loses all GPU state. `rebuild` on a new generation starts
a new epoch for the effect. A `rebuild` on the same generation (a resize, a republished visual) keeps the epoch
and the state, unless one of the effect's own descriptors changed, which also starts a new epoch. A GPU error
found at completion takes the device-loss path, as the temporal history does.

### The display phase (WP4)

**What was wrong.** Rule 1 gives an effect the frame's interpolation alpha as its place between two ticks, and
`SEA` and `CLOUDS` read `C − 1 + α` ticks as "the display's time", where `C` is the frame's committed tick. Alpha
cannot carry that time. The clock is sampled at a target `T` ahead of the observed time, by a lead of at most
50 ms toward the moment the platform expects to show the frame (`display_target_lead`). Only observed time
commits ticks (C02). The tick a frame shows was due at `t_C`, at or before the frame's observed time, which
[the frame's pair](#the-frames-pair) keeps true on a world on its own thread, and the frame shows the world one
tick behind its target. In ticks of duration `d` that display time is

`C − 1 + p`, with `p = (T − t_C) / d`,

where `t_C` and `T` are times on the host's clock and the result is a time of the world's
([the timing model](#the-timing-model)).

Alpha is `p` held to [0, 1]. Whenever the lead carries the target past the next tick's due time, `p` is above 1,
alpha stops at 1, and the display time it stands for stops with it; the first frame of the next tick then makes
up the difference. With 60 Hz ticks and a 120 Hz display that can be every second frame, and in `CLOUDS` WP2's
60× capture it was: alpha alternated 0.92 and 1.0, so wind travel alternated 3.2 m and 36.8 m where 20 m was
due ([the planner's reading](../../../.orchestrate/evidence/EFFECT-GRAPH/WP4-plan/README.md) of the builder's record). Every
quantity shown by alpha under a lead steps this way, not only clouds.

<a id="the-timing-model"></a>
**The timing model.** WP4's rules about time are stated here once, as one set. Its first two builds each met a
frame the rules did not cover, a delayed frame and then a resumed one, and both came from writing a frame's
display time as a time on the host's clock. The parts after this one give the lead's meanings, the hosts, the
pair rule and the reasons, and add no rule.

*Two times.* **Host time** is the monotonic clock a host reads and hands the world's clock, which C02 calls
callback time. **World time** is ticks times the tick duration `d`, C02's world seconds: tick `n` is world time
`n · d`. A world's clock stops and resumes, so no fixed offset joins the two, and the time a clock was stopped
exists in host time only. "The display target" had also meant two different host times. Each time has one name:

| Name | What it is | Which time | Who has it |
|---|---|---|---|
| **Observation** `o` | The time the host hands the world for this frame (`FrameHost::advance`'s `now`). It alone admits ticks | Host | The host and the world |
| **Platform target** | When the platform, or the offscreen runner's opportunity clock, expects to show the frame | Host | The host alone |
| **Lead** `L` | What `display_target_lead` makes of the platform target at one read of a clock: the time remaining, with the status `future`; zero for a target that has expired (`late`) or is not a time (`invalid`); 50 ms for one farther off (`clamped`) | A length of host time | The host, which gives it to the frame host |
| **Consumed target** `T = o + L` | The time the clock is sampled at. Alpha is taken here today | Host | The clock |
| **Due time** `t_C` | When the frame's tick `C` fell due, on the clock that admitted it | Host | The clock sample of the frame's pair |
| **Origin** `O = t_C − C · d` | Where world time zero lies on the host's clock, in the span the frame's pair belongs to | Host | Anyone, from the two above |
| **Display time** `W = (C − 1 + p) · d` | The world time at which the frame shows a quantity | World | Every effect, from its tick and phase |

*The rules.*

1. **A clock runs in spans.** A span begins when the clock starts or resumes, at host time `R`, with the world
   on tick `C_R`. In it tick `C_R + k` falls due at `R + k · d`: ticks are due one tick apart, and the origin
   is one number, `R − C_R · d`. A span ends when the clock stops, at a pause or an overload. The part of a
   tick that had elapsed is discarded with the stopped time, so the tick a resume finds is due at the resume,
   and a resume is refused until every admitted tick has committed. When something other than the clock sets
   the world's tick, as a rewind or a replay branch does, the clock is untouched: the tick the world lands on
   takes the due time of the tick it left, which begins a span with another origin. Ticks waiting in the
   queue, a tick that commits late and a hold end no span and move no due time.
2. **A frame consumes a target only while its clock interpolates.** That is while the clock is running, every
   admitted tick has committed, and a tick has committed since the clock last resumed. From a resume until
   that tick the clock **holds**. A clock's first span has no hold: its frames interpolate at once, from one
   tick before the tick it started on, as alpha always has. A frame whose clock is paused, overloaded, holding
   or has ticks queued, and a capture frame, consumes no target: its phase and alpha are exactly 1, its display
   time is the committed tick's, `C · d`, and its record says the clock was not interpolating, so this 1 is
   never read as a target one tick past `t_C`.
3. **The frame's tick was due by its observation:** `t_C ≤ o` in every interpolating frame. An inline world's
   own observation admits its ticks. A world on its own thread reads the host clock its frames are observed
   on, and [the frame's pair](#the-frames-pair) keeps the rule there.
4. **The phase** is `p = (T − t_C) / d`: the host time from the frame's tick falling due to its consumed
   target, in ticks, with no stop at 1. It is held to at most 50 ms of ticks above 1, the same 50 ms that caps
   a lead: 4 at 60 Hz. A target farther past the committed tick holds there. Rule 3 and a lead that is never
   negative keep it from going below 0.
5. **Alpha is the same function.** For one clock sample and one consumed target, alpha is what it was before
   WP4, bit for bit, and equals `min(p, 1)`. A frame with no lead consumes its observation, as before, so a
   `still`, a browser frame and a capture frame are unchanged. WP4 changes two things a frame is given, and no
   exact image rests on either: one host's lead, [below](#one-read-of-the-clock), and the tick a delayed
   frame shows on a world on its own thread, [below](#the-frames-pair).
6. **Display time** is `(C − 1 + p) · d`. In an interpolating frame under the phase's bound that is
   `(C − 1) · d + (T − t_C)`: the world time of the tick before the frame's, plus the host time from the
   frame's tick falling due to its consumed target. With the origin it reads `T − O − d`, the consumed target
   on the world's time, less one tick. It holds whichever tick the frame shows, on an inline world or a world
   on its own thread, however late that tick's commit landed, however long after its observation the frame
   polled the world, and whatever the lead's status.
7. **In one span, equal intervals between consumed targets are equal steps of display time.** Two
   interpolating frames under the bound whose pairs have one origin differ in display time by the difference
   of their consumed targets, across a tick's commit as within a tick.
8. **Display time holds no stopped time.** Between frames of two spans it advances by the difference of their
   consumed targets less the difference of their origins. A stop moves the origin later by the time the clock
   was stopped and by the part-tick it discarded, and a rewind by the ticks rewound.
9. **At the edge of a span display time may step once, by a bounded amount,** and nowhere else:

   | Frame | Its display time | The step |
   |---|---|---|
   | The first that shows a stop, or ticks queued | The committed tick's, `C · d` | From an interpolating frame on that tick: forward by at most a tick where its phase was under 1, as alpha does today, and back by at most 50 ms where it was past 1. Each tick that commits meanwhile adds a tick |
   | Each later frame of the stop, the queue or the hold | `C · d` | None, but one tick for each queued tick that commits |
   | The first to interpolate once a queue has drained | `T − O − d`, in the span it never left | Forward, to where that span's frames would have been |
   | The first to interpolate after a resume | `T − O − d` in the new span: `C_R · d`, plus the host time from the resume's first tick falling due to the frame's consumed target | Forward from `C_R · d`, never back. Showing that first tick, by less than a tick plus 50 ms: the time the tick's commit and the frame took to arrive, plus the lead. With a 30 ms lead the planner's run steps 38.3 ms |
   | A frame whose target is past the bound | `C · d + 50 ms` | Held there until the world commits or stops |
   | The first after a rewind, or on another timeline | That of the tick it now shows | The change of tick. Rule 1 of [state and time](#state-time-and-the-tentative-frame-wp2) resets the effect's state in the same frame |

10. **Sampling is read-only.** Reading the phase or the due time admits no tick and changes no clock. Rule 1
    of [state and time](#state-time-and-the-tentative-frame-wp2) reads the frame's tick only, so what an
    effect's state does in a frame is the same with any phase.
11. **Every source of world time states its phase and its tick's due time from its own clock.** None forms a
    phase from an alpha already held at 1, and there is no default for either.

    | Source | Its clock | What it states |
    |---|---|---|
    | The clocked world and the lane in `thrum-core`; the frame host's world observer in `thrum-render` | `TickClock` | The phase and the due time of the clock sample in the pair the frame shows |
    | The V1 game's inline and threaded worlds, which the iPhone app also runs | A clocked world started at host time zero on tick 0 | The same. An inline world resumes at the host time its host names. A world on its own thread resumes at its worker's own read of the clock, between the request and the reply |
    | `thrum-app`'s sky world with a tick duration (`SkyGame::with_tick_duration`, which `examples/water` runs) | Tick `⌊o / d⌋`: one span from host time zero that never stops | A clock that interpolates in every frame; the phase `(o + L) / d − C`, held at the bound; the alpha it gave before WP4; the due time `C · d`. As first built it reported a phase of 1 and a clock not interpolating beside an alpha that moved, which is the stand-in this rule forbids |
    | The sky world with no tick duration; a `still` | None | A clock not interpolating; exactly 1 |
    | A test fixture that scripts its alpha | None | It is not a source of world time. It reports a clock not interpolating, and nothing reads a phase from it |
    | `SESSION` WP7's [driven world](../SESSION/DESIGN.md#12-a-driven-world) | Its driver's position, with a rate, at a time | A phase and a due time from that position, as a clocked world gives them from its clock |

12. **It is on the record, and an effect is given no host time.** The frame's completed sample and a submitted
    capture's stamp carry the observation, the lead, the phase, alpha, whether the clock was interpolating
    and, when it was, the due time of the frame's tick. The V1 game's capture time record writes them with the
    lead's status and the platform target the runner asked for. Display time can then be worked out from the
    record without the phase, as `(C − 1) · d + o + L − t_C`; two frames are in one span when their origins
    agree; and how far a consumed target lies from its platform target can be read frame by frame. Effect time
    stays as built: the tick, the tick duration, alpha and the phase. An effect that counted time on the
    host's clock would put the stopped time back.

The V1 game starts its world's clock at host time zero on tick 0, on every host. Until that world first stops,
its origin is zero and a frame's display time is its consumed target less one tick as plain numbers, which is
how WP4's first tests passed on a statement that was wrong. A capture that warms up paused and resumes when it
begins, as `examples/v1-game/src/bin/window/capture.rs` does at 1 s, and a window that has recovered from an
overload, run at another origin.

**The consumed target, by the lead's status.** It follows from the lead, so each status has one meaning:

| The platform target, at the host's read | Lead | Consumed target |
|---|---|---|
| Ahead by 50 ms or less: `future` | The time remaining | The platform target, when the lead was measured from the observation. Otherwise the platform target plus the time from the lead's read to the observation |
| Expired or not a time: `late`, `invalid` | 0 | The observation |
| Farther ahead: `clamped` | 50 ms | The observation plus 50 ms |
| None given: the browser shell, a `still`, a warm-up step | 0 | The observation |

An expired frame is not an exception to rule 6. Its consumed target is its observation, and its display time
is that, on the world's time, less one tick.

**What an effect does with the phase.** Each frame gives every effect its display phase beside alpha, in the
frame's effect time (`EffectTime` in `contribution/state.rs`).

- **What an effect may do with it.** Evaluate presentation-only motion at `C − 1 + p`. At or below 1 this is
  what alpha gave. Above 1 the effect takes its quantity up to `p − 1` ticks past the committed endpoint, from
  committed state alone. A function of time is evaluated there. A quantity that a step integrates is carried
  forward **a tick at a time, by that step's own arithmetic with the inputs the committed state holds**, and
  read between the predicted endpoints. One tick's increment carried across several ticks is not this: while a
  rate is itself changing, the same display time would be shown differently from tick `C` and from tick `C + 1`
  ([6 m in the cloud's case](../../../.orchestrate/evidence/CLOUDS/WP2-plan/display-travel.txt)). A quantity known only at its endpoints
  keeps alpha.
- **What a prediction guarantees.** Carried forward so, a quantity is exact for as long as the inputs the
  committed state holds stay what they were: two frames that show one display time from two committed ticks
  show one value, to the quantity's rounding, in a steady state and through a ramp. When an input does change
  at a commit, as when a game sets a new target, the frames after it correct by the difference between the
  quantity as predicted before and as predicted after, at that display time. The consumer states how large that
  can be for its quantity; [`CLOUDS` does for travel](../CLOUDS/DESIGN.md#time-drift-at-the-displays-time-evolution-on-the-tick).

<a id="one-read-of-the-clock"></a>
**One read of the clock.** A host measures the lead from the observation it hands the world, wherever one clock
gives both. The consumed target of a `future` frame is then the platform target exactly, and a delay before the
read costs nothing: the lead is shorter by the delay, and once the read passes the platform target the frame is
`late` and consumes its observation.

- **The offscreen runner** reads one clock twice today: `Offscreen::step_for` measures the lead, and `drive`
  then reads the observation. A scheduling delay between the two puts the consumed target that far past the
  platform target. In the review's case a 2 ms delay gives alpha 0.72 where the platform target asks for 0.60.
  WP4 makes it one read. This is the only lead WP4 changes: where the clock is interpolating, the alpha of a
  frame driven through `step_for` moves by the delay it used to include. The Mac's capture driver calls
  `step_for` in `realtime` and in `fixed` mode, against targets on the machine's own clock, so that alpha
  already differs from run to run and no exact image can rest on its value. WP4.5 holds WP4 to that.
- **The Mac's render actor and the iPhone's render owner** take the platform target on Core Animation's media
  clock and the observation on the world's clock, in two adjacent reads. While a target is `future` their
  consumed target lies off the platform target by the time between those reads. WP4 leaves both as they are.
  Nobody has measured that time, and no WP4 item captures through a window.

<a id="the-frames-pair"></a>
**The frame's pair.** A frame shows one **pair**: a clock sample and the page committed with it. Rule 3 needs
the pair's tick to have been due at or before the frame's observation, `t_C ≤ o`. A phase from 0 up cannot
describe a target that lies before the tick it is counted from. Pairs are ordered by publication, which on one
timeline is the order of their ticks, and "newest" below means that order.

On an inline world the frame's own observation admits the ticks, so the newest pair always qualifies. A world on
its own thread admits ticks from its own reads of the clock. The render owner reads its observation, the frame
host collects GPU completions, and only then is the world polled. A tick that fell due in that gap and that the
world has already committed is due after the observation. WP4's first review met it: observed at 1 ms and polled
once the world had reached 40 ms, a frame with no lead was given tick 2, due at 33.3 ms. The identity then asks
for a phase of −1.94 ([reproduction](../../../.orchestrate/evidence/EFFECT-GRAPH/WP4-review/zero-lead.txt)). Such a frame is a
**delayed frame**.

**Decision.** A world on its own thread holds two pairs at most for its frames: the one it shows, and the
**refused pair**, the newest one a poll has seen and could not show. At each poll it takes the newest of three,
the pair the world has just published, the refused pair and the pair it shows, whose clock is not interpolating
or whose tick was due at or before the poll's observation. That is the pair the frame shows. If the published
pair is newer than it, the published pair becomes the refused pair. A frame never shows an earlier pair than
one already shown. The world's thread, its clock and what it commits are untouched. The phase, alpha and due
time a frame is given are those of the pair it shows.

- **It cannot stall.** A pair seen at one poll was due before that poll. Every host that reads a running clock
  reads each observation after its previous poll, so the refused pair qualifies at the next one: a frame shows
  a tick no older than the newest its previous poll saw. Holding only the pair shown, as WP4 was first built,
  does not have this. On a 60 Hz display whose frames are observed 0.5 ms before each tick falls due and
  polled 1 ms later, every newest pair is refused: in
  [the planner's probe](../../../.orchestrate/evidence/EFFECT-GRAPH/WP4-replan/README.md), which runs the repository's clock,
  600 frames stay on one tick and 596 of them sit at the bound. A 60.01 Hz display drifts through that place
  in the tick and holds up to 73 frames, 1.2 s, as it passes. With the refused pair the same frames follow the
  world tick for tick, and their phase does not pass 1.
- **A stop is shown at once.** A pair whose clock is not interpolating always qualifies, so a poll shows a
  pause, an overload or a hold the first time it sees one, whatever its observation. Until it does, the frame
  takes its target in the span its pair belongs to, and the bound keeps its display time within 50 ms of that
  pair's tick. After a resume the first pair that interpolates is due one tick after the worker's resume, and
  it waits as a refused pair for an observation that reaches it. In
  [the planner's run of the lane](../../../.orchestrate/evidence/EFFECT-GRAPH/WP4-timing/README.md#the-lane-paused-and-resumed)
  every frame of a resumed span has one origin, to 7 ns, and the resume's tick is due between the request
  and its reply.
- **Under the bound.** A delayed frame is under the phase's bound while the time from its previous poll to
  its consumed target, with any lateness of the world's own, is at most 50 ms. A test that scripts its
  observations may hand one from before its previous poll, and the refused pair then waits for an observation
  that reaches it.
- **What changes.** Before WP4 every poll took the newest pair.

  | Frame | The pair it shows | Against before WP4 |
  |---|---|---|
  | Any frame of an inline world: the browser, a `still`, a test at scripted times | The newest | Unchanged |
  | Any frame whose clock is not interpolating: paused, overloaded, holding, with ticks queued | The newest | Unchanged |
  | On a world on its own thread, the newest tick due at or before the observation | The newest | Unchanged: the same tick, page and alpha, bit for bit |
  | A delayed frame, with a lead or without | The newest earlier pair the lane holds whose tick was due | Changed |

  A delayed frame used to take the newest pair at a target before its tick. Its alpha stopped at 0, the frame
  showed the world ahead of its own time by up to the gap, 32.3 ms in the review's frame, and the next frame
  stepped short by as much. It now shows the display time rule 6 gives, like every other frame. When one
  tick was committed in the gap and there is no lead, a quantity read by alpha is drawn nearly where it was:
  the earlier pair near alpha 1 instead of the newer pair at alpha 0. Where a lead carried the old frame's
  target past the newer tick, that frame was already right. It now shows the same display time from the
  earlier pair, and a quantity read by alpha holds at that pair's tick, as it does in a frame polled at once.
- **Who meets a delayed frame.**

  | Who | What it sees | What it has to do |
  |---|---|---|
  | Hosts whose world has its own thread: the V1 game in the Mac window and in Mac captures, and the iPhone app (`PLATFORM-RUNTIME-MAC`, `PLATFORM-RUNTIME-IOS`, `T2-CAPTURE`) | The frame shows an earlier tick than the world's newest. The frame's report gives the tick shown, and counts each completed tick once, in the first frame that shows it or a later one | Nothing. None of their files or items changes |
  | Readers of alpha and the page: the game's camera and entities, `SEA`'s and `WATERS`' wave time, `RENDER-2D`'s sprites | A pair and an alpha that describe the frame's own consumed target | Nothing |
  | Readers of the phase: `CLOUDS` from its WP2 | The same identity as in any frame. With no lead the phase can pass 1 in a delayed frame, when the pair it shows is more than a tick behind its observation | Nothing: a phase past 1 is already theirs to predict from |
  | Effect state (rule 1) | A frame on the tick the state already holds, then an advance of several ticks. Never an earlier tick | Nothing: both are rows of rule 1's table |
  | Records: the completed sample, the capture time record, `thrum-app`'s profile recording | Completed ticks land in a later frame's row than before. The tick, observation, lead, phase and due time are on the record | Nothing |
  | Later owners of these files: `HOST` WP2 to WP5, `SESSION` WP7's driven world, `GAME-SPLIT` | The rule and its tests | Keep the tests passing. A driven world on its own thread shows pairs by the same rule |
  | The browser, `still`s and inline tests | No delayed frame | Nothing |
  | Every exact baseline | Held by WP4.5: none has moved | Nothing |

- **Its cost.** One comparison in each poll, and at most one more leased page. The snapshot ring has four
  pages, keeps its newest two and writes a third. The lane's two pairs are among those except when a delayed
  poll is followed by three ticks with no poll, 50 ms at 60 Hz. The world then finds no page to write, commits
  the tick, skips that publication and counts it, as C02 has optional pages do.
- **Why this rule.**

  | Choice | Verdict |
  |---|---|
  | Take the newest pair and let the phase go below 0 | The identity would hold. Every reader would then have to show a time before the tick it is given. The page holds that tick and the one before it, so alpha has nothing to show there, and a stepped quantity cannot be carried backwards from committed state |
  | Take the newest pair and hold the phase at 0, as before WP4 | Display time is wrong by up to the gap, and the next frame steps short: the unevenness WP4 exists to remove |
  | Report such a frame as not interpolating, or drop it | The world is running. A 1 there would be read as the committed tick, and a dropped frame is a hitch |
  | Hold only the pair shown | Right for one delayed frame. Stalls when frames are delayed in a run, as above |
  | Have the world publish its newest two pairs | No extra page. Stalls when two ticks are committed in the gap of every frame, as the probe's 30 frames a second with a 20 ms gap do |
  | Hold a queue of pairs | The exact pair for any gap, for more leased pages. The refused pair already reaches every host that reads a clock |
  | Judge the pair by the consumed target, not the observation | Fewer frames change, and under a lead a delayed frame predicts less far: 1.97 ticks against 3.97 in the probe's 30 Hz frames with a 50 ms lead. But the poll would need the lead, the tick a frame shows would depend on its lead, and a threaded frame would show a tick that no inline frame with that observation can. Not selected. Whoever plans the alpha readers under a lead may reopen it |
  | **Show the newest pair due by the observation, and hold the refused pair** | Selected |

**Where a lead comes from.** The Mac's render actor, the iPhone's render owner and the offscreen runner's
`step_for` pass a lead. The browser shell passes none, and neither does a `still` capture or a warm-up step. With
no lead the phase passes 1 only when a tick's commit lands late or a frame is delayed, which an inline
world's never is. `SESSION`
M7's [driven world](../SESSION/DESIGN.md#12-a-driven-world) states its presentation as a position with a rate,
which gives a phase the same way.

**Why a prediction, and not something else.**

| Choice | Verdict |
|---|---|
| Hold at the committed tick (today's alpha) | Uneven whenever there is a lead; the defect |
| Take alpha from the observed time and ignore the target | Never passes the endpoint, but motion is then only as even as the callbacks, which is the jitter a lead exists to remove; it also shows everything a lead later |
| Show everything a further tick behind, from three endpoints | Exact for any lead up to a tick. Every interpolating consumer and the tick page would keep a third endpoint, and the camera and input would gain a tick of delay. Too much to change for effects whose motion is known ahead |
| Commit ticks up to the consumed target | Breaks C02: observed time alone advances authority, a world on its own thread does not know the target, and a driven world commits what its session names |
| **Give the unclamped phase and let each quantity choose** | Selected. C02 already says interpolation is defined per quantity. Motion with a known rate is exact past the endpoint. Unity offers the same choice per body: interpolation "has a time lag of one physics update", and extrapolation predicts ahead of the last update and suits motion whose rate holds ([Rigidbody interpolation](https://docs.unity3d.com/2022.2/Documentation/Manual/rigidbody-interpolation.html), read 16 November) |

And for which target the phase describes:

| Choice | Verdict |
|---|---|
| The platform target, with alpha left as it is | Cannot hold: alpha is taken at the consumed target, so wherever the two differ, alpha is no longer the phase held to 1. An expired target also lies behind the observation, where the world has already been admitted |
| The platform target, with every host made to consume it exactly | Right where one clock gives both, and done there. The window hosts read two clocks, so exactness needs the clocks paired once for a session. That is a change to both platforms' frame loops with no item here to prove it |
| **The consumed target, with one read where one clock serves** | Selected. It is what the clock evaluates in every frame, so one identity covers `future`, `late` and `clamped` frames. The runner the Mac's captures use then consumes the platform target exactly |

And for what display time is counted in, after WP4's second review
([its frames](../../../.orchestrate/evidence/EFFECT-GRAPH/WP4-review/resume-identity.txt)):

| Choice | Verdict |
|---|---|
| Keep "display time is the consumed target less one tick", both on the host's clock | True only on a clock that started at host time zero on tick 0 and has never stopped. After the review's 1 s pause it asks for a phase of 60.5, against a bound of 4 and an alpha of 0.5 |
| Let the world advance through the stopped time, or count that time into the phase | Breaks C02: a pause keeps elapsed time out of authority, and hidden wall-clock time is never applied |
| Give the resumed frame another tick number | The tick is the identity of the page and of the effect's state |
| Report frames after a resume as not interpolating | The world is running. Motion by the phase would stop for good at the first pause |
| Record the observation on the world's time | The observation, the lead and the platform target are compared on the host's clock, as a `future` frame's are. One field cannot be both |
| Carry the origin with each pair | It is negative for a world that starts on a high tick. The due time says the same and is never before the clock's start |
| **Two times, joined by the due time of the frame's tick** | Selected. The clock sample already holds it, and the clock's and the lane's arithmetic do not change ([the check](../../../.orchestrate/evidence/EFFECT-GRAPH/WP4-timing/README.md)) |

The one step a resume leaves, in rule 9, is the hold's: alpha has waited for the first tick after a resume
since before WP4, and every reader of alpha resumes that way. Interpolating through the hold would move when
alpha starts after a pause for all of them, which is theirs and `HOST`'s to plan, with the readers of alpha
under a lead.

**Not built here.** `CLOUDS` WP2 is the first reader. `SEA`'s and `WATERS`' wave time, `RENDER-2D`'s sprites and
a game's camera and entities stay on alpha: under a lead they step unevenly by the same arithmetic, which has
not been captured. Each is its owner's to plan, and the game-facing half belongs with the contract `HOST` WP4
writes. The window hosts' two reads are their owners' too. `HOST` WP2 to WP5 and `SESSION` WP7 rework the files
this work package touches and keep its tests passing.

**Check that it can work.**

- **The uneven capture.** In [the planner's reading](../../../.orchestrate/evidence/EFFECT-GRAPH/WP4-plan/README.md), the
  targets of the 60× rows (submission time plus consumed lead) put the six frames at `p` = 0.92, 1.40, 0.92,
  1.40, 0.92, 1.40 on ticks 927, 927, 928, 928, 929, 929. Display time `C − 1 + p` then advances 0.48, 0.52,
  0.48, 0.52, 0.48 ticks, which is 19.0 to 20.8 m of travel a frame against the capture's 3.2 and 36.8 m. The
  spread left over is the record's: it holds the submission time, not the observation, which is why WP4 writes
  the observation. The arithmetic was run on the rows while planning and is not a capture of the fix.
- **The consumed target.** [`consumed_target.py`](../../../.orchestrate/evidence/EFFECT-GRAPH/WP4-plan/consumed_target.py), in
  the clock's own whole-nanosecond arithmetic ([output](../../../.orchestrate/evidence/EFFECT-GRAPH/WP4-plan/consumed-target.txt)):
  in the review's delayed frame, its expired frame and a clamped one, display time is the consumed target less
  one tick and alpha is the phase held to 1; one consumed target seen from ticks 60 and 61 is one display time;
  and over 2,000 platform targets 1/120 s apart, one read gives steps of 8.33 ms in every frame, where two
  reads up to 2 ms apart give 6.39 to 10.27 ms.
- **The timing model.** [`spans.rs`](../../../.orchestrate/evidence/EFFECT-GRAPH/WP4-timing/spans.rs) drives the repository's
  clock from a start at host time 7.3 s on tick 100 through queued ticks, a pause partway through a tick, a
  resume, the hold, two overloads and a ten-tick rewind, at 60 and 100 Hz with leads of 0 to 50 ms, and holds
  every frame to rules 1 to 9 from the script's own times
  ([output](../../../.orchestrate/evidence/EFFECT-GRAPH/WP4-timing/spans.txt)). Across 272 frames at each rate the worst distance
  from the model is 4.2 ns, the phase's `f32`. The review's resumed frames, 1.000 s and 1.008 s from the
  literal identity, are exact with origins of 1000 and 1008 ms.
  [`lane_lifecycle.rs`](../../../.orchestrate/evidence/EFFECT-GRAPH/WP4-timing/lane_lifecycle.rs) pauses and resumes the
  repository's lane on its own thread, with 0, 3 and 20 ms between each observation and its poll
  ([output](../../../.orchestrate/evidence/EFFECT-GRAPH/WP4-timing/lane-lifecycle.txt)): origin zero before the pause, one
  origin after the resume, and the hold shown as the committed tick. So the clock and the lane already keep
  the model, and WP4 has to state the due time, carry it and test it.
- **The frame's pair.** [`kept_pair.rs`](../../../.orchestrate/evidence/EFFECT-GRAPH/WP4-replan/kept_pair.rs) runs the
  repository's clock against four lanes: the newest pair, one held pair, the world's newest two pairs, and a
  held pair with a refused pair ([output](../../../.orchestrate/evidence/EFFECT-GRAPH/WP4-replan/kept-pair.txt)). In the review's frame the newest pair is
  32.3 ms from the identity and either held pair meets it at phase 0.06. In runs of delayed frames at 30, 60,
  60.01 and 120 frames a second, with leads of 0, 8 and 50 ms, the refused pair keeps every frame within 3 ns
  of the identity and under the bound, and no frame waits on a tick that has fallen due. It models the lane.
  WP4's items hold the lane itself to the same.

### Ping-pong histories (WP2)

A ping-pong history is a history-scope pair. The existing scope binds "the last submitted side" for reads and
flips on every submitted frame. That is right for the view's temporal history and wrong for an effect that may
skip. Say side A holds step 4 and a submitted update writes step 5 to B. On the next frame the update is skipped.
A reader bound to that frame's nominal read side sees step 4, and a flip on that submission makes the next update
start from step 4 again. Persistence alone does not fix either.

So each effect history pair has its own **committed side**, kept by the seam, and a pass declares one of three
bindings to it:

- `previous`: the committed side. Any pass may read it.
- `next`: the other side. Only the pair's producer writes it, on a frame the producer runs, and it writes all of
  it.
- `result`: what a consumer ordered after the producer reads. On a frame the producer ran it is `next`, the
  tentative output. On a frame the producer was skipped it is the committed side.

When a frame is submitted, a pair flips its committed side if and only if its producer ran in that frame. Pairs
flip independently, so two histories at different cadences each stay consistent. A discarded frame flips nothing:
the next frame reads the same committed side and writes over the same `next`. A capture frame runs no producer.

In the example: frame 1 runs the update, its consumers read B, and submission commits B. Frame 2 skips; its
consumers read B and nothing flips. Frame 3 runs, writes step 6 to A and is discarded; B stays committed. Frame 4
runs, reads B and writes step 6 to A again.

Bindings for both sides are prepared in `rebuild`, so choosing a side creates nothing in a frame. A producer may
be a group of passes, such as one per mip, that run or skip together. The graphs cited above are rebuilt every
frame and need no such rule; this one is derived here, and WP2's cadence witness is its proof.

**World-anchored wrapping.** A wrapping history is addressed toroidally by world position, so a moving camera
exposes only a strip of new texels. Addressing is the effect's shader. The seam owns the one part that must follow
the tentative rule: the **anchor**, an integer texel origin snapped from a world position, with its shift since
the last *committed* frame. An anchor moved in a discarded frame is not committed.

### Cadence

A pass declares one of three cadence classes. Each frame the effect says, from runtime values and with no
recompilation, whether a skippable pass runs and what a time-sliced pass covers:

- **every frame**;
- **skippable**: it runs on some frames, and writes its whole output when it does;
- **time-sliced**: it runs every frame over a part of its output that the effect chooses, in place.

A pass that steps state by the tick is skippable: on an equal-tick frame it does not run. So skipped producers are
the ordinary case at 120 Hz, not a rare one.

These rules keep that sound against a graph compiled once:

- A skippable or time-sliced pass writes only persistent or history resources, never a transient or an attachment
  another pass loads. The derived load and store operations, always "store" for those scopes, then never change.
- A reader of an in-place persistent resource sees its last written contents. A reader of a history follows the
  pair's bindings above.
- A time-sliced pass never writes a history's `next`: the texels it left would be two steps old. It writes a
  persistent resource in place.
- **Full frames.** The first frame of an epoch, and a frame that resets, are full for that effect: every one of
  its passes runs whatever its cadence, a time-sliced pass covers its whole output, and each producer writes from
  the seed or reset state, not from `previous`. So no texel is read before it is written, on the first epoch or
  after a reset. A full frame that is discarded is full again next time, because "seeded" commits only on
  submission.

The compiler refuses, at preparation: a skippable or time-sliced pass that writes a transient or a loaded
attachment; a time-sliced pass that writes a history; two producers of one pair, or producers of one pair with
different cadences; and a `result` read by a pass ordered before the producer.

A skipped pass reports "not run" in timing, not zero. Cadence values are **quality knobs**: each is a named
runtime value with a range and a declared invalidation (none, the effect's own history, or the view's
reconstruction history). The seam lists them and commits a change at a frame boundary, applying the declared
invalidation. The governor in `APPLE-PRESENTATION:WP7` is the consumer; this module builds no policy.

### After opaque (WP3)

An `after-opaque` pass may read scene depth, load and write scene colour, and write motion and reactive coverage
under [the temporal contribution](../APPLE-PRESENTATION/DESIGN.md#seams-for-thrum-core-proposals): motion from
unjittered positions, invalid history marked, the frame's reset flag honoured. Effects in this stage run in list
order. That is an execution order and nothing more: C08 also requires medium segments to be ordered and disjoint,
or to carry an explicit joint or approximate integration policy. Admitting and composing segments stays with
`SEA`, `CLOUDS` and V3 composition, and this foundation builds no integrator.

- **Scene depth and the colour copy are separate requests.** Depth is stored and sampled when any reader declares
  it: an after-opaque pass, such as a volume that refracts nothing and still stops at terrain; reconstruction,
  which `APPLE-PRESENTATION` WP2 stores depth for whenever it is enabled; or any other pass that samples it. The
  graph derives storage from the union of readers (C06), so dropping one reader removes storage only when no
  other reader is left.
- **The refraction source** is a copy of scene colour taken after opaque, sky and stars, declared only when an
  effect requests it and shared by all that do. On a tile GPU it ends a render pass, so its cost is recorded; a
  cheaper recipe-specific source is `SEA`'s decision.
- **An absent effect adds nothing:** no pass, no copy, and no storage beyond what the other readers already need.
- **Where it sits.** After the star composite, so a volume that writes no depth still dims the stars behind it;
  before the pre-tone input that `APPLE-TILE:WP3` exposes, so reconstruction and tone see the effect.

### Async-compute eligibility

A `simulate` pass may declare itself eligible. The compiler checks that it is compute and touches no attachment or
surface, and the compiled graph lists eligible passes. Nothing is scheduled differently: wgpu exposes no second
queue on Metal or WebGPU. The flag exists so a later recipe can use it without changing a consumer.

### Check that it can work

- **Kinds.** wgpu 30 creates 3D and cube-compatible textures on Metal and WebGPU, and the atmosphere already
  writes a cube through a 2D-array view. WP1 proves each kind by exact readback before anything depends on it.
- **A compiled graph with skipped passes.** The soundness argument is the cadence rules; the atmosphere's validity
  bits have run this way since G3s. The static graph declares both sides of each pair, and no frame reads and
  writes one side: `previous` and `next` are different sides, and `result` is read only after the producer.
- **Repeated ticks.** The equal-tick row keeps the state, so a nonempty trail survives the second display frame of
  a tick and a pause of any length. A CPU oracle that shared a wrong rule with the GPU would agree with it, so
  WP2's test also asserts the table's rows directly: the field is nonempty and unchanged after repeated ticks and
  a pause, and at its declared reset after a backward tick and after a timeline change.
- **State under discard.** A discarded encoder executes nothing, so GPU state is the last submitted state, and the
  commit point is already `submitted`. The diagnostic's state is integer, so WP2 compares it bit for bit after
  every kind of frame.
- **Time-slicing from nothing.** A full frame covers every texel before any sliced frame reads one. WP2 reads every
  texel back after the first frame of an epoch and after a reset.
- **Portable limits.** Ping-pong removes the need for read-write float storage. The remaining limits are counts,
  which the compiler checks from the record.

## The system as it runs

| State | Owner |
|---|---|
| Descriptor kinds, validation, allocation, pass order, load and store | `thrum-render::graph` (`APPLE-TILE` owns the split and the storage derivation; this module extends the descriptor) |
| The contribution list, stages, the tentative-commit protocol, the anchor, knobs | This module (`contribution.rs`, `synthetic/{contributed,scene_graph}.rs`, the frame host's list) |
| The frame's effect time: tick, timeline, tick duration, alpha and the display phase | This module states it; the world's clock in `thrum-core` and the frame host produce it |
| The due time of a frame's tick, and so its span and origin | The world's clock; the pair's clock sample carries it and the frame host records it |
| The observation and the lead a frame consumes | The host that drives the frame; the frame host records them |
| An effect's resources, epoch, state tick and shaders | The effect's module (`SEA`, `CLOUDS`) |
| The view record, motion and reactive targets, history acceptance | `APPLE-PRESENTATION` |
| The pre-tone input and pass-name timing | `APPLE-TILE:WP3` |
| Environment products an effect lights with | `ATMOSPHERE:WP5` |

**A frame with one effect, in order.** The host reads the tick page and builds the frame record. `upload` copies.
`simulate` does what the state table says for the frame's tick, each pass as its cadence allows, with consumers
reading each history's `result`. Sun passes draw, with the effect's casters. Forward draws opaque geometry, the
sky and the effect's opaque draw; stars composite. The refraction copy runs if requested. `after-opaque` passes
draw. The pre-tone input, reconstruction, tone and present follow. On submission the effect commits its tick, its
anchor and the side of each history whose producer ran; on discard it commits nothing.

**The diagnostic effect** lives in `examples/v1-game`, behind a selection that is off by default, and uses only the
public seam, as `SEA` and `CLOUDS` will from their own crates. It carries one resource of every kind and a rule a
CPU can reproduce exactly: a wrapping ping-pong field, stepped every tick, in which a moving source leaves a trail
that spreads and fades; a second ping-pong field derived from the first at a runtime cadence; a consumer pass that
reads both results into a persistent texture the tests read back; a 3D volume rewritten time-sliced in place; a
mipped array at a runtime cadence; a cube; and in WP3 a slab drawn after opaque that samples them, refracts the
scene behind it and reports motion.

## Shared files and exact baselines

`graph.rs`, `scene_graph.rs`, `contribution.rs` and the frame host are also edited by `APPLE-TILE` WP3 (pre-tone
input, pass-name timing), `APPLE-PRESENTATION` WP2 (the third forward target, history protocol), `CULLING` and
`CREATOR` WP2 (the timing-demand switch). WP1 stays inside the graph's descriptor, validation and allocation. WP2 and
WP3 integrate sequentially with whatever has landed and rerun the affected witnesses of each. Once the
timing-demand switch exists, effect timing is not gathered while it is off.

WP4 also edits `thrum-core`'s clock, world lane and clocked world, the frame host's world observer and sample,
`thrum-app`'s sky world and offscreen runner, the V1 game's world wrappers, and the V1 game's capture time
record, under [the module list's ruling](../ARCHITECTURE.md#seam-ruling-for-effect-graph-m4).
It adds to each and changes two things any of them gives today: the lead `Offscreen::step_for` measures, by
[one read of the clock](#one-read-of-the-clock), and the pair a delayed frame shows on a world on its own
thread, by [the frame's pair](#the-frames-pair). The Mac's render actor and the iPhone's render owner are not
edited.

[The module list](../ARCHITECTURE.md#exact-baselines) says how a change that alters another module's exact images
is caught. This module must move none, on the Mac or in Chrome: with no effect attached, every existing scene
compiles to the same passes and storage and draws the same bytes. Exact images are verified on main after a
merge, by the main-branch check `exact-images`, under
[the ruling that proofs run once, on main](../ARCHITECTURE.md#proofs-run-once-on-main) and `ORCHESTRATE.md`. No
proof is needed before a merge and none is repeated after a rebase. WP3.8 and WP4.5 pass from that shared run.
WP1 to WP3 were built under the earlier rule, a proof gate in `./dev check` before each landing, which that
ruling replaced on 15 November.

A verifier run can fail with no image moved. Until `CULLING` WP3's coverage output is on main, the valley is not
one image under GPU load on the Mac, and Chrome shows the same signature: one frame of a capture can differ from
its reference along shared triangle edges, as WP3's qualification at `324586ee` did at one Chrome record of 481.
[The module list's ruling](../ARCHITECTURE.md#a-frame-that-varies-under-load) gives the four tests by which such
a run is set aside as load variation, and the record it needs. WP3.9 holds this module to that ruling. A failed
run that misses any test is rule 1's: the builder stops, writes the proposal in the log and ends `BLOCKED`.

## Cost account

- **Absent:** no pass, no resource, no copy, no per-frame call beyond iterating an empty list.
- **Per effect and frame:** one callback per declared pass; skipped passes cost a branch.
- **Stored depth and the refraction copy:** only on request. The copy is one scene-extent colour copy and a render
  pass break. The earlier culling session's harness put an empty second pass that loads and stores colour, motion
  and depth at about 0.07 ms at 720p on this Mac, contended; its logs are not committed, so WP3 measures it.
- **Memory:** an effect's own resources, reported per effect from their descriptors.

Recorded as tripwires, never gates: per-pass GPU time by name, bytes per effect, and about 25% paired regression
on the valley route with no effect attached.

## Risks

1. **The cadence rules are too strict for a real consumer** (a time-sliced cloud march that wants a transient, or
   a partial write to a history). Early sign: `CLOUDS`'s cost account. Fallback: prepared variants for that pass,
   or a copy-forward history write, designed here before anyone builds it.
2. **Chrome refuses a kind or count the Mac accepts.** Early sign: WP1's recorded limits; WP2's Chrome run. The
   compiler's record check turns it into a preparation error, not a lost frame.
3. **The seam grows into a framework.** Each mechanism names its consumer above; a mechanism without one is cut.
4. **A graph change moves an exact image.** The main-branch check `exact-images` finds it on main after the
   merge, and the module that moved it fixes it.
5. **A tile GPU pays for the pass break.** Recorded in WP3; `SEA` and `CLOUDS` choose their recipes from it.
6. **A consumer needs true sub-steps in one frame**, such as a stability-limited solver after a hitch. Early sign:
   `SEA`'s design. Fallback: a declared, fixed number of sub-step passes, designed here before anyone builds it.

## Proof and builder handoff

- **Targets.** `mac-metal` on both recipes, and `chrome-webgpu` through `scripts/check_v1_browser.mjs`, which needs
  a browser launch the Codex sandbox grants per run, as the existing cases do. `ios-sim` is not a target here.
- **Capture.** The inspected items use T2 engine capture and name their mode: `realtime` for the trail in WP2,
  `fixed` stills for the slab in WP3. The stills prove composition. No item here judges reconstructed motion.
- **Timing.** WP2 reads per-pass GPU time from the executor's named pass timings (`PassTiming` in `timing.rs`),
  which exist today, in its own test or bench. It does not change the host's frame-sample fields, which
  `APPLE-TILE:WP3` moves to pass names. WP3 here needs that work package and reports a skipped pass through its
  schema.
- **WP4.** `mac-metal` only: Chrome gives no lead and runs its world inline, so nothing there can differ, and
  the exact baselines cover it. Its tests script the observations, the leads, the start and each resume, so
  they do not depend on how busy the machine is, and every expected due time and display time comes from that
  script and never from a phase or a due time the engine returned. A delayed frame's test reads its
  observation from the running clock and waits, within a time limit, for the world to commit past it before
  it polls. A test that resumes a world on its own thread cannot script the worker's read of the clock: it
  reads the host clock before its request and after the reply, and holds the resume's tick to that bracket
  and the span to one origin. The one `realtime` run holds every interpolating frame under the bound to rule
  6, expired targets included, and counts the frames at the bound, the frames whose clock was stopped and
  each change of origin instead of failing on them. No WP4 item drives a window, so none needs an attended
  display.
- **GPU.** Every GPU check holds the `gpu` lock. Scratch goes in `tmp/effect-graph-<work-package>/`; selected evidence
  in `.orchestrate/evidence/EFFECT-GRAPH/`.
- **Maturity.** A ledger row for the seam is added at WP3, with implementation, correctness and device results kept
  apart.

## Open questions

| Question | Settled by |
|---|---|
| Which storage formats, counts and view dimensions does Chrome's record allow? | WP1 records them |
| Do these cadence rules serve the first sea and cloud frames? | Their designs' cost accounts; a failure reopens [cadence](#cadence) here |
| Is one shared refraction copy the right source on Apple tile? | WP3 records its cost; `SEA` decides |
| Do the camera, entities, waves and sprites move evenly under a display lead? | Not by WP4. Their owners plan it; see [the display phase](#the-display-phase-wp4) |
| How far does a window's consumed target lie from the platform's, between the two clock reads of the Mac's render actor and the iPhone's render owner? | Not by WP4, which leaves both hosts as they are. Whoever next plans those frame loops measures it, and pairs the two clocks once if it shows |

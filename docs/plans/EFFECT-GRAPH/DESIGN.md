# Effect graph: engine modules run their own passes inside the one frame

11 November 2026. [Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) · [module list](../ARCHITECTURE.md) ·
[the brief](../BRIEF.md). C02, C04, C06, C07 and C12 in [CONTRACTS](../../design/CONTRACTS.md), the
[graph decision](../../design/decisions/G3-EXECUTING-RENDER-GRAPH.md) and
[the port decision](../../design/decisions/SEA-AND-CLOUD-PORT.md) govern.

## The request

The owner's planning task of 11 November 2026, word for word:

> Plan the modules CULLING and EFFECT-GRAPH from docs/plans/ARCHITECTURE.md: write each one's DESIGN, PLAN and
> done-when file. Both depend on APPLE-TILE M2, which is in its second review now, so builders can start them once
> it passes. Everything else is waiting on APPLE-TILE M2 or M3, so these are the next work. Keep each first
> milestone small enough to start right after APPLE-TILE M2 lands. Also: APPLE-PRESENTATION M1's jitter change broke
> APPLE-TILE's exact baseline comparisons, and nothing caught it before it merged
> (docs/evidence/APPLE-TILE/M2-review.md MF1). Say in ARCHITECTURE.md or the affected designs how a module's change
> that alters another module's exact baselines gets caught before landing.

What the module is for comes from [the brief](../BRIEF.md), decision 6: "Sea and clouds are finished as Thrum
modules and are not part of the V1 exit." [The port decision](../../design/decisions/SEA-AND-CLOUD-PORT.md#readiness-6-november)
names the graph condition this module meets: 3D and cube resources, cadence, and async-compute eligibility as a
declared seam. The module list's brief adds the rest: mipped storage arrays, persistent state advanced by
tick-derived time, ping-pong and world-anchored wrapping histories, compute before the fragment passes that read
it, and module passes after opaque geometry that read scene depth and a refraction source.

This design reconciles the code on main; it is not a blind study. Read for it: `graph.rs`, `graph_record.rs`,
`contribution.rs`, `synthetic/{scene_graph,contributed}.rs`, `atmosphere/graph.rs`, the [SEA](../SEA/BRIEF.md) and
[CLOUDS](../CLOUDS/BRIEF.md) briefs, and the designs of the three modules whose files it shares. For the revision
after [the design review](../../evidence/EFFECT-GRAPH/design-review.md), also read: the history scope in
`graph.rs`, the encoded-frame token in `runtime/tokens.rs`, `timing.rs`, and `APPLE-PRESENTATION`'s M2 and M3 items.

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
| **N2 Keep simulated state that advances with the world, not with the display** | 1 to several display frames per tick; hitches of many ticks; discarded and capture frames; pause; a tick that goes backwards; a new timeline | State that advances once per drawn frame, or that resets when a tick repeats. Rejected by comparing GPU state exactly with expectations written from [the state table](#state-time-and-the-tentative-frame-m2) after a scripted mix of those frames |
| **N3 Run a pass less often than every frame, by a runtime value** | Every frame to once in many; changed while running; two histories at different cadences | Recompiling the graph on change, or a skipped pass whose readers get an older side. Rejected by creation counters across a change and by exact readback of a consumer's output on every run and skip boundary |
| **N4 Draw after opaque geometry, from scene depth and the scene behind** | Both native recipes and Chrome; a depth-only effect and a refracting one | A pass that reads last frame's depth or colour. Rejected by a moving occluder: the effect is hidden behind it in the same frame |
| **N5 Feed reconstruction's inputs** | Motion, reactive coverage and reset on cuts, replacement and effect reset | Zero motion on a moving effect. Rejected by reading motion from submitted frames against the known displacement |
| **N6 Survive resize and device replacement** | Mid-run, with state in flight | Reusing an old generation's texture. Rejected by generation checks and exact state after re-seed |
| **N7 Cost nothing when absent** | Every existing scene and variant | A seam that adds an empty pass or a copy. Rejected by identical compiled pass lists and the exact baselines |

This is internal work: its outcomes are properties measured through the diagnostic effect, with inspected items
where a picture is the evidence.

N5 stops at the inputs. This module proves that an effect's motion, reactive coverage and reset reach the targets
reconstruction reads. How an effect looks after reconstruction, with no shimmer or ghost, is accepted in `SEA`'s
and `CLOUDS`' own milestones on `APPLE-PRESENTATION:M3`'s reconstructor; the diagnostic slab is not a product and
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
  specification today; M1 records them from the device's capability record in Chrome and the graph validates
  against the record, never against this paragraph.
- **What would change the selection.** If the cadence rules cannot keep readers defined for some consumer, that
  consumer's pass gets prepared graph variants (as day and night have) and the rule narrows. If a second consumer
  needs a stage this design lacks, the stage list grows here, with its consumers, before anyone builds it.

## Mechanisms

### Resource kinds (M1)

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

### Effects and stages (M2, M3)

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
| `after-opaque` | Render or compute | After opaque geometry, sky and stars; before the pre-tone input | New (M3) |

Compute in `simulate` therefore always precedes the fragment passes that read it, by graph order, not by
convention. All of an effect's pipelines are prepared in `rebuild`, before readiness. Its passes are timed under
their own names.

### State, time and the tentative frame (M2)

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

### Ping-pong histories (M2)

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
frame and need no such rule; this one is derived here, and M2's cadence witness is its proof.

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
invalidation. The governor in `APPLE-PRESENTATION:M7` is the consumer; this module builds no policy.

### After opaque (M3)

An `after-opaque` pass may read scene depth, load and write scene colour, and write motion and reactive coverage
under [the temporal contribution](../APPLE-PRESENTATION/DESIGN.md#seams-for-thrum-core-proposals): motion from
unjittered positions, invalid history marked, the frame's reset flag honoured. Effects in this stage run in list
order. That is an execution order and nothing more: C08 also requires medium segments to be ordered and disjoint,
or to carry an explicit joint or approximate integration policy. Admitting and composing segments stays with
`SEA`, `CLOUDS` and V3 composition, and this foundation builds no integrator.

- **Scene depth and the colour copy are separate requests.** Depth is stored and sampled when any reader declares
  it: an after-opaque pass, such as a volume that refracts nothing and still stops at terrain; reconstruction,
  which `APPLE-PRESENTATION` M2 stores depth for whenever it is enabled; or any other pass that samples it. The
  graph derives storage from the union of readers (C06), so dropping one reader removes storage only when no
  other reader is left.
- **The refraction source** is a copy of scene colour taken after opaque, sky and stars, declared only when an
  effect requests it and shared by all that do. On a tile GPU it ends a render pass, so its cost is recorded; a
  cheaper recipe-specific source is `SEA`'s decision.
- **An absent effect adds nothing:** no pass, no copy, and no storage beyond what the other readers already need.
- **Where it sits.** After the star composite, so a volume that writes no depth still dims the stars behind it;
  before the pre-tone input that `APPLE-TILE:M3` exposes, so reconstruction and tone see the effect.

### Async-compute eligibility

A `simulate` pass may declare itself eligible. The compiler checks that it is compute and touches no attachment or
surface, and the compiled graph lists eligible passes. Nothing is scheduled differently: wgpu exposes no second
queue on Metal or WebGPU. The flag exists so a later recipe can use it without changing a consumer.

### Check that it can work

- **Kinds.** wgpu 30 creates 3D and cube-compatible textures on Metal and WebGPU, and the atmosphere already
  writes a cube through a 2D-array view. M1 proves each kind by exact readback before anything depends on it.
- **A compiled graph with skipped passes.** The soundness argument is the cadence rules; the atmosphere's validity
  bits have run this way since G3s. The static graph declares both sides of each pair, and no frame reads and
  writes one side: `previous` and `next` are different sides, and `result` is read only after the producer.
- **Repeated ticks.** The equal-tick row keeps the state, so a nonempty trail survives the second display frame of
  a tick and a pause of any length. A CPU oracle that shared a wrong rule with the GPU would agree with it, so
  M2's test also asserts the table's rows directly: the field is nonempty and unchanged after repeated ticks and
  a pause, and at its declared reset after a backward tick and after a timeline change.
- **State under discard.** A discarded encoder executes nothing, so GPU state is the last submitted state, and the
  commit point is already `submitted`. The diagnostic's state is integer, so M2 compares it bit for bit after
  every kind of frame.
- **Time-slicing from nothing.** A full frame covers every texel before any sliced frame reads one. M2 reads every
  texel back after the first frame of an epoch and after a reset.
- **Portable limits.** Ping-pong removes the need for read-write float storage. The remaining limits are counts,
  which the compiler checks from the record.

## The system as it runs

| State | Owner |
|---|---|
| Descriptor kinds, validation, allocation, pass order, load and store | `thrum-render::graph` (`APPLE-TILE` owns the split and the storage derivation; this module extends the descriptor) |
| The contribution list, stages, the tentative-commit protocol, the anchor, knobs | This module (`contribution.rs`, `synthetic/{contributed,scene_graph}.rs`, the frame host's list) |
| An effect's resources, epoch, state tick and shaders | The effect's module (`SEA`, `CLOUDS`) |
| The view record, motion and reactive targets, history acceptance | `APPLE-PRESENTATION` |
| The pre-tone input and pass-name timing | `APPLE-TILE:M3` |
| Environment products an effect lights with | `ATMOSPHERE:M5` |

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
mipped array at a runtime cadence; a cube; and in M3 a slab drawn after opaque that samples them, refracts the
scene behind it and reports motion.

## Shared files and exact baselines

`graph.rs`, `scene_graph.rs`, `contribution.rs` and the frame host are also edited by `APPLE-TILE` M3 (pre-tone
input, pass-name timing), `APPLE-PRESENTATION` M2 (the third forward target, history protocol), `CULLING` and
`CREATOR` M2 (the timing-demand switch). M1 stays inside the graph's descriptor, validation and allocation. M2 and
M3 integrate sequentially with whatever has landed and rerun the affected witnesses of each. Once the
timing-demand switch exists, effect timing is not gathered while it is off.

[The module list](../ARCHITECTURE.md#exact-baselines) says how a change that alters another module's exact images
is caught. This module must move none, on the Mac or in Chrome: with no effect attached, every existing scene
compiles to the same passes and storage and draws the same bytes. Once `BASELINE-GUARD` M1 is Done, the proof gate
in `./dev check` refuses any landing whose exact content has not passed the verifier; M2 and M3 wait for it and
carry the verifier as their claim-time check. M1 may be built before the gate exists: it carries the native
matrix as an automated check, its builder reruns that check after every rebase and before `land`, and Chrome's
baseline is first rechecked when `BASELINE-GUARD`'s builder runs the verifier on main.

## Cost account

- **Absent:** no pass, no resource, no copy, no per-frame call beyond iterating an empty list.
- **Per effect and frame:** one callback per declared pass; skipped passes cost a branch.
- **Stored depth and the refraction copy:** only on request. The copy is one scene-extent colour copy and a render
  pass break. The earlier culling session's harness put an empty second pass that loads and stores colour, motion
  and depth at about 0.07 ms at 720p on this Mac, contended; its logs are not committed, so M3 measures it.
- **Memory:** an effect's own resources, reported per effect from their descriptors.

Recorded as tripwires, never gates: per-pass GPU time by name, bytes per effect, and about 25% paired regression
on the valley route with no effect attached.

## Risks

1. **The cadence rules are too strict for a real consumer** (a time-sliced cloud march that wants a transient, or
   a partial write to a history). Early sign: `CLOUDS`'s cost account. Fallback: prepared variants for that pass,
   or a copy-forward history write, designed here before anyone builds it.
2. **Chrome refuses a kind or count the Mac accepts.** Early sign: M1's recorded limits; M2's Chrome run. The
   compiler's record check turns it into a preparation error, not a lost frame.
3. **The seam grows into a framework.** Each mechanism names its consumer above; a mechanism without one is cut.
4. **A graph change moves an exact image.** The proof gate refuses it at every landing once it exists; until then
   M1's native matrix is the check.
5. **A tile GPU pays for the pass break.** Recorded in M3; `SEA` and `CLOUDS` choose their recipes from it.
6. **A consumer needs true sub-steps in one frame**, such as a stability-limited solver after a hitch. Early sign:
   `SEA`'s design. Fallback: a declared, fixed number of sub-step passes, designed here before anyone builds it.

## Proof and builder handoff

- **Targets.** `mac-metal` on both recipes, and `chrome-webgpu` through `scripts/check_v1_browser.mjs`, which needs
  a browser launch the Codex sandbox grants per run, as the existing cases do. `ios-sim` is not a target here.
- **Capture.** The inspected items use T2 engine capture and name their mode: `realtime` for the trail in M2,
  `fixed` stills for the slab in M3. The stills prove composition. No item here judges reconstructed motion.
- **Timing.** M2 reads per-pass GPU time from the executor's named pass timings (`PassTiming` in `timing.rs`),
  which exist today, in its own test or bench. It does not change the host's frame-sample fields, which
  `APPLE-TILE:M3` moves to pass names. M3 here needs that milestone and reports a skipped pass through its
  schema.
- **GPU.** Every GPU check holds the `gpu` lock. Scratch goes in `tmp/effect-graph-<milestone>/`; selected evidence
  in `docs/evidence/EFFECT-GRAPH/`.
- **Maturity.** A ledger row for the seam is added at M3, with implementation, correctness and device results kept
  apart.

## Open questions

| Question | Settled by |
|---|---|
| Which storage formats, counts and view dimensions does Chrome's record allow? | M1 records them |
| Do these cadence rules serve the first sea and cloud frames? | Their designs' cost accounts; a failure reopens [cadence](#cadence) here |
| Is one shared refraction copy the right source on Apple tile? | M3 records its cost; `SEA` decides |
